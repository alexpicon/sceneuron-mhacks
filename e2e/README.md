# End-to-end tests

## Proof in a real OBS

`run.mjs` checks the first proof from [the backlog](../docs/backlog.md) inside a real OBS Studio:
the plugin loads and, as a fresh install, starts from the starter setup, the editor attaches camera
analysis to an existing source and adds an overlay, the face detector drives the overlay from a
test clip, the editor API previews the clip and moves the Sceneuron Voice filter to the source
chosen, manual controls and hotkeys work, the starter project's planning poker deck (or the pack,
added through the project API when the project lacks it) is raised and lowered with vendor requests
and with its hotkeys, the voice filter and push-to-talk hotkey register and a voice filter attaches
to a source, a scripted vision model and Jev (`mock-vlm.mjs`) turn conditions on from camera frames
and the assistant's plan is applied, the starter project's subtitles bars (or the Subtitles pack,
added the same way) show a line simulated through `POST /api/subtitles/simulate` (no audio) and the
overlay draws the bar and hides it when the line clears, Set up OBS for me adds the overlay back
and leaves the clip, a media file, to be picked by hand, automation pauses and resumes, and the
layout, rules, and cards survive an OBS restart. Each check names the requirement it covers.

The Mic and Preview checks (`camera-mic-obs-steps.mjs`) run once camera analysis runs on the clip.
`GET /api/audio-sources` lists the clip among the sources that carry audio, after OBS's global
audio devices, which come first as obs-websocket's `GetSpecialInputs` names them: Mic/Aux before
Desktop Audio, each once and marked global. Choosing the clip through
`POST /api/audio-sources/attach` adds the Sceneuron Voice filter to it; when the scratch OBS has a
global audio device, choosing that device moves the filter to it, and choosing the clip moves it
back. `GET /api/camera/preview` answers a whole JPEG of the clip, at most 640 pixels wide and in
its 16:9 shape, with `Cache-Control: no-store`, `GET /api/camera/analysis` reports the clip's
face with its box, and the live description in `GET /api/state` counts the clip's person. In the runs on 2026-10-03 the scratch OBS had no global audio device, so the
check that global devices come first had none to order, and moving the filter to one was not
checked.

The Starter and Setup checks are in `starter-obs-steps.mjs`. The Starter checks run once the plugin
has loaded in the scratch OBS, which has no Sceneuron files yet: the project has the starter's
planning poker, meeting signals, classroom, and status decks, its subtitles bars, and its six
rules; the AI settings file the plugin wrote holds no key, no token, and no consent, with watching,
voice, and subtitles off; and `GET /api/ai` reads them as Google Gemini, with voice control set to
listen all the time and subtitles into Japanese, each waiting for consent. The Setup checks run
after the AI checks. They take away the overlay, camera analysis, and the voice filters, check that
no source is analyzed and no microphone has the filter, and call `POST /api/setup`, what the
editor's Set up OBS for me button sends: it adds the Sceneuron Overlay to the current scene,
analyzes no media file, not even the only video source, and says to pick the source, gives
Mic/Aux the Sceneuron Voice filter or, without Mic/Aux, skips the microphone and says why, and
sends no Stream Deck pairing link, since the scratch home has no Sceneuron plugin for Stream Deck,
and says how to install it; that also shows the plugin looks for Stream Deck plugins in the scratch
home, never yours. Once the clip is picked, a second call finds everything in place and adds
nothing. In the run on 2026-10-03
the scratch OBS had no Mic/Aux and no camera device, so adding the filter to Mic/Aux and starting
analysis on a camera device were not checked in OBS.

It has 81 checks: 15 for the AI path, 10 for cards, 3 for voice, 5 for subtitles, 3 for the
microphone list, 3 for the camera preview, 3 for the starter setup, 6 for Set up OBS for me, and
33 for the rest. The last run, in OBS Studio 32.2.2 on macOS on 2026-10-03, passed 79 of 79,
before the live description and Stream Deck setup checks were added; those have not run in OBS
yet. No
check covers the shared room, camera hand signals, overlay themes, or fun reactions, and the clip
has no sound, so voice is checked only as far as its filter and hotkey, and subtitles only with a
simulated line. On 2026-10-03 one run failed only the check that the second OBS run quits cleanly,
from the known OBS and Chromium shutdown issue when obs-browser unloads, and the rerun passed.

### Isolation

The test starts a second OBS process with `CFFIXED_USER_HOME` pointing at a scratch directory, so
it gets its own config, plugins, scene collections, and logs. `--multi` lets it run beside an OBS
you already have open. Your own OBS setup is never read or changed. The test uses the Sceneuron
editor on 127.0.0.1 port 4471 and obs-websocket on port 4460 without a password. obs-websocket has
no setting for its listen address, so while the test runs it accepts connections on every network
interface; run it on a trusted network or with a firewall that blocks port 4460.

### Requirements

- macOS with OBS Studio 32 in `/Applications/OBS.app`
- Node.js 22 or newer (built-in `WebSocket` and `fetch`)
- `ffmpeg` with libx264, to render the test clip from the vision fixtures
- A packaged plugin bundle (see [scripts/README.md](../scripts/README.md))

### Run

```bash
cargo build --release -p sceneuron-obs
cargo run -p sceneuron-notices -- --target aarch64-apple-darwin --out notices.txt
scripts/package.sh --lib target/release/libsceneuron.dylib --os macos --version 0.1.0 \
  --notices notices.txt --out dist/macos
node e2e/run.mjs --plugin dist/macos/sceneuron.plugin
```

Pass `--work-dir DIR` to choose the scratch directory, which the test then never deletes, and
`--keep` to keep its own scratch directory after a passing run. A failing run always keeps it; the
OBS log is under `home/Library/Application Support/obs-studio/logs`.

The test clip shows a face for 8 seconds and nobody for 12 seconds, looping, so one full run takes
about two minutes.

## Browser regression test for the editor and overlay

`editor-browser.mjs` drives the editor and overlay pages in headless Chrome against `sceneuron-dev`,
without OBS. It starts the dev server on a free port with a fixed token, `--demo-presence`, and
`--demo-ai`, opens the editor in a throwaway Chrome profile, and prints PASS or FAIL for each step:

- the editor loads, connects, shows simulated presence, and loads every stylesheet
- elements are selected from the list with clicks and arrow keys, and added and deleted
- property edits reach the draft and the preview; invalid entries are flagged and block saving
- Ctrl+S saves; a click in the preview selects without moving anything; dragging and arrow keys move
  the selected element; discard restores the saved draft
- manual holds, text holds, release, and effects; pausing and resuming automation, which stays
  paused through a save
- simulated presence runs the rules; tabs switch with arrow keys
- rules are added, edited, saved, and deleted; detection settings are validated and saved
- a failed save shows an alert toast that the keyboard dismisses
- the Camera & mic tab says no camera is analyzed yet, lists the dev server's simulated sources,
  starts analysis, and adds the overlay
- the camera preview shows the simulated camera's drawn picture, marked simulated, and updates it a
  few times a second, with what on-device analysis sees and a box for each face it finds; leaving
  the tab or hiding the page stops the requests and drops the picture, and showing them again brings
  it back
- the AI tab's Voice and Subtitles sections say no microphone has the Sceneuron Voice filter yet,
  and Choose microphone leads to the Camera & mic tab's list, which shows the simulated Mic/Aux, a
  global audio device, before Desktop Audio; choosing Mic/Aux adds the filter and marks it
  Listening, the server's list marks it as the one in use, and both AI sections then name it
- on a second `sceneuron-dev` of their own, with a new project file, a simulated OBS with
  nothing set up yet, and a simulated Stream Deck plugin (`--stream-deck-links`), the project
  starts from the starter setup, with its four decks and eleven elements; the Camera & mic tab's
  Set up OBS for me adds the overlay, analyzes the simulated webcam, gives the simulated Mic/Aux
  the Sceneuron Voice filter, and sends the Stream Deck the editor's own pairing link, says so in
  the tab and in a notification, and marks the camera Analyzing and the microphone Listening; the
  link's code trades for the control token as the plugin would; a second click finds it all in
  place, the Stream Deck paired, and changes nothing; that tab logs no console error
- the overlay page is transparent, scales the canvas to the window, and follows live overrides and
  effects
- the AI tab shows the simulated AI, set up as Google Gemini, the default provider, and keeps
  watching off until consent is given; its consent text says images go to
  `generativelanguage.googleapis.com (Google Gemini)` and what a free-tier key lets Google do with
  them, and before consent the assistant's camera note says the same of the request and project; a
  new condition takes its id from its name and starts a rule; a text placeholder is inserted and
  previewed
- watching turns a condition on, and the overlay shows what the AI reported
- with consent, the assistant's camera note says it sends one still image, and what a free-tier
  key lets Google do with what is sent
- the setup wizard proposes a setup and applies it; a proposal can be stopped; applying a plan over
  unsaved edits asks first
- a plan or a draft saved after the project changed elsewhere is refused, with a way to propose
  again or reload
- the Cards tab adds the planning poker pack (and refuses it twice); its card buttons and card
  editor follow a card slot color changed on the Properties tab
- a refused card entry blocks saving, which shows it on the Cards tab, and stays when the tab is
  shown again; the tab redraws once, only after the last refused entry is fixed and any input
  method's composition has ended, keeping the caret where it was, and moving only a card's alpha,
  before or after the fix, uses the slot's new color
- the Cards tab raises a card from the keyboard that the preview then shows, lowers it, and saves
  an edited card and deck settings without losing the card's hand signal
- connecting a Stream Deck shows a pairing code and the port to enter, and the code trades for the
  control token; disconnecting asks first and then revokes that token
- deleting a deck in the Cards tab also deletes its card slot when asked, and deleting a card slot in
  the element list removes its deck
- a Meeting signals deck saved before the pack gained its fist, pointing, and peace signs keeps the
  signals it was saved with; a card's Hand signal list saves the signal chosen, with notes on poses
  of two hands that show two signals and on an open palm, which hands often show while talking
- a hand signal another card of the deck already has is refused, naming both cards, and blocks
  saving; one that a deck whose hand signals are on also has is warned about, naming that deck and
  card and saying how a vote in that deck decides what is raised, and saves
- the Cards tab's line on where hand signals go, a status region, shows once the meeting deck's
  hand signals are on beside planning poker's: they go to Meeting signals while no vote is open in
  Planning poker; a poker card raised through the API opens a vote, and the line then says finger
  counts go to Planning poker until a given time or until the card is lowered; lowering it brings
  the first line back; the steps then delete their decks
- Pair the Stream Deck on this computer opens a link to
  `streamdeck://plugins/message/ai.chaski.sceneuron/pair` with the editor's port, a fresh six-digit
  code, and `streamdeck=hidden`, which the page records instead of following, so no Stream Deck app
  is asked; the code in it trades once for the control token, and the editor then says Paired
- in a tab with the user agent of an OBS dock, 460 pixels wide, the editor leaves that button out,
  says to type the code instead, and follows no link
- the Cards tab's Room section offers only saved decks and the local SpacetimeDB by default, points
  at a missing name or a code that cannot be one, and reports a SpacetimeDB that does not answer
  without joining a room
- the Room section joins a room on a stand-in for SpacetimeDB (`room-stand-in.mjs`, which speaks
  SpacetimeDB's WebSocket text protocol with the room module's rules) through Sceneuron's real
  client, shows another participant's vote cast from a raw WebSocket (`room-probe.mjs`), and moves
  focus to the room's status when a reveal from the API hides the focused Reveal button, and to the
  Room heading when leaving from the API hides the focused Next round button; `room-live.mjs`
  checks the real module
- the AI tab's Voice section shows the simulated voice and keeps listening off until consent is
  given; a deck's voice and speech switches and a card's trigger phrases and speech line are saved,
  and a refused phrase is flagged
- Hold to talk, with the mouse and with the keyboard, hears the simulated voice's next scripted
  command, which raises a card as a voice raise; the live heard line says what it did, and it is no
  live region, so screen readers hear only the final result, through the page's announcer
- the overlay page plays each spoken card once, from this server; Chrome may play media without a
  click, as OBS's browser sources may, and the overlay's tab is shown, since Chrome holds back media
  in a hidden tab. Each card gets 15 s to start, because a second card once took about 7 s. Every
  run reports each card's time from its raise to its audio being ready, to play(), to canplay, and
  to playing. In nine such runs on 2026-10-02 the audio was ready and play() called within 25 ms;
  one second card took 4.9 s from play() to playing, and wherever canplay was timed it came with
  playing, so that time is Chrome loading and decoding the audio, not Sceneuron. A card that does
  not start in time is also reported with the overlay's plays and errors, the audio element's
  ready and network states and error, and the server's speech cue
- the AI tab's Subtitles section shows the simulated subtitles and keeps them off until consent is
  given; its preview is no live region
- a typed target code that is not a language code is flagged in the form, and Spanish, picked by
  name instead, is saved and named in the consent text, which says translations go to
  `generativelanguage.googleapis.com (Google Gemini)` and what a free-tier key lets Google do
- the target lists offer Quenya and Klingon under Constructed and fictional and Japanese under
  Languages; the spoken language list leaves Quenya out, and the API refuses it as the spoken
  language; Quenya picked as a second target is saved, shown by name, and named in the consent text
  as Quenya (Elvish)
- turned on, the preview shows the simulated speaker's final line and each translation by name: the
  Spanish one, and the Quenya one in its romanization; a screen reader hears each new final line once
  through the page's announcer, and the event log never shows the words
- the Cards tab adds the Subtitles pack, and the overlay shows a line and its translation in its two
  bars, labeled "English (detected)" and "Spanish"; a bar held on the second target writes the
  Quenya in Tengwar, labeled "Quenya (Elvish)", once the bundled Tengwar font has loaded; the labels
  switch hides the labels and shows them again; and both bars hide once subtitles are turned off
- the Settings tab's theme picker shows a theme in the preview before it is saved; saving shows it
  on the overlay, which keeps the name label's panel in its box and its words
- Windows 95 titles each element's window with its name, puts a frame's title bar above the
  picture, and draws card slots as dialogs titled with their deck's name; the property form says
  what the theme draws its own way and shows on stream, and None takes those notes away
- every theme keeps the label's panel in its box and fits two held lines on each subtitle bar and a
  raised card; under Windows 95, live subtitles keep one size while their lines arrive; an unknown
  theme is refused, and None brings back each element's own look
- the Settings tab's fun reactions are on by default, with 20 seconds between floods, say they are
  unavailable with the simulated AI, which cannot see a tongue, and save their switch and seconds
- a rule whose action floods the overlay plays when simulated presence changes: the flood covers at
  least 28 of 32 parts of the overlay page, above every element, and then removes itself; the
  editor's Test the flood plays at once on the overlay and in the preview, whatever the seconds
  between floods, and the event log says so
- Add live description, with its help saying the line is free and each caption costs a look, puts
  the ready-made bar in the draft once and selects it; once saved, the overlay shows the simulated
  camera's line, with the simulated AI's caption and its age below it when there is one, and
  follows the line from "1 person" to "Nobody in view" or back, as the Camera & mic tab does
- the Voice section's What you can say lists the server's guide; a reaction's voice switch names
  its phrase and is saved as `reactions.voice_off`; Type a command plays a reaction, switches the
  theme, and hides an element at once, and words no local command knows come back from the
  simulated AI as a project edit, each said on the heard line
- neither page logs a console error or a Content Security Policy violation

The test stops Chrome and the dev server and deletes the Chrome profile when it ends, and exits
nonzero when any step fails. It has 95 steps, and a run on 2026-10-03 passed 95 of 95. It takes
about a minute and a half.

### Requirements

- Node.js 22 or newer
- Google Chrome, which the test runs from `/Applications/Google Chrome.app`. Pass `--chrome PATH`
  or set `CHROME` to use another browser binary, such as Chromium.

### Run

```bash
cargo build -p sceneuron-dev
node e2e/editor-browser.mjs
```

Pass `--dev-binary PATH` to test another build (the default is `target/debug/sceneuron-dev`). The
Chrome profile and, after a failure, screenshots of both pages go to a new temporary directory,
which a passing run deletes unless you pass `--keep`. `--work-dir DIR` uses DIR instead and never
deletes it; the Chrome profile inside is always deleted. The fun reaction steps also save
screenshots of a flood there: `flood-1.png` to `flood-4.png` of the overlay, and `editor-flood.png`.

`chrome.mjs` launches Chrome, `cdp.mjs` drives a page over the DevTools Protocol, and
`editor-steps.mjs`, `live-steps.mjs`, `camera-mic-steps.mjs`, `setup-steps.mjs`,
`editor-ai-steps.mjs`, `assistant-steps.mjs`, `card-editor-steps.mjs`, `hand-signal-steps.mjs`,
`streamdeck-editor-steps.mjs`, `room-editor-steps.mjs`, `voice-steps.mjs`, `subtitle-steps.mjs`,
`theme-steps.mjs`, `reaction-steps.mjs`, `live-description-steps.mjs`, and
`voice-command-steps.mjs` hold the editor steps.

## Stream Deck plugin against a real Sceneuron

`streamdeck-live.mjs` checks the built Stream Deck plugin against `sceneuron-dev`, without Stream
Deck or OBS. It starts the dev server on a free port with a fixed editor token, adds the poker pack
through `PUT /api/project`, and runs `streamdeck/ai.chaski.sceneuron.sdPlugin/bin/plugin.js` the way
Stream Deck runs it, under the plugin tests' fake Stream Deck app
(`streamdeck/tests/fake-stream-deck.mjs`). It prints PASS or FAIL for each step and exits nonzero
when any step fails:

- every deck and card that the bundled 15-key layout names is one that `GET /api/packs` offers
- the editor token asks `POST /api/control/pairing` for a code, and the plugin pairs with it from
  its property inspector, with no `Origin` header
- the plugin proves the server through `/api/hello` with a fresh nonce before any request carries
  the token: apart from the hello with which pairing checks the new token, its connection begins
  with a proof of its own, answered before the event stream is asked for, and every stream it opens
  needs a new one; it then follows changes made in the editor without asking
- a card key raises its card and lowers it again; the lower key lowers the deck
- the dial steps the deck with next and previous, starting from the card its strip shows while the
  deck is lowered, and a push lowers it
- an element key shows or hides its element and holds the change; the automation key pauses and
  resumes automation
- push to talk holds while its key is down, and Sceneuron lets go when the key comes up or leaves
  the Stream Deck while held
- with no room joined, Reveal votes changes nothing, shows the alert, and both room keys say "No
  room"; in a room on a stand-in SpacetimeDB (`room-stand-in.mjs`), Reveal votes reveals the vote
  cast, and Next round starts the next round and lowers it
- revoking the control token with the editor token turns the plugin refused at once: the state its
  property inspector is shown never turns offline on the way, and every key and the dial say
  "Pair again", with none drawn as "Offline" since the revoke; after that a press shows the alert,
  and no request carries the old token
- the token never appears in a message to Stream Deck other than the global settings, in an
  address, or in the plugin's log
- a fresh plugin with nothing saved gets the deep link that the editor's Pair the Stream Deck on
  this computer opens, with a code the editor token asked for, as a `didReceiveDeepLink` event
  whose address leaves out the link's `streamdeck://plugins/message/ai.chaski.sceneuron` prefix;
  it pairs through the same exchange and hello proof as a typed code, every key and the dial go
  live, and `GET /api/control/pairing` counts the pairing, as the editor sees it
- the same link again, a wrong code, and a port where nothing answers pair nothing, and links with
  port 0, port 65536, or no code are ignored; the pairing is kept, no key goes dark, and a card key
  still works
- neither the codes nor the token appear in a message to Stream Deck other than the global
  settings, in an address, or in the plugin's log

Each check reads Sceneuron's state through the editor token's API, the images the plugin drew, and
the requests it made: `streamdeck-fetch-hook.mjs` is loaded into the plugin with `node --import` and
writes a numbered line for each request as it is sent (whether it carried the token, never the
token itself) and another when its answer comes, which `streamdeck-log.mjs` reads, so the order
checked is the order sent. `streamdeck-steps.mjs` holds the steps, `streamdeck-link-steps.mjs` the
deep link steps, and `dev-server.mjs`, which `editor-browser.mjs` uses too, starts and stops the
dev server. It has 21 steps, adding the poker pack among them, and a run on 2026-10-03 passed 21
of 21. A run takes a few seconds.

### Requirements

- Node.js 24 or newer, the runtime Stream Deck gives the plugin
- The plugin's dependencies and build in `streamdeck/`; the script builds nothing

### Run

```bash
cargo build --locked -p sceneuron-dev
(cd streamdeck && npm ci && npm run build)
node e2e/streamdeck-live.mjs
```

Pass `--dev-binary PATH` to test another build (the default is `target/debug/sceneuron-dev`).

## Shared room against a real SpacetimeDB

`room-live.mjs` checks the shared planning poker room end to end, by hand, since it needs
SpacetimeDB. It starts `spacetime start` on a free port on 127.0.0.1, with its data directory,
signing keys, and CLI config in the run's work directory, so the SpacetimeDB install on the computer
is never read or changed; publishes `room/module` as `sceneuron-room`; starts two `sceneuron-dev`
servers on free ports; and prints PASS or FAIL for each step:

- each server adds the planning poker pack and a text element showing `{{room}}`
- Ana creates a room, and Ben joins it with the code typed in lowercase
- a client outside Sceneuron, on a raw WebSocket (`room-probe.mjs`), subscribes to the module's four
  public tables before anyone votes
- each raises a different poker card through `/api/card`, and each overlay shows only "✓ Voted"
- each sees that the other voted, but no answer of either server names the other's card, and
  `{{room}}` shows "Voted: 2 of 2"
- the outside client saw two ballots but no card in any message, and the private `vote` table cannot
  be read by a one-off query or through `POST /v1/database/sceneuron-room/sql`
- Ana's control token reveals, and its answer carries no names or votes
- both overlays then show their own card, and `{{room}}` shows "3, 8" on both
- the outside client first sees a card in the reveal's own update, which sets the room revealed and
  publishes both votes
- the outside client joins the room, and the module refuses its vote after the reveal
- closing the outside client's connection takes it out of the room
- Ben starts the next round, which lowers both cards and shows "Voted: 0 of 2"
- quitting Ben's `sceneuron-dev` takes him out of the room, and Ana's `{{room}}` shows "Voted: 0 of 1"
- Ana leaves, and joining the empty room's code is then refused

`spacetime-server.mjs` starts, publishes to, and stops the server, and `room-steps.mjs` holds the
steps. It has 17 steps, starting the server and publishing the module among them, and on
2026-10-02 it passed 17 of 17 against SpacetimeDB 2.10.2. Everything runs on one computer over
plain HTTP, so it does not check two computers, TLS, or Maincloud. A first run spends a minute or
two building the module for wasm; later runs take seconds. With the module's `vote` table made
public, the outside client's step fails.

### Requirements

- The spacetime CLI 2.10.2, and Rust's `wasm32-unknown-unknown` target through rustup;
  [docs/development.md](../docs/development.md) says how to install them
- Node.js 22 or newer

### Run

```bash
cargo build --locked -p sceneuron-dev
node e2e/room-live.mjs
```

Pass `--dev-binary PATH` to test another build, and `--spacetime PATH` (or set `SPACETIME`) when the
CLI is not `~/.local/bin/spacetime`. The work directory is a new temporary directory, which a
passing run deletes unless you pass `--keep`; `--work-dir DIR` uses DIR instead and never deletes it.
The server's log is `spacetime/spacetime.log` inside it.

### Solo demo with simulated teammates

`room-teammates.mjs` lets one person show the room: each teammate is its own connection to the
database, with its own identity, so the module counts it as another person. They join by the room
code, vote one after another a few seconds apart around one value, wait for the reveal, and vote
again in each next round. Ctrl+C disconnects them, which takes them out of the room. Checked on
2026-10-03 against a local SpacetimeDB and `sceneuron-dev`: four teammates joined and voted, the
reveal showed all five cards, they voted again in round 2, and Ctrl+C left the creator alone.

1. Install SpacetimeDB once, then start the server, which also publishes the module:

   ```bash
   scripts/spacetime-install.sh
   scripts/spacetime.sh start
   ```

2. Create a room in the editor's Cards tab (its Room section; the Stream Deck cannot create or
   join one), and note its code.
3. Bring the team in, then vote yourself:

   ```bash
   node e2e/room-teammates.mjs ABC123
   ```

   `--names Maya,Sam,Priya,Leo` picks who joins, `--cards p3,p5,p5,p8` fixes their cards, and
   `--wait` and `--gap` set the milliseconds before the first vote and between votes.
4. Show that the database keeps votes secret before the reveal. Ask as an anonymous client, as a
   viewer would; the `spacetime` CLI is logged in as the database's owner, who may read private
   tables, so `spacetime sql` would show the votes:

   ```bash
   curl -s -X POST http://127.0.0.1:3000/v1/database/sceneuron-room/sql -d 'SELECT * FROM vote'
   curl -s -X POST http://127.0.0.1:3000/v1/database/sceneuron-room/sql -d 'SELECT * FROM ballot'
   ```

   The first is refused because `vote` is private; the second shows who voted, not what.
5. Reveal: every card flips at once. Start the next round and the teammates vote again.

## Voice control against the real ElevenLabs API

`voice-live.mjs` checks voice control end to end without OBS or a microphone, against the real
ElevenLabs API. **It spends ElevenLabs credits**, so it is opt-in: it refuses to run unless
`SCENEURON_ELEVENLABS_API_KEY` holds a key. First, at no cost, it:

- checks that the build has voice control (`sceneuron-dev --help` lists `--voice-wav` and keyterms)
- starts `sceneuron-dev` on a free port with a new project file, which starts from the starter
  setup and so has the meeting deck already (the meeting pack is added only when it is missing),
  turns voice on for that deck through `PUT /api/project`, reads the project back to check that the
  deck kept its voice switch, and quits, which saves the project
- lists the account's voices (`GET /v2/voices`, free), which also checks the key, and chooses one:
  `--voice-id` when the account may have it, else "George" from the quickstart, else a premade
  voice of the account

If any of these fails, it prints that nothing was spent and exits with status 2. Otherwise it says
it is about to spend credits, and:

- synthesizes "Sceneuron, wrap it up" with text to speech (Flash v2.5) as 16 kHz PCM and writes it
  to a WAV file with half a second of silence on each side
- starts `sceneuron-dev` again with that project and `--voice-wav`, which sends the file to Scribe
  v2 Realtime speech to text as one push-to-talk turn, with the wake word and the meeting deck's
  card names and trigger phrases as keyterms, and checks that the meeting card "Wrap it up" rises
  within 30 seconds
- prints the characters of text to speech and the seconds of audio sent to speech to text

It has 5 steps, 3 free and 2 that spend. The phrase is 21 characters; the printed count is
ElevenLabs' `character-cost` header when it sends one, else that length. It passed on 2026-10-02:
text to speech returned 1.44 s of speech, for which ElevenLabs reported a character cost of 5,
Scribe v2 Realtime heard the phrase in the 2.4 s file, and the meeting card rose. It does not
check a real microphone, OBS, or a real call. `SCENEURON_ELEVENLABS_URL`, such as
`http://127.0.0.1:9`, sends every request to that server instead of `https://api.elevenlabs.io`,
speech to text included (through `--voice-url`). `tests/voice-live.test.mjs` checks the refusals
without a key and with a build that cannot run, with the variable on a closed local address, so
even a broken guard reaches no server.

```bash
cargo build --locked -p sceneuron-dev
SCENEURON_ELEVENLABS_API_KEY=... node e2e/voice-live.mjs
```

Pass `--dev-binary PATH` to test another build, `--voice-id ID` to use another voice, and
`--work-dir DIR` or `--keep` as for the browser test. `wav.mjs` writes the WAV file and
`voices.mjs` chooses the voice; the Rust tests of `sceneuron-voice`, `sceneuron-ai`, and
`sceneuron-dev` cover the same path against an in-process fake ElevenLabs server, at no cost.

## Subtitles against the real ElevenLabs and Gemini APIs

`subtitles-live.mjs` checks subtitles end to end without OBS or a microphone, against the real
ElevenLabs and Gemini APIs. **It spends credits on both**, so it is opt-in: it refuses to run
unless `SCENEURON_ELEVENLABS_API_KEY` and `SCENEURON_GEMINI_API_KEY` hold keys, which it reads only
from the environment, never from a `.env` file. The Gemini key must allow the Gemini API (see the
[development guide](../docs/development.md#ai-providers)). First, at no cost, it:

- checks that the build has subtitles (`sceneuron-dev --help` lists `--voice-wav` and subtitles)
- starts `sceneuron-dev --ai` without either key, with AI settings on Google Gemini that turn
  subtitles on, translated into Spanish by Gemini 3.5 Flash-Lite (`gemini-3.5-flash-lite`), checks
  that it kept them and that subtitles wait for a key, adds a `{{subtitles}}` and a
  `{{subtitles:es}}` text element, and quits
- lists the ElevenLabs account's voices (`GET /v2/voices`, free) and chooses one as `voice-live.mjs`
  does
- asks Gemini about the translation model (`GET /v1beta/models/{model}`, free), which also checks
  the key: a key Google refuses for the Gemini API (`ACCESS_TOKEN_TYPE_UNSUPPORTED` or
  `API_KEY_SERVICE_BLOCKED`) fails the check with what to do instead of Google's own words, and a
  model that does not take `generateContent` fails it too (`gemini.mjs` reads these answers)

If any of these fails, it prints that nothing was spent and exits with status 2. Otherwise it says
it is about to spend credits, and:

- synthesizes "Good morning everyone, let's start the weekly meeting." with text to speech (Flash
  v2.5) as 16 kHz PCM and writes it to a WAV file with half a second of silence on each side
- starts `sceneuron-dev --ai` with both keys, that project, and `--voice-wav`, which sends the file to
  Scribe v2 Realtime as one push-to-talk turn; the final line goes to Gemini 3.5 Flash-Lite for
  Spanish
- checks within 45 seconds that the English element shows "morning" and "meeting" and the Spanish
  one a Spanish sentence
- prints the characters of text to speech, the seconds of audio sent to speech to text, and the
  translation's input and output tokens and estimated cost

A run uses 54 characters of text to speech, about 4 seconds of speech to text, and one translation
of a few hundred tokens. It has not been run yet. `SCENEURON_ELEVENLABS_URL` and
`SCENEURON_GEMINI_URL`, such as `http://127.0.0.1:9`, send every request to those servers instead of
`https://api.elevenlabs.io` and `https://generativelanguage.googleapis.com`.
`tests/subtitles-live.test.mjs` points both at a closed local address and checks the refusals
without one or both keys, a build that cannot run, and, with a built `sceneuron-dev`, that the free
checks run on this computer and the first request to ElevenLabs stops the test before anything is
spent. It looks for that build in `CARGO_TARGET_DIR` when it is set, else in `target/`, and skips
the case without one; with `SCENEURON_REQUIRE_DEV_BINARY=1`, a build it cannot find fails it
instead.

```bash
cargo build --locked -p sceneuron-dev
SCENEURON_ELEVENLABS_API_KEY=... SCENEURON_GEMINI_API_KEY=... node e2e/subtitles-live.mjs
```

Pass `--dev-binary PATH`, `--voice-id ID`, `--work-dir DIR`, or `--keep` as for `voice-live.mjs`.

## Linux and Windows

Sceneuron is Mac-only for now, by the owner's decision of 2026-10-03; Windows and Linux are
planned, coming very soon. Every test here runs on macOS. The headless libobs load test on Linux,
which loaded a Linux build of the plugin into libobs in a Docker container, was removed from `main`
with the other Linux and Windows code paths. The last `main` with it is the pushed tag
`archive/linux-windows-2026-10-03` (commit `74d062e`), under `e2e/linux/`.

## Pictures of the themes and the Stream Deck keys

`theme-shots.mjs` draws the overlay in every theme. It starts `sceneuron-dev` on a free port, with
the project kept in memory, adds the meeting and subtitles packs, raises "Wrap it up", and holds a
line and its translation on the subtitle bars; headless Chrome then screenshots the real overlay
page at 1920x1080 over a dark backdrop, once per theme. The default name label and camera frame
move to the top, where the packs leave room, so nothing overlaps. It writes one picture per theme,
named by its id (`none.png`, `hacker.png`, `eighties.png`, `windows-95.png`, `classic-mac.png`, and
`vaporwave.png`), and `themes.png`, which shows them side by side, to `--out DIR`, by default
`target/theme-shots`.

`streamdeck-keys.mjs` draws `docs/images/streamdeck-keys.png`, every page of the Stream Deck
plugin's ready-made 15-key layout side by side. The plugin's own code (`faces.ts` and `svg.ts`)
draws every key from a real control view: `sceneuron-dev` runs with the meeting, planning poker,
and status packs and a card up in each deck, and push to talk shows held on the first page. Stream
Deck draws its own Go to Page keys; the picture draws them the plugin's way, with the page each
goes to. `--out FILE` writes the picture elsewhere.

Both need a built `sceneuron-dev` and Chrome, as the browser test does, and take
`--dev-binary PATH`, `--chrome PATH`, and `--work-dir DIR` as it does. `streamdeck-keys.mjs`
imports the plugin's TypeScript sources, so it needs a Node.js that strips types, such as Node.js
24 or newer.

```bash
cargo build --locked -p sceneuron-dev
node e2e/theme-shots.mjs
node e2e/streamdeck-keys.mjs
```

Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved. Proprietary software; see [LICENSE](../LICENSE).
