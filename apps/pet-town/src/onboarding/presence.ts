import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export async function installOnboardingPresence(
  root: HTMLElement,
  isPrimaryStreet: () => boolean = () => true,
): Promise<void> {
  let deadline = 0;
  let timer: number | null = null;
  const report = () => {
    // Only the primary street reports presence; every street answers the
    // request event, so unfiltered reporting would flap the shared state.
    if (!isPrimaryStreet()) return;
    const ids = [...root.querySelectorAll<HTMLElement>(".citizen")]
      .filter((pet) => {
        const image = pet.querySelector<HTMLImageElement>("img.pet");
        const rect = pet.getBoundingClientRect();
        return (
          pet.dataset.flowVisible === "true" &&
          pet.dataset.preferenceHidden !== "true" &&
          !pet.classList.contains("retiring") &&
          image &&
          !image.hidden &&
          image.naturalWidth > 0 &&
          getComputedStyle(pet).visibility !== "hidden" &&
          getComputedStyle(pet).display !== "none" &&
          rect.right > 0 &&
          rect.left < innerWidth &&
          rect.bottom > 0 &&
          rect.top < innerHeight
        );
      })
      .map((pet) => pet.dataset.agentId)
      .filter((id): id is string => Boolean(id));
    void invoke("report_onboarding_pets", { ids }).catch(() => {});
  };
  await listen("onboarding-presence-request", () => {
    deadline = Date.now() + 10_000;
    report();
    timer ??= window.setInterval(() => {
      if (Date.now() > deadline) {
        window.clearInterval(timer!);
        timer = null;
      } else report();
    }, 1_000);
  });
}
