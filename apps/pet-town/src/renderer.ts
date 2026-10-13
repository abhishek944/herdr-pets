import { advanceTrack, BehaviorMachine, remapTrackPosition, type CompiledBehaviorPack } from "./flow-runtime";
import { behaviorPackForCharacter } from "./character-packs"; import { RendererScenery } from "./renderer-scenery";
import type { CitizenState } from "./village"; import { freezePetFrame } from "./pet-freeze";
import type { PreferencesFile } from "./preferences-types";
import { collectHitRegions } from "./renderer-layout";
import { resolveLabelOverlaps } from "./renderer-labels";
import { applyCitizenPreferences, resizeCitizens, shouldHideCompleted, shouldHideCompletedElement, travelDistanceFor } from "./renderer-preferences";
import { applyFlowSample, CITIZEN_TRACK_WIDTH, createCitizenElement, distanceWhileAssetPending, type HitRegion, motionSeed, refreshCitizenLabelPosition, SUSPENSION_GAP_MS, updateCitizenElement } from "./renderer-view";
import { setPreferenceHidden } from "./renderer-visibility"; import { tokenMotionDistance, type TokenMotionState } from "./token-motion";
export type { HitRegion } from "./renderer-view";
interface MotionState {
  x: number; direction: -1 | 1; maximumX: number;
  behavior: BehaviorMachine; packFingerprint: string; fallbackAssetUrl: string;
  pendingElapsedMs: number; dragging: boolean; dragOffsetX: number; tokenMotion: TokenMotionState;
}
export class VillageRenderer {
  private readonly scenery: RendererScenery;
  private readonly elements = new Map<string, HTMLElement>();
  private readonly motions = new Map<string, MotionState>();
  private currentCitizenSize = 44; private currentWidth = window.innerWidth;
  private lastTimestamp = 0; private logicalRemainderMs = 0;
  private lastHitRegionUpdate = 0;
  private preferences: PreferencesFile | null = null; private paused = false; private systemReducedMotion = false;
  constructor(
    private readonly root: HTMLElement,
    private readonly updateHitRegions: (regions: HitRegion[]) => void = () => {},
    private readonly behaviorPackForCitizen: (citizen: CitizenState) => CompiledBehaviorPack = (citizen) => behaviorPackForCharacter(citizen.sprite),
  ) {
    this.scenery = new RendererScenery(this.root);
    window.requestAnimationFrame(this.animate);
  }
  render(citizens: ReadonlyMap<string, CitizenState>, width: number): void {
    const ordered = [...citizens.values()].sort((left, right) => left.id.localeCompare(right.id));
    this.currentWidth = width;
    const liveIds = new Set(ordered.map((citizen) => citizen.id));
    for (const [id, element] of this.elements) {
      if (!liveIds.has(id)) {
        element.dispatchEvent(new Event("citizen-hidden", { bubbles: true }));
        element.remove();
        this.elements.delete(id); this.motions.delete(id);
      }
    }
    const maximumX = Math.max(0, width - CITIZEN_TRACK_WIDTH);
    const waveSampler = this.currentWaveSampler();
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
          packFingerprint: pack.fingerprint, fallbackAssetUrl,
          pendingElapsedMs: 0, dragging: false, dragOffsetX: 0, tokenMotion: { activity: citizen.tokenActivity, scale: motion?.tokenMotion.scale ?? 1 },
        };
        this.motions.set(citizen.id, motion);
      } else {
        motion.fallbackAssetUrl = fallbackAssetUrl; motion.tokenMotion.activity = citizen.tokenActivity;
      }
      updateCitizenElement(element, citizen);
      setPreferenceHidden(element, shouldHideCompleted(citizen, this.preferences), this.refreshGeometry);
      applyCitizenPreferences(element, this.currentCitizenSize, this.preferences);
      refreshCitizenLabelPosition(element);
      if (maximumX !== motion.maximumX) {
        motion.x = remapTrackPosition(motion.x, motion.maximumX, maximumX);
        motion.maximumX = maximumX;
      }
      if (motion.behavior.setStatus(citizen.status)) motion.pendingElapsedMs = 0;
      const sample = motion.behavior.advance(0).sample;
      applyFlowSample(element, sample,
        motion.fallbackAssetUrl, this.refreshGeometry, this.themeAssetFor(element, sample.state));
      this.applyDirectionAndPosition(element, motion, waveSampler);
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
    const oceanChanged = this.scenery.advance(this.paused || shouldPause ? 0 : elapsedMs, this.systemReducedMotion);
    if (this.paused) {
      if (oceanChanged) this.refreshOceanWakes();
      for (const element of this.elements.values()) freezePetFrame(element);
      window.requestAnimationFrame(this.animate);
      return;
    }
    const waveSampler = this.currentWaveSampler();
    for (const [id, element] of this.elements) {
      const motion = this.motions.get(id);
      if (!motion || element.classList.contains("retiring")) continue;
      const currentSample = motion.behavior.sample();
      const themedAssetUrl = this.themeAssetFor(element, currentSample.state);
      if (motion.dragging) {
        applyFlowSample(element, currentSample, motion.fallbackAssetUrl, this.refreshGeometry, themedAssetUrl);
        this.applyDirectionAndPosition(element, motion, waveSampler);
        continue;
      }
      const assetReady = applyFlowSample(element, currentSample, motion.fallbackAssetUrl, this.refreshGeometry, themedAssetUrl);
      const advance = assetReady
        ? motion.behavior.advance(elapsedMs + motion.pendingElapsedMs, true)
        : {
            distancePx: distanceWhileAssetPending(currentSample, elapsedMs),
            remainingMs: motion.pendingElapsedMs,
            sample: currentSample,
          };
      motion.pendingElapsedMs = advance.remainingMs;
      const distance = this.scenery.travelDistance(travelDistanceFor(element, tokenMotionDistance(motion.tokenMotion, advance.distancePx, element.dataset.status === "working", elapsedMs), this.preferences, this.systemReducedMotion), element);
      const position = advanceTrack(motion.x, motion.direction, distance, motion.maximumX);
      motion.x = position.x;
      motion.direction = position.direction;
      applyFlowSample(element, advance.sample, motion.fallbackAssetUrl, this.refreshGeometry,
        this.themeAssetFor(element, advance.sample.state));
      this.applyDirectionAndPosition(element, motion, waveSampler, distance !== 0 && advance.sample.moving);
    }
    this.refreshOceanWakes();
    if (timestamp - this.lastHitRegionUpdate >= 50) {
      this.lastHitRegionUpdate = timestamp;
      this.publishHitRegions();
    }
    window.requestAnimationFrame(this.animate);
  };
  refreshPreferenceVisibility(citizens: ReadonlyMap<string, CitizenState>): void {
    for (const [id, element] of this.elements) { const citizen = citizens.get(id);
      if (citizen) setPreferenceHidden(element, shouldHideCompleted(citizen, this.preferences), this.refreshGeometry); }
  }
  setPreferences(preferences: PreferencesFile): void {
    this.preferences = preferences;
    this.scenery.setPreferences(preferences);
    const waveSampler = this.currentWaveSampler();
    for (const [id, element] of this.elements) {
      setPreferenceHidden(element, shouldHideCompletedElement(element, this.preferences), this.refreshGeometry);
      applyCitizenPreferences(element, this.currentCitizenSize, this.preferences);
      refreshCitizenLabelPosition(element);
      const motion = this.motions.get(id);
      if (motion) this.applyDirectionAndPosition(element, motion, waveSampler);
    }
    this.refreshGeometry();
  }
  setSystemReducedMotion(reduced: boolean): void {
    this.systemReducedMotion = reduced;
    this.scenery.setReduced(reduced, this.elements, this.motions);
    this.publishHitRegions();
  }
  setPaused(paused: boolean): void {
    this.paused = paused;
    this.scenery.setPaused(paused, this.elements, this.motions);
    this.lastTimestamp = 0;
    this.publishHitRegions();
  }
  beginDrag(id: string, clientX: number): boolean {
    const motion = this.motions.get(id);
    if (this.paused || !motion) return false;
    motion.dragging = true;
    motion.dragOffsetX = clientX - motion.x;
    this.publishHitRegions();
    return true;
  }
  moveDrag(id: string, clientX: number): void {
    const motion = this.motions.get(id);
    const element = this.elements.get(id);
    if (!motion || !element || !motion.dragging) return;
    motion.x = Math.max(0, Math.min(motion.maximumX, clientX - motion.dragOffsetX));
    this.applyDirectionAndPosition(element, motion, this.currentWaveSampler());
    this.publishHitRegions();
  }
  endDrag(id: string): void {
    const motion = this.motions.get(id);
    if (motion) { motion.dragging = false; this.publishHitRegions(); }
  }
  refreshHitRegions(): void { this.publishHitRegions(); }
  private themeAssetFor(element: HTMLElement, state: import("./flow-types").HerdrState): string | null { return this.scenery.assetFor(element, state); }
  private currentWaveSampler(): ((x: number) => number) | null { return this.scenery.sampler(); }
  private applyDirectionAndPosition(element: HTMLElement, motion: MotionState, sampler: ((x: number) => number) | null, walking?: boolean): void { this.scenery.position(element, motion, sampler, walking); }
  private refreshOceanWakes(): void { this.scenery.updateWakes(this.elements, this.motions, this.currentCitizenSize); }
  private readonly refreshGeometry = (): void => {
    this.currentCitizenSize = resizeCitizens(this.root, this.elements.values(), this.currentWidth, this.preferences, this.currentCitizenSize);
    this.refreshOceanWakes();
    this.publishHitRegions();
  };
  private readonly publishHitRegions = (): void => {
    if (this.paused) { this.updateHitRegions([]); return; }
    const dragging = [...this.motions.values()].some((motion) => motion.dragging);
    resolveLabelOverlaps(this.elements.values()); this.updateHitRegions(
      collectHitRegions(this.root, this.elements.values(), dragging));
  };
}
