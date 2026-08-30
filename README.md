# Herdr Pets

Herdr Pets turns every live Herdr agent into a small animated citizen in a
transparent village along the bottom of your desktop.

This is a **Tauri v2 application**, built from scratch with a Rust backend and a
TypeScript/CSS web interface. It uses one lightweight window for the whole
village—never one window per agent and never an arbitrary agent limit.

## What it does

- Polls every registered Herdr session once per second through safe Rust commands (no shell), with a two-second timeout and a 1 MiB output limit.
- Assigns agents from a deliberately small approved cast: walking man, corgi, cat, and Viking worker.
- Runs state-authored sprite flows across the full screen while each name follows its character.
- Turns characters only at screen edges and always faces them in their movement direction.
- Selects a validated declarative behavior flow solely from agent state:
  - `working` → walk, act, wait, choose, and repeat authored steps
  - `blocked` → waiting flow
  - `idle` / `unknown` → gentle wandering flow
  - `done` → celebration flow
- Lets behavior packs define clips and timed flows without scripts; see [the behavior pack format](docs/behavior-packs.md).
- Gives the Viking a working cycle that alternates walking and hammering, plus a seated thinking animation when blocked.
- Uses no floating status symbols; the sprite animation communicates the current state.
- Waits for three missed polls before a departed citizen fades away.
- Shrinks citizens automatically for large crowds.
- Labels each character with its Herdr pane name when present, otherwise its custom tab name, otherwise the current folder name.
- Keeps the window transparent, undecorated, always on top, and visible across
  macOS workspaces. Empty pixels are click-through while citizen pixels remain
  interactive.
- Starts and stops through Herdr plugin actions instead of a login item.

## Architecture

```text
Herdr plugin (herdr-plugin.toml)
  └─ scripts/supervisor.sh
       └─ Tauri process
            ├─ Rust: reads agent state plus cached pane and tab labels from Herdr
            └─ WebView: TypeScript state + CSS citizens and animations
```

The plugin is only a lifecycle supervisor. Every Herdr startup registers its
session socket in the shared plugin state; the one Tauri process polls those
sessions in parallel and namespaces pane IDs before merging them. If Herdr is
unavailable, the village quietly empties and retries.

## Requirements

- macOS (v0.1 target)
- Herdr 0.8 or newer
Prebuilt macOS arm64 and x64 renderers are included for normal plugin installs.
Building from source additionally requires:

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

## Link to Herdr

```bash
herdr plugin link "$PWD"
```

The startup hook runs when the Herdr server starts or hands off. You can also
control it directly:

```bash
herdr plugin action invoke herdr-pets.village-on
herdr plugin action invoke herdr-pets.village-off
herdr plugin action invoke herdr-pets.village-status
```

Herdr stores the renderer PID, exact executable path, process start time, and
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
3. Confirm citizens appear centered just above the Dock and clicks pass through.
4. Confirm each pet moves to a screen edge, turns only there, and continues in the direction it faces with its project name following above.
5. Change agents between working, blocked, done, idle, and unknown; confirm each state immediately selects its configured flow.
6. Watch a working pet move, perform its extreme action, and resume moving.
7. Invoke `herdr-pets.village-off` and confirm the strip disappears.

## Privacy

The app makes no network requests. It reads only Herdr's local agent-list output.
Only pane IDs, normalized status, and sanitized pane, tab, or folder labels cross into the web interface. Full paths and prompts are never displayed.

## License

MIT
