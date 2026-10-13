/**
 * Pet Street activity theme: Gold Mine — per-pet concept props.
 *
 * Approved source, user-selected 2026-10-10 (themes-mine:B3 -> B):
 *   var/street-themes/ui-ux-grill-me/themes-mine/mockups/B3-variant-B.html
 *   var/street-themes/ui-ux-grill-me/themes-mine/screenshots/B3-variant-B.png
 *
 * Every prop is the original artwork string from that mockup, placed in a nested SVG
 * viewport at the exact box the mockup actually rendered it in, mapped from the mockup
 * citizen (pet centre x=65, feet y=82) into this prop box (pet centre x=52, feet
 * baseline y=90): propX = mockX - 13, propY = mockY + 8. Displayed sizes stay 1:1 at a
 * 44px pet — cart 51 wide (viewBox 0 0 55 40), pick 35x35 (0 0 36 36), ore 25x21.43
 * (0 0 35 30), nugget 12x10.8 (0 0 20 18). Those boxes are the rendered ones: the
 * mockup's inline svgs sit on the text baseline, so auto-height props land 3 units
 * above their CSS box bottom, and the cart's used height is its 51 * 40/55 ratio, so it
 * overflows the source's 36px span. Art may overflow x=104 on purpose: the parent
 * .activity-props svg is overflow visible, so the cart keeps its source width instead
 * of being squeezed.
 *
 * The parent common clock rotates [data-activity-tool] about data-pivot-x /
 * data-pivot-y, so that group never authors a transform of its own and looks correct
 * when frozen. The detached pickaxe group is omitted for a dedicated sidecar action whose
 * sprite already holds the pick, while the approved resting pick, ore, cart and nugget
 * props stay. Props are decorative, aria-hidden and pointer-events none: no pet
 * images, no names, no user text, no resource or progress claim.
 */

/** Mockup .pickaxe: viewBox 0 0 36 36 drawn in a 35x35 box at citizen (69, 42). */
const PICK_ART =
  `<path d="M7 32L26 7" fill="none" stroke="#c88b4c" stroke-width="4" stroke-linecap="round"/>` +
  `<path d="M15 4Q29 2 34 18L26 12L17 10Z" fill="#b2c5d1" stroke="#5a737f"/>`;

/** Mockup .ore: viewBox 0 0 35 30 in a 25x21.43 box at citizen (103, 60.57). */
const ORE_ART =
  `<path d="M0 28L5 12L19 5L30 11L35 29Z" fill="#55514c" stroke="#7c7465"/>` +
  `<path d="M11 12L18 8L23 13L18 18Z" fill="#ecba58"/>` +
  `<path d="M5 23L10 19L15 23L11 28Z" fill="#bf8e40"/>`;

/** Mockup .cart: viewBox 0 0 55 40 in a 51x36 box at citizen (86, 48). */
const CART_ART =
  `<path d="M9 16L15 5L27 9L37 4L47 16" fill="#dfac47" stroke="#efce7a"/>` +
  `<path d="M5 15H51L46 31H11Z" fill="#6f5641" stroke="#b99870" stroke-width="2"/>` +
  `<path d="M20 16V30M37 16V30" fill="none" stroke="#b99870"/>` +
  `<circle cx="16" cy="34" r="5" fill="#161e25" stroke="#83949b"/>` +
  `<circle cx="42" cy="34" r="5" fill="#161e25" stroke="#83949b"/>`;

/** Mockup .nugget: viewBox 0 0 20 18 in a 12x10.8 box at citizen (65, 57.2). */
const NUGGET_ART = `<path d="M2 6L9 1L17 4L19 12L11 17L1 13Z" fill="#e6b552" stroke="#f4d889"/>`;

/** Working dig: the parent rotates this group, so no transform may be authored here. */
const TOOL =
  `<g data-activity-tool="mine-pickaxe" data-pivot-x="61" data-pivot-y="80">` +
  `<svg x="56" y="50" width="35" height="35" viewBox="0 0 36 36">${PICK_ART}</svg></g>`;

/** Blocked and idle: mockup .rest-pick 30x30 svg at citizen (77, 49), whose 33-unit
 * span is rotated 28deg about its centre at citizen (92, 65.5). */
const RESTED =
  `<g transform="rotate(28 79 73.5)">` +
  `<svg x="64" y="57" width="30" height="30" viewBox="0 0 36 36">${PICK_ART}</svg></g>`;

/** Mockup .ore: the 25x21.43 svg sits on the text baseline at citizen (103, 57.58). */
const ORE = `<svg x="90" y="65.58" width="25" height="21.43" viewBox="0 0 35 30">${ORE_ART}</svg>`;

/** Mockup .cart: 51 wide at citizen (86, 48); as an inline svg its used height is the
 * 55:40 ratio, 51 * 40/55 = 37.09, so it overflows the source's 36px span by 1.09. */
const CART = `<svg x="73" y="56" width="51" height="37.09" viewBox="0 0 55 40">${CART_ART}</svg>`;

/** Mockup .nugget: 12 x 10.8 svg on the text baseline at citizen (65, 54.2). */
const NUGGET = `<svg x="52" y="62.2" width="12" height="10.8" viewBox="0 0 20 18">${NUGGET_ART}</svg>`;

function propsFor(state: string, dedicated: boolean): string {
  switch (state) {
    case "working":
      // Dedicated sidecar art swings its own pick, so the detached tool is never duplicated.
      return dedicated ? ORE : ORE + TOOL;
    case "blocked":
      return RESTED;
    case "done":
      return CART + NUGGET;
    case "idle":
    case "listening":
    case "speaking":
      return RESTED;
    default:
      return "";
  }
}

/** Full prop SVG for one actor state; unknown or unexpected states stay empty. */
export function createMineProps(state: string, dedicated = false): string {
  return (
    `<svg viewBox="0 0 104 100" preserveAspectRatio="xMidYMid meet" pointer-events="none"` +
    ` xmlns="http://www.w3.org/2000/svg" aria-hidden="true">${propsFor(state, dedicated)}</svg>`
  );
}
