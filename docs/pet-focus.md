# Pet click-to-agent focus

**Required outcome:** One click on a 2D pet goes directly to its existing agent destination. There must be no intermediate activation of Pet Town, no jump to the default desktop/window, and no new terminal, session, or application instance. Herdr selects the exact validated pane; Codex selects the validated local conversation when available.

**Verified baseline:** On 2026-09-29, after reporting an intermediate desktop jump, the user confirmed the final behavior: “its good now.” The local debug trace also showed the original adapter remaining foreground before dispatch, successful Herdr activation in the helper, and the intended adapter foreground afterward. This confirms the exercised Herdr/Mayor/Codex workflow, not every supported adapter or macOS version.

## Working flow

```text
2D pet or name badge pointerdown → pointerup, below the drag threshold
  → main.ts invokes focus_agent with the opaque agent ID
  → the GUI launches its current executable with --focus-agent <ID>
  → main.rs handles the argument before Tauri/singleton initialization
  → focus_current_agent recollects and validates the private route
  → Herdr pane / running application / exact Codex installation

3D Open in Herdr → scoped town_action IPC with the opaque agent ID
  → focus_agent launches the same --focus-agent child process
  → focus_current_agent → same validation and destination logic
```

The two renderers share the focus function, but neither should perform adapter activation inside the 2D overlay's GUI process. The process boundary matters; sharing a function alone did not give them equivalent behavior.

Native Godot **Open Pet Town** reopening uses a separate early CLI route,
`--focus-native-town <PID> <parent-PID>`, owned by
[godot_bridge/focus.rs](../apps/pet-town/src-tauri/src/godot_bridge/focus.rs).
It verifies that the existing Godot process belongs to the launching desktop,
then activates it from the helper's main thread while servicing `NSRunLoop`.
This route does not change pet-to-agent `--focus-agent` dispatch. Successful
helper completion and native focus-heartbeat probes pass; the final direct
visible Godot transition still awaits user observation because Computer access
to that window is unreliable.

## Preserve these together

| Part                               | Owner                                                                                                    | Why it matters                                                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Nonactivating overlay              | [window.rs](../apps/pet-town/src-tauri/src/window.rs)                                                    | The window is converted from `NSWindow` to `NSPanel`. In addition to the nonactivating style mask, the guarded, typed `_setPreventsActivation: true` call synchronizes WindowServer's activation state. Without it, a pet click can activate Pet Town's desktop before reaching the adapter. Run configuration on the main thread and retain the selector-availability guard. |
| Separate focus helper              | [focus.rs](../apps/pet-town/src-tauri/src/focus.rs), [main.rs](../apps/pet-town/src-tauri/src/main.rs)   | The 2D command launches the current executable with an opaque ID as an argument, without a shell. The helper handles focus before starting a GUI or taking the application singleton lock. Keep this isolation instead of calling `focus_current_agent` directly inside the overlay command.                                                                                  |
| Main event loop during activation  | [macos_activation.rs](../apps/pet-town/src-tauri/src/macos_activation.rs)                                | `NSRunningApplication.isActive` is cached until the main run loop advances. The helper must service `NSRunLoop` during its bounded wait; sleeping and polling alone falsely timed out for three seconds even when Herdr became visible. Both 2D and Three.js town focus enter this code on the isolated helper process's main thread.                                         |
| Exact, revalidated Herdr selection | [focus/herdr.rs](../apps/pet-town/src-tauri/src/focus/herdr.rs)                                          | Validate session identity, select workspace and tab, revalidate, then select the agent pane. The current outer route selects before host activation and again after it. Preserve session checks, the host lookup, activation-error reporting, and the post-activation selection. Herdr host activation uses empty options rather than raising every host window.              |
| Exact Codex installation           | [broker focus.rs](../packages/pet-town-agent-broker/src/focus.rs)                                        | Retain the selected running application's `bundleURL` and send its validated thread link with `open -a <that path>`. Do not independently resolve the app again with `open -b com.openai.codex`: ChatGPT and Codex installations were observed sharing that bundle ID. Generic/fallback activation may succeed without a bundle path; only deep-link delivery requires one.   |
| Click versus drag                  | [pet-interactions.ts](../apps/pet-town/src/pet-interactions.ts), [main.ts](../apps/pet-town/src/main.ts) | Pets and name badges share the same pointer handler. Focus uses the ID captured on pointerdown and dispatches on a non-drag pointerup. Dragging and context-menu actions must not also focus an agent.                                                                                                                                                                        |

`_setPreventsActivation:` is private macOS SPI. Its guarded use is a deliberate compatibility workaround, not an incidental cleanup candidate. If replacing it or changing the panel library, demonstrate an equivalent direct transition before removing it. If the selector is unavailable, the guard avoids a crash but does not guarantee the activation fix. The underlying style-mask/WindowServer mismatch is described in [the original FB16484811 investigation](https://philz.blog/nspanel-nonactivating-style-mask-flag/).

## Pet Street visibility while playing Pet Town

Pet Street hides while native Pet Town is foreground and returns when
switching away or closing the town, unless the user explicitly hid Pet Street.
`village_visibility.rs` retains the requested visibility and renderer-ready gates.
On macOS, `godot_bridge/mod.rs` reconciles the owned Godot PID's activation on the
GUI main run loop: bridge focus notifications request an immediate refresh, and
the existing 80 ms control loop checks both focus directions even if Godot is
busy or disconnected. Do not replace this with background-thread `isActive`
checks or depend solely on render-loop heartbeats. Showing Pet Street continues
to use `orderFrontRegardless` without activating Pet Town.

Check switching away, returning, minimizing and closing the town, plus explicit
Hide/Show Pets and direct pet-to-agent focus. Builds do not verify visible timing.

## What the investigation ruled out

- Recorded failing clicks already produced pointerdown, pointerup, click, and a focus request. Do not infer that missing mouse events are the cause merely because a pet fails to open its adapter.
- `Ok(())` or an accepted macOS activation request does not prove the correct window is visible. Early failures returned success while the user still saw the wrong destination.
- A panel-only workaround plus a global pressed-mouse-button passthrough latch was tried before the helper change and the user reported a regression. Both were reverted. That experiment did not isolate which part failed. The confirmed final combination restores only activation-state synchronization alongside helper isolation and main-loop servicing; it does **not** include the mouse-button latch.
- Moving focus into the helper corrected the destination but still produced the intermediate desktop jump. Helper isolation alone is therefore not the complete fix.
- Sleeping longer while polling a cached `isActive` value is not a substitute for servicing the helper's main event loop.

## Verification before changing this flow

1. Start from another foreground app. Click a Herdr pet once. Confirm a direct transition to its exact pane, with no Pet Town/default-desktop flash and no false three-second timeout.
2. Repeat for another Herdr workspace/tab and the Mayor's saved Firstmate pane. Where available, repeat from another full-screen Space.
3. Click a local Codex pet. Confirm the exact conversation opens in the existing installation, especially when more than one installed app shares the bundle ID.
4. Check dragging, right-click preferences, and click-through outside pets. Confirm they do not accidentally focus an agent.
5. Check the existing 3D Open in Herdr action and stale/ended-agent handling. Do not open or resume a stale session.
6. Rebuild embedded frontend assets when frontend code changes; this app uses Tauri's custom protocol. Confirm the replacement debug process is running. Build/type/format checks are necessary but do not establish the visual transition.

Use Computer Use for the live check when available. If it cannot bind Pet Street or denies access to an adapter, report that limit and obtain the user's observation. Do not claim visual verification from process existence, a successful command, or a build alone. In this repair the final visible result was user-confirmed.

## Diagnostic evidence

Temporary diagnostics currently live in [focus_trace.rs](../apps/pet-town/src-tauri/src/focus_trace.rs), with event capture in `main.ts` and focus/activation call sites. The local debug log is `var/2d-pet-focus/implement-details/input.log`; it records event stages, public route IDs, application identity/activation flags, and outcomes. The log and `var/` investigation notes are local supporting evidence, not the required implementation.

For a future regression, distinguish these stages: input received → route selected → target activation requested → target active → foreground after settling. Before the final fix the trace repeatedly showed Pet Town foreground before dispatch and stale helper timeouts. After it, the original adapter stayed foreground until switching directly to the target, and Herdr completed successfully in roughly 0.6 seconds for the observed clicks.

Diagnostics can be removed separately after preserving the three-part fix and its verification. Do not treat temporary tracing or local PID values as part of the product contract.

## Remote companions

A remote companion retains its saved machine ID in the private focus route. The
headless focus helper resolves its opaque ID against the current enabled machine
catalog, queries only that machine, and validates the native agent session before
issuing any focus commands. All commands retain `--machine <profile-id>`; local
socket and pane context are removed. A changed session or unavailable machine
fails without falling back to a local pane with the same ID.

Remote IDs go directly to that resolver without collecting local sessions or
starting a remote discovery round. Only the desktop GUI owns a `RemoteMonitor`;
snapshot reads are passive, and the monitor cancels and joins its subprocesses
on shutdown. The one-shot `--snapshot` refresh joins all its work before returning.

The existing local focus and macOS panel/main-run-loop flow is preserved. Remote
server selection does not switch a local TUI's selected machine or activate a
terminal host. The 3D action therefore says **Select remote pane**, displays the
manual machine-selection step. Remote companions can use the existing embedded
Observe flow to watch output without switching the selected Herdr machine.
Remote observation never grants terminal input or takeover authority.
The 2D strip keeps that instruction in the pet's hover tooltip and shows only
errors in the strip without activating a window; progress and successful selection
do not display popup bubbles. Local pets retain the direct visible-transition
requirement; remote selection may require the manual machine-selection step.

Run `pnpm run check:broker` and `pnpm run check:remote-ui` for subprocess ownership,
cancellation, timeouts, per-machine scheduling, and renderer capability checks.
