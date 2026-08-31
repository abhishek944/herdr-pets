export const HERDR_STATES = ["idle", "working", "blocked", "done", "unknown"] as const;

export type HerdrState = (typeof HERDR_STATES)[number];
export type ClipRole = "stationary" | "locomotion";
export type SourceFacing = "left" | "right";

export interface ClipManifest {
  durationMs: number;
  role: ClipRole;
  asset?: string;
  holdAsset?: string;
  sourceFacing?: SourceFacing;
  mirror?: boolean;
  scale?: number;
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

export interface HideNode {
  type: "hide";
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

export type FlowNode = SequenceNode | PlayNode | MoveNode | WaitNode | HideNode | ChooseNode | RepeatNode;

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
  holdAssetPath: string | null;
  holdAssetUrl: string | null;
  sourceFacing: SourceFacing;
  mirror: boolean;
  scale: number;
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

export const LIMITS = {
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
