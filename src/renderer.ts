import {
  advanceTrack,
  BehaviorMachine,
  remapTrackPosition,
  type CompiledBehaviorPack,
  type FlowSample,
} from "./flow-runtime";
import { behaviorPackForCharacter } from "./character-packs";
import { citizenSize, type CitizenState } from "./village";

export interface HitRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

type Direction = -1 | 1;

interface MotionState {
  x: number;
  direction: Direction;
  maximumX: number;
  behavior: BehaviorMachine;
  packFingerprint: string;
}

const CITIZEN_TRACK_WIDTH = 104;
const SUSPENSION_GAP_MS = 250;

function motionSeed(id: string): number {
  let value = 2166136261;
  for (let index = 0; index < id.length; index += 1) {
    value = Math.imul(value ^ id.charCodeAt(index), 16777619) >>> 0;
  }
  return value;
}

function createCitizenElement(): HTMLElement {
  const wrapper = document.createElement("section");
  wrapper.className = "citizen";

  const project = document.createElement("span");
  project.className = "project";

  const pet = document.createElement("span");
  pet.className = "pet";
  pet.setAttribute("aria-hidden", "true");

  const shadow = document.createElement("span");
  shadow.className = "shadow";
  shadow.setAttribute("aria-hidden", "true");

  wrapper.append(project, pet, shadow);
  return wrapper;
}

function updateCitizenElement(element: HTMLElement, citizen: CitizenState): void {
  const kind = citizen.sprite === "human-male" || citizen.sprite === "viking"
    ? "human"
    : "animal";
  element.className = `citizen kind-${kind} character-${citizen.sprite}${citizen.retiring ? " retiring" : ""}`;
  element.dataset.agentId = citizen.id;
  element.setAttribute("aria-label", `${citizen.label}, ${citizen.status}`);

  const project = element.querySelector<HTMLElement>(".project");
  if (project && project.textContent !== citizen.label) project.textContent = citizen.label;
}

function projectBottom(citizen: CitizenState, sample: FlowSample, size: number): number {
  let height: number;
  if (citizen.sprite === "human-male") height = size * (558 / 343);
  else if (citizen.sprite === "dog") height = size * (232 / 271);
  else if (citizen.sprite === "cat") height = size * (209 / 260);
  else if (sample.clip?.name === "hammer") height = size * 1.7 * (219 / 190);
  else if (sample.clip?.name === "think") height = size * 1.2 * (413 / 294);
  else height = size * (343 / 267);
  return Math.ceil(7 + height + 6);
}

function applyFlowSample(
  element: HTMLElement,
  citizen: CitizenState,
  sample: FlowSample,
  size: number,
): void {
  const pet = element.querySelector<HTMLElement>(".pet");
  if (!pet) return;

  const clip = sample.clip;
  const className = [
    "pet",
    `sprite-${citizen.sprite}`,
    clip?.cssClass,
    clip?.mirror ? "clip-mirror-safe" : "clip-fixed-facing",
    clip?.sourceFacing === "left" ? "clip-source-left" : "clip-source-right",
    sample.moving ? "flow-moving" : "flow-stationary",
    sample.failed ? "flow-failed" : "",
  ].filter(Boolean).join(" ");
  if (pet.className !== className) pet.className = className;

  pet.dataset.clip = clip?.name ?? "fallback";
  pet.dataset.clipEpoch = String(sample.clipEpoch);
  const assetUrl = clip?.assetUrl ?? "";
  if (pet.dataset.assetUrl !== assetUrl) {
    pet.dataset.assetUrl = assetUrl;
    if (assetUrl) pet.style.backgroundImage = `url(${JSON.stringify(assetUrl)})`;
    else pet.style.removeProperty("background-image");
  }
  pet.style.setProperty("--clip-duration-ms", String(clip?.durationMs ?? 0));
  pet.style.setProperty("--clip-elapsed-ms", String(sample.clipElapsedMs));
  const labelBottom = `${projectBottom(citizen, sample, size)}px`;
  if (element.style.getPropertyValue("--project-bottom") !== labelBottom) {
    element.style.setProperty("--project-bottom", labelBottom);
  }
}

function regionFor(element: Element): HitRegion | null {
  const bounds = element.getBoundingClientRect();
  if (bounds.width <= 0 || bounds.height <= 0) return null;
  return { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height };
}

export class VillageRenderer {
  private readonly elements = new Map<string, HTMLElement>();
  private readonly motions = new Map<string, MotionState>();
  private readonly citizenViews = new Map<string, CitizenState>();
  private currentCitizenSize = 44;
  private lastTimestamp = 0;
  private logicalRemainderMs = 0;
  private lastHitRegionUpdate = 0;

  constructor(
    private readonly root: HTMLElement,
    private readonly updateHitRegions: (regions: HitRegion[]) => void = () => {},
    private readonly behaviorPackForCitizen: (citizen: CitizenState) => CompiledBehaviorPack = (citizen) => behaviorPackForCharacter(citizen.sprite),
  ) {
    window.requestAnimationFrame(this.animate);
  }

  render(citizens: ReadonlyMap<string, CitizenState>, width: number): void {
    const ordered = [...citizens.values()].sort((left, right) => left.id.localeCompare(right.id));
    this.currentCitizenSize = citizenSize(ordered.length, width);
    this.root.style.setProperty("--citizen-size", `${this.currentCitizenSize}px`);

    const liveIds = new Set(ordered.map((citizen) => citizen.id));
    for (const [id, element] of this.elements) {
      if (!liveIds.has(id)) {
        element.remove();
        this.elements.delete(id);
        this.motions.delete(id);
        this.citizenViews.delete(id);
      }
    }

    const maximumX = Math.max(0, width - CITIZEN_TRACK_WIDTH);
    for (const citizen of ordered) {
      this.citizenViews.set(citizen.id, citizen);
      const pack = this.behaviorPackForCitizen(citizen);
      let element = this.elements.get(citizen.id);
      let motion = this.motions.get(citizen.id);
      if (!element) {
        element = createCitizenElement();
        this.root.append(element);
        this.elements.set(citizen.id, element);
      }
      if (!motion || motion.packFingerprint !== pack.fingerprint) {
        const seed = motionSeed(citizen.id);
        motion = {
          x: motion?.x ?? maximumX * (((seed >>> 16) % 1_000) / 1_000),
          direction: motion?.direction ?? ((seed & 1) === 0 ? 1 : -1),
          maximumX: motion?.maximumX ?? maximumX,
          behavior: new BehaviorMachine(pack, citizen.id, citizen.status),
          packFingerprint: pack.fingerprint,
        };
        this.motions.set(citizen.id, motion);
      }

      updateCitizenElement(element, citizen);
      if (maximumX !== motion.maximumX) {
        motion.x = remapTrackPosition(motion.x, motion.maximumX, maximumX);
        motion.maximumX = maximumX;
      }
      motion.behavior.setStatus(citizen.status);
      applyFlowSample(element, citizen, motion.behavior.advance(0).sample, this.currentCitizenSize);
      this.applyDirectionAndPosition(element, motion);
    }
  }

  private readonly animate = (timestamp: number): void => {
    const frameGap = this.lastTimestamp === 0 ? 0 : timestamp - this.lastTimestamp;
    this.lastTimestamp = timestamp;
    const shouldPause = document.hidden || frameGap > SUSPENSION_GAP_MS;
    const logicalElapsed = shouldPause ? 0 : Math.max(0, frameGap) + this.logicalRemainderMs;
    const elapsedMs = Math.floor(logicalElapsed);
    this.logicalRemainderMs = shouldPause ? 0 : logicalElapsed - elapsedMs;

    for (const [id, element] of this.elements) {
      const motion = this.motions.get(id);
      if (!motion || element.classList.contains("retiring")) continue;
      const advance = motion.behavior.advance(elapsedMs);
      const position = advanceTrack(motion.x, motion.direction, advance.distancePx, motion.maximumX);
      motion.x = position.x;
      motion.direction = position.direction;
      const citizen = this.citizenViews.get(id);
      if (citizen) applyFlowSample(element, citizen, advance.sample, this.currentCitizenSize);
      this.applyDirectionAndPosition(element, motion);
    }

    if (timestamp - this.lastHitRegionUpdate >= 160) {
      this.lastHitRegionUpdate = timestamp;
      this.publishHitRegions();
    }
    window.requestAnimationFrame(this.animate);
  };

  private applyDirectionAndPosition(element: HTMLElement, motion: MotionState): void {
    element.classList.toggle("direction-right", motion.direction === 1);
    element.classList.toggle("direction-left", motion.direction === -1);
    element.style.transform = `translate3d(${motion.x.toFixed(2)}px, 0, 0)`;
  }

  private publishHitRegions(): void {
    const regions = [...this.elements.values()]
      .filter((element) => !element.classList.contains("retiring"))
      .flatMap((element) => [
        element.querySelector(".pet"),
        element.querySelector(".project"),
      ])
      .filter((element): element is Element => element !== null)
      .map(regionFor)
      .filter((region): region is HitRegion => region !== null);
    this.updateHitRegions(regions);
  }
}
