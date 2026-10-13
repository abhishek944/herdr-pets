import type { HerdrState } from "../flow-types";
import { effectivePetTheme } from "../ocean-assets";
import {
  isActivityTheme,
  motionFactor,
  type PetPreferences,
  type PreferencesFile,
} from "../preferences-types";
import { byId } from "../settings-dom";
import { selectedPreviewAnimationId, type PreviewAnimationOption } from "../settings-preview";
import { configurePetPreview } from "../settings-state-map";

export class SettingsPetPreview {
  private readonly previewPet = byId<HTMLImageElement>("preview-pet");
  private readonly preview = byId<HTMLElement>("preview");
  private readonly selectedAnimationByPet = new Map<string, string>();
  private animationOptions: PreviewAnimationOption[] = [];
  private selectedAnimationId = "";
  private previewAction: "idle" | "walking" | null = null;

  constructor(
    private readonly draft: () => PreferencesFile,
    private readonly selectedPetId: () => string,
    private readonly render: () => void,
    private readonly editState: (petId: string, state: HerdrState, file: File) => void,
  ) {}

  selectAnimations(): void {
    this.previewAction = null;
    const draft = this.draft();
    const selectedPetId = this.selectedPetId();
    this.animationOptions = configurePetPreview(
      selectedPetId,
      (clip, action) => {
        const option = this.animationOptions.find((item) => item.id === `clip:${clip}`);
        if (!option) return;
        this.selectedAnimationId = option.id;
        this.previewAction = action;
        this.selectedAnimationByPet.set(this.selectedPetId(), option.id);
        this.render();
      },
      this.editState,
      isActivityTheme(draft.app.stripTheme)
        ? draft.app.stripTheme
        : effectivePetTheme(draft, selectedPetId),
    );
    this.selectedAnimationId = selectedPreviewAnimationId(
      this.animationOptions,
      this.selectedAnimationByPet.get(selectedPetId),
    );
    this.selectedAnimationByPet.set(selectedPetId, this.selectedAnimationId);
  }

  renderPet(item: PetPreferences): void {
    const animation = this.animationOptions.find(
      (option) => option.id === this.selectedAnimationId,
    );
    if (animation && this.previewPet.dataset.previewAsset !== animation.assetUrl) {
      this.previewPet.dataset.previewAsset = animation.assetUrl;
      this.previewPet.src = animation.assetUrl;
    }
    this.previewPet.hidden = !animation;
    const clipScale = animation?.scale ?? 1;
    this.preview.style.setProperty(
      "--preview-size",
      `${Math.min(190, Math.round((148 * clipScale * item.appearance.scalePercent) / 100))}px`,
    );
    this.preview.style.setProperty(
      "--preview-opacity",
      String(item.appearance.opacityPercent / 100),
    );
    this.preview.style.setProperty(
      "--preview-walk-duration",
      `${(4 / motionFactor(item.motion.level)).toFixed(2)}s`,
    );
    this.preview.dataset.pauseOnHover = String(item.motion.pauseOnHover);
    this.preview.dataset.locomotion = String(
      this.previewAction ? this.previewAction === "walking" : (animation?.locomotion ?? false),
    );
  }
}
