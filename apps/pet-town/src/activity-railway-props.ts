/**
 * Tiny Railway — Stone Viaduct Inspection per-agent repair props.
 *
 * Approved source: var/street-themes/ui-ux-grill-me/themes-extra/mockups/maintenance.html
 * (?theme=bridge) with screenshots/B3-D-bridge.png (1280x290, device scale 1), selection-B3.json
 * and result.md. Full native transcription: source pet centre x maps to 52 and the source wheel
 * bottom y=236 maps to y=90, so local = (sourceX - centreX + 52, sourceY - 146). Wheels stay r=7
 * at cy=83, the deck stays 76x10 at y=61..71 with its wood grain, the undercarriage bar 82x5 at
 * y=72..77, and every fixture keeps its source offsets, sizes, fills and layering.
 *
 * Frame contract: viewBox remains 0 0 104 100 and the wheel/floor baseline remains y=90, but
 * containment is deliberately relaxed. The parent's .activity-props CSS uses overflow: visible, so
 * approved fixtures may extend past x=104: the working joint reaches x=134, the blocked barricade
 * x=150 and the done bench reaches x=194. Extra state signals and the paper check glyph are
 * omitted per the latest refinement request; material fixtures stay intact. The bridge
 * per-pet timber post sits at x=-16..-3. Props are decorative, static, pointer-transparent, have
 * no filter and no clock, and never contain pet art, names or caller text; the source drop-shadow
 * filter is replaced by one static shadow ellipse. The parent scopes ids, mounts the SVG, keeps
 * real badges and pets, and rotates the data-activity-tool group about its reported pivot.
 */

const WOOD = "#cfbb92";
const TIE = "#9f927b";
const STEEL = "#a9b8af";
const ACCENT = "#deac71";

export type RailwayPropState = "working" | "blocked" | "done" | "idle";

/** Keyword normalisation: any string is accepted and no caller text reaches the markup. */
export function railwayPropState(state: string): RailwayPropState {
  const value = state.toLowerCase();
  if (/(block|wait|input|held|stuck|error|fail)/.test(value)) return "blocked";
  if (/(done|finish|complete|success|approv|closed)/.test(value)) return "done";
  if (/(work|build|busy|active|running|progress)/.test(value)) return "working";
  return "idle";
}

const propDefs = (): string =>
  `<defs><linearGradient id="railway-prop-timber" x2="0" y2="1">` +
  `<stop stop-color="${WOOD}"/><stop offset="1" stop-color="${TIE}"/>` +
  `</linearGradient></defs>`;

const rect = (x: number, y: number, w: number, h: number, fill: string, rx = 0): string =>
  `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${rx}" fill="${fill}"/>`;

const line = (d: string, stroke: string, width: number): string =>
  `<path d="${d}" fill="none" stroke="${stroke}" stroke-width="${width}"` +
  ` stroke-linecap="round" stroke-linejoin="round"/>`;

const circle = (x: number, y: number, r: number, fill: string): string =>
  `<circle cx="${x}" cy="${y}" r="${r}" fill="${fill}"/>`;

const bolt = (x: number, y: number): string =>
  circle(x, y, 2.5, "#24383b") + line(`M${x - 1} ${y} h2`, "#d9d7bd", 0.8);

const wheel = (x: number): string =>
  circle(x, 83, 7, "#1b2c32") +
  circle(x, 83, 4, STEEL) +
  circle(x, 83, 1.7, "#3d5357") +
  line(`M${x - 5} 83 h10 m-5 -5 v10`, "#526768", 0.8);

/** Source toolbox: body, steel lid, accent latch, steel handle and optional open lid arms. */
const toolBox = (x: number, y: number, open: boolean): string =>
  rect(x, y, 29, 18, "#587574", 3) +
  rect(x - 1, y - 4, 31, 5, STEEL, 2) +
  rect(x + 11, y + 3, 7, 5, ACCENT, 1) +
  line(`M${x + 9} ${y - 4} v-4 h10 v4`, STEEL, 2) +
  (open ? line(`M${x + 6} ${y - 4} l2 -12 m9 12 l-4 -10`, WOOD, 2) : "");

/** Approved per-pet trolley at native source scale: deck, bar, wheels, steel post, toolbox. */
const trolley = (open: boolean): string =>
  `<ellipse cx="54" cy="88.6" rx="42" ry="2.4" fill="#0e1b22" opacity=".22"/>` +
  rect(16, 61, 76, 10, "url(#railway-prop-timber)", 3) +
  rect(13, 72, 82, 5, STEEL, 2) +
  wheel(29) +
  wheel(79) +
  line("M25 63 v-18 h17", STEEL, 2) +
  toolBox(77, 44, open);

/** Working: wrench on the bolted rail joint with its clip, lever and accent ball. */
const wrenchTool = (): string =>
  `<g data-activity-tool="rail-fastening-wrench" data-pivot-x="67" data-pivot-y="42">` +
  `<g transform="translate(67 42) rotate(-41)">` +
  line("M0 0 l0 21", "#d4dacf", 3) +
  line("M-4 -5 v5 q4 6 8 0 v-5", "#d4dacf", 2) +
  circle(0, 22, 3, "#d4dacf") +
  circle(0, 22, 1.3, "#526966") +
  `</g></g>`;

const workingFixture = (): string =>
  rect(95, 74, 39, 6, "#354f52", 2) +
  line("M95 76 h39", ACCENT, 2) +
  bolt(100, 77) +
  bolt(127, 77) +
  rect(105, 60, 7, 16, STEEL, 1) +
  line("M108 59 l14 -6", WOOD, 2) +
  circle(122, 53, 2, ACCENT);

/** Blocked: striped safety barricade and locked case on the toolbox, without a state signal. */
const blockedFixture = (): string =>
  line("M96 81 l7 -22 h39 l8 22", STEEL, 3) +
  rect(101, 58, 43, 8, ACCENT, 2) +
  line("M109 59 l-4 6 M121 59 l-4 6 M133 59 l-4 6", "#3f514e", 4) +
  rect(84, 50, 14, 10, "#21383b", 2) +
  line("M88 50 v-3 q3 -5 6 0 v3", ACCENT, 1.5);

/** Done: bench, parked toolbox and ruled inspection card, without extra check glyphs. */
const doneFixture = (): string =>
  rect(107, 80, 87, 7, "url(#railway-prop-timber)", 2) +
  line("M116 86 v9 m67 -9 v9", STEEL, 3) +
  toolBox(114, 62, false) +
  rect(153, 50, 23, 27, "#d5d1b5", 2) +
  line("M157 58 h15 m-15 4 h8", TIE, 1);

/** Bridge per-pet timber post, drawn last exactly as in the source loop. */
const timberPost = (): string => rect(-16, 55, 13, 31, WOOD, 2) + line("M-10 57 h27", WOOD, 2);

/**
 * Approved B20-D machine group: per-pet trolley, toolbox and attached state hardware scale to 25%
 * about the native floor centre 52,90 so the wheel floor stays grounded at y90, while the timber
 * post, viaduct, labels and pet artwork keep their size.
 */
const machineGroup = (fixture: string, open: boolean): string =>
  `<g data-machine-scale="0.25" transform="translate(52 90) scale(0.25) translate(-52 -90)">` +
  trolley(open) +
  fixture +
  `</g>`;

export function createRailwayProps(state: string, dedicated = false): string {
  const variant = railwayPropState(state);
  const fixture =
    variant === "working"
      ? // Dedicated sidecar art holds its own ratchet, so the detached tool is never duplicated.
        (dedicated ? "" : wrenchTool()) + workingFixture()
      : variant === "blocked"
        ? blockedFixture()
        : variant === "done"
          ? doneFixture()
          : "";
  return (
    `<svg viewBox="0 0 104 100" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"` +
    ` pointer-events="none">` +
    propDefs() +
    machineGroup(fixture, variant === "working") +
    timberPost() +
    `</svg>`
  );
}
