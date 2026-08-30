import {
  compileBehaviorPack,
  type CompiledBehaviorPack,
} from "./flow-runtime";
import type { CharacterId } from "./village";

const manifestModules = import.meta.glob("./packs/*.json", {
  eager: true,
  import: "default",
}) as Record<string, unknown>;

const spriteModules = import.meta.glob("./assets/sprites/*.png", {
  eager: true,
  import: "default",
  query: "?url",
}) as Record<string, string>;

const assets = Object.fromEntries(
  Object.entries(spriteModules).map(([path, url]) => [
    `sprites/${path.split("/").pop() ?? ""}`,
    url,
  ]),
);

const compiled = Object.values(manifestModules)
  .map((manifest) => {
    const result = compileBehaviorPack(manifest, assets);
    if (!result.pack) {
      const messages = result.diagnostics
        .map((diagnostic) => `${diagnostic.path}: ${diagnostic.message}`)
        .join("; ");
      throw new Error(`Invalid bundled character pack: ${messages}`);
    }
    return result.pack;
  })
  .sort((left, right) => left.id.localeCompare(right.id));

export const CHARACTER_BEHAVIOR_PACKS = Object.freeze(
  Object.fromEntries(compiled.map((pack) => [pack.id, pack])) as Record<
    CharacterId,
    CompiledBehaviorPack
  >,
);

export function behaviorPackForCharacter(id: CharacterId): CompiledBehaviorPack {
  const pack = CHARACTER_BEHAVIOR_PACKS[id];
  if (!pack) throw new Error(`Missing bundled character pack for ${id}`);
  return pack;
}
