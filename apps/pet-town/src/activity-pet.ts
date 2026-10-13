import {
  activitySidecarClip,
  activitySpriteOffset,
  type ActivityClipAsset,
} from "./activity-assets";
import {
  behaviorPackForCharacter,
  bundledBehaviorPackForCharacter,
  characterDisplayName,
  isSystemPet,
} from "./character-packs";
import { friendlyPetName, type PreferencesFile } from "./preferences-types";
import type { CompiledClip } from "./flow-types";

/**
 * Dedicated action art is bundled per pet. Installed Pet Studio extensions and user-installed
 * packs own their clips and state assignments, so they never inherit bundled sidecars.
 */
function activitySidecarFor(theme: string, id: string, state: string): ActivityClipAsset | null {
  if (!isSystemPet(id)) return null;
  if (behaviorPackForCharacter(id) !== bundledBehaviorPackForCharacter(id)) return null;
  return activitySidecarClip(id, theme, state);
}

/** Dedicated loops are stationary, unmiritored and keep the sidecar's citizen-relative scale. */
function dedicatedClip(theme: string, state: string, clip: ActivityClipAsset): CompiledClip {
  return {
    name: `activity:${theme}:${state}`,
    durationMs: clip.durationMs,
    role: "stationary",
    assetPath: clip.assetPath,
    assetUrl: clip.assetUrl,
    holdAssetPath: null,
    holdAssetUrl: null,
    sourceFacing: "right",
    mirror: false,
    scale: clip.scale,
  };
}

export function activityPoseFor(theme: string, id: string, state: string): CompiledClip | null {
  const sidecar = activitySidecarFor(theme, id, state);
  if (sidecar) return dedicatedClip(theme, state, sidecar);
  if (!["rollercoaster", "goldmine", "railway"].includes(theme) || !isSystemPet(id)) return null;
  // Installed Pet Studio extensions own their clips and state assignments.
  if (behaviorPackForCharacter(id) !== bundledBehaviorPackForCharacter(id)) return null;
  const stem =
    theme === "rollercoaster"
      ? state === "working"
        ? "walk"
        : state === "blocked"
          ? "wave"
          : state === "done"
            ? "done"
            : null
      : theme === "railway"
        ? state === "working"
          ? "work"
          : state === "blocked"
            ? "done"
            : state === "done"
              ? "wave"
              : null
        : state === "working" || state === "blocked" || state === "done"
          ? "walk"
          : null;
  if (!stem) return null;
  const clips = Object.values(behaviorPackForCharacter(id).clips);
  const clip = clips.find((item) => item.assetPath === `${stem}.webp` && item.assetUrl);
  if (clip) return clip;
  const held = clips.find((item) => item.holdAssetPath === `${stem}.webp` && item.holdAssetUrl);
  return held
    ? {
        ...held,
        name: `activity:${stem}`,
        assetPath: held.holdAssetPath,
        assetUrl: held.holdAssetUrl,
      }
    : null;
}

export function activityClipFor(theme: string, id: string, state: string): string | null {
  return activityPoseFor(theme, id, state)?.name ?? null;
}

export function activityAssetFor(theme: string, element: HTMLElement): string | null {
  if (element.dataset.agentSource === "orchestrator") return null;
  const id = element.dataset.characterId ?? "";
  return activityPoseFor(theme, id, element.dataset.status ?? "unknown")?.assetUrl ?? null;
}

export function activityPresentationFor(
  element: HTMLElement,
  assetUrl: string | null,
): CompiledClip | null {
  const id = element.dataset.characterId ?? "";
  const presentation = activityPoseFor(
    element.parentElement?.dataset.stripTheme ?? "",
    id,
    element.dataset.status ?? "unknown",
  );
  return assetUrl && presentation?.assetUrl === assetUrl ? presentation : null;
}

export function syncActivityName(
  theme: string,
  element: HTMLElement,
  preferences: PreferencesFile | null,
): void {
  let name = element.querySelector<HTMLElement>(".activity-name");
  if (theme !== "rollercoaster") {
    name?.remove();
    return;
  }
  if (!name) {
    name = element.ownerDocument.createElement("span");
    name.className = "activity-name";
    element.querySelector(".pet-stack")?.append(name);
  }
  const id = element.dataset.characterId ?? "";
  name.textContent =
    preferences?.pets[id]?.customName?.trim() ||
    characterDisplayName(id) ||
    friendlyPetName(id).split(" ")[0];
}

const placements = new WeakMap<HTMLElement, { key: string; y: number }>();

function setSpriteOffset(element: HTMLElement, name: string, value: number | null): void {
  const text = value === null ? "" : value.toFixed(3);
  if (element.style.getPropertyValue(name) === text) return;
  if (text) element.style.setProperty(name, text);
  else element.style.removeProperty(name);
}

/**
 * Clears dedicated anchor state when a citizen leaves an activity theme for Standard or Ocean, so
 * the activity sprite translation never survives the exit. The cached placement is dropped as well:
 * a stale key would otherwise suppress the recomputation a later theme re-entry needs.
 */
export function clearActivityPlacement(element: HTMLElement): void {
  placements.delete(element);
  setSpriteOffset(element, "--activity-sprite-offset-x", null);
  setSpriteOffset(element, "--activity-sprite-offset-y", null);
  delete element.dataset.activityAnchor;
}

/**
 * Align one dedicated clip with the theme support line and the scene centre. Only the sprite image
 * moves, so the shared stack, labels and prop frame keep their existing geometry; the measured box
 * is cached per clip, state and citizen size, and the previously applied offset is subtracted so
 * repeated measurements stay exact. Returns whether this state owns accepted sidecar art, which
 * also tells callers to skip detached props the sprite already holds.
 */
export function applyActivityPlacement(
  theme: string,
  element: HTMLElement,
  state: string,
): boolean {
  const id = element.dataset.characterId ?? "";
  const clip =
    element.dataset.agentSource === "orchestrator" ? null : activitySidecarFor(theme, id, state);
  if (!clip) {
    clearActivityPlacement(element);
    return false;
  }
  const image = element.querySelector<HTMLImageElement>("img.pet");
  const scale = (Number.parseFloat(element.style.getPropertyValue("--citizen-size")) || 44) / 44;
  const key = `${theme}:${state}:${scale}:${image?.dataset.assetReadyKey ?? ""}`;
  const previous = placements.get(element);
  if (!previous || previous.key !== key) {
    const citizen = element.getBoundingClientRect();
    const box = image?.getBoundingClientRect();
    if (element.hidden || !box || box.width <= 0 || box.height <= 0) {
      clearActivityPlacement(element);
      return true;
    }
    const offset =
      clip && image && box && !image.hidden && image.naturalWidth > 0
        ? activitySpriteOffset(
            theme,
            clip,
            { width: box.width, height: box.height },
            citizen.bottom - box.bottom + (previous?.y ?? 0) * scale,
            scale,
          )
        : null;
    placements.set(element, { key, y: offset?.y ?? 0 });
    setSpriteOffset(element, "--activity-sprite-offset-x", offset?.x ?? null);
    setSpriteOffset(element, "--activity-sprite-offset-y", offset?.y ?? null);
    const anchored = String(Boolean(offset));
    if (element.dataset.activityAnchor !== anchored) element.dataset.activityAnchor = anchored;
  }
  return Boolean(clip);
}
