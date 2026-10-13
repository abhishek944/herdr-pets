import { behaviorPackForCharacter } from "./character-packs";
import { activityClipFor, activityPoseFor } from "./activity-pet";
import { PET_STATES, type HerdrState } from "./flow-types";
import { oceanClipName, oceanRowingUrl, oceanStateUrl } from "./ocean-assets";
import type { StripTheme } from "./preferences-types";
import { previewAnimations, type PreviewAnimationOption } from "./settings-preview";

export function configurePetPreview(
  petId: string,
  onStatePreview: (clip: string, action: "idle" | "walking") => void,
  onEditState: (petId: string, state: HerdrState, file: File) => void,
  theme: StripTheme = "standard",
): PreviewAnimationOption[] {
  const oceanUrl = theme === "ocean" ? oceanRowingUrl(petId) : null;
  const options = oceanUrl
    ? PET_STATES.flatMap((state) => {
        const clip = oceanClipName(state);
        const assetUrl = oceanStateUrl(petId, state);
        return clip && assetUrl
          ? [
              {
                id: `clip:${clip}`,
                label: `Ocean ${state === "working" ? "rowing" : state}`,
                assetUrl,
                scale: 1,
                locomotion: state === "working",
              },
            ]
          : [];
      })
    : previewAnimations(behaviorPackForCharacter(petId));
  for (const state of PET_STATES) {
    const pose = activityPoseFor(theme, petId, state);
    if (!pose?.assetUrl || options.some((item) => item.id === `clip:${pose.name}`)) continue;
    options.push({
      id: `clip:${pose.name}`,
      label: `Activity ${state}`,
      assetUrl: pose.assetUrl,
      scale: pose.scale,
      locomotion: false,
    });
  }
  renderStateMap(petId, onStatePreview, onEditState, options, oceanUrl !== null, theme);
  return options;
}

export function renderStateMap(
  petId: string,
  preview: (clip: string, action: "idle" | "walking") => void,
  edit: (petId: string, state: HerdrState, file: File) => void,
  options: readonly PreviewAnimationOption[] = [],
  ocean = false,
  theme: StripTheme = "standard",
): void {
  const pack = behaviorPackForCharacter(petId);
  const states: readonly HerdrState[] = PET_STATES;
  const labels: Record<HerdrState, string> = {
    idle: "Idle",
    working: "Running",
    blocked: "Blocked",
    done: "Completed",
    unknown: "Unknown",
    listening: "Listening (mayor)",
    speaking: "Speaking (mayor)",
  };
  const root = document.getElementById("pet-state-map");
  if (!root) return;
  root.replaceChildren(
    ...states.map((state) => {
      const assignment = pack.stateAssignments[state];
      const assigned = assignment.visible && assignment.animation;
      const row = document.createElement("div");
      row.className = "state-map-row";
      const previewButton = document.createElement("button");
      previewButton.type = "button";
      previewButton.className = "state-preview";
      const name = document.createElement("strong");
      name.textContent = labels[state];
      const action = document.createElement("small");
      const stationary =
        theme === "goldmine" ||
        theme === "railway" ||
        (theme === "rollercoaster" && (state === "blocked" || state === "done"));
      const presentationAction = stationary ? "idle" : assignment.action;
      action.textContent = `Action: ${presentationAction === "walking" ? "Walking" : "Idle"}`;
      const heading = document.createElement("span");
      heading.className = "state-heading";
      heading.append(name, action);
      previewButton.append(heading);
      const clipName = ocean
        ? oceanClipName(state)
        : (activityClipFor(theme, petId, state) ?? assignment.animation);
      const displayName = ocean ? `Ocean ${state === "working" ? "rowing" : state}` : clipName;
      if (assigned) {
        const assetUrl = options.find((item) => item.id === `clip:${clipName}`)?.assetUrl;
        if (assetUrl) {
          const thumb = document.createElement("img");
          thumb.className = "state-thumb";
          thumb.src = assetUrl;
          thumb.alt = `${displayName} animation preview`;
          previewButton.append(thumb);
        }
      } else {
        const empty = document.createElement("span");
        empty.className = "state-empty";
        empty.textContent = "No APNG";
        previewButton.append(empty);
      }
      const animation = document.createElement("span");
      animation.textContent = assigned ? `Pet Street: ${displayName}` : "Hidden";
      previewButton.append(animation);
      previewButton.disabled = !assigned;
      previewButton.setAttribute(
        "aria-label",
        assigned ? `Preview ${labels[state]}: ${displayName}` : `${labels[state]} is hidden`,
      );
      previewButton.addEventListener("click", () => {
        if (assigned && clipName) preview(clipName, presentationAction);
      });
      const editButton = document.createElement("button");
      editButton.type = "button";
      editButton.className = "state-edit";
      editButton.textContent = assigned ? "Replace" : "+ Add";
      editButton.hidden = ocean;
      editButton.setAttribute(
        "aria-label",
        `${assigned ? "Choose a replacement APNG for" : "Choose an APNG for"} ${labels[state]}`,
      );
      const filePicker = document.createElement("input");
      filePicker.type = "file";
      filePicker.accept = ".apng,.png,image/apng,image/png";
      filePicker.className = "state-file-picker";
      filePicker.tabIndex = -1;
      filePicker.setAttribute("aria-hidden", "true");
      filePicker.addEventListener("change", () => {
        const file = filePicker.files?.[0];
        if (file) edit(petId, state, file);
        filePicker.value = "";
      });
      editButton.addEventListener("click", () => filePicker.click());
      row.append(previewButton, editButton, filePicker);
      return row;
    }),
  );
}
