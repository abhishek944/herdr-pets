# Herdr Pets

Herdr Pets turns every live Herdr agent into a small animated citizen in a
transparent village along the bottom of your desktop.

![Herdr Pets demo — 13s from Screen Recording (1:17–1:30) showing 15 agents + village pets](assets/pets-demo.mp4)

<video src="https://raw.githubusercontent.com/abhishek944/herdr-pets/main/assets/pets-demo.mp4" autoplay loop muted playsinline controls width="100%" poster="assets/icon.svg">
  <a href="https://raw.githubusercontent.com/abhishek944/herdr-pets/main/assets/pets-demo.mp4">Download the 13s demo video</a> — 1280×832, 30fps, 642 KB.
</video>

This is a **Tauri v2 application**, built from scratch with a Rust backend and a
TypeScript/CSS web interface. It uses one lightweight window for the whole
village—never one window per agent and never an arbitrary agent limit.

## Install

Herdr Pets is a Herdr plugin. Install it from GitHub:

```bash
herdr plugin install abhishek944/herdr-pets
```

Herdr clones the repository, shows the manifest and the commands it will run for
review, then registers it. It is also listed in the [Herdr marketplace](https://herdr.dev/plugins/) — search for **herdr-pets**, or open its card from
any marketplace listing page.

The village starts automatically on the next Herdr startup (or live handoff)
because the plugin declares a startup hook. Prebuilt macOS arm64 and x64
binaries are bundled in the repository, so **no Node.js or Rust toolchain is
required to install** — you only need macOS and Herdr 0.8 or newer.

Control it with the plugin actions once it is running:

```bash
herdr plugin action invoke herdr-pets.village-on
herdr plugin action invoke herdr-pets.village-off
herdr plugin action invoke herdr-pets.village-status
```

Marketplace listings are discovered automatically from the public
`herdr-plugin` topic and are **not reviewed**. Herdr runs plugin code with your
user permissions, so review the manifest and source before installing.

## What it does

- Polls every registered Herdr session once per second through safe Rust commands (no shell), with a two-second timeout and a 1 MiB output limit.
- Assigns agents from an approved cast of eleven companions: cat, corgi, walking man, Viking, fox ronin, turtle monk, raccoon sky pirate, dune scout, clockwork apprentice, automaton porter, and slime knight.
- Runs state-authored sprite flows across the full screen while each name follows its character.
- Turns characters only at screen edges and always faces them in their movement direction.
- Selects a validated declarative behavior flow solely from agent state:
  - `working` → continuously move through distinct walk and action animations
  - `blocked` → waiting flow
  - `idle` → hidden through the pack's ordinary `hide` flow action
  - `unknown` → each pet's cautious fallback flow
  - `done` → celebration flow
- Keeps each pet's `flow.json` and APNG assets together in one self-contained folder; see [the behavior pack format](docs/behavior-packs.md).
- Gives the Viking a working cycle that alternates walking and hammering, plus a seated thinking animation when blocked.
- Uses no floating status symbols; the sprite animation communicates the current state.
- Waits for three missed polls before a departed citizen fades away.
- Shrinks citizens automatically for large crowds.
- Labels each character with its Herdr pane name when present, otherwise its custom tab name, otherwise the current folder name.
- Focuses the matching Herdr agent directly when its pet is clicked.
- Keeps the window transparent, undecorated, always on top, and visible across
  macOS workspaces. Empty pixels are click-through while citizen pixels remain
  interactive.
- Starts and stops through Herdr plugin actions instead of a login item.

## Architecture

```text
Herdr plugin (herdr-plugin.toml)
  └─ scripts/supervisor.sh
       └─ Tauri process
            ├─ Rust: reads agent state, routes focus, and caches pane and tab labels
            └─ WebView: TypeScript state, pet clicks, CSS citizens, and animations
```

The plugin is only a lifecycle supervisor. Every Herdr startup registers its
session socket in the shared plugin state; the one Tauri process polls those
sessions in parallel and namespaces pane IDs before merging them. If Herdr is
unavailable, the village quietly empties and retries.

## Requirements

**To install and run:**

- macOS (arm64 or x86_64)
- Herdr 0.8 or newer

The plugin registers with `platforms = ["macos"]`, so Herdr refuses to install it
on Linux or Windows. On a supported Mac, nothing else is needed — the prebuilt
binaries are bundled with the repository.

**To build from source instead** (for example, during development or to apply a
change to the bundled renderer), additionally need:

- Node.js 20 or newer
- Rust 1.88 or newer
- Xcode Command Line Tools

The window uses Tauri's macOS private API for transparency. That is suitable for
direct distribution but not for Apple's Mac App Store. Linux and Windows can be
added later; always-on-top behavior on Linux depends on the desktop compositor.

## Build

```bash
./scripts/build.sh
```

The release executable is written to `src-tauri/target/release/herdr-pets` and
copied into the matching `bin/macos-*` plugin package directory.

For development:

```bash
npm install
npm run tauri dev
```

## Link for local development

While working on this repository, link the working tree instead of installing
from GitHub:

```bash
herdr plugin link "$PWD"
```

Linking does not run build commands and does not register against GitHub. The
startup hook runs when the Herdr server starts or hands off. Herdr stores the
renderer PID, exact executable path, process start time, and
session registry in its plugin state directory. The supervisor verifies all
process identity fields immediately before signaling and uses macOS `lockf` for
race-free control operations. Renderer output is discarded so it cannot grow an
unbounded background log.

## Checks

```bash
./scripts/check.sh
```

The checks cover declarative flow validation, deterministic choices, safe state interruption, edge-only direction changes, TypeScript and production frontend builds, Rust formatting and compilation, and packaged arm64/x86_64 binaries. Validation runs from repository scripts without a checked-in test suite or Vitest dependency.

GitHub Actions also builds both macOS targets with the declared Rust 1.88 minimum.

## Manual visual check

1. Run `./scripts/build.sh` and link the plugin.
2. Invoke `herdr-pets.village-on` while at least two Herdr agents exist.
3. Confirm citizens appear centered just above the Dock, empty window space passes clicks through, and clicking a pet focuses its exact Herdr agent pane.
4. Confirm each pet moves to a screen edge, turns only there, and continues in the direction it faces with its project name following above.
5. Change agents between working, blocked, done, idle, and unknown; confirm idle flows hide the complete citizen while the other states restore it.
6. Watch working pets use distinct movement and action animations. Confirm the Viking hammers, Ember practices with a sword, Mossback bows, Skiff scans, Mira checks her route map and drills with a spear, Jun repairs his bird, Brassbell sorts parcels, and Pebble blocks and flourishes its spoon.
7. Invoke `herdr-pets.village-off` and confirm the strip disappears.

## Privacy

The app makes no network requests. It reads Herdr's local agent and label data and sends a local focus command only when a pet is clicked.
Only public agent IDs, normalized status, and sanitized pane, tab, or folder labels cross into the web interface. Full paths and prompts are never displayed.

## License

MIT
