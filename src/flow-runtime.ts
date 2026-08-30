export const HERDR_STATES = ["idle", "working", "blocked", "done", "unknown"] as const;

export type HerdrState = (typeof HERDR_STATES)[number];
export type ClipRole = "stationary" | "locomotion";
export type SourceFacing = "left" | "right";

export interface ClipManifest {
  durationMs: number;
  role: ClipRole;
  asset?: string;
  cssClass?: string;
  sourceFacing?: SourceFacing;
  mirror?: boolean;
}

export interface SequenceNode {
  type: "sequence";
  steps: FlowNode[];
}

export interface PlayNode {
  type: "play";
  clip: string;
  count?: number;
}

export interface MoveNode {
  type: "move";
  clip: string;
  durationMs: number;
  speedPxPerSecond: number;
}

export interface WaitNode {
  type: "wait";
  durationMs: number;
}

export interface ChoiceBranch {
  weight: number;
  flow: FlowNode;
}

export interface ChooseNode {
  type: "choose";
  choices: ChoiceBranch[];
}

export interface RepeatNode {
  type: "repeat";
  count: number;
  flow: FlowNode;
}

export type FlowNode = SequenceNode | PlayNode | MoveNode | WaitNode | ChooseNode | RepeatNode;

export interface StateFlowManifest {
  completion: "restart" | "hold";
  flow: FlowNode;
}

export interface BehaviorPackManifest {
  formatVersion: 1;
  id: string;
  packVersion: string;
  clips: Record<string, ClipManifest>;
  states: Record<HerdrState, StateFlowManifest>;
}

export interface PackDiagnostic {
  code: string;
  path: string;
  message: string;
}

export interface CompiledClip {
  name: string;
  durationMs: number;
  role: ClipRole;
  assetPath: string | null;
  assetUrl: string | null;
  cssClass: string;
  sourceFacing: SourceFacing;
  mirror: boolean;
}

export interface CompiledBehaviorPack {
  readonly formatVersion: 1;
  readonly id: string;
  readonly packVersion: string;
  readonly fingerprint: string;
  readonly clips: Readonly<Record<string, CompiledClip>>;
  readonly states: Readonly<Record<HerdrState, StateFlowManifest>>;
}

export interface PackCompilation {
  pack: CompiledBehaviorPack | null;
  diagnostics: PackDiagnostic[];
}

export interface ResolvedBehaviorPack {
  pack: CompiledBehaviorPack;
  diagnostics: PackDiagnostic[];
  usedFallback: boolean;
}

const LIMITS = {
  durationMinMs: 16,
  durationMaxMs: 60_000,
  restartMinMs: 100,
  restartMaxMs: 300_000,
  repeatMax: 100,
  moveSpeedMax: 200,
  nestingMax: 16,
  nodesMax: 512,
  choicesMax: 32,
  transitionBudget: 2_048,
} as const;

const IDENTIFIER_PATTERN = /^[a-z][a-z0-9-]{0,63}$/;
const CSS_CLASS_PATTERN = /^[a-z_][a-z0-9_-]{0,63}$/i;
const OWN = Object.prototype.hasOwnProperty;
const COMPILED_PACKS = new WeakSet<object>();

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasOwn(value: object, key: PropertyKey): boolean {
  return OWN.call(value, key);
}

function isIntegerBetween(value: unknown, minimum: number, maximum: number): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= minimum && value <= maximum;
}

function isFiniteBetween(value: unknown, minimum: number, maximum: number): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum && value <= maximum;
}

function normalizeAssetPath(value: string): string | null {
  if (
    value.length === 0 ||
    value.length > 240 ||
    value.includes("\\") ||
    /[\u0000-\u001f\u007f]/.test(value) ||
    /^(?:[a-z]+:|\/)/i.test(value)
  ) return null;
  const normalized = value.startsWith("./") ? value.slice(2) : value;
  const parts = normalized.split("/");
  if (parts.some((part) => part === "" || part === "." || part === "..")) return null;
  return normalized;
}

function stableStringify(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
  if (isRecord(value)) {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

export function fnv1a(value: string): number {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash;
}

export function normalizeHerdrState(status: string): HerdrState {
  return (HERDR_STATES as readonly string[]).includes(status) ? (status as HerdrState) : "unknown";
}

export interface TrackPosition {
  x: number;
  direction: -1 | 1;
}

export function advanceTrack(
  x: number,
  direction: -1 | 1,
  distancePx: number,
  maximumX: number,
): TrackPosition {
  if (!Number.isFinite(maximumX) || maximumX <= 0) return { x: 0, direction };
  const start = Math.min(maximumX, Math.max(0, Number.isFinite(x) ? x : 0));
  if (!Number.isFinite(distancePx) || distancePx <= 0) return { x: start, direction };

  const period = maximumX * 2;
  const unfoldedStart = direction === 1 ? start : period - start;
  const phase = ((unfoldedStart + distancePx) % period + period) % period;
  if (phase === 0) return { x: 0, direction: 1 };
  if (phase === maximumX) return { x: maximumX, direction: -1 };
  return phase < maximumX
    ? { x: phase, direction: 1 }
    : { x: period - phase, direction: -1 };
}

export function remapTrackPosition(x: number, oldMaximumX: number, newMaximumX: number): number {
  if (!Number.isFinite(newMaximumX) || newMaximumX <= 0) return 0;
  if (!Number.isFinite(oldMaximumX) || oldMaximumX <= 0) return Math.min(newMaximumX, Math.max(0, x));
  return Math.min(newMaximumX, Math.max(0, (x / oldMaximumX) * newMaximumX));
}

function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  Object.freeze(value);
  for (const child of Object.values(value)) deepFreeze(child);
  return value;
}

interface ValidationContext {
  diagnostics: PackDiagnostic[];
  clips: Record<string, CompiledClip>;
  nodeCount: number;
}

function diagnostic(context: ValidationContext, code: string, path: string, message: string): void {
  context.diagnostics.push({ code, path, message });
}

function rejectUnknownFields(
  context: ValidationContext,
  value: Record<string, unknown>,
  allowed: readonly string[],
  path: string,
): void {
  for (const key of Object.keys(value)) {
    if (!allowed.includes(key)) diagnostic(context, "E_FIELD_UNKNOWN", `${path}/${key}`, "Unknown field");
  }
}

function validateNode(
  value: unknown,
  path: string,
  depth: number,
  context: ValidationContext,
): { node: FlowNode | null; minimumMs: number; maximumMs: number } {
  context.nodeCount += 1;
  if (context.nodeCount > LIMITS.nodesMax) {
    diagnostic(context, "E_NODE_LIMIT", path, `A pack may contain at most ${LIMITS.nodesMax} flow nodes`);
    return { node: null, minimumMs: 0, maximumMs: 0 };
  }
  if (depth > LIMITS.nestingMax) {
    diagnostic(context, "E_NESTING_LIMIT", path, `Flow nesting may not exceed ${LIMITS.nestingMax}`);
    return { node: null, minimumMs: 0, maximumMs: 0 };
  }
  if (!isRecord(value) || typeof value.type !== "string") {
    diagnostic(context, "E_NODE_TYPE", path, "Flow node must be an object with a supported type");
    return { node: null, minimumMs: 0, maximumMs: 0 };
  }

  if (value.type === "wait") {
    rejectUnknownFields(context, value, ["type", "durationMs"], path);
    if (!isIntegerBetween(value.durationMs, LIMITS.durationMinMs, LIMITS.durationMaxMs)) {
      diagnostic(context, "E_DURATION", `${path}/durationMs`, "Wait duration is outside the supported range");
      return { node: null, minimumMs: 0, maximumMs: 0 };
    }
    return { node: { type: "wait", durationMs: value.durationMs }, minimumMs: value.durationMs, maximumMs: value.durationMs };
  }

  if (value.type === "play") {
    rejectUnknownFields(context, value, ["type", "clip", "count"], path);
    const clip = typeof value.clip === "string" ? context.clips[value.clip] : undefined;
    if (!clip) diagnostic(context, "E_REF_MISSING", `${path}/clip`, "Play references an unknown clip");
    const count = value.count === undefined ? 1 : value.count;
    if (!isIntegerBetween(count, 1, LIMITS.repeatMax)) {
      diagnostic(context, "E_REPEAT_COUNT", `${path}/count`, "Play count must be a supported positive integer");
    }
    if (!clip || !isIntegerBetween(count, 1, LIMITS.repeatMax)) return { node: null, minimumMs: 0, maximumMs: 0 };
    const duration = clip.durationMs * count;
    return { node: { type: "play", clip: clip.name, count }, minimumMs: duration, maximumMs: duration };
  }

  if (value.type === "move") {
    rejectUnknownFields(context, value, ["type", "clip", "durationMs", "speedPxPerSecond"], path);
    const clip = typeof value.clip === "string" ? context.clips[value.clip] : undefined;
    if (!clip) diagnostic(context, "E_REF_MISSING", `${path}/clip`, "Move references an unknown clip");
    else if (clip.role !== "locomotion") diagnostic(context, "E_MOVE_CLIP_ROLE", `${path}/clip`, "Move requires a locomotion clip");
    else if (!clip.mirror) diagnostic(context, "E_FACING_UNSAFE", `${path}/clip`, "A locomotion clip must be safe to mirror at boundaries");
    if (!isIntegerBetween(value.durationMs, LIMITS.durationMinMs, LIMITS.durationMaxMs)) {
      diagnostic(context, "E_DURATION", `${path}/durationMs`, "Move duration is outside the supported range");
    }
    if (!isFiniteBetween(value.speedPxPerSecond, 1, LIMITS.moveSpeedMax)) {
      diagnostic(context, "E_MOVE_SPEED", `${path}/speedPxPerSecond`, "Movement speed is outside the supported range");
    }
    if (
      !clip ||
      clip.role !== "locomotion" ||
      !clip.mirror ||
      !isIntegerBetween(value.durationMs, LIMITS.durationMinMs, LIMITS.durationMaxMs) ||
      !isFiniteBetween(value.speedPxPerSecond, 1, LIMITS.moveSpeedMax)
    ) return { node: null, minimumMs: 0, maximumMs: 0 };
    return {
      node: { type: "move", clip: clip.name, durationMs: value.durationMs, speedPxPerSecond: value.speedPxPerSecond },
      minimumMs: value.durationMs,
      maximumMs: value.durationMs,
    };
  }

  if (value.type === "sequence") {
    rejectUnknownFields(context, value, ["type", "steps"], path);
    if (!Array.isArray(value.steps) || value.steps.length === 0 || value.steps.length > 64) {
      diagnostic(context, "E_SEQUENCE", `${path}/steps`, "Sequence must contain between 1 and 64 steps");
      return { node: null, minimumMs: 0, maximumMs: 0 };
    }
    const results = value.steps.map((step, index) => validateNode(step, `${path}/steps/${index}`, depth + 1, context));
    if (results.some((result) => result.node === null)) return { node: null, minimumMs: 0, maximumMs: 0 };
    return {
      node: { type: "sequence", steps: results.map((result) => result.node as FlowNode) },
      minimumMs: results.reduce((sum, result) => sum + result.minimumMs, 0),
      maximumMs: results.reduce((sum, result) => sum + result.maximumMs, 0),
    };
  }

  if (value.type === "repeat") {
    rejectUnknownFields(context, value, ["type", "count", "flow"], path);
    if (!isIntegerBetween(value.count, 1, LIMITS.repeatMax)) {
      diagnostic(context, "E_REPEAT_COUNT", `${path}/count`, "Repeat count must be a supported positive integer");
    }
    const child = validateNode(value.flow, `${path}/flow`, depth + 1, context);
    if (!isIntegerBetween(value.count, 1, LIMITS.repeatMax) || child.node === null) {
      return { node: null, minimumMs: 0, maximumMs: 0 };
    }
    return {
      node: { type: "repeat", count: value.count, flow: child.node },
      minimumMs: child.minimumMs * value.count,
      maximumMs: child.maximumMs * value.count,
    };
  }

  if (value.type === "choose") {
    rejectUnknownFields(context, value, ["type", "choices"], path);
    if (!Array.isArray(value.choices) || value.choices.length === 0 || value.choices.length > LIMITS.choicesMax) {
      diagnostic(context, "E_CHOICE", `${path}/choices`, `Choose must contain between 1 and ${LIMITS.choicesMax} branches`);
      return { node: null, minimumMs: 0, maximumMs: 0 };
    }
    const choices: ChoiceBranch[] = [];
    const durations: Array<{ minimumMs: number; maximumMs: number }> = [];
    value.choices.forEach((choice, index) => {
      const choicePath = `${path}/choices/${index}`;
      if (!isRecord(choice)) {
        diagnostic(context, "E_CHOICE", choicePath, "Choice branch must be an object");
        return;
      }
      rejectUnknownFields(context, choice, ["weight", "flow"], choicePath);
      if (!isIntegerBetween(choice.weight, 1, Number.MAX_SAFE_INTEGER)) {
        diagnostic(context, "E_CHOICE_WEIGHT", `${choicePath}/weight`, "Choice weight must be a positive integer");
      }
      const child = validateNode(choice.flow, `${choicePath}/flow`, depth + 1, context);
      if (isIntegerBetween(choice.weight, 1, Number.MAX_SAFE_INTEGER) && child.node) {
        choices.push({ weight: choice.weight, flow: child.node });
        durations.push(child);
      }
    });
    const weightTotal = choices.reduce((sum, choice) => sum + choice.weight, 0);
    if (!Number.isSafeInteger(weightTotal)) diagnostic(context, "E_CHOICE_WEIGHT", `${path}/choices`, "Choice weights overflow the safe integer range");
    if (choices.length !== value.choices.length || !Number.isSafeInteger(weightTotal)) {
      return { node: null, minimumMs: 0, maximumMs: 0 };
    }
    return {
      node: { type: "choose", choices },
      minimumMs: Math.min(...durations.map((duration) => duration.minimumMs)),
      maximumMs: Math.max(...durations.map((duration) => duration.maximumMs)),
    };
  }

  diagnostic(context, "E_NODE_TYPE", `${path}/type`, `Unsupported flow node type: ${value.type}`);
  return { node: null, minimumMs: 0, maximumMs: 0 };
}

export function compileBehaviorPack(
  input: unknown,
  assets?: Readonly<Record<string, string>>,
): PackCompilation {
  const context: ValidationContext = { diagnostics: [], clips: {}, nodeCount: 0 };
  if (!isRecord(input)) {
    return { pack: null, diagnostics: [{ code: "E_MANIFEST", path: "", message: "Pack manifest must be an object" }] };
  }
  rejectUnknownFields(context, input, ["formatVersion", "id", "packVersion", "clips", "states"], "");
  if (input.formatVersion !== 1) diagnostic(context, "E_FORMAT_VERSION", "/formatVersion", "Only formatVersion 1 is supported");
  if (typeof input.id !== "string" || !IDENTIFIER_PATTERN.test(input.id)) {
    diagnostic(context, "E_PACK_ID", "/id", "Pack id must be a lowercase identifier");
  }
  if (typeof input.packVersion !== "string" || input.packVersion.length < 1 || input.packVersion.length > 64) {
    diagnostic(context, "E_PACK_VERSION", "/packVersion", "Pack version must be a short non-empty string");
  }

  if (!isRecord(input.clips) || Object.keys(input.clips).length === 0) {
    diagnostic(context, "E_CLIPS", "/clips", "Pack must define at least one clip");
  } else {
    for (const [name, value] of Object.entries(input.clips)) {
      const path = `/clips/${name}`;
      if (!IDENTIFIER_PATTERN.test(name) || !isRecord(value)) {
        diagnostic(context, "E_CLIP", path, "Clip must have a lowercase identifier and object definition");
        continue;
      }
      rejectUnknownFields(context, value, ["durationMs", "role", "asset", "cssClass", "sourceFacing", "mirror"], path);
      const durationMs = value.durationMs;
      const role = value.role;
      const assetPath = typeof value.asset === "string" ? normalizeAssetPath(value.asset) : null;
      const cssClass = value.cssClass === undefined ? `clip-${name}` : value.cssClass;
      const sourceFacing = value.sourceFacing === undefined ? "right" : value.sourceFacing;
      const mirror = value.mirror === undefined ? true : value.mirror;
      if (!isIntegerBetween(durationMs, LIMITS.durationMinMs, LIMITS.durationMaxMs)) {
        diagnostic(context, "E_DURATION", `${path}/durationMs`, "Clip duration is outside the supported range");
      }
      if (value.asset !== undefined && assetPath === null) diagnostic(context, "E_PATH_ESCAPE", `${path}/asset`, "Asset must be a safe pack-relative path");
      if (assetPath && assets && !hasOwn(assets, assetPath)) diagnostic(context, "E_ASSET_MISSING", `${path}/asset`, "Referenced asset is missing from the pack");
      if (role !== "stationary" && role !== "locomotion") diagnostic(context, "E_CLIP_ROLE", `${path}/role`, "Clip role is invalid");
      if (typeof cssClass !== "string" || !CSS_CLASS_PATTERN.test(cssClass)) diagnostic(context, "E_CSS_CLASS", `${path}/cssClass`, "Clip CSS class is invalid");
      if (sourceFacing !== "left" && sourceFacing !== "right") diagnostic(context, "E_SOURCE_FACING", `${path}/sourceFacing`, "Source facing is invalid");
      if (typeof mirror !== "boolean") diagnostic(context, "E_MIRROR", `${path}/mirror`, "Mirror flag must be boolean");
      if (
        isIntegerBetween(durationMs, LIMITS.durationMinMs, LIMITS.durationMaxMs) &&
        (role === "stationary" || role === "locomotion") &&
        typeof cssClass === "string" && CSS_CLASS_PATTERN.test(cssClass) &&
        (sourceFacing === "left" || sourceFacing === "right") &&
        typeof mirror === "boolean"
      ) context.clips[name] = {
        name,
        durationMs,
        role,
        assetPath,
        assetUrl: assetPath && assets ? assets[assetPath] ?? null : null,
        cssClass,
        sourceFacing,
        mirror,
      };
    }
  }

  const compiledStates = {} as Record<HerdrState, StateFlowManifest>;
  if (!isRecord(input.states)) {
    diagnostic(context, "E_STATES", "/states", "Pack must define all Herdr states");
  } else {
    for (const key of Object.keys(input.states)) {
      if (!(HERDR_STATES as readonly string[]).includes(key)) diagnostic(context, "E_STATE_UNKNOWN", `/states/${key}`, "Only canonical Herdr states may select behavior");
    }
    for (const state of HERDR_STATES) {
      const stateValue = input.states[state];
      const statePath = `/states/${state}`;
      if (!isRecord(stateValue)) {
        diagnostic(context, "E_STATE_MISSING", statePath, "Required Herdr state is missing");
        continue;
      }
      rejectUnknownFields(context, stateValue, ["completion", "flow"], statePath);
      if (stateValue.completion !== "restart" && stateValue.completion !== "hold") {
        diagnostic(context, "E_COMPLETION", `${statePath}/completion`, "Completion must be restart or hold");
      }
      const result = validateNode(stateValue.flow, `${statePath}/flow`, 1, context);
      if (result.minimumMs <= 0) diagnostic(context, "E_ZERO_TIME_CYCLE", `${statePath}/flow`, "Every state path must consume logical time");
      if (!Number.isSafeInteger(result.minimumMs) || !Number.isSafeInteger(result.maximumMs)) {
        diagnostic(context, "E_DURATION_OVERFLOW", `${statePath}/flow`, "Flow duration exceeds the safe logical-time range");
      }
      if (
        stateValue.completion === "restart" &&
        (result.minimumMs < LIMITS.restartMinMs || result.maximumMs > LIMITS.restartMaxMs)
      ) diagnostic(context, "E_STATE_CYCLE_DURATION", statePath, "Restarting state cycle is outside the supported duration range");
      if ((stateValue.completion === "restart" || stateValue.completion === "hold") && result.node) {
        compiledStates[state] = { completion: stateValue.completion, flow: result.node };
      }
    }
  }

  if (context.diagnostics.length > 0 || HERDR_STATES.some((state) => !hasOwn(compiledStates, state))) {
    return { pack: null, diagnostics: context.diagnostics };
  }
  const fingerprint = fnv1a(stableStringify(input)).toString(16).padStart(8, "0");
  const pack: CompiledBehaviorPack = deepFreeze({
    formatVersion: 1,
    id: input.id as string,
    packVersion: input.packVersion as string,
    fingerprint,
    clips: context.clips,
    states: compiledStates,
  });
  COMPILED_PACKS.add(pack);
  return { pack, diagnostics: [] };
}

const BUILT_IN_MANIFEST: BehaviorPackManifest = {
  formatVersion: 1,
  id: "built-in-safe",
  packVersion: "1",
  clips: {
    loaf: { durationMs: 1_200, role: "stationary", cssClass: "clip-loaf" },
    busy: { durationMs: 700, role: "stationary", cssClass: "clip-busy" },
    waiting: { durationMs: 1_000, role: "stationary", cssClass: "clip-waiting" },
    celebrate: { durationMs: 900, role: "stationary", cssClass: "clip-celebrate" },
    walk: { durationMs: 800, role: "locomotion", cssClass: "clip-walk", mirror: true },
  },
  states: {
    idle: { completion: "restart", flow: { type: "sequence", steps: [{ type: "move", clip: "walk", durationMs: 2_400, speedPxPerSecond: 28 }, { type: "play", clip: "loaf" }] } },
    working: { completion: "restart", flow: { type: "sequence", steps: [{ type: "move", clip: "walk", durationMs: 3_000, speedPxPerSecond: 38 }, { type: "play", clip: "busy" }] } },
    blocked: { completion: "restart", flow: { type: "sequence", steps: [{ type: "play", clip: "waiting" }, { type: "wait", durationMs: 300 }] } },
    done: { completion: "hold", flow: { type: "play", clip: "celebrate" } },
    unknown: { completion: "restart", flow: { type: "play", clip: "loaf" } },
  },
};

const builtInCompilation = compileBehaviorPack(BUILT_IN_MANIFEST);
if (!builtInCompilation.pack) throw new Error("Built-in behavior pack is invalid");
export const BUILT_IN_BEHAVIOR_PACK = builtInCompilation.pack;

export function resolveBehaviorPack(input: unknown): ResolvedBehaviorPack {
  if (isCompiledPack(input)) return { pack: input, diagnostics: [], usedFallback: false };
  const result = compileBehaviorPack(input);
  return result.pack
    ? { pack: result.pack, diagnostics: [], usedFallback: false }
    : { pack: BUILT_IN_BEHAVIOR_PACK, diagnostics: result.diagnostics, usedFallback: true };
}

function isCompiledPack(value: unknown): value is CompiledBehaviorPack {
  return isRecord(value) && COMPILED_PACKS.has(value);
}

interface PendingNode {
  node: FlowNode;
  path: string;
}

interface ActiveAction {
  kind: "play" | "move" | "wait";
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
  held: boolean;
  failed: boolean;
}

export interface FlowAdvance {
  distancePx: number;
  sample: FlowSample;
}

export class BehaviorMachine {
  private state: HerdrState;
  private stateEntryOrdinal = 0;
  private pending: PendingNode[] = [];
  private active: ActiveAction | null = null;
  private currentClip: string | null = null;
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

  advance(deltaMs: number): FlowAdvance {
    const duration = Number.isFinite(deltaMs) ? Math.max(0, Math.floor(deltaMs)) : 0;
    let remaining = duration;
    let distancePx = 0;
    let transitions = 0;

    while (!this.failed && (remaining > 0 || !this.active) && !this.held) {
      if (!this.active) {
        transitions += this.startNextAction();
        if (transitions > LIMITS.transitionBudget || (!this.active && !this.held)) {
          this.activateFallback();
          break;
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

    return { distancePx, sample: this.sample() };
  }

  sample(): FlowSample {
    const clipName = this.active?.clip ?? this.currentClip;
    return {
      state: this.state,
      clip: clipName ? this.pack.clips[clipName] ?? null : null,
      clipElapsedMs: this.active?.elapsedMs ?? 0,
      clipEpoch: this.active?.epoch ?? this.actionEpoch,
      moving: this.active?.kind === "move",
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
    this.nodeVisits.clear();
    if (!preserveClip) this.currentClip = null;
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
        const clip = node.type === "wait" ? null : node.clip;
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
        if (clip) this.currentClip = clip;
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
  }
}
