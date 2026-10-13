import { RendererRainforest } from "../rainforest";
import { RendererSnowy } from "../snow";
import { RendererDesert } from "../desert";
import { RendererActivities } from "../activity-themes";
import { isActivityTheme } from "../preferences-types";
import { createOceanWater, updateOceanWater, advanceOceanWater } from "../ocean-water";
import "../ocean-water.css";
import type { PreferencesFile, StripTheme } from "../preferences-types";
const names: Record<StripTheme, string> = {
  standard: "Classic",
  ocean: "Ocean",
  rainforest: "Forest",
  snowy: "Snow",
  desert: "Desert Oasis",
  rollercoaster: "Rollercoaster",
  goldmine: "Gold Mine",
  railway: "Tiny Railway",
};
export function renderScenery(
  root: HTMLElement,
  theme: StripTheme,
  defaults: PreferencesFile,
): void {
  root
    .querySelectorAll(".rainforest-scene,.snow-scene,.desert-scene,.ocean-water,.activity-scene")
    .forEach((n) => n.remove());
  const prefs = { ...defaults, app: { ...defaults.app, stripTheme: theme } };
  const frozen = root.querySelector<HTMLCanvasElement>("canvas.scene-pet");
  if (frozen) {
    const image = document.createElement("img");
    image.className = "scene-pet";
    image.alt = frozen.getAttribute("aria-label") ?? "Sample companion";
    frozen.replaceWith(image);
  }
  const portrait = root.querySelector<HTMLImageElement>("img.scene-pet");
  if (portrait)
    portrait.src = theme === "ocean" ? "/onboarding/nib-ocean.png" : "/onboarding/nib-walk.png";
  const Scenery =
    theme === "rainforest"
      ? RendererRainforest
      : theme === "snowy"
        ? RendererSnowy
        : theme === "desert"
          ? RendererDesert
          : null;
  if (Scenery) {
    const scenery = new Scenery(root);
    scenery.setPreferences(prefs);
    scenery.advance(0, true, true);
  }
  if (isActivityTheme(theme)) {
    const scenery = new RendererActivities(root);
    scenery.setPreferences(prefs);
    scenery.advance(0, true, true);
  }
  if (theme === "ocean") {
    const water = createOceanWater();
    root.prepend(water);
    updateOceanWater(water, "ocean", 36, Math.min(88, root.clientHeight * 0.4));
    advanceOceanWater(water, 0, true);
  }
  root.dataset.theme = theme;
}
export function chooseScenery(
  root: HTMLElement,
  theme: StripTheme,
  defaults: PreferencesFile,
): void {
  root
    .querySelectorAll<HTMLElement>("[data-main-preview]")
    .forEach((el) => renderScenery(el, theme, defaults));
  root
    .querySelectorAll<HTMLElement>("[data-theme-choice]")
    .forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.themeChoice === theme)));
  root
    .querySelectorAll<HTMLElement>("[data-theme-name]")
    .forEach((el) => (el.textContent = names[theme]));
}
export function mountScenery(
  root: HTMLElement,
  theme: StripTheme,
  defaults: PreferencesFile,
): void {
  root
    .querySelectorAll<HTMLElement>("[data-theme-preview]")
    .forEach((el) => renderScenery(el, el.dataset.themePreview as StripTheme, defaults));
  chooseScenery(root, theme, defaults);
}
