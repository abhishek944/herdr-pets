import { LIMITS, type CompiledBehaviorPack, type CompiledClip, type FlowNode, type HerdrState } from "./flow-types";
import { fnv1a, normalizeHerdrState } from "./flow-utils";
interface PendingNode {
  node: FlowNode;
  path: string;
}
interface ActiveAction {
  kind: "play" | "move" | "wait" | "hide";
  clip: string | null;
  durationMs: number;
  elapsedMs: number;
  speedPxPerSecond: number;
  epoch: number;
}
export interface FlowSample {
  state: HerdrState;
  clip: CompiledClip | null;
  clipElapsedMs: number;
  clipEpoch: number;
  moving: boolean;
  speedPxPerSecond: number;
  visible: boolean;
  held: boolean;
  failed: boolean;
}
export interface FlowAdvance {
  distancePx: number;
  remainingMs: number;
  sample: FlowSample;
}
export class BehaviorMachine {
  private state: HerdrState;
  private stateEntryOrdinal = 0;
  private pending: PendingNode[] = [];
  private active: ActiveAction | null = null;
  private currentClip: string | null = null;
  private currentClipEpoch = 0;
  private visible = true;
  private held = false;
  private failed = false;
  private actionEpoch = 0;
  private readonly nodeVisits = new Map<string, number>();
  constructor(
    readonly pack: CompiledBehaviorPack,
    private readonly agentId: string,
    initialStatus: string,
  ) {
    this.state = normalizeHerdrState(initialStatus);
    this.enterState(this.state, false);
  }
  setStatus(status: string): boolean {
    const next = normalizeHerdrState(status);
    if (next === this.state) return false;
    this.stateEntryOrdinal += 1;
    this.enterState(next, true);
    return true;
  }
  advance(deltaMs: number, stopAtClipBoundary = false): FlowAdvance {
    const duration = Number.isFinite(deltaMs) ? Math.max(0, Math.floor(deltaMs)) : 0;
    let remaining = duration;
    let distancePx = 0;
    let transitions = 0;
    let stoppedAtClipBoundary = false;
    let observedClipEpoch = this.currentClip ? this.currentClipEpoch : null;
    while (!this.failed && (remaining > 0 || !this.active) && !this.held) {
      if (!this.active) {
        transitions += this.startNextAction();
        if (transitions > LIMITS.transitionBudget || (!this.active && !this.held)) {
          this.activateFallback();
          break;
        }
        if (this.currentClip) {
          if (observedClipEpoch === null) {
            observedClipEpoch = this.currentClipEpoch;
            if (stopAtClipBoundary) {
              stoppedAtClipBoundary = true;
              break;
            }
          } else if (stopAtClipBoundary && this.currentClipEpoch !== observedClipEpoch) {
            stoppedAtClipBoundary = true;
            break;
          }
        }
      }
      if (!this.active || remaining <= 0) break;
      const consumed = Math.min(remaining, this.active.durationMs - this.active.elapsedMs);
      if (this.active.kind === "move") distancePx += this.active.speedPxPerSecond * (consumed / 1_000);
      this.active.elapsedMs += consumed;
      remaining -= consumed;
      if (this.active.elapsedMs >= this.active.durationMs) {
        this.currentClip = this.active.clip ?? this.currentClip;
        this.active = null;
      }
    }
    return { distancePx, remainingMs: stoppedAtClipBoundary ? remaining : 0, sample: this.sample() };
  }
  sample(): FlowSample {
    const clipName = this.active?.clip ?? this.currentClip;
    return {
      state: this.state,
      clip: clipName ? this.pack.clips[clipName] ?? null : null,
      clipElapsedMs: this.active?.elapsedMs ?? 0,
      clipEpoch: this.currentClipEpoch,
      moving: this.active?.kind === "move",
      speedPxPerSecond: this.active?.kind === "move" ? this.active.speedPxPerSecond : 0,
      visible: this.visible,
      held: this.held,
      failed: this.failed,
    };
  }
  private enterState(state: HerdrState, preserveClip: boolean): void {
    this.state = state;
    this.pending = [{ node: this.pack.states[state].flow, path: `/states/${state}/flow` }];
    this.active = null;
    this.held = false;
    this.failed = false;
    this.visible = true;
    this.nodeVisits.clear();
    if (!preserveClip) {
      this.currentClip = null;
    }
  }

  private startNextAction(): number {
    let transitions = 0;
    while (!this.active && !this.held && transitions <= LIMITS.transitionBudget) {
      const next = this.pending.pop();
      if (!next) {
        if (this.pack.states[this.state].completion === "hold") {
          this.held = true;
          break;
        }
        this.pending.push({ node: this.pack.states[this.state].flow, path: `/states/${this.state}/flow` });
        transitions += 1;
        continue;
      }
      transitions += 1;
      const node = next.node;
      if (node.type === "sequence") {
        for (let index = node.steps.length - 1; index >= 0; index -= 1) {
          this.pending.push({ node: node.steps[index], path: `${next.path}/steps/${index}` });
        }
      } else if (node.type === "repeat") {
        for (let index = node.count - 1; index >= 0; index -= 1) {
          this.pending.push({ node: node.flow, path: `${next.path}/flow` });
        }
      } else if (node.type === "choose") {
        const visit = this.nodeVisits.get(next.path) ?? 0;
        this.nodeVisits.set(next.path, visit + 1);
        const total = node.choices.reduce((sum, choice) => sum + choice.weight, 0);
        const seed = `${this.pack.fingerprint}\u0000${this.agentId}\u0000${this.state}\u0000${this.stateEntryOrdinal}\u0000${next.path}\u0000${visit}`;
        let selection = fnv1a(seed) % total;
        let branch = node.choices[node.choices.length - 1];
        let branchIndex = node.choices.length - 1;
        for (let index = 0; index < node.choices.length; index += 1) {
          if (selection < node.choices[index].weight) {
            branch = node.choices[index];
            branchIndex = index;
            break;
          }
          selection -= node.choices[index].weight;
        }
        this.pending.push({ node: branch.flow, path: `${next.path}/choices/${branchIndex}/flow` });
      } else {
        const clip = node.type === "play" || node.type === "move" ? node.clip : null;
        const durationMs = node.type === "play"
          ? this.pack.clips[node.clip].durationMs * (node.count ?? 1)
          : node.durationMs;
        this.actionEpoch += 1;
        this.active = {
          kind: node.type,
          clip,
          durationMs,
          elapsedMs: 0,
          speedPxPerSecond: node.type === "move" ? node.speedPxPerSecond : 0,
          epoch: this.actionEpoch,
        };
        if (node.type === "hide") {
          this.currentClip = null;
          this.visible = false;
        } else if (clip) {
          this.currentClip = clip;
          this.currentClipEpoch = this.active.epoch;
          this.visible = true;
        }
      }
    }
    return transitions;
  }

  private activateFallback(): void {
    this.failed = true;
    this.pending = [];
    this.active = null;
    this.held = true;
    this.currentClip = null;
    this.visible = false;
  }
}
