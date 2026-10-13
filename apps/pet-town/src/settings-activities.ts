import { RendererActivities } from "./activity-themes";
import { isActivityTheme, type CoasterMode, type PreferencesFile } from "./preferences-types";
import { byId } from "./settings-dom";
import { bindRange } from "./settings-range";

/** Static draft preview; no animation clock or native agent actions. */
export class SettingsActivities {
  private readonly preview = byId<HTMLElement>("activity-preview");
  private readonly scene = new RendererActivities(this.preview);
  private readonly mode = byId<HTMLSelectElement>("coaster-mode");
  constructor(draft: () => PreferencesFile, render: () => void) {
    bindRange(
      "coaster-height",
      (value) => {
        draft().app.coasterHeightPx = value;
      },
      render,
    );
    bindRange(
      "goldmine-opacity",
      (value) => {
        draft().app.goldmineOpacityPercent = value;
      },
      render,
    );
    bindRange(
      "railway-opacity",
      (value) => {
        draft().app.railwayOpacityPercent = value;
      },
      render,
    );
    this.mode.addEventListener("change", () => {
      if (
        !["low-rolling", "twilight-carnival", "firefly-gardens", "cloudline"].includes(
          this.mode.value,
        )
      )
        return;
      draft().app.coasterMode = this.mode.value as CoasterMode;
      render();
    });
  }
  render(draft: PreferencesFile, readOnly: boolean): void {
    const coaster = draft.app.stripTheme === "rollercoaster";
    this.mode.value = draft.app.coasterMode ?? "low-rolling";
    this.mode.disabled = readOnly || !coaster;
    byId<HTMLElement>("coaster-mode-row").hidden = !coaster;
    for (const [id, theme, value, unit] of [
      ["coaster-height", "rollercoaster", draft.app.coasterHeightPx ?? 88, "px"],
      ["goldmine-opacity", "goldmine", draft.app.goldmineOpacityPercent ?? 100, "%"],
      ["railway-opacity", "railway", draft.app.railwayOpacityPercent ?? 100, "%"],
    ] as const) {
      const active = draft.app.stripTheme === theme;
      const slider = byId<HTMLInputElement>(id);
      slider.value = String(value);
      slider.disabled = readOnly || !active;
      slider.setAttribute(
        "aria-valuetext",
        unit === "px" ? `${value}px ride height` : `${value}% opaque, ${100 - value}% transparent`,
      );
      byId<HTMLOutputElement>(`${id}-value`).value = `${value}${unit}`;
      byId<HTMLElement>(`${id}-row`).hidden = !active;
    }
    this.preview.hidden = !isActivityTheme(draft.app.stripTheme);
    this.scene.setPreferences(draft);
    this.scene.advance(0, true, true);
  }
}
