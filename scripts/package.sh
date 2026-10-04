#!/usr/bin/env bash
# Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
# Proprietary software; see LICENSE.

# Packages a built Sceneuron library into the OBS 32 user-plugin bundle for macOS
# and zips it. Runs on macOS (bash 3.2+).
set -euo pipefail

readonly PLUGIN_NAME="sceneuron"
readonly DISPLAY_NAME="Sceneuron"
readonly BUNDLE_ID="ai.chaski.sceneuron.obs"
readonly DESCRIPTION="AI-driven, editable live overlays that react to your camera."
readonly REPOSITORY_URL="https://github.com/alexpicon/sceneuron"
readonly MACOS_MIN_VERSION="13.0"
# Semantic Versioning 2.0.0 as a POSIX extended regular expression. Accepted
# versions contain no characters that need escaping in JSON or XML.
readonly SEMVER_RE='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-(0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)(\.(0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'
readonly ARCH_RE='^[A-Za-z0-9_]+$'

usage() {
    cat <<'EOF'
Usage: scripts/package.sh --lib PATH --os macos --version SEMVER
                          --notices FILE [--arch NAME] --out DIR

Packages a built Sceneuron library into the OBS 32 user-plugin bundle and zips it.

  --lib PATH      The built cdylib, libsceneuron.dylib.
  --os OS         Target OS: macos. Sceneuron supports macOS only for now.
  --version VER   Plugin version, a Semantic Versioning 2.0.0 string such as 0.1.0.
  --notices FILE  Third-party license notices for the library, written by
                  `cargo run -p sceneuron-notices`; shipped as THIRD-PARTY-NOTICES.txt.
  --arch NAME     Architecture label for the manifest and zip name, such as arm64,
                  x86_64, or universal (letters, digits, and underscores).
                  Defaults to the host CPU reported by uname -m.
  --out DIR       Output directory, created when missing. A previous layout and zip
                  of the same name in DIR are replaced.
  -h, --help      Show this help.

Produces DIR/sceneuron.plugin and DIR/sceneuron-VERSION-macos-ARCH.zip, and
prints the zip path.
EOF
}

die() {
    printf 'package.sh: error: %s\n' "$1" >&2
    exit 1
}

usage_error() {
    printf 'package.sh: error: %s\n' "$1" >&2
    printf "Run 'scripts/package.sh --help' for usage.\n" >&2
    exit 2
}

warn() {
    printf 'package.sh: warning: %s\n' "$1" >&2
}

has() {
    command -v "$1" >/dev/null 2>&1
}

parse_args() {
    while (($# > 0)); do
        case "$1" in
            -h | --help)
                usage
                exit 0
                ;;
            --*=*)
                set -- "${1%%=*}" "${1#*=}" "${@:2}"
                continue
                ;;
            --lib | --os | --version | --notices | --arch | --out)
                if (($# < 2)) || [[ "$2" == --* ]]; then
                    usage_error "$1 needs a value"
                fi
                case "$1" in
                    --lib) lib=$2 ;;
                    --os) os=$2 ;;
                    --version) version=$2 ;;
                    --notices) notices=$2 ;;
                    --arch) arch=$2 ;;
                    --out) out=$2 ;;
                esac
                shift 2
                ;;
            *) usage_error "unknown argument: $1" ;;
        esac
    done
}

validate_args() {
    local name
    for name in lib os version notices out; do
        [[ -n "${!name}" ]] || usage_error "missing --$name"
    done
    case "$os" in
        macos) ;;
        linux | windows) usage_error "Sceneuron supports macOS only for now; Linux and Windows are planned" ;;
        *) usage_error "--os must be macos, not '$os'" ;;
    esac
    [[ "$version" =~ $SEMVER_RE ]] || usage_error "--version '$version' is not a semantic version such as 0.1.0"
    [[ -n "$arch" ]] || arch=$(uname -m)
    [[ "$arch" =~ $ARCH_RE ]] || usage_error "--arch '$arch' may contain only letters, digits, and underscores"
    # Keeps a path that starts with a hyphen from being read as an option.
    [[ "$lib" != -* ]] || lib="./$lib"
    [[ "$out" != -* ]] || out="./$out"
    [[ "$notices" != -* ]] || notices="./$notices"
    [[ -f "$lib" ]] || die "library not found: $lib"
    [[ -s "$lib" ]] || die "library is empty: $lib"
    [[ -s "$notices" ]] || die "notices file not found or empty: $notices"
    [[ ! -e "$out" || -d "$out" ]] || die "--out exists and is not a directory: $out"
}

# The first bytes identify the binary format, which catches a library built for
# another OS.
check_binary_format() {
    local magic
    magic=$(od -A n -t x1 -N 4 "$lib" | tr -d ' \n')
    case "$magic" in
        cffaedfe | cafebabe | cafebabf) ;;
        *) die "$lib is not a macOS shared library (first bytes: ${magic:-none})" ;;
    esac
}

check_macos_arch() {
    if ! has lipo; then
        warn "lipo not found; not checking that $lib holds the '$arch' architecture"
        return
    fi
    local expected="$arch" actual
    [[ "$arch" != universal ]] || expected="arm64 x86_64"
    actual=$(lipo -archs "$lib" | tr ' ' '\n' | sort | xargs)
    [[ "$actual" == "$expected" ]] || die "$lib holds '$actual' but --arch '$arch' needs '$expected'"
}

write_manifest() {
    cat >"$1" <<EOF
{
  "id": "$PLUGIN_NAME",
  "name": "$PLUGIN_NAME",
  "display_name": "$DISPLAY_NAME",
  "version": "$version",
  "os_arch": "$os-$arch",
  "description": "$DESCRIPTION",
  "urls": {
    "repository": "$REPOSITORY_URL"
  },
  "has_banner": false,
  "has_icon": false
}
EOF
}

write_info_plist() {
    cat >"$1" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key><string>en</string>
	<key>CFBundleDisplayName</key><string>$DISPLAY_NAME</string>
	<key>CFBundleExecutable</key><string>$PLUGIN_NAME</string>
	<key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleName</key><string>$PLUGIN_NAME</string>
	<key>CFBundlePackageType</key><string>BNDL</string>
	<key>CFBundleShortVersionString</key><string>$version</string>
	<key>CFBundleSupportedPlatforms</key><array><string>MacOSX</string></array>
	<key>CFBundleVersion</key><string>$version</string>
	<key>LSMinimumSystemVersion</key><string>$MACOS_MIN_VERSION</string>
</dict>
</plist>
EOF
    if has plutil; then
        plutil -lint -s "$1" || die "generated an invalid Info.plist: $1"
    fi
}

sign_bundle() {
    if ! has codesign; then
        warn "codesign not found; leaving $1 unsigned"
        return
    fi
    codesign --force --sign - "$1" || die "could not sign $1"
    codesign --verify --strict "$1" || die "the signature of $1 does not verify"
}

layout_macos() {
    local bundle="$out/$PLUGIN_NAME.plugin"
    rm -rf "$bundle"
    mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
    cp "$lib" "$bundle/Contents/MacOS/$PLUGIN_NAME"
    chmod 755 "$bundle/Contents/MacOS/$PLUGIN_NAME"
    write_info_plist "$bundle/Contents/Info.plist"
    write_manifest "$bundle/Contents/Resources/manifest.json"
    cp "$notices" "$bundle/Contents/Resources/THIRD-PARTY-NOTICES.txt"
    sign_bundle "$bundle"
}

# Archives $out/$1 into $out/$2 with $1 as the single top-level entry, by running
# zip inside $out on relative names.
make_zip() {
    local root=$1 archive=$2
    rm -f "$out/$archive"
    has zip || die "zip not found"
    (cd "$out" && zip -q -r -X "$archive" "$root")
    [[ -s "$out/$archive" ]] || die "zipping produced no archive at $out/$archive"
}

main() {
    lib="" os="" version="" notices="" arch="" out=""
    parse_args "$@"
    validate_args
    check_binary_format
    check_macos_arch
    mkdir -p "$out"
    local archive="$PLUGIN_NAME-$version-$os-$arch.zip"
    layout_macos
    make_zip "$PLUGIN_NAME.plugin" "$archive"
    printf 'package.sh: packaged %s %s for %s-%s\n' "$DISPLAY_NAME" "$version" "$os" "$arch" >&2
    printf '%s\n' "$out/$archive"
}

main "$@"
