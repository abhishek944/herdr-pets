import { compileBehaviorPack, isCompiledPack } from "./behavior-pack";
import type { BehaviorPackManifest, ResolvedBehaviorPack } from "./flow-types";

const BUILT_IN_MANIFEST: BehaviorPackManifest = {
  formatVersion: 1,
  id: "built-in-safe",
  packVersion: "1",
  clips: {
    loaf: { durationMs: 1_200, role: "stationary" },
    busy: { durationMs: 700, role: "stationary" },
    waiting: { durationMs: 1_000, role: "stationary" },
    celebrate: { durationMs: 900, role: "stationary" },
    walk: { durationMs: 800, role: "locomotion", mirror: true },
  },
  states: {
    idle: { completion: "restart", flow: { type: "hide", durationMs: 1_000 } },
    working: { completion: "restart", flow: { type: "hide", durationMs: 1_000 } },
    blocked: { completion: "restart", flow: { type: "hide", durationMs: 1_000 } },
    done: { completion: "hold", flow: { type: "hide", durationMs: 1_000 } },
    unknown: { completion: "restart", flow: { type: "hide", durationMs: 1_000 } },
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
