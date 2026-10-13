import { characterDisplayName } from "../character-packs";
import { effectivePetTheme, oceanRowingUrl } from "../ocean-assets";
import {
  friendlyPetName,
  isActivityTheme,
  type PreferencesFile,
  type PreferencesSnapshot,
} from "../preferences-types";
import { renderChoiceControls, setSettingsReadOnly } from "../settings-choice-controls";
import { byId, setSwitch } from "../settings-dom";

export function renderPetControls(
  draft: PreferencesFile,
  snapshot: PreferencesSnapshot,
  selectedPetId: string,
): void {
  const item = draft.pets[selectedPetId];
  const petSelect = byId<HTMLSelectElement>("pet-select");
  byId<HTMLInputElement>("pet-size").value = String(item.appearance.scalePercent);
  const customName = byId<HTMLInputElement>("pet-custom-name");
  if (customName.value !== item.customName) customName.value = item.customName;
  const selectedOption = petSelect.selectedOptions[0];
  if (selectedOption)
    selectedOption.textContent =
      item.customName.trim() ||
      characterDisplayName(selectedPetId) ||
      friendlyPetName(selectedPetId);
  byId<HTMLOutputElement>("pet-size-value").value = `${item.appearance.scalePercent}%`;
  byId<HTMLInputElement>("pet-opacity").value = String(item.appearance.opacityPercent);
  byId<HTMLOutputElement>("pet-opacity-value").value = `${item.appearance.opacityPercent}%`;
  byId<HTMLSelectElement>("label-visibility").value = item.labels.visibility;
  byId<HTMLInputElement>("label-size").value = String(item.labels.textScalePercent);
  byId<HTMLOutputElement>("label-size-value").value = `${item.labels.textScalePercent}%`;
  byId<HTMLSelectElement>("motion-level").value = item.motion.level;
  const themeSelect = byId<HTMLSelectElement>("pet-theme");
  const hasOceanArt = Boolean(oceanRowingUrl(selectedPetId));
  const selectedTheme = effectivePetTheme(draft, selectedPetId);
  themeSelect.value = selectedTheme;
  themeSelect.querySelector<HTMLOptionElement>('option[value="ocean"]')!.disabled = !hasOceanArt;
  byId<HTMLElement>("pet-theme-hint").textContent = isActivityTheme(draft.app.stripTheme)
    ? "The App activity theme adds props to Standard pet art. Switch App to Standard for individual Ocean choices."
    : draft.app.stripTheme === "snowy"
      ? "App Snowy uses Standard assets for every pet. Switch App to Standard for individual Ocean choices."
      : draft.app.stripTheme === "desert"
        ? "App Desert Oasis uses Standard assets for every pet. Switch App to Standard for individual Ocean choices."
        : draft.app.stripTheme === "rainforest"
          ? "App Rainforest uses Standard animations for every pet. Switch App to Standard for individual Ocean choices."
          : !hasOceanArt
            ? "Ocean animations are not available for this custom pet."
            : draft.app.stripTheme === "ocean"
              ? "App Ocean applies to every bundled pet. Switch App to Standard for individual choices."
              : "Ocean currently uses one rowing APNG for every visible state.";
  byId<HTMLElement>("state-map-hint").textContent =
    selectedTheme === "ocean"
      ? "Ocean currently shares its rowing APNG across visible states. Switch to Standard to replace state APNGs."
      : "Each state picks an APNG and whether the pet walks or stays in place. Select a visible state to preview its APNG.";
  setSwitch("pause-hover", item.motion.pauseOnHover);
  renderChoiceControls(draft, snapshot, item);
  setSettingsReadOnly(snapshot.readOnly);
  themeSelect.disabled =
    snapshot.readOnly ||
    !hasOceanArt ||
    draft.app.stripTheme === "ocean" ||
    draft.app.stripTheme === "rainforest" ||
    draft.app.stripTheme === "snowy" ||
    draft.app.stripTheme === "desert" ||
    isActivityTheme(draft.app.stripTheme);
}
