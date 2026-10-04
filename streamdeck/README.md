# Sceneuron for Stream Deck

A Stream Deck plugin for Sceneuron. Its keys raise Sceneuron cards (meeting signals, planning
poker, classroom and status cards), lower them, show or hide overlay elements, pause automation,
hold push to talk, and reveal the votes or start the next round in a shared planning poker room.
It comes with a ready-made profile for the 15-key Stream Deck, and a copy of it for the Stream Deck
XL, and the editor pairs it in one click on the same computer. On a Stream Deck + a dial steps
through a deck, showing each card as it turns. Each card key is drawn in the Graphite look of
Sceneuron's overlay: the card's line icon and a short label on a graphite key, or the card's own
words for a poker value, solid amber while that card is up. Keys stay current through one event
stream from Sceneuron, whether a card was raised from the Stream Deck, the
editor, a hotkey, the camera, or voice. Elgato's own OBS plugin for Stream Deck cannot send
obs-websocket vendor requests, which is how other tools raise Sceneuron cards, so Sceneuron has
this plugin of its own.

The plugin talks only to Sceneuron on this computer (`127.0.0.1`), with a control token that
Sceneuron takes only in a header and only on an explicit list of routes: cards, showing and hiding
elements, releasing manual changes, effects, emoji floods, automation, push to talk, and a shared
room's reveal and next round. Any other route refuses it. The plugin uses every one of them but
releasing manual changes, effects, and emoji floods. It sees deck, card, and element names, each
card's text as written (a placeholder such as `{{scene}}` is not filled in) and colors, which cards
are up, whether the camera or voice raised them, whether a shared room covers them, and when they
lower by themselves, which elements show or hold a manual change, and whether automation is paused.
It cannot change the project, see the camera, read the room's people or votes, or read the AI
settings, what the AI sees or hears, or the recent events. Holding push to talk never turns voice
control on or changes consent: audio goes anywhere only after voice control is enabled and consent
is granted in the editor.

The plugin is part of Sceneuron, proprietary software of Alexander Picon and Chaski AI (see
[LICENSE](../LICENSE)): `package.json` declares it `UNLICENSED` and private, and the manifest names
Chaski AI as its author. The npm packages bundled into it keep their own licenses, which the build
lists in `bin/third-party-notices.txt`.

## Requirements

- Stream Deck 7.1 or newer, on macOS 13 or newer. Sceneuron is Mac-only for now, so the manifest
  offers the plugin on macOS only; Windows and Linux are planned, coming very soon. Stream Deck
  downloads the Node.js 24 runtime the plugin asks for.
- Sceneuron with the control API: the OBS plugin, or `sceneuron-dev` for trying it without OBS.
- To build: Node.js 24 or newer and npm.

## Build

```bash
cd streamdeck
npm ci
npm run build      # writes bin/plugin.js and the profiles in ai.chaski.sceneuron.sdPlugin/
npm test           # builds, then runs every test
npm run check      # type-checks with tsc
npm run validate   # Stream Deck's own checks of the manifest and files (after a build)
npm run package    # builds and writes dist/ai.chaski.sceneuron.streamDeckPlugin
```

`.npmrc` saves exact versions, skips package install scripts, and refuses releases younger than a
day. `package-lock.json` fixes every dependency.

The tests need no Stream Deck and no Sceneuron. `tests/fake-sceneuron.mjs` serves the control API
on `127.0.0.1` the way Sceneuron does, and `tests/fake-stream-deck.mjs` starts the built plugin the
way Stream Deck starts it, each run in its own folder under `.test-runs/`, and plays the app's side
of the WebSocket. Test files follow the sources: `tests/actions/` drives each action through the
built plugin, `tests/profiles/` reads the profiles the build wrote, and `tests/ui/` runs the
inspector page; `tests/deep-link.test.mjs` pairs the built plugin from the editor's link. There are
206 tests; run them on Node 24 with the type check and `npm run validate` before committing.

`e2e/streamdeck-live.mjs` runs the built plugin against a real Sceneuron (`sceneuron-dev`) under
the same fake Stream Deck: it pairs, proves the server, follows the event stream, drives every
action, holds push to talk, reveals the votes and starts the next round in a room on a stand-in
SpacetimeDB (`e2e/room-stand-in.mjs`), and checks that a revoke shows "Pair again"; then a fresh
plugin pairs from the editor's link alone, and links that are replayed, wrong, or malformed pair
nothing. It has 21 steps, passed 21 of 21 on 2026-10-03. It builds nothing; see
[e2e/README.md](../e2e/README.md) for how to run it.

On 2026-10-02 the plugin was linked into the Stream Deck app on macOS, which fetched the Node.js 24
runtime and started it, and the app's log says the plugin connected. On 2026-10-03 the plugin
installed in the Stream Deck app on a Stream Deck XL, its profile installed, and the plugin's log
shows that it drew every key. Not yet checked: a key or dial pressed while paired, and the profile
for the 15-key Stream Deck on a 15-key device.

## Install

The plugin is version 0.4.0 (`0.4.0.0` in the manifest). Get the package: build it with
`npm ci && npm run package` in this folder, which writes
`dist/ai.chaski.sceneuron.streamDeckPlugin`. Double-click the `.streamDeckPlugin` file; Stream Deck
asks to install it, installs the ready-made profile for your device ("Sceneuron" on a 15-key Stream
Deck, "Sceneuron (15-key layout)" on an XL), and switches to it, so every key is already set. Then
pair once, as below. The actions also appear under Sceneuron in the actions list, for other devices
or layouts of your own.

For development, link the folder instead so each build is picked up:

```bash
npx streamdeck link ai.chaski.sceneuron.sdPlugin   # symlink into Stream Deck's plugins folder
npx streamdeck dev                                 # developer mode, for the inspector's debugger
npm run watch                                      # rebuild and restart the plugin on each change
```

`npx streamdeck unlink ai.chaski.sceneuron` removes the link. While linked, the plugin's log is in
`ai.chaski.sceneuron.sdPlugin/logs/`.

## Ready-made profiles

The plugin brings a profile for the 15-key Stream Deck (five columns, three rows), and a copy of it
for the Stream Deck XL that puts the same keys in the XL's top-left five by three and leaves every
other key empty, so the 15-key layout can be tried on an XL. The manifest's `Profiles` names them
for device types 0 (Stream Deck) and 2 (Stream Deck XL); they show in Stream Deck as "Sceneuron"
and "Sceneuron (15-key layout)". Stream Deck installs them with the plugin and switches the device
to its profile.

The three pages' bottom rows match where it counts: the corners go to the previous and next page,
and push to talk is second.

Page 1, Meeting:

| | 1 | 2 | 3 | 4 | 5 |
| --- | --- | --- | --- | --- | --- |
| Row 1 | Wrap it up | We're off track | I have a question | Let's take this offline | Next topic |
| Row 2 | Agree | Disagree | Can't hear you | I'm on mute | Back in 5 |
| Row 3 | Go to page 3 | Push to talk | Lower cards (Meeting signals) | Pause automation | Go to page 2 |

Page 2, Planning poker:

| | 1 | 2 | 3 | 4 | 5 |
| --- | --- | --- | --- | --- | --- |
| Row 1 | 0 | 1 | 2 | 3 | 5 |
| Row 2 | 8 | 13 | 21 | ? | Coffee break |
| Row 3 | Go to page 1 | Push to talk | Reveal votes | Next round | Go to page 3 |

Page 3, Status:

| | 1 | 2 | 3 | 4 | 5 |
| --- | --- | --- | --- | --- | --- |
| Row 1 | I'm on mute | Back in 5 | At lunch | Focus time | Presenting |
| Row 2 | On another call | Lower cards (Status) | | | |
| Row 3 | Go to page 2 | Push to talk | | Pause automation | Go to page 1 |

Before it is paired, and when Sceneuron is out of reach, each card key still shows its card: the
plugin carries the built-in packs' cards (`src/lib/built-in-cards.ts`), with the icon and short
label each shows (`web/icons.js`, shared with the overlay and bundled by the build), and a
caption says why a press cannot work. `crates/sceneuron-server/tests/built_in_cards.rs` fails when
that file differs from what Sceneuron's control view sends for the packs, and run with
`SCENEURON_WRITE_BUILT_IN_CARDS=1` it writes the file again. Push to talk, Lower, Pause automation,
Reveal votes, and Next round each have their own icon, and their images in Stream
Deck's actions list show the same icons. When the plugin starts, its keys wait until it has read
the saved pairing before they draw, so a paired Stream Deck never shows "Not paired" on the way.

The card keys need the Meeting signals, Planning poker, and Status packs in the project (the
editor's **Cards** tab), and the room keys a room joined there. Go to Page is Stream Deck's own
page action. Pair once, as below, and every key is live.

`src/profiles/layout.ts` holds the layout, and every `npm run build` writes both
`ai.chaski.sceneuron.sdPlugin/profiles/*.streamDeckProfile` from it, so no profile is kept by hand;
`tests/profiles/` checks each key of both. Elgato documents bundling a profile exported from the
Stream Deck app, but not the file's format. The build writes the format of the default profiles
the Stream Deck app ships for these two devices (profile format 2.0: a ZIP of one `.sdProfile`
folder), which Stream Deck 7.1 and newer import. Elgato says profiles it distributes are not
updated on a device once installed, so to try a changed layout, delete the old profile in Stream
Deck's preferences, then install the plugin again.

To try the 15-key layout on a Stream Deck XL: `npm run package`, double-click
`dist/ai.chaski.sceneuron.streamDeckPlugin`, and choose "Sceneuron (15-key layout)" for the XL if
Stream Deck has not switched to it.

## Pair

Pairing trades a six-digit code from the editor, which works once, for two minutes, for the
control token. Every Sceneuron key shares that connection.

Sceneuron pairs this plugin by itself when it is installed (its folder is in
`~/Library/Application Support/com.elgato.StreamDeck/Plugins/`) and no Stream Deck is paired, that
is, no control surface traded a code for the control token or made a request with it since OBS
started. OBS sends the pairing link below, in the background, 40 seconds after it starts, so a
Stream Deck paired before reconnects first (it tries again at most 30 seconds apart), and at most
once a run; it logs what it did, never the code. **Set up OBS for me** in the editor's **Camera &
mic** tab sends it at once. A Stream Deck that was paired but did not reconnect in time, such as
one whose app was not running, gets the link too, and the Stream Deck app may start for it; pairing
again hands it the same token.

In one click, with the editor open in a browser on the computer with the Stream Deck:

1. Start OBS with Sceneuron, or `cargo run -p sceneuron-dev`, and the Stream Deck app with this
   plugin.
2. In the Sceneuron editor's **Cards** tab, choose **Pair the Stream Deck on this computer**. The
   editor asks for a fresh code and sends it to the plugin in a deep link; your browser may ask
   first whether to open Elgato Stream Deck. The editor asks Sceneuron once a second whether the
   code was traded, and says "Paired" once it was. **Send it again** opens the same link while the
   code works, and the code shows with the steps to type it, as below. If the code expires first,
   the editor says to check that the Stream Deck app, 7.1 or newer, is running with the plugin.

In an OBS Custom Browser Dock the editor leaves that button out and says to type the code instead:
OBS's browser does not hand such a link to the operating system, and following it would put OBS's
error page in place of the editor.

By typing the code:

1. Start OBS with Sceneuron, or `cargo run -p sceneuron-dev`.
2. Drag any Sceneuron action onto a key and select it. Its settings say it is not paired.
3. In the Sceneuron editor's **Cards** tab, choose **Connect a Stream Deck**. The editor shows the
   code.
4. In the key's settings, enter the port (4466 unless you changed it; it is the number after
   `127.0.0.1:` in the editor's address) and the code, then choose **Pair**.

The link has the editor's own port and the code:

```text
streamdeck://plugins/message/ai.chaski.sceneuron/pair?port=4466&code=123456&streamdeck=hidden
```

Stream Deck routes it to the plugin whose UUID follows `streamdeck://plugins/message/`, and
`streamdeck=hidden` makes it a passive deep link, which leaves the Stream Deck window where it is
(Stream Deck 7.0 and newer), so the editor stays in front to say when the plugin paired. The plugin
reads the rest strictly: the path `/pair`, exactly one `port`, written in digits from 1 to 65535,
and exactly one `code` of six digits. It ignores any other link, with a log line that leaves the
link out. A link pairs exactly as a typed code does: the same exchange, the same hello proof before
the token is kept, and the same global settings, so every key goes live at once. A used, expired,
or wrong code, or a port where nothing answers, pairs nothing and keeps the pairing the plugin has;
a key whose settings are open shows the result. A link that comes while the plugin starts waits
until the saved pairing is read, so that older pairing cannot undo it. Neither the code nor the
token is logged, and the log line leaves the code out even when Sceneuron's answer repeats it.

Sceneuron refuses to trade a code with a web page, and five wrong tries use it up. Any program or
web page on this computer can open a pairing link, though, and the plugin then tries its code as it
would a typed one: wrong codes from links count toward those five tries, so a page could use up a
waiting code, but it never sees the token.

The token lives in Stream Deck's global settings for this plugin, which Stream Deck keeps on this
computer; exported profiles carry only the ids of the deck, card, or element each key controls.
Before the plugin sends the token, after every start and reconnect, Sceneuron must prove it holds
that token for a fresh random nonce on that port. If OBS quits and some other program takes the
port, it learns nothing.

To disconnect, choose **Disconnect Stream Decks** in the editor's **Cards** tab, which revokes the
token for every Stream Deck, or **Forget this Sceneuron** in a key's settings, which deletes it
from this one.
Revoking ends the plugin's event stream, so its keys show "Pair again" right away.

## Use

| Action | Settings | Press | The key shows |
| --- | --- | --- | --- |
| Raise a card | Deck and card | Raises the card, or lowers it when it is up | The card's icon and label, or its words; amber while it is up, "Camera" when the camera raised it, and "Voted" while a shared room hides it on the overlay |
| Lower cards | A deck, or all decks | Lowers the card that is up | Amber while a card is up |
| Show or hide | Element | Shows or hides the element and holds that until it is released | The element's name, "Shown" (amber) or "Hidden", and "Held" while a manual change holds it |
| Pause automation | None | Pauses or resumes rules, camera signals, and voice trigger phrases | "On", or "Paused" in amber |
| Card dial (Stream Deck +) | Deck | Each tick of a turn shows the next or previous card, counting from the card on the strip even while the deck is lowered; push or tap to lower the card or raise it again; touch and hold to lower the deck | The card that is up, or unframed the one it last showed, and its place in the deck, amber while it is up |
| Push to talk | None | Hold while you speak to voice control; letting go, or the key leaving the Stream Deck while held (its page changes, its device is unplugged), lets go | "Talking", in amber, while push to talk is held: as Sceneuron says when it shares voice control's state, else while the key is held; "Voice on" or "Voice off" when Sceneuron says which |
| Reveal votes | None | Shows everyone's votes at once in the shared room | "No room" for 4 seconds after a press found no room; how many voted ("2/5 voted"), or "Revealed", when Sceneuron shares the room's counts |
| Next round | None | Lowers every card in the shared room and clears the votes | As Reveal votes |

The dial steps the deck the same way the editor and hotkeys do, so everyone in the meeting sees
each card it passes. For a private planning poker vote, give each value its own Raise a card key.

Push to talk works like the editor's **Hold to talk**: voice control hears while the key is held,
and only while voice control is on and consented to in the editor. Sceneuron lets go by itself a
minute after the last hold, so the plugin sends the hold again every 30 seconds while the key is
held. A let-go that fails is tried again after 1, 2, and 4 seconds while the plugin stays
connected, and again whenever it connects after a drop; a hold that failed after Sceneuron may
have taken it, such as one that timed out, is let go the same way. A card raised by voice looks
like one raised by hand.

Sceneuron's control view does not carry voice control's state or the room's counts yet. The plugin
reads them when it does (`voice: {enabled, talking}` and `room: {joined, revealed, voted, people}`);
until then the push to talk key lights only while held on this Stream Deck, and the room keys
learn that there is no room from a press that finds none, which the room refuses and which changes
nothing. They say so for 4 seconds, since nothing tells the plugin when a room is joined. A press
of a room key waits up to 12 seconds for an answer, longer than Sceneuron's 10 second wait for the
room's server, so a slow room does not show the alert for an action that went through.

A darkened key says why it cannot act:

- **Offline**: Sceneuron is not answering. The plugin reconnects by itself, waiting from half a
  second up to 30 seconds between tries.
- **Not paired**: pair as above.
- **Pair again**: Sceneuron no longer accepts this Stream Deck. Its token was revoked with
  **Disconnect Stream Decks**, or Sceneuron moved to another port (OBS does that when its port is
  taken). This does not fix itself: pair again as above, with the port the editor's address shows.
- **Not found**: the deck, card, or element was deleted or renamed in the editor. Choose another in
  the key's settings.
- **No room**: this Sceneuron is in no shared room. Create or join one in the editor's **Cards**
  tab.
- **Voice off**: voice control is off in the editor, so push to talk holds nothing.

Show or hide holds the element the way the editor's manual changes do, so rules cannot change it
until you release it in the editor or with the "Release manual overrides" hotkey.

## How it works

| Path | Contents |
| --- | --- |
| `src/plugin.ts` | Entry point: registers the actions, connects to Stream Deck, and reads the saved pairing. |
| `src/board.ts`, `src/actions/` | The actions, and the board that draws every visible control from the latest view. |
| `src/inspector-bridge.ts` | Answers the property inspector, and pairs with the code typed there. |
| `src/deep-link.ts` | Pairs from the editor's deep link, once the saved pairing is read. |
| `src/keep-pairing.ts` | The pairing that a typed code and a link share: the token goes into the global settings and onto the link only once the server proved it. |
| `src/lib/link.ts`, `session.ts`, `listeners.ts` | The connection: hello proof, one event stream, reconnecting with backoff, and requests; a stream that ends is followed by a proof at once, so a revoked token shows as refused right away, and a refused token ends the stream so the server must prove itself again. |
| `src/lib/api.ts`, `proof.ts`, `sse.ts`, `backoff.ts` | HTTP requests, the hello proof, the event stream reader, and the waits. |
| `src/lib/view.ts`, `faces.ts`, `reach.ts`, `room-note.ts`, `svg.ts`, `text-fit.ts`, `color.ts` | The control view, what each control shows (keeping its face for up to 1.5 s while the link connects, and "No room" for 4 s after a press found none), and the SVG drawings. |
| `src/lib/built-in-cards.ts`, `icons.ts` | The built-in packs' decks, which card keys show before pairing, and the control keys' line icons. |
| `src/lib/talk.ts` | Push to talk for every talk key: held while any is held and sent again every 30 s, requests in the order made, and an owed let-go tried again. |
| `src/lib/pairing.ts`, `settings.ts`, `inspector.ts` | Pairing, reading a pairing link, stored settings, and messages to the inspector. |
| `src/profiles/layout.ts`, `profile.ts`, `zip.ts` | The 15-key layout and the device profiles, a profile's files, and the ZIP writer they go through. |
| `ai.chaski.sceneuron.sdPlugin/` | The plugin folder: manifest, images, the dial layout, the inspector (`ui/`, plain HTML with no outside scripts), and the built profiles (`profiles/`). |
| `assets/marketplace.svg` | Source of the plugin icon; `rsvg-convert -w 256` and `-w 512` render the two PNGs. |
| `rollup-notices.mjs` | Writes `bin/third-party-notices.txt` with the license of every package in the bundle. |
| `rollup-profiles.mjs` | Writes the profiles from the layout on every build, reading the layout afresh so `npm run watch` follows edits to it. |
| `../e2e/streamdeck-keys.mjs` | Draws `docs/images/streamdeck-keys.png`, every page of the 15-key layout, with the plugin's own drawing code in headless Chrome, from the control view of a `sceneuron-dev` it starts. |

The plugin uses these routes of Sceneuron's local server, always with the token in the
`X-Sceneuron-Token` header and never in an address:

| Route | Use |
| --- | --- |
| `GET /api/hello?nonce=N` | The proof, without the token: `control` must be HMAC-SHA256 of `N:port` under it. |
| `POST /api/control/pair` | Trades the code for the token. |
| `GET /api/control/events` | The control view, sent again whenever it changes. |
| `GET /api/control/state` | The control view once, right after the stream opens. |
| `POST /api/card` | `{deck, card, action: "toggle"}` from card keys and dial presses; `{deck, action: "next"}` or `"previous"` once per dial tick, except that a turn's first tick while the deck is lowered sends `{deck, card, action: "raise"}` for the neighbor of the card on the strip; `{deck, action: "lower"}` to lower. |
| `POST /api/control/element` | `{element, action: "toggle"}`. |
| `POST /api/automation` | `{enabled}`. |
| `POST /api/voice/talk` | `{down: true}` when a talk key is pressed and every 30 s while one is held, `{down: false}` once no talk key is held. |
| `POST /api/room` | `{action: "reveal"}` and `{action: "next"}`, waiting up to 12 s for the answer; the answer to the control token carries no room state, and with no room joined it is refused and changes nothing. |

The control view names decks, cards, and elements and says what is showing. It carries no camera
captions, AI reasons, heard speech, or event history, and of a shared room only whether a card is
covered, so nothing the camera, the AI, or the microphone picked up reaches the Stream Deck.

Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved. Proprietary software; see [LICENSE](../LICENSE).
