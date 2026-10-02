# Pet Town

Pet Town turns live Herdr, Claude Code, Codex, OpenCode, Pi, Factory Droid,
and Cursor sessions into small animated citizens in **Pet Street**, the 2D
experience along the bottom of your desktop. **Pet Town** is the 3D world.

![Pet Town demo — 13s from Screen Recording (1:17–1:30) showing 15 agents + village pets](assets/pets-demo.gif)

<p align="center">
  <a href="assets/pets-demo.mp4">MP4 (642 KB, 1280×832)</a> · GIF autoplays above (2.0 MB, 800×520, 12 fps) — full 13s clip at <code>1:17–1:30</code>
</p>

The desktop is a **Tauri v2 application** with a Rust backend and a
TypeScript/CSS interface. Pet Street uses one lightweight window for the whole
village, with no arbitrary agent limit. **Open Pet Town** opens the editable
native Godot world with the same live agents and Mayor. Its terrain, scenery and
characters are exported from the original Three.js town, retaining all 27,037
source dry-land cells. The [Godot guide](apps/pet-town-godot-sample/README.md)
describes native gameplay; the [Three.js guide](apps/pet-town-3d/README.md)
retains browser instructions and source provenance.

## Install

Pet Town is a standalone macOS desktop app. Download the DMG for your Mac,
open it, and drag **Pet Town.app** into **Applications**. Launching the app opens
Settings and shows Pet Street; use **Show Pet Street / Hide Pet Street** in Settings, the app
menu, or the tray menu without quitting the monitor. The application bundle
includes the private Node/Pi runtime used by the optional voice orchestrator;
users do not install Node.js, Pi, or Rust. The village discovers configured
coding-agent connections without requiring Herdr.

Herdr users can additionally install the optional Herdr adapter:

```bash
herdr plugin install abhishek944/pet-town
```

That adapter starts the same desktop app with Herdr and contributes validated
Herdr sessions. Its optional actions remain available for village status,
Preferences, reload, and start/stop control. Marketplace plugins run with your
user permissions, so review their manifest and source before installing.

## What it does

- Polls every registered Herdr session once per second through safe Rust commands (no shell), with a two-second timeout and a 1 MiB output limit.
- Assigns agents from an approved cast of ten KayKit pets, with distinct packs reused only after every character has appeared.
- Runs each state's chosen APNG with either an idle or walking action while each name follows its pet.
- Turns characters only at screen edges and always faces them in their movement direction.
- Selects an APNG and action solely from agent state: Running walks; Blocked and Completed stay in place; Idle and Unknown are hidden. The voice assistant also has a separate Listening state.
- Keeps each pet's `flow.json` and APNG assets together in one self-contained folder; see [the behavior pack format](docs/behavior-packs.md).
- Uses no floating status symbols; the sprite animation communicates the current state.
- Waits for three missed polls before a departed citizen fades away.
- Shrinks citizens automatically for large crowds.
- Labels each character with its Herdr pane name when present, otherwise `{space-name}-{tab}` (for example `pet-town-1`), and uses the current folder name only when Herdr naming is unavailable.
- Focuses an exact revalidated Herdr pane when its pet is clicked, or the best already-running application known for a standalone harness without launching or resuming anything.
- Lets users drag a pet horizontally; it pauses while held and resumes its existing movement from the drop point.
- Keeps **Preferences…** in each pet's right-click menu; menu animation actions are retired.
- Opens a native macOS Settings window when the installed app launches, from **Preferences…** in a pet menu, or from the Herdr plugin action, with direct **Show Pet Street / Hide Pet Street** controls, per-character controls, a **Pet Studio**, and an **Agents** tab for atomic, reversible setup of all six standalone harness integrations.
- Lets users create or extend a pet by importing transparent looping APNGs in Settings and choosing an APNG and idle or walking action for each state. The Pets page shows those assignments.
- Adds a user-named Mayor using the bundled Knight in Pet Street and a crowned explorer in Pet Town, backed by one persistent Firstmate primary in Herdr. Live mode uses GPT-Live for speech; Standard mode uses transcription and generated speech.
- Runs Firstmate with GPT-5.6 Luna at medium thinking by default, supports task interruption, and keeps Live voice transcripts ephemeral.
- Keeps the assistant independent from Pet Studio: its bundled Knight walk animation is fixed and cannot be replaced by user pet creation or extensions.
- Includes every character in random assignment by default, lets users deselect unwanted characters, and immediately replaces visible deselected pets after Apply while preserving allowed assignments.
- Optionally hides completed pets after 1, 5, 15, 30, or 60 minutes while continuing to monitor them and restoring them immediately when their state changes.
- Pauses the visible village while Settings edits a draft; **Apply** saves the complete versioned file at `~/.pet-town/preferences.json`, while closing discards unapplied changes and resumes current agent states.
- Keeps the window transparent, undecorated, always on top, and visible across
  macOS workspaces. Empty pixels are click-through while citizen pixels remain
  interactive.
- Runs directly as a standalone app; the optional Herdr adapter can also start and stop it through plugin actions.

The Godot town includes the full source world, native animals, building tools,
26 library designs, 15 land destinations and six ocean places. Its **Companions** roster follows broker state, with **Option+A**
to cycle, **C** to control, **V** for first person, and **Escape** to leave a companion.
Mayor controls reuse the desktop voice runtime through an authenticated local
bridge. Native Godot saves remain separate from Three.js browser saves. Public
browser play remains available. Full native visual and voice acceptance remains
pending; see the
[3D flow](docs/3d-game.md) for the implemented behavior and verification limits.

## Architecture

```text
Standalone Tauri process
  ├─ Rust adapter broker: lifecycle records, Herdr discovery, and focus routes
  ├─ Rust orchestrator state: local wake bridge, GPT-Live session creation, and Firstmate lifecycle
  ├─ WebViews: Pet Street, Settings, and ephemeral WebRTC voice conversation
  ├─ native Godot process: source world, public snapshots and scoped actions over authenticated loopback TCP
  └─ dedicated Herdr pane: trusted Firstmate checkout → selected Pi model
       ↑
optional Herdr plugin adapter (herdr-plugin.toml + scripts/supervisor.sh)
```

The optional Herdr plugin is only an adapter and lifecycle convenience. Every
Herdr startup registers its session socket; the desktop process polls those
sessions in parallel and namespaces pane IDs before merging them. If Herdr is
unavailable, standalone harness pets continue normally.

The backend places Herdr and six standalone harness sources behind one adapter
broker. Reversible lifecycle hooks or plugins feed a 24-hour-retained, fail-open local
event bridge; validated Herdr ownership prevents duplicate pets. See the
[multi-harness adapter architecture](docs/multi-harness-adapters.md) and
[Agent connections guide](docs/agent-connections.md).

## Requirements

**To install and run:**

- macOS (arm64 or x86_64)
- Herdr 0.9 or newer when using the optional adapter or voice orchestrator

The optional plugin registers with `platforms = ["macos"]`. On a supported Mac,
nothing else is needed—the prebuilt standalone binaries are bundled with the
repository.

**To build from source instead** (for example, during development or to apply a
change to the bundled renderer), additionally need:

- Node.js 20.19 or newer
- pnpm 10.28.2 (Corepack can install the version declared in `package.json`)
- Rust 1.88 or newer
- Xcode Command Line Tools
- Godot 4.7+ universal macOS app for native world packaging (`GODOT_APP` can select it)
- Ruff, ShellCheck, shfmt, actionlint, and Taplo (`brew install ruff shellcheck shfmt actionlint taplo`)

The window uses Tauri's macOS private API for transparency. That is suitable for
direct distribution but not for Apple's Mac App Store. Linux and Windows can be
added later; always-on-top behavior on Linux depends on the desktop compositor.

## Develop and build both towns

See [development, packaging, and browser workflows](docs/development.md#develop-and-build-both-towns).

## Link for local development

See [linking this checkout with Herdr](docs/development.md#link-for-local-development).

## Checks

See [quality and build checks](docs/development.md#checks).

## Manual visual check

See the [user-owned visual checklist](docs/development.md#manual-visual-check).

## Privacy

Ordinary village monitoring remains local. It does not send coding-agent prompts, code, tool arguments, output, credentials, full paths, or raw harness session IDs to OpenAI. Only opaque public IDs, normalized status, source names, and sanitized labels cross into the village interface.

Voice is off until the user starts Mayor in Settings. In Live mode, on-device recognition listens locally for the configured wake phrase; after the wake phrase is heard, microphone audio and ephemeral transcripts pass through GPT-Live 1 over WebRTC without opening another visible window. Delegated requests and completed Firstmate replies pass between GPT-Live and the selected Firstmate primary. In Standard mode, recorded speech is transcribed and sent to that same primary; completed replies use generated speech. Ending the conversation stops microphone capture immediately and clears app-held transcripts; five minutes without user speech also ends the paid Live session. The OpenAI API key stays in Rust memory and may come from `OPENAI_API_KEY` or the macOS Keychain item with service `pet-town.openai` and account `api-key`.

Pet Studio does not contact an image provider. Users supply their own APNGs, which are validated and stored locally under `~/.pet-town/`. Existing-pet extensions are stored separately under `~/.pet-town/pet-packs/extensions/` as validated, versioned overlays, so bundled pet files remain immutable. The OpenAI API key is used only by the optional voice assistant and is never placed in command arguments, preferences, frontend state, or logs.

## License

MIT

### Remote Herdr companions

Pet Town also monitors enabled saved SSH machines from `herdr machine list --json`.
Both the local CLI and each remote installation must support Herdr's
`--machine <profile-id>` API forwarding. Existing SSH authentication is reused;
Pet Town does not install or restart remote servers or answer authentication prompts.

Remote discovery runs in the background, with at most eight simultaneous queries,
a five-second polling interval and an eight-second command timeout. Local pets
continue updating while a remote machine is unreachable. A disconnected pet is
shown with unknown status and expires after 30 seconds without a successful poll.
Disabled or removed profiles disappear on the next successful machine-list refresh.
Labels include the saved machine name; pet identity uses the profile ID and native
agent session so identical pane/session IDs on different hosts stay separate.

Remote pet clicks validate and select the pane on the correct remote server.
Herdr's machine forwarding does not switch the selected machine in an existing
local TUI; select the machine in Herdr to view that pane. Pet Town's embedded
terminal viewer currently supports local sessions only. The `--snapshot` CLI
performs an explicit synchronous remote refresh before printing its result.
