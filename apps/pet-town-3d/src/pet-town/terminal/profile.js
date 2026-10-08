import "./profile.css";

/** Companion actions are the entry point; terminal attachment is always explicit. */
export function createDockProfile(dock) {
  const root = document.createElement("section");
  root.className = "pt-companion-profile";
  root.innerHTML = `
    <h3>Your companion</h3>
    <p data-profile="following"></p>
    <div class="pt-companion-profile__actions">
      <button type="button" data-action="open">Open agent ↗</button>
      <button type="button" data-action="observe">Observe</button>
      <button type="button" data-action="interact">Interact</button>
    </div>
    <p data-profile="hint">Open the agent in its coding tool.</p>
    <h3>In the town</h3>
    <div class="pt-companion-profile__actions">
      <button type="button" data-action="control">Control companion</button>
      <button type="button" data-action="camera">First person</button>
      <button type="button" data-action="leave">Leave companion</button>
    </div>`;
  dock.querySelector(".pt-terminal__task").after(root);
  const controls = Object.fromEntries(
    [...root.querySelectorAll("[data-action]")].map((button) => [button.dataset.action, button]),
  );
  let previousState;
  return {
    root,
    controls,
    render(record, controller) {
      const remote = record.remoteMachine ?? "";
      const terminalAvailable = record.source === "herdr" && record.supportsTerminal === true;
      const state = `${record.source}:${remote}:${terminalAvailable}:${Boolean(record.controlled)}:${controller.firstPerson}`;
      if (state === previousState) return;
      previousState = state;
      controls.open.textContent = remote
        ? "Select remote pane"
        : terminalAvailable
          ? "Open in Herdr ↗"
          : "Open agent ↗";
      controls.observe.hidden = controls.interact.hidden = !terminalAvailable;
      root.querySelector('[data-profile="hint"]').textContent = remote
        ? `Open Herdr and select “${remote}” to view the selected pane. Remote terminals are not available in Pet Town.`
        : terminalAvailable
          ? "Observe the terminal, or choose Interact to type into it."
          : "Open the agent in its coding tool.";
      root.querySelector('[data-profile="following"]').textContent = record.controlled
        ? "You’re in control of this companion."
        : "Following · roaming freely";
      controls.control.textContent = record.controlled ? "Release control" : "Control companion";
      controls.control.setAttribute("aria-pressed", String(Boolean(record.controlled)));
      controls.camera.textContent = controller.firstPerson ? "Third person" : "First person";
      controls.camera.setAttribute("aria-pressed", String(controller.firstPerson));
    },
  };
}
