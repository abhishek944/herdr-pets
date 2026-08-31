import type { FlowSample } from "./flow-runtime";
import type { CitizenState } from "./village";

export interface HitRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const ASSET_READY_TIMEOUT_MS = 1_500;
export const CITIZEN_TRACK_WIDTH = 104;
export const SUSPENSION_GAP_MS = 250;

export function distanceWhileAssetPending(sample: FlowSample, elapsedMs: number): number {
  if (!sample.moving || !Number.isFinite(sample.speedPxPerSecond)) return 0;
  const duration = Number.isFinite(elapsedMs) ? Math.max(0, elapsedMs) : 0;
  return sample.speedPxPerSecond * (duration / 1_000);
}

export function motionSeed(id: string): number {
  let value = 2166136261;
  for (let index = 0; index < id.length; index += 1) {
    value = Math.imul(value ^ id.charCodeAt(index), 16777619) >>> 0;
  }
  return value;
}

export function createCitizenElement(): HTMLElement {
  const wrapper = document.createElement("section");
  wrapper.className = "citizen";

  const project = document.createElement("span");
  project.className = "project";

  const stack = document.createElement("span");
  stack.className = "pet-stack";

  const pet = document.createElement("img");
  pet.className = "pet";
  pet.alt = "";
  pet.draggable = false;
  pet.hidden = true;

  const shadow = document.createElement("span");
  shadow.className = "shadow";
  shadow.setAttribute("aria-hidden", "true");

  stack.append(project, pet);
  wrapper.append(stack, shadow);
  return wrapper;
}

export function updateCitizenElement(element: HTMLElement, citizen: CitizenState): void {
  element.className = `citizen${citizen.retiring ? " retiring" : ""}`;
  element.dataset.agentId = citizen.id;
  element.setAttribute("aria-label", `${citizen.label}, ${citizen.status}`);

  const project = element.querySelector<HTMLElement>(".project");
  if (project && project.textContent !== citizen.label) project.textContent = citizen.label;
}

function syncCitizenVisibility(
  element: HTMLElement,
  pet: HTMLImageElement,
  onGeometryChange: () => void,
): void {
  const hidden = element.dataset.flowVisible !== "true" || pet.hidden;
  if (element.hidden === hidden) return;
  element.hidden = hidden;
  onGeometryChange();
}

function applyPetPresentation(pet: HTMLImageElement, sample: FlowSample): void {
  const clip = sample.clip;
  const className = [
    "pet",
    clip?.mirror ? "clip-mirror-safe" : "clip-fixed-facing",
    clip?.sourceFacing === "left" ? "clip-source-left" : "clip-source-right",
    sample.moving ? "flow-moving" : "flow-stationary",
    sample.failed ? "flow-failed" : "",
  ].filter(Boolean).join(" ");
  if (pet.className !== className) pet.className = className;
  const scale = String(clip?.scale ?? 1);
  if (pet.style.getPropertyValue("--clip-scale") !== scale) {
    pet.style.setProperty("--clip-scale", scale);
  }
}

export function applyFlowSample(
  element: HTMLElement,
  sample: FlowSample,
  fallbackAssetUrl = "",
  onGeometryChange: () => void = () => {},
): boolean {
  const pet = element.querySelector<HTMLImageElement>(".pet");
  if (!pet) return true;

  element.dataset.flowVisible = String(sample.visible);
  if (!sample.visible) {
    syncCitizenVisibility(element, pet, onGeometryChange);
    return true;
  }

  const clip = sample.clip;
  const assetUrl = (sample.held ? clip?.holdAssetUrl : clip?.assetUrl)
    ?? clip?.assetUrl
    ?? fallbackAssetUrl;
  const assetKey = JSON.stringify([assetUrl, sample.clipEpoch, sample.held]);
  if (pet.dataset.assetKey !== assetKey) {
    const request = String((Number(pet.dataset.assetRequest) || 0) + 1);
    const hasVisibleAsset = Boolean(pet.getAttribute("src")) && !pet.hidden;
    pet.dataset.assetRequest = request;
    pet.dataset.assetKey = assetKey;
    if (assetUrl) {
      if (!hasVisibleAsset) pet.hidden = true;
      syncCitizenVisibility(element, pet, onGeometryChange);
      const loader = new Image();
      loader.src = assetUrl;
      const releaseTimer = globalThis.setTimeout(() => {
        if (pet.dataset.assetRequest !== request) return;
        pet.dataset.assetReleasedKey = assetKey;
      }, ASSET_READY_TIMEOUT_MS);
      void loader.decode().then(() => {
        globalThis.clearTimeout(releaseTimer);
        if (pet.dataset.assetRequest !== request) return;
        applyPetPresentation(pet, sample);
        pet.removeAttribute("src");
        pet.src = assetUrl;
        pet.hidden = false;
        pet.dataset.assetReadyKey = assetKey;
        syncCitizenVisibility(element, pet, onGeometryChange);
      }).catch(() => {
        globalThis.clearTimeout(releaseTimer);
        if (pet.dataset.assetRequest !== request) return;
        pet.hidden = true;
        pet.removeAttribute("src");
        pet.dataset.assetReadyKey = assetKey;
        syncCitizenVisibility(element, pet, onGeometryChange);
      });
    } else {
      applyPetPresentation(pet, sample);
      pet.hidden = true;
      pet.removeAttribute("src");
      pet.dataset.assetReadyKey = assetKey;
      syncCitizenVisibility(element, pet, onGeometryChange);
    }
  } else if (pet.dataset.assetReadyKey === assetKey) {
    applyPetPresentation(pet, sample);
    syncCitizenVisibility(element, pet, onGeometryChange);
  }
  return pet.dataset.assetReadyKey === assetKey || pet.dataset.assetReleasedKey === assetKey;
}

export function regionFor(element: Element): HitRegion | null {
  const bounds = element.getBoundingClientRect();
  if (bounds.width <= 0 || bounds.height <= 0) return null;
  return { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height };
}
