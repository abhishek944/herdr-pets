# Behavior pack format

Behavior packs are data, not scripts. Each bundled pet owns one folder under `src/pets/<pet-id>/`. The folder contains `flow.json` and every APNG asset referenced by that file. During the build, each folder is discovered, validated as a complete unit, and compiled into immutable data before an agent can use it.

Only these five top-level state keys are allowed and required:

- `idle`
- `working`
- `blocked`
- `done`
- `unknown`

A repeated poll with the same normalized state keeps the current flow position. A different state cancels the old flow immediately while preserving screen position and facing, then evaluates the new flow from a visible baseline. Visibility is flow-authored: no state name is treated specially by the renderer.

## Add a pet

1. Create `src/pets/my-pet/`.
2. Add transparent APNG assets such as `walk.png` and `work.png` inside that folder.
3. Add `src/pets/my-pet/flow.json` with an `id` matching the folder name.
4. Reference assets relative to that folder and author all five state flows.
5. Run `./scripts/check.sh`. Invalid references, states, durations, movement clips, or APNG payloads fail the build-time checks instead of partially loading the pack.

The runtime discovers pet IDs from these folders and assigns distinct packs before reusing a pack for larger crowds. APNG assets provide their own aspect ratios.

## Example

```json
{
  "formatVersion": 1,
  "id": "my-pet",
  "packVersion": "1",
  "clips": {
    "walk-fast": {
      "asset": "walk.png",
      "durationMs": 800,
      "role": "locomotion",
      "sourceFacing": "right",
      "mirror": true
    },
    "jump": {
      "asset": "work.png",
      "holdAsset": "done.png",
      "durationMs": 500,
      "role": "stationary"
    }
  },
  "states": {
    "idle": {
      "completion": "restart",
      "flow": { "type": "hide", "durationMs": 1000 }
    },
    "working": {
      "completion": "restart",
      "flow": {
        "type": "sequence",
        "steps": [
          {
            "type": "move",
            "clip": "walk-fast",
            "durationMs": 3000,
            "speedPxPerSecond": 32
          },
          { "type": "play", "clip": "jump" }
        ]
      }
    },
    "blocked": {
      "completion": "restart",
      "flow": { "type": "wait", "durationMs": 1000 }
    },
    "done": {
      "completion": "hold",
      "flow": { "type": "play", "clip": "jump" }
    },
    "unknown": {
      "completion": "restart",
      "flow": { "type": "play", "clip": "jump" }
    }
  }
}
```

## Clip names

Keys inside `clips` are local, user-defined names. They are not predefined behaviors and do not need to match filenames. A key must start with a lowercase letter, use only lowercase letters, numbers, and hyphens, and contain at most 64 characters. `play` and `move` nodes reference these keys exactly.

A clip may set a generic `scale` from `0.5` through `2.5` when its composition needs more or less screen space. Scaling preserves the APNG's intrinsic aspect ratio and is independent of the clip name. Its `durationMs` must exactly match one APNG cycle. A transparent static `holdAsset` supplies the frozen final pose for any clip used by a completed visible `hold` state.

## Flow nodes

- `sequence` runs non-empty `steps` in order.
- `play` shows a named clip for its declared duration, optionally for a finite `count`.
- `move` shows a locomotion clip and moves forward for a fixed duration and positive speed.
- `wait` consumes time without moving or changing the current clip or visibility.
- `hide` hides the whole pet, including its label, shadow, and hit region, for a fixed duration.
- `choose` selects one positive-integer-weighted branch deterministically for that agent and state entry.
- `repeat` runs one child flow a finite positive number of times.

A later `play` or `move` action restores visibility after `hide`. A `wait` in the same flow preserves the hidden state, while entry into a different Herdr state starts visible unless its new flow hides again. This makes hiding available to any state or sequence without state-specific renderer logic.

A state with `completion: "restart"` begins again when its root flow finishes. A state with `completion: "hold"` keeps its final pose and visibility without movement, using the final clip's required `holdAsset`. Flows cannot request directions, coordinates, turns, state changes, expressions, callbacks, or code execution.

## Validation and fallback

`compileBehaviorPack()` rejects unknown fields, missing states or clips, unsafe asset paths, missing pack-local files, unsafe clip scales, stationary movement clips, unsafe locomotion mirroring, invalid weights or timing, excessive nesting, and flows that can complete without consuming time. The asset check also rejects corrupt or misordered PNG chunks, invalid animation sequence numbers, unsafe dimensions, finite loops, invalid frame operations, timing mismatches, opaque canvases, effectively blank frames, hidden-RGB-only changes, and animations without perceptible visible changes. Bundled pack IDs must match their folder names. `resolveBehaviorPack()` returns a hidden built-in safe pack when standalone compilation fails.

The runtime owns movement and facing. A pack can only request forward movement. Direction changes happen only when the character reaches a screen edge; resizes preserve proportional position without turning the character.
