import { bindPetGallery } from "./pet-gallery.js";
import { createPortrait, companionNames } from "../../pet-town/ui/portrait.js";

export function bindCompanionControls(root, context) {
  const field = (name) => root.querySelector(`[data-field="${name}"]`);
  const control = (name) => root.querySelector(`[data-control="${name}"]`);
  const picker = control("companion");
  const gallery = bindPetGallery(root, context);
  let portraitId;
  let pending = false;
  const runtime = () => context.petTown;
  picker.onchange = () => {
    if (runtime()?.agents.records.has(picker.value)) runtime().controller.select(picker.value);
    render();
  };
  control("control").onclick = () => {
    runtime()?.controller.toggleControl();
    render();
  };
  for (const [name, firstPerson] of [
    ["first", true],
    ["third", false],
  ]) {
    control(name).onclick = () => {
      const controller = runtime()?.controller;
      if (controller?.selected && controller.firstPerson !== firstPerson) controller.toggleView();
      render();
    };
  }
  control("leave").onclick = () => {
    runtime()?.controller.select(null);
    render();
  };
  control("focus").onclick = async () => {
    const town = runtime();
    const selected = town?.controller.selected;
    if (pending || !selected || !town.agents.records.has(selected.id)) return;
    pending = true;
    field("feedback").hidden = true;
    render();
    try {
      await town.bridge.action("focusAgent", { id: selected.id });
      if (selected.remoteMachine && town.controller.selected?.id === selected.id) {
        field("feedback").textContent =
          `Pane selected. Open Herdr and select “${selected.remoteMachine}” to view it.`;
        field("feedback").hidden = false;
      }
    } catch (error) {
      field("feedback").textContent = String(error.message ?? error);
      field("feedback").hidden = false;
    } finally {
      pending = false;
      render();
    }
  };
  function render() {
    const town = runtime();
    const records = town?.agents.records ?? new Map();
    const controller = town?.controller;
    const selected = controller?.selected;
    const names = companionNames(records);
    const editing = document.activeElement === picker;
    const choice = picker.value;
    for (const option of [...picker.options]) {
      if (option.value && !records.has(option.value)) option.remove();
    }
    for (const [id, name] of names) {
      let option = [...picker.options].find((o) => o.value === id);
      if (!option) {
        option = new Option(name, id);
        picker.add(option);
      }
      option.textContent = name;
    }
    picker.options[0].textContent = selected ? "Choose…" : "Choose a companion";
    picker.options[0].disabled = true;
    picker.value = editing && records.has(choice) ? choice : (selected?.id ?? "");
    picker.disabled = !records.size;
    field("name").textContent = selected ? names.get(selected.id) : "Choose a companion";
    field("following").textContent = selected
      ? selected.controlled
        ? "You’re in control"
        : "Following · roaming freely"
      : records.size
        ? "Pick someone from the list."
        : "No companions available. Check the town connection.";
    field("controlHint").textContent = selected
      ? `Walk and jump as ${selected.label}.`
      : "Follow a companion to walk and jump.";
    field("appearance").textContent = selected
      ? (selected.appearanceLabel ?? "Town companion")
      : "Choose a companion to change its pet.";
    const portraitKey = selected ? `${selected.id}:${selected.petId ?? ""}` : "";
    if (portraitId !== portraitKey) {
      portraitId = portraitKey;
      field("portrait").replaceChildren(...(selected ? [createPortrait(selected)] : []));
    }
    control("control").querySelector("span").textContent = selected?.controlled
      ? "Release control"
      : "Control";
    control("control").setAttribute("aria-pressed", String(Boolean(selected?.controlled)));
    control("first").setAttribute(
      "aria-pressed",
      String(Boolean(selected && controller.firstPerson)),
    );
    control("third").setAttribute(
      "aria-pressed",
      String(Boolean(selected && !controller.firstPerson)),
    );
    control("focus").textContent = pending
      ? selected?.remoteMachine
        ? "Selecting…"
        : "Opening…"
      : selected?.remoteMachine
        ? "Select remote pane"
        : selected?.source === "herdr" || selected?.isMayor
          ? "Open in Herdr ↗"
          : "Open agent ↗";
    control("focus").title = selected?.remoteMachine
      ? `Open Herdr and select “${selected.remoteMachine}” to view the selected pane. Remote terminals are not available in Pet Town.`
      : "";
    field("remoteHint").hidden = !selected?.remoteMachine;
    field("remoteHint").textContent = control("focus").title;
    for (const name of ["control", "first", "third", "leave", "focus"])
      control(name).disabled = !selected || (name === "focus" && pending);
    gallery.render();
  }
  return { render, setVisible: gallery.setVisible };
}
