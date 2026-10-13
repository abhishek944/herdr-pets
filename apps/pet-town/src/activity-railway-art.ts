/**
 * Tiny Railway — Stone Viaduct Inspection static strip scenery.
 *
 * Approved source: var/street-themes/ui-ux-grill-me/themes-extra/mockups/maintenance.html
 * (?theme=bridge) with screenshots/B3-D-bridge.png (1280x290, device scale 1), selection-B3.json
 * and result.md; shared contract in ../themes-parent/visual-contract.md. Geometry, palette and
 * materials are transcribed from that source: shallow stone viaduct, ballast, wood sleepers,
 * bolted steel rails, one quiet edge shed, lamp and mile marker. The desktop gradient proxy,
 * concept captions, pets, badges and per-agent trolleys stay out of this factory; the parent
 * scopes ids and mounts the SVG behind real actors. Upper desktop space stays transparent.
 */

const GROUND = "#6c6960";
const TIE = "#9f927b";
const WOOD = "#cfbb92";
const ACCENT = "#deac71";
const STEEL = "#a9b8af";

export type { RailwayPropState } from "./activity-railway-props";
export { createRailwayProps, railwayPropState } from "./activity-railway-props";

const defs = (): string =>
  `<defs>` +
  `<linearGradient id="railway-bed" x2="0" y2="1"><stop stop-color="${GROUND}" stop-opacity=".78"/>` +
  `<stop offset="1" stop-color="${GROUND}" stop-opacity=".48"/></linearGradient>` +
  `<linearGradient id="railway-metal" x2="0" y2="1"><stop stop-color="#d7e1d6"/>` +
  `<stop offset=".45" stop-color="${STEEL}"/><stop offset="1" stop-color="#3d5357"/>` +
  `</linearGradient>` +
  `<linearGradient id="railway-timber" x2="0" y2="1"><stop stop-color="${WOOD}"/>` +
  `<stop offset="1" stop-color="${TIE}"/></linearGradient>` +
  `</defs>`;

const bolt = (x: number, y: number): string =>
  `<circle cx="${x}" cy="${y}" r="2.5" fill="#24383b"/>` +
  `<path d="M${x - 1} ${y} h2" fill="none" stroke="#d9d7bd" stroke-width=".8"` +
  ` stroke-linecap="round"/>`;

/** Deterministic ballast scatter, not a generated image or a new asset pack. */
const ballast = (): string => {
  let art = "";
  for (let i = 0; i < 180; i++) {
    const x = (i * 127 + 23) % 1280;
    const y = 240 + ((i * 17) % 22);
    const fill = i % 3 === 0 ? STEEL : TIE;
    art += `<ellipse cx="${x}" cy="${y}" rx="${1 + (i % 3)}" ry="1" fill="${fill}" opacity=".35"/>`;
  }
  return art;
};

const sleeper = (x: number): string =>
  `<rect x="${x}" y="222" width="17" height="22" rx="2" fill="url(#railway-timber)"/>` +
  `<path d="M${x + 3} 225 v15 m7 -14 v12" fill="none" stroke="#483f36" stroke-width=".7"/>` +
  bolt(x + 8, 225) +
  bolt(x + 8, 240);

const sleepers = (): string => {
  let art = "";
  for (let x = 0; x < 1280; x += 29) art += sleeper(x);
  return art;
};

/** Stone band, shallow arches and pier detail of the approved viaduct. */
const viaduct = (): string => {
  let art = `<rect x="0" y="238" width="1280" height="52" fill="#a18e74" opacity=".44"/>`;
  for (let x = 0; x < 1280; x += 120) {
    art +=
      `<path d="M${x + 15} 290 v-23 q45 -36 90 0 v23" fill="#263c47" opacity=".56"/>` +
      `<path d="M${x + 4} 247 h112 m-106 10 h12 m83 0 h14" fill="none" stroke="${WOOD}"` +
      ` stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>`;
  }
  return art;
};

const rails = (): string =>
  `<rect x="0" y="222" width="1280" height="4" rx="1" fill="url(#railway-metal)"/>` +
  `<rect x="0" y="238" width="1280" height="4" rx="1" fill="url(#railway-metal)"/>` +
  `<path d="M0 222 H1280 M0 238 H1280" fill="none" stroke="#dce1d3" stroke-width=".7"/>`;

/** Quiet edge-world detail: one low shed, a lamp and a matching mile marker. */
const edgeWorld = (): string =>
  `<rect x="31" y="209" width="52" height="25" rx="2" fill="#536761"/>` +
  `<path d="M26 209 l31 -14 l31 14Z" fill="${TIE}"/>` +
  `<rect x="39" y="214" width="12" height="10" rx="1" fill="${ACCENT}" opacity=".7"/>` +
  `<rect x="59" y="214" width="14" height="19" rx="1" fill="#2b4145"/>` +
  `<path d="M112 229 v-38 h12" fill="none" stroke="${STEEL}" stroke-width="2"` +
  ` stroke-linecap="round"/>` +
  `<rect x="117" y="190" width="13" height="6" rx="2" fill="${WOOD}"/>` +
  `<circle cx="124" cy="197" r="4" fill="${ACCENT}"/>` +
  `<rect x="1204" y="214" width="20" height="24" rx="2" fill="#aeb9a4"/>` +
  `<text x="1214" y="224" fill="#e1e7df" text-anchor="middle" font-size="10"` +
  ` font-family="-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif">04</text>`;

const BALLAST_STONES = ballast();
const SLEEPER_ROW = sleepers();

export function createRailwayArt(): string {
  return (
    `<svg viewBox="0 0 1280 290" preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg"` +
    ` aria-hidden="true" pointer-events="none">` +
    defs() +
    `<path d="M0 238 Q140 226 300 233 T650 231 T1000 235 T1280 232 V290 H0Z"` +
    ` fill="url(#railway-bed)"/>` +
    BALLAST_STONES +
    viaduct() +
    `<path d="M0 237 H1280" fill="none" stroke="${WOOD}" stroke-width="5" stroke-linecap="round"/>` +
    SLEEPER_ROW +
    rails() +
    edgeWorld() +
    `</svg>`
  );
}
