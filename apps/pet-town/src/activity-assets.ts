/**
 * Pet Street activity artwork: per-pet sidecars discovered from the bundled pet folders.
 *
 * `pets/<id>/activity.json` publishes the nine approved action loops (Rollercoaster, Gold Mine and
 * Tiny Railway for working, blocked and done) as `activity-<theme>-<state>.webp` with the encoded
 * loop duration, the citizen-relative scale and the measured normalized support anchor. Discovery
 * runs at build time: invalid metadata is reported by activitySidecarDiagnostics() and never
 * rendered, so staged folders can arrive one at a time.
 */

export type ActivityTheme = "rollercoaster" | "goldmine" | "railway";
export type ActivityState = "working" | "blocked" | "done";

export interface ActivityClipAsset {
  readonly assetPath: string;
  readonly assetUrl: string;
  readonly durationMs: number;
  readonly scale: number;
  readonly anchorX: number;
  readonly anchorY: number;
}

export interface ActivitySpriteOffset {
  /** Prop units right, so the clip anchor shares the cart and scene centre. */
  readonly x: number;
  /** Prop units down, so the clip anchor lands on the theme support line. */
  readonly y: number;
}

const THEMES: readonly ActivityTheme[] = ["rollercoaster", "goldmine", "railway"];
const STATES: readonly ActivityState[] = ["working", "blocked", "done"];
const CLIP_KEYS = THEMES.flatMap((theme) => STATES.map((state) => `${theme}-${state}`));

const DURATION_RANGE = [16, 60_000] as const;
const SCALE_RANGE = [0.25, 4] as const;

/**
 * Support line of the shared 104x100 prop frame where each clip anchor must rest, measured from the
 * approved selected scenes: the seated Coaster cart rim, the Gold Mine cavern floor and the
 * approved 25% Railway deck support.
 */
const SUPPORT_Y: Readonly<Record<string, number>> = {
  rollercoaster: 71.02,
  goldmine: 89,
  railway: 84.25,
};

/** Prop frame baseline y90 sits 10px above the citizen bottom, and 8px for the Gold Mine -2px props. */
const BASELINE_PX: Readonly<Record<string, number>> = {
  rollercoaster: 10,
  goldmine: 8,
  railway: 10,
};

const FRAME_BASELINE = 90;

const sidecarModules = import.meta.glob("./pets/*/activity.json", {
  eager: true,
  import: "default",
}) as Record<string, unknown>;

const assetModules = import.meta.glob("./pets/*/activity-*.webp", {
  eager: true,
  import: "default",
  query: "?url",
}) as Record<string, string>;

const activityClips = new Map<string, ReadonlyMap<string, ActivityClipAsset>>();
const sidecarPets = new Set<string>();
const diagnostics: string[] = [];

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

/** Bundled pet folder name of one discovered file path. */
function petOf(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 2] ?? path;
}

function bounded(value: unknown, min: number, max: number): number | null {
  return typeof value === "number" && Number.isFinite(value) && value >= min && value <= max
    ? value
    : null;
}

function readClip(value: unknown, theme: string, state: string, assets: Record<string, string>) {
  const clip = record(value);
  const anchor = Array.isArray(clip?.anchorNormalized) ? clip.anchorNormalized : [];
  const assetPath = `activity-${theme}-${state}.webp`;
  const durationMs = bounded(clip?.durationMs, ...DURATION_RANGE);
  const scale = bounded(clip?.scale, ...SCALE_RANGE);
  const anchorX = bounded(anchor[0], 0, 1);
  const anchorY = bounded(anchor[1], 0, 1);
  const assetUrl = clip?.asset === assetPath ? assets[assetPath] : undefined;
  if (
    !clip ||
    !assetUrl ||
    durationMs === null ||
    scale === null ||
    anchorX === null ||
    anchorY === null
  )
    return null;
  return { assetPath, assetUrl, durationMs, scale, anchorX, anchorY };
}

function readSidecar(
  value: unknown,
  assets: Record<string, string>,
): Map<string, ActivityClipAsset> | null {
  const sidecar = record(value);
  const clips = record(sidecar?.clips);
  if (sidecar?.formatVersion !== 1 || !clips || Object.keys(clips).length !== CLIP_KEYS.length)
    return null;
  const result = new Map<string, ActivityClipAsset>();
  for (const key of CLIP_KEYS) {
    const split = key.split("-");
    const clip = readClip(clips[key], split[0], split[1], assets);
    if (!clip) return null;
    result.set(key, clip);
  }
  return result;
}

function assetsFor(sidecarPath: string): Record<string, string> {
  const prefix = sidecarPath.slice(0, sidecarPath.lastIndexOf("/") + 1);
  return Object.fromEntries(
    Object.entries(assetModules)
      .filter(([path]) => path.startsWith(prefix))
      .map(([path, url]) => [path.slice(prefix.length), url]),
  );
}

for (const [sidecarPath, sidecar] of Object.entries(sidecarModules)) {
  const petId = petOf(sidecarPath);
  sidecarPets.add(petId);
  const clips = readSidecar(sidecar, assetsFor(sidecarPath));
  if (clips) activityClips.set(petId, clips);
  else diagnostics.push(`${sidecarPath}: invalid activity sidecar`);
}

const artPets = new Map<string, number>();
for (const path of Object.keys(assetModules)) {
  const petId = petOf(path);
  artPets.set(petId, (artPets.get(petId) ?? 0) + 1);
}
for (const [petId, count] of artPets) {
  if (!sidecarPets.has(petId))
    diagnostics.push(`pets/${petId}: ${count} activity assets without an activity.json sidecar`);
}

/** Validated sidecar clip for one bundled pet, theme and state, or null when none is accepted. */
export function activitySidecarClip(
  petId: string,
  theme: string,
  state: string,
): ActivityClipAsset | null {
  return activityClips.get(petId)?.get(`${theme}-${state}`) ?? null;
}

/** Offline coverage hook: every rejected sidecar and every pet whose staged art has no sidecar. */
export function activitySidecarDiagnostics(): readonly string[] {
  return diagnostics;
}

/**
 * Prop-unit sprite offset for one dedicated clip. The rendered image box and its measured bottom
 * above the citizen bottom let the runtime reproduce the approved scene placement exactly, so the
 * shared stack layout, labels and prop frame keep their existing geometry.
 */
export function activitySpriteOffset(
  theme: string,
  clip: ActivityClipAsset,
  box: { readonly width: number; readonly height: number },
  boxBottomPx: number,
  scale: number,
): ActivitySpriteOffset | null {
  const supportY = SUPPORT_Y[theme];
  const baselinePx = BASELINE_PX[theme];
  if (supportY === undefined || baselinePx === undefined || !(scale > 0)) return null;
  const width = box.width / scale;
  const height = box.height / scale;
  return {
    x: (0.5 - clip.anchorX) * width,
    y:
      supportY +
      (1 - clip.anchorY) * height -
      (FRAME_BASELINE - (boxBottomPx - baselinePx) / scale),
  };
}
