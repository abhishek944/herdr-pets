/**
 * Pet Street Rollercoaster — per-agent cart props for the four approved modes.
 *
 * Approved source: var/street-themes/ui-ux-grill-me/themes-coaster (mockups and screenshots
 * B1-B Low Rolling, B2-B Twilight Carnival, B2-D Firefly Gardens, B2-E Cloudline) with
 * result.md, visual-decision-tree.md, selection-B1.json, selection-B2.json and the shared
 * ../themes-parent/visual-contract.md. Frame is the shared prop contract viewBox 0 0 104 100:
 * pet centre x=52 (pet box x30..74, y40..84), wheel/floor baseline y=90. Ground modes ride on
 * that baseline. Cloudline keeps the source hanging geometry: suspension bars meet the raised
 * rail at y=40 and the gondola carries the pet, so the root reports its rail reference through
 * data-coaster-rail-y and the tool group reports its rotation pivot. Props are decorative,
 * static, pointer-transparent and never contain pet art, names or caller text. The parent
 * scopes ids (this module defines none), mounts the SVG behind real actors and rotates
 * data-activity-tool about the reported pivot. The source's top inner cart highlight is drawn
 * as a light rim line. Floating state markers are omitted per the latest refinement request;
 * existing agent text labels remain the caller's responsibility.
 */

import type { CoasterMode } from "./activity-coaster-art";

export interface CoasterPalette {
  /** Land gradient base of the mode. */
  land: string;
  /** Rail supports, station bases and drawn structure outlines. */
  post: string;
  /** Painted rail face and structural tops. */
  rail: string;
  /** Cart colour while the agent is riding. */
  car: string;
}

export const COASTER_PALETTE: Record<CoasterMode, CoasterPalette> = {
  "low-rolling": { land: "#4d6f5f", post: "#5d4740", rail: "#d39a69", car: "#d39a69" },
  "twilight-carnival": { land: "#4e526e", post: "#66668f", rail: "#cca0cb", car: "#cca0cb" },
  "firefly-gardens": { land: "#3a6355", post: "#42675b", rail: "#9db894", car: "#9db894" },
  cloudline: { land: "#445f76", post: "#658d9f", rail: "#a5dae1", car: "#a5dae1" },
};

/** Compatibility fallback for a stored preference that carries an unknown mode. */
export function coasterPalette(mode: CoasterMode): CoasterPalette {
  return COASTER_PALETTE[mode] ?? COASTER_PALETTE["low-rolling"];
}

const BLOCKED_CAR = "#f1c46f";
const IDLE_CAR = "#829a9a";
const DONE_CAR = "#85bd9b";
const CUE_STOP = "#f5d28e";
const CUE_DONE = "#bfe9ca";
const TIRE = "#172733";
const RIM = "#c8d4d9";
const HUB = "#8fa2ab";
const BAR = "#b9d3d6";

/** State vocabulary shared with the renderer; unrecognised states rest as a plain cart. */
export type CoasterPropState = "working" | "blocked" | "done" | "idle" | "rest";

export function coasterPropState(state: string): CoasterPropState {
  const value = (state ?? "").toLowerCase();
  if (/(block|wait|input|held|stuck|error|fail|denied)/.test(value)) return "blocked";
  if (/(done|finish|complete|success|approv|closed)/.test(value)) return "done";
  if (/(idle|ready|rest|away|sleep)/.test(value)) return "idle";
  if (/(work|build|busy|active|running|progress|listen|speak|talk)/.test(value)) return "working";
  return "rest";
}

const wheel = (x: number): string =>
  `<circle cx="${x}" cy="86.5" r="3.5" fill="${TIRE}"/>` +
  `<circle cx="${x}" cy="86.5" r="2.6" fill="${RIM}"/>` +
  `<circle cx="${x}" cy="86.5" r="1.1" fill="${HUB}"/>`;

const cartHighlight =
  `<path d="M37 74.5H67" fill="none" stroke="#ffffff" stroke-opacity=".28"` +
  ` stroke-width="2.5" stroke-linecap="round"/>`;

/** Ground cart: wheels on the y=90 baseline, body behind the seated pet. */
const groundCart = (car: string, edge: string): string =>
  `<g data-activity-tool="coaster-car" data-pivot-x="52" data-pivot-y="90">` +
  wheel(38.5) +
  wheel(65.5) +
  `<path d="M35.5 71H68.5Q71 71 71 73.5V79A7 7 0 0 1 64 85H40A7 7 0 0 1 33 79V73.5Q33 71` +
  ` 35.5 71Z" fill="${car}" stroke="${edge}" stroke-width="2"/>` +
  cartHighlight +
  `</g>`;

/** Cloudline gondola: bars reach the raised rail at y=40, deeper rounded tub, feet in the car. */
const hangingCart = (car: string, edge: string): string =>
  `<g data-activity-tool="coaster-gondola" data-pivot-x="52" data-pivot-y="40">` +
  `<rect x="39" y="40" width="2" height="32" fill="${BAR}"/>` +
  `<rect x="63" y="40" width="2" height="32" fill="${BAR}"/>` +
  `<path d="M35.5 71H68.5Q71 71 71 73.5V81A10 10 0 0 1 61 91H43A10 10 0 0 1 33 81V73.5Q33 71` +
  ` 35.5 71Z" fill="${car}" stroke="${edge}" stroke-width="2"/>` +
  cartHighlight +
  `</g>`;

/** Blocked: striped material safety board on a ground post, without a floating glyph. */
const stopBoard = (post: string, hanging: boolean): string =>
  (hanging ? "" : `<rect x="9" y="78" width="2" height="12" fill="${post}"/>`) +
  `<rect x="7" y="77" width="27" height="11" rx="2" fill="${CUE_STOP}"/>` +
  `<path d="M13 78.5l-3.6 8.5M20 78.5l-3.6 8.5M27 78.5l-3.6 8.5" fill="none"` +
  ` stroke="#4a4436" stroke-width="2.2"/>`;

/** Done: material finish flag from the approved platform, without a floating check. */
const finishFlag = (post: string): string =>
  `<rect x="88" y="52" width="2" height="38" fill="${post}"/>` +
  `<path d="M90 52H102V61H90Z" fill="${CUE_DONE}"/>` +
  `<path d="M90 52H94V56.5H90ZM98 52H102V56.5H98ZM94 56.5H98V61H94Z" fill="${post}"` +
  ` fill-opacity=".55"/>`;

export function createCoasterProps(state: string, mode: CoasterMode): string {
  const palette = coasterPalette(mode);
  const variant = coasterPropState(state);
  const hanging = mode === "cloudline";
  const car =
    variant === "blocked"
      ? BLOCKED_CAR
      : variant === "idle"
        ? IDLE_CAR
        : variant === "done"
          ? DONE_CAR
          : palette.car;
  const cue =
    variant === "blocked"
      ? stopBoard(palette.post, hanging)
      : variant === "done"
        ? finishFlag(palette.post)
        : "";
  return (
    `<svg viewBox="0 0 104 100" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"` +
    ` pointer-events="none" data-coaster-mode="${mode}"` +
    ` data-coaster-rail-y="${hanging ? 40 : 90}">` +
    (hanging ? hangingCart(car, palette.post) : groundCart(car, palette.post)) +
    cue +
    `</svg>`
  );
}
