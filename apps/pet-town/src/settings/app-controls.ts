import { oceanRowingUrl } from "../ocean-assets";
import { isActivityTheme } from "../preferences-types";
import type {
  PreferencesFile,
  RainforestMode,
  SettingsAppearance,
  StripTheme,
} from "../preferences-types";
import { byId } from "../settings-dom";
import { bindRange as bindInputRange, bindSwitch as bindInputSwitch } from "../settings-range";
import { bindTownActions } from "../settings-town-actions";

export function bindAppControls(
  draft: () => PreferencesFile,
  selectPreviewAnimations: () => void,
  render: () => void,
  bindVillageVisibility: () => void,
  showError: (error: string) => void,
): void {
  const bindSwitch = (id: string, update: (checked: boolean) => void) =>
    bindInputSwitch(id, update, render);
  bindInputRange(
    "ocean-opacity",
    (value) => {
      draft().app.oceanOpacityPercent = value;
    },
    render,
  );
  bindInputRange(
    "ocean-waterline",
    (value) => {
      draft().app.oceanWaterlineHeightPx = value;
    },
    render,
  );
  bindSwitch("hide-completed-pets", (checked) => {
    draft().app.hideCompletedPets = checked;
  });
  byId<HTMLSelectElement>("completed-hide-delay").addEventListener("change", (event) => {
    draft().app.completedHideDelayMinutes = Number(
      (event.currentTarget as HTMLSelectElement).value,
    );
    render();
  });
  bindSwitch("open-with-herdr", (checked) => {
    draft().app.openWithHerdr = checked;
  });
  bindVillageVisibility();
  bindTownActions(showError);
  byId<HTMLSelectElement>("settings-appearance").addEventListener("change", (event) => {
    draft().app.settingsAppearance = (event.currentTarget as HTMLSelectElement)
      .value as SettingsAppearance;
    render();
  });
  byId<HTMLSelectElement>("strip-theme").addEventListener("change", (event) => {
    const theme = (event.currentTarget as HTMLSelectElement).value as StripTheme;
    const previousTheme = draft().app.stripTheme;
    draft().app.stripTheme = theme;
    // Snowy and Desert override presentation without erasing individual Ocean choices.
    const preservePetThemes =
      theme === "snowy" ||
      theme === "desert" ||
      isActivityTheme(theme) ||
      (theme === "standard" &&
        (previousTheme === "snowy" ||
          previousTheme === "desert" ||
          isActivityTheme(previousTheme)));
    if (!preservePetThemes) {
      for (const [id, item] of Object.entries(draft().pets)) {
        if (oceanRowingUrl(id)) item.theme = theme === "ocean" ? "ocean" : "standard";
      }
    }
    selectPreviewAnimations();
    render();
  });
  byId<HTMLSelectElement>("rainforest-mode").addEventListener("change", (event) => {
    const mode = (event.currentTarget as HTMLSelectElement).value;
    if (mode !== "after-rain" && mode !== "firefly") {
      showError("Choose a valid Rainforest mode.");
      return;
    }
    draft().app.rainforestMode = mode as RainforestMode;
    render();
  });
}
