/**
 * Pet Street activity theme: Gold Mine — "Carved Amber Cavern".
 *
 * Approved visual source, user-selected 2026-10-10 (themes-mine:B3 -> B):
 *   var/street-themes/ui-ux-grill-me/themes-mine/screenshots/B3-variant-B.png
 *   var/street-themes/ui-ux-grill-me/themes-mine/mockups/B3-scenery-B.svg
 * Continuous rough amber rock cave across the bottom 115 of the 290-unit strip
 * (y=175..290), dark inset timber galleries, amber seams, lantern light pools,
 * gravel floor and rails. Native editable vector only: no bitmap, no embedded
 * image, no tracing, no CSS keyframes and no clock of its own.
 *
 * The scene is the approved SVG itself, copied verbatim to
 * activity-mine-scene.svg and imported once as a raw string, so the selected
 * geometry, facets, grain filter and lighting survive exactly and mounting never
 * regenerates paths. The parent scopes the defs ids, stretches the strip to the
 * window and keeps the upper 175 units transparent.
 */
import scene from "./activity-mine-scene.svg?raw";

export { createMineProps } from "./activity-mine-props";

/** Full Gold Mine strip scenery, identical on every mount. */
export function createMineArt(): string {
  return scene;
}
