import { createOceanWater, updateOceanWater } from "../ocean-water";
import { startOceanPreview } from "../ocean-preview";
import type { PreferencesFile } from "../preferences-types";
import { byId } from "../settings-dom";
import { SettingsRainforest } from "../settings-rainforest";
import { SettingsSnow } from "../settings-snow";
import { SettingsDesert } from "../settings-desert";
import { SettingsActivities } from "../settings-activities";

export class SettingsScenePreviews {
  private readonly oceanPreview = byId<HTMLElement>("ocean-preview");
  private readonly oceanPreviewWater = createOceanWater();
  private readonly rainforestSettings: SettingsRainforest;
  private readonly snowSettings: SettingsSnow;
  private readonly desertSettings: SettingsDesert;
  private readonly activitySettings: SettingsActivities;

  constructor(draft: () => PreferencesFile, render: () => void) {
    this.rainforestSettings = new SettingsRainforest(draft, render);
    this.snowSettings = new SettingsSnow(draft, render);
    this.desertSettings = new SettingsDesert(draft, render);
    this.activitySettings = new SettingsActivities(draft, render);
  }

  start(): void {
    this.oceanPreview.append(this.oceanPreviewWater);
    startOceanPreview(this.oceanPreviewWater);
  }

  render(draft: PreferencesFile, readOnly: boolean): void {
    byId<HTMLSelectElement>("settings-appearance").value = draft.app.settingsAppearance;
    byId<HTMLSelectElement>("strip-theme").value = draft.app.stripTheme;
    const rainforestMode = byId<HTMLSelectElement>("rainforest-mode");
    rainforestMode.value = draft.app.rainforestMode;
    rainforestMode.disabled = readOnly || draft.app.stripTheme !== "rainforest";
    byId<HTMLElement>("rainforest-mode-row").hidden = draft.app.stripTheme !== "rainforest";
    this.rainforestSettings.render(draft, readOnly);
    this.snowSettings.render(draft, readOnly);
    this.desertSettings.render(draft, readOnly);
    this.activitySettings.render(draft, readOnly);
    const waterlineSlider = byId<HTMLInputElement>("ocean-waterline");
    waterlineSlider.value = String(draft.app.oceanWaterlineHeightPx);
    waterlineSlider.disabled = readOnly || draft.app.stripTheme !== "ocean";
    byId<HTMLOutputElement>("ocean-waterline-value").value =
      `${draft.app.oceanWaterlineHeightPx}px`;
    byId<HTMLElement>("ocean-waterline-row").hidden = draft.app.stripTheme !== "ocean";
    byId<HTMLElement>("ocean-waves-heading").hidden = draft.app.stripTheme !== "ocean";
    const opacitySlider = byId<HTMLInputElement>("ocean-opacity");
    opacitySlider.value = String(draft.app.oceanOpacityPercent);
    opacitySlider.disabled = readOnly || draft.app.stripTheme !== "ocean";
    byId<HTMLOutputElement>("ocean-opacity-value").value = `${draft.app.oceanOpacityPercent}%`;
    byId<HTMLElement>("ocean-opacity-row").hidden = draft.app.stripTheme !== "ocean";
    this.oceanPreview.hidden = draft.app.stripTheme !== "ocean";
    updateOceanWater(
      this.oceanPreviewWater,
      "ocean",
      draft.app.oceanOpacityPercent,
      draft.app.oceanWaterlineHeightPx,
    );
    document.documentElement.dataset.theme = draft.app.settingsAppearance;
  }
}
