/**
 * Pet Street Rollercoaster — four approved scenic modes for the bottom desktop strip.
 *
 * Approved source: var/street-themes/ui-ux-grill-me/themes-coaster — screenshots and mockups
 * B1-B (low rolling), B2-B (twilight carnival), B2-D (firefly gardens), B2-E (cloudline), with
 * result.md, visual-decision-tree.md, selection-B1.json, selection-B2.json and the shared
 * ../themes-parent/visual-contract.md. Track shape, supports, palettes, station/finish
 * platforms and scenic stations are transcribed from those sources; the concept desktop
 * gradient, pets, badges and captions stay out of this factory, so the upper strip stays
 * transparent. Lower band only: land at y>=248, ground rail around y=197..244, raised cloudline
 * rail at y=157..204, everything well inside the 1280x290 design box and shallow for the
 * narrow 1100px strip. The parent scopes ids, mounts this SVG behind real actors, samples
 * data-activity-rail (monotonic in x) for vertical placement and mounts createCoasterProps per
 * agent. State carts, stop boards and finish cues live in activity-coaster-props.ts.
 */

import { coasterPalette } from "./activity-coaster-props";

export { createCoasterProps } from "./activity-coaster-props";

/** Must stay identical to the persisted coaster union in preferences-types.ts. */
export type CoasterMode = "low-rolling" | "twilight-carnival" | "firefly-gardens" | "cloudline";

const LAND_PATH = "M0 259Q130 253 300 268T640 264T980 262T1280 264V290H0Z";
/** Matched cubic handles give equal endpoint derivatives at every join; x stays increasing. */
const GROUND_RAIL =
  "M-20 244C50 244 105 238 145 227C185 216 245 229 300 239C355 249 390 231 432 211" +
  "C474 191 500 193 530 197C560 201 602 203 635 218C668 233 731 238 795 237" +
  "C859 236 895 236 930 233C965 230 1021 222 1080 216C1139 210 1207 222 1300 244";
// Bake the offset into d: getPointAtLength samples local coordinates, not SVG transforms.
const CLOUD_RAIL = GROUND_RAIL.replace(/(-?\d+) (-?\d+)/g, (_, x, y) => `${x} ${Number(y) - 40}`);

type Support = readonly [x: number, top: number];
const GROUND_SUPPORTS: readonly Support[] = [
  [55, 241.33],
  [145, 227],
  [400, 225.82],
  [530, 197],
  [635, 218],
  [795, 237],
  [930, 233],
  [1130, 214.92],
  [1230, 228.56],
];

const supports = (list: readonly Support[], stroke: string, offset: number): string =>
  list
    .map(
      ([x, top]) =>
        `<path d="M${x} ${top + offset}V282M${x - 8} 282L${x} ${top + offset}" fill="none"` +
        ` stroke="${stroke}" stroke-width="3" opacity=".55"/>`,
    )
    .join("");

const rails = (d: string, post: string, light: string): string =>
  `<path data-activity-rail="" d="${d}" fill="none" stroke="${post}" stroke-width="9"/>` +
  `<path d="${d}" fill="none" stroke="${light}" stroke-width="5"/>` +
  `<path d="${d}" fill="none" stroke="#ffffff88" stroke-width="1" transform="translate(0,-3)"/>`;

/** Blocked stop gantry from the source: platform, side posts, roof and STOP board. */
const stopStation = (post: string, car: string): string =>
  `<g transform="translate(224,239)"><rect x="-44" y="6" width="88" height="9" rx="3"` +
  ` fill="${post}"/><path d="M-44 6V-31H-24M44 6V-31H24" fill="none" stroke="${car}"` +
  ` stroke-width="3"/><path d="M-46 -32H46L35 -42H-35Z" fill="${car}"/>` +
  `<path d="M-36 -17H-9" stroke="#f5d28e" stroke-width="4"/>` +
  `<text x="-31" y="-21" fill="#fff" font-size="8">STOP</text></g>`;

/** Done finish platform from the source: platform, pole and checkered flag. */
const finishStation = (post: string, car: string): string =>
  `<g transform="translate(950,236)"><rect x="-41" y="9" width="82" height="7" rx="2"` +
  ` fill="${post}"/><path d="M40 9V-38" stroke="${car}" stroke-width="2"/>` +
  `<path d="M41 -38H58V-27H41Z" fill="#bfe9ca"/>` +
  `<path d="M42 -37H48V-32H54V-27H58V-32H54V-37" stroke="${post}" stroke-width="3"/></g>`;

const tufts = (): string => `<path d="M35 265l6-14 7 14M1190 269l6-14 7 14" fill="#94ac80"/>`;

const hut = (): string =>
  `<g fill="#cfa179"><path d="M71 280V259H91V280Z"/><path d="M65 259L81 248L97 259Z"/></g>`;

const stringLights = (): string =>
  `<path d="M48 263V216M145 265V210M48 216Q90 232 145 210" stroke="#b2acc7" fill="none"/>` +
  `<g fill="#f2dca0"><circle cx="52" cy="218" r="3"/><circle cx="75" cy="223" r="3"/>` +
  `<circle cx="100" cy="222" r="3"/><circle cx="123" cy="217" r="3"/>` +
  `<circle cx="142" cy="212" r="3"/></g>`;

const tent = (): string =>
  `<path d="M1130 281V243H1210V281Z" fill="#93647a"/>` +
  `<path d="M1119 243L1170 199L1221 243Z" fill="#bc909d"/>` +
  `<path d="M1170 199L1150 243H1170V199L1190 243" fill="#e0b9ab"/>` +
  `<path d="M1161 281V253Q1170 245 1179 253V281" fill="#353b50"/>`;

const clouds = (): string =>
  `<g fill="#b7d3df" opacity=".25"><path d="M15 262q-15-12 7-18q5-22 22-9q20-10 22 13q20 5 9` +
  ` 14Z"/><path d="M1130 259q-15-12 7-18q5-22 22-9q20-10 22 13q20 5 9 14Z"/></g>`;

const gardenBeds = (): string =>
  `<g opacity=".9"><path d="M35 278Q46 256 64 278Q79 246 96 278Q115 257 130 278Z"` +
  ` fill="#629275"/><path d="M1070 278Q1082 251 1100 278Q1120 240 1139 278Q1163 252 1180` +
  ` 278Z" fill="#629275"/><g fill="#edd995"><circle cx="86" cy="239" r="2"/>` +
  `<circle cx="112" cy="226" r="2"/><circle cx="1099" cy="223" r="2"/>` +
  `<circle cx="1160" cy="246" r="2"/></g><g fill="#c6bba2"><rect x="28" y="278" width="110"` +
  ` height="5" rx="2"/><rect x="1064" y="278" width="127" height="5" rx="2"/></g></g>`;

function scenery(mode: CoasterMode): string {
  if (mode === "twilight-carnival") return stringLights() + tent();
  if (mode === "cloudline") return clouds();
  return tufts() + hut();
}

export function createCoasterArt(mode: CoasterMode): string {
  const palette = coasterPalette(mode);
  const cloud = mode === "cloudline";
  return (
    `<svg viewBox="0 0 1280 290" preserveAspectRatio="none"` +
    ` xmlns="http://www.w3.org/2000/svg" aria-hidden="true" pointer-events="none"` +
    ` data-coaster-mode="${mode}">` +
    `<defs><linearGradient id="coaster-land" x2="0" y2="1">` +
    `<stop stop-color="${palette.land}" stop-opacity=".62"/>` +
    `<stop offset="1" stop-color="${palette.land}" stop-opacity=".82"/></linearGradient></defs>` +
    `<path d="${LAND_PATH}" fill="url(#coaster-land)"/>` +
    scenery(mode) +
    supports(GROUND_SUPPORTS, palette.post, cloud ? -40 : 0) +
    rails(cloud ? CLOUD_RAIL : GROUND_RAIL, palette.post, palette.rail) +
    stopStation(palette.post, palette.rail) +
    finishStation(palette.post, palette.rail) +
    (mode === "firefly-gardens" ? gardenBeds() : "") +
    `</svg>`
  );
}
