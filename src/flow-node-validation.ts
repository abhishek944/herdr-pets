import { LIMITS, type ChoiceBranch, type CompiledClip, type FlowNode, type PackDiagnostic } from "./flow-types";
import { isFiniteBetween, isIntegerBetween, isRecord } from "./flow-utils";

export interface ValidationContext {
  diagnostics: PackDiagnostic[];
  clips: Record<string, CompiledClip>;
  nodeCount: number;
}

export function diagnostic(context: ValidationContext, code: string, path: string, message: string): void {
  context.diagnostics.push({ code, path, message });
}

export function rejectUnknownFields(
  context: ValidationContext,
  value: Record<string, unknown>,
  allowed: readonly string[],
  path: string,
): void {
  for (const key of Object.keys(value)) {
    if (!allowed.includes(key)) diagnostic(context, "E_FIELD_UNKNOWN", `${path}/${key}`, "Unknown field");
  }
}

export function validateNode(
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

  if (value.type === "wait" || value.type === "hide") {
    rejectUnknownFields(context, value, ["type", "durationMs"], path);
    if (!isIntegerBetween(value.durationMs, LIMITS.durationMinMs, LIMITS.durationMaxMs)) {
      diagnostic(context, "E_DURATION", `${path}/durationMs`, `${value.type === "hide" ? "Hide" : "Wait"} duration is outside the supported range`);
      return { node: null, minimumMs: 0, maximumMs: 0 };
    }
    return {
      node: { type: value.type, durationMs: value.durationMs },
      minimumMs: value.durationMs,
      maximumMs: value.durationMs,
    };
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
