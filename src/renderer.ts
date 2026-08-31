import { advanceTrack, BehaviorMachine, remapTrackPosition, type CompiledBehaviorPack } from "./flow-runtime";
import { behaviorPackForCharacter } from "./character-packs";
import { citizenSize, type CitizenState } from "./village";
import { applyFlowSample, CITIZEN_TRACK_WIDTH, createCitizenElement, distanceWhileAssetPending, type HitRegion, motionSeed, regionFor, SUSPENSION_GAP_MS, updateCitizenElement } from "./renderer-view";

export type { HitRegion } from "./renderer-view";

interface MotionState {
  x: number;
  direction: -1 | 1;
  maximumX: number;
  behavior: BehaviorMachine;
  packFingerprint: string;
  fallbackAssetUrl: string;
  pendingElapsedMs: number;
}

export class VillageRenderer {
  private readonly elements = new Map<string, HTMLElement>();
  private readonly motions = new Map<string, MotionState>();
  private currentCitizenSize = 44;
  private currentWidth = window.innerWidth;
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
    this.currentWidth = width;

    const liveIds = new Set(ordered.map((citizen) => citizen.id));
    for (const [id, element] of this.elements) {
      if (!liveIds.has(id)) {
        element.remove();
        this.elements.delete(id);
        this.motions.delete(id);
      }
    }

    const maximumX = Math.max(0, width - CITIZEN_TRACK_WIDTH);
    for (const citizen of ordered) {
      const pack = this.behaviorPackForCitizen(citizen);
      const fallbackAssetUrl = Object.values(pack.clips)
        .find((clip) => clip.assetUrl)?.assetUrl ?? "";
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
          fallbackAssetUrl,
          pendingElapsedMs: 0,
        };
        this.motions.set(citizen.id, motion);
      } else {
        motion.fallbackAssetUrl = fallbackAssetUrl;
      }

      updateCitizenElement(element, citizen);
      if (maximumX !== motion.maximumX) {
        motion.x = remapTrackPosition(motion.x, motion.maximumX, maximumX);
        motion.maximumX = maximumX;
      }
      if (motion.behavior.setStatus(citizen.status)) motion.pendingElapsedMs = 0;
      applyFlowSample(element, motion.behavior.advance(0).sample, motion.fallbackAssetUrl, this.refreshGeometry);
      this.applyDirectionAndPosition(element, motion);
    }
    this.refreshGeometry();
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
      const currentSample = motion.behavior.sample();
      const assetReady = applyFlowSample(
        element,
        currentSample,
        motion.fallbackAssetUrl,
        this.refreshGeometry,
      );
      const advance = assetReady
        ? motion.behavior.advance(elapsedMs + motion.pendingElapsedMs, true)
        : {
            distancePx: distanceWhileAssetPending(currentSample, elapsedMs),
            remainingMs: motion.pendingElapsedMs,
            sample: currentSample,
          };
      motion.pendingElapsedMs = advance.remainingMs;
      const position = advanceTrack(motion.x, motion.direction, advance.distancePx, motion.maximumX);
      motion.x = position.x;
      motion.direction = position.direction;
      applyFlowSample(element, advance.sample, motion.fallbackAssetUrl, this.refreshGeometry);
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

  private readonly refreshGeometry = (): void => {
    const visibleCount = [...this.elements.values()]
      .filter((element) => !element.hidden)
      .length;
    const size = citizenSize(visibleCount, this.currentWidth);
    if (size !== this.currentCitizenSize) {
      this.currentCitizenSize = size;
      this.root.style.setProperty("--citizen-size", `${size}px`);
    }
    this.publishHitRegions();
  };

  private readonly publishHitRegions = (): void => {
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
  };
}
