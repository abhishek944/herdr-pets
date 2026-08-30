# Behavior pack format

Behavior packs are data, not scripts. Every JSON file under `src/packs/` is discovered during the build, validated as a complete unit, and compiled into immutable data before any agent can use it. Sprite files live under `src/assets/sprites/`. New character IDs and canvas ratios must also be added to `src/village.ts` and `src/assets/characters.css`.

Only these five top-level state keys are allowed and required:

- `idle`
- `working`
- `blocked`
- `done`
- `unknown`

A repeated poll with the same normalized state keeps the current flow position. A different state cancels the old flow immediately while preserving screen position and facing.

## Add a character

1. Add transparent sprite files such as `my-pet.png` and `my-pet-action.png` under `src/assets/sprites/`.
2. Copy one bundled JSON file in `src/packs/` to `src/packs/my-pet.json`.
3. Give it a unique lowercase `id`, reference the new files as `sprites/my-pet.png`, and author all five state flows.
4. Add the character ID to `CHARACTER_IDS` and its frame ratio and fallback image to `src/assets/characters.css`.
5. Run `./scripts/check.sh`. An invalid reference, state, duration, movement clip, or loop fails the build-time checks instead of partially loading the pack.

The runtime assigns distinct packs across the approved character cast before reusing a pack for larger crowds.

## Example

```json
{
  "formatVersion": 1,
  "id": "example-pet",
  "packVersion": "1",
  "clips": {
    "walk": {
      "asset": "walk.png",
      "durationMs": 800,
      "role": "locomotion",
      "sourceFacing": "right",
      "mirror": true
    },
    "jump": {
      "asset": "jump.png",
      "durationMs": 500,
      "role": "stationary"
    }
  },
  "states": {
    "idle": {
      "completion": "restart",
      "flow": { "type": "play", "clip": "jump" }
    },
    "working": {
      "completion": "restart",
      "flow": {
        "type": "sequence",
        "steps": [
          {
            "type": "move",
            "clip": "walk",
            "durationMs": 3000,
            "speedPxPerSecond": 32
          },
          { "type": "play", "clip": "jump", "count": 1 }
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
      "flow": { "type": "wait", "durationMs": 1000 }
    }
  }
}
```

## Flow nodes

- `sequence` runs non-empty `steps` in order.
- `play` displays a named clip for its declared duration, optionally for a finite `count`.
- `move` displays a locomotion clip and moves forward for a fixed duration and positive speed.
- `wait` consumes time without moving or changing the current clip.
- `choose` selects one positive-integer-weighted branch deterministically for that agent and state entry.
- `repeat` runs one child flow a finite positive number of times.

A state with `completion: "restart"` begins again when its root flow finishes. A state with `completion: "hold"` keeps its final pose without movement. Flows cannot request directions, coordinates, turns, state changes, expressions, callbacks, or code execution.

## Validation and fallback

`compileBehaviorPack()` rejects unknown fields, missing states or clips, unsafe asset paths, missing files when an asset inventory is supplied, stationary movement clips, unsafe locomotion mirroring, invalid weights or timing, excessive nesting, and flows that can complete without consuming time. `resolveBehaviorPack()` returns the built-in safe pack when compilation fails.

The runtime owns movement and facing. A pack can only request forward movement. Direction changes happen only when the character reaches a screen edge; resizes preserve proportional position without turning the character.
