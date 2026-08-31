import { HERDR_STATES, type HerdrState } from "./flow-types";

export const IDENTIFIER_PATTERN = /^[a-z][a-z0-9-]{0,63}$/;
const OWN = Object.prototype.hasOwnProperty;

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function hasOwn(value: object, key: PropertyKey): boolean {
  return OWN.call(value, key);
}

export function isIntegerBetween(value: unknown, minimum: number, maximum: number): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= minimum && value <= maximum;
}

export function isFiniteBetween(value: unknown, minimum: number, maximum: number): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum && value <= maximum;
}

export function normalizeAssetPath(value: string): string | null {
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

export function stableStringify(value: unknown): string {
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

export function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  Object.freeze(value);
  for (const child of Object.values(value)) deepFreeze(child);
  return value;
}
