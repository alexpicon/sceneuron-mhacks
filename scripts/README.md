# Scripts

## package.sh

`package.sh` turns a built Sceneuron library into the OBS 32 user-plugin bundle
for macOS and zips it. It runs on macOS.

Sceneuron is Mac-only for now, by the owner's decision of 2026-10-03; Windows
and Linux are planned, coming very soon. Asked for `--os linux` or
`--os windows`, the script stops with a usage error that says so. The last
`main` that packaged for Linux and Windows is the pushed tag
`archive/linux-windows-2026-10-03` (commit `74d062e`).

Sceneuron is proprietary software of Alexander Picon and Chaski AI; see
[LICENSE](../LICENSE). The plugin links libobs, which is GPL-2.0-or-later, so
its package is for Chaski AI's own use and demos, and it is not distributed
until the plugin is restructured into a thin open OBS bridge and a proprietary
app that talk over a local socket, an item in [the backlog](../docs/backlog.md).

The Stream Deck plugin is packaged on its own, with `npm run package` in
`streamdeck/`; see [streamdeck/README.md](../streamdeck/README.md).

```text
scripts/package.sh --lib PATH --os macos --version SEMVER --notices FILE [--arch NAME] --out DIR
```

| Option | Meaning |
| --- | --- |
| `--lib` | The built cdylib, `libsceneuron.dylib`. Its first bytes must mark a macOS library, of one architecture or universal. |
| `--os` | Target OS: `macos`, the only one for now. |
| `--version` | A Semantic Versioning 2.0.0 version such as `0.1.0`. |
| `--notices` | Third-party license notices for the library, shipped as `THIRD-PARTY-NOTICES.txt`. Write them with `cargo run -p sceneuron-notices -- --target TRIPLE --out FILE`, naming every target triple the library was built for. |
| `--arch` | Label used in the manifest's `os_arch` and the zip name, such as `arm64`, `x86_64`, or `universal`. Defaults to the host CPU. The library must hold exactly that architecture (`universal` means `arm64` and `x86_64`). |
| `--out` | Output directory. A previous bundle and zip with the same names are replaced. |

The script prints the zip path on standard output. It exits with status 2 for a
usage error and with another non-zero status for any other failure.

### Output

The script writes the bundle `sceneuron.plugin` to `--out`, ad-hoc signed:

- `Contents/Info.plist`, with the bundle identifier `ai.chaski.sceneuron.obs`
- `Contents/MacOS/sceneuron`, the library
- `Contents/Resources/manifest.json`
- `Contents/Resources/THIRD-PARTY-NOTICES.txt`

It also zips the bundle to `sceneuron-VERSION-macos-ARCH.zip`, with
`sceneuron.plugin` at the root of the archive. `manifest.json` carries the
metadata that the OBS 32 plugin manager reads. `THIRD-PARTY-NOTICES.txt` holds
the license texts of the face model and every crate linked into the plugin for
the named targets, as their licenses require; Sceneuron's own crates, which are
proprietary, are not listed.

`cargo run -p sceneuron-notices -- --check --target TRIPLE...` writes nothing
and instead fails when a crate that ships has a license outside the allowed
list, or none it can read. Run it locally before committing, for the two macOS
targets, `aarch64-apple-darwin` and `x86_64-apple-darwin`; the
[license gate](../docs/development.md#license-gate) lists the allowed licenses
and why.

The bundle is signed with `codesign --force --sign -`, and the signature is
verified. When `codesign` or `lipo` is missing, the script warns and skips
signing or the architecture check. Zipping needs `zip`.

### Package locally

Read the version from the crate rather than typing it:

```sh
pkgid=$(cargo pkgid -p sceneuron-obs)
version=${pkgid##*[#@]}
cargo build --release --locked -p sceneuron-obs
```

Then write the notices and package:

```sh
# Apple silicon, native architecture
cargo run --locked -p sceneuron-notices -- --target aarch64-apple-darwin --out notices.txt
scripts/package.sh --lib target/release/libsceneuron.dylib --os macos --version "$version" \
  --notices notices.txt --out dist
```

A universal build needs the other architecture's standard library, which
rustup provides (`rustup target add x86_64-apple-darwin`). Run this on an Apple
silicon Mac:

```sh
cargo build --release --locked -p sceneuron-obs --target x86_64-apple-darwin
mkdir -p target/universal
lipo -create -output target/universal/libsceneuron.dylib \
  target/release/libsceneuron.dylib \
  target/x86_64-apple-darwin/release/libsceneuron.dylib
cargo run --locked -p sceneuron-notices -- --target aarch64-apple-darwin --target x86_64-apple-darwin \
  --out notices.txt
scripts/package.sh --lib target/universal/libsceneuron.dylib --os macos --arch universal \
  --version "$version" --notices notices.txt --out dist
```

Set `MACOSX_DEPLOYMENT_TARGET=13.0` for both builds to match the bundle's
`LSMinimumSystemVersion`.

## Install into OBS

Quit OBS first: it loads plugins only at startup. Then extract the zip into
OBS's user plugin folder, `~/Library/Application Support/obs-studio/plugins/`,
so that the bundle sits directly inside it as `plugins/sceneuron.plugin`:

```sh
mkdir -p ~/Library/Application\ Support/obs-studio/plugins
ditto -x -k sceneuron-0.1.0-macos-universal.zip ~/Library/Application\ Support/obs-studio/plugins
```

The bundle is ad-hoc signed, not notarized. If the zip was downloaded with a
browser, macOS quarantines the extracted files and can refuse to load the
plugin. Clear the quarantine flag on the installed bundle only:

```sh
xattr -dr com.apple.quarantine ~/Library/Application\ Support/obs-studio/plugins/sceneuron.plugin
```

Start OBS and open Tools > Plugin Manager: Sceneuron should be listed. If it
is not, look for `sceneuron` in the current log (Help > Log Files > View
Current Log).

To uninstall, quit OBS and delete `sceneuron.plugin` from the plugin folder.

## copyright.mjs

Every tracked source and text file starts with the two-line copyright header, in the comment
syntax of its type and after any shebang, XML declaration, or doctype line:

```text
Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
Proprietary software; see LICENSE.
```

`node scripts/copyright.mjs --check` lists the tracked files that lack it and exits with status 1
when any file needs attention; run it locally before committing. `node scripts/copyright.mjs --fix`
adds the missing headers, and leaves a header that is incomplete or not at the top for a person to
fix. Every tracked file needs a rule in the script: a comment style for its type, or an exclusion
with the reason, so a file of a new type fails the check until it gets one. Excluded are `LICENSE`
itself, the folder of the third-party UltraFace model, the overlay's third-party fonts and their
license texts in `web/fonts/`, the AI prompts, which go to the model verbatim, lock files, JSON,
which has no comments, binary files, and Markdown, which the checker does not cover yet; Sceneuron's
own Markdown documents end with the notice as their last line instead, except the two READMEs that
describe third-party material: the UltraFace model, and the test frames made from a public-domain
photo.

## SpacetimeDB for the shared room

`spacetime-install.sh` installs what the shared planning poker room needs on a Mac, skipping what
is already there: the spacetime CLI through SpacetimeDB's official installer, SpacetimeDB 2.10.2
as the current version, and Rust's `wasm32-unknown-unknown` target. It needs rustup.

`spacetime.sh` runs the local server in the background and publishes `room/module` to it as
`sceneuron-room`, the database the editor's Room section is prefilled with:

```bash
scripts/spacetime-install.sh
scripts/spacetime.sh start
```

`restart`, `stop`, `status`, `publish` (after changing the module), and `log` do what they say.
The server keeps its data across restarts. Its pid and log are in
`~/.local/share/spacetime/sceneuron/`. `SPACETIME_LISTEN=0.0.0.0:3000` lets other computers on a
trusted network join; the default listens on 127.0.0.1:3000 only.

Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved. Proprietary software; see [LICENSE](../LICENSE).
