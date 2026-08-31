import {
  compileBehaviorPack,
  type CompiledBehaviorPack,
} from "./flow-runtime";

const manifestModules = import.meta.glob("./pets/*/flow.json", {
  eager: true,
  import: "default",
}) as Record<string, unknown>;

const assetModules = import.meta.glob("./pets/**/*.png", {
  eager: true,
  import: "default",
  query: "?url",
}) as Record<string, string>;

function assetsForManifest(manifestPath: string): Record<string, string> {
  const directory = manifestPath.slice(0, manifestPath.lastIndexOf("/"));
  const prefix = `${directory}/`;
  return Object.fromEntries(
    Object.entries(assetModules)
      .filter(([path]) => path.startsWith(prefix))
      .map(([path, url]) => [path.slice(prefix.length), url]),
  );
}

const compiled = Object.entries(manifestModules)
  .map(([manifestPath, manifest]) => {
    const result = compileBehaviorPack(manifest, assetsForManifest(manifestPath));
    if (!result.pack) {
      const messages = result.diagnostics
        .map((diagnostic) => `${diagnostic.path}: ${diagnostic.message}`)
        .join("; ");
      throw new Error(`Invalid bundled pet pack ${manifestPath}: ${messages}`);
    }
    const pathParts = manifestPath.split("/");
    const folderId = pathParts[pathParts.length - 2];
    if (result.pack.id !== folderId) {
      throw new Error(`Pet pack id ${result.pack.id} must match folder ${folderId}`);
    }
    return result.pack;
  })
  .sort((left, right) => left.id.localeCompare(right.id));

export type CharacterId = string;

export const CHARACTER_BEHAVIOR_PACKS = Object.freeze(
  Object.fromEntries(compiled.map((pack) => [pack.id, pack])) as Readonly<
    Record<CharacterId, CompiledBehaviorPack>
  >,
);

export const CHARACTER_IDS = Object.freeze(
  compiled.map((pack) => pack.id as CharacterId),
);

export function behaviorPackForCharacter(id: CharacterId): CompiledBehaviorPack {
  const pack = CHARACTER_BEHAVIOR_PACKS[id];
  if (!pack) throw new Error(`Missing bundled pet pack for ${id}`);
  return pack;
}
