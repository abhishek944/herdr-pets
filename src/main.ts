import { invoke } from "@tauri-apps/api/core";
import { VillageRenderer, type HitRegion } from "./renderer";
import {
  reconcileCitizens,
  type AgentSnapshot,
  type CitizenState,
} from "./village";

const POLL_INTERVAL_MS = 1_000;
const RETIRE_ANIMATION_MS = 520;
const villageElement = document.querySelector<HTMLElement>("#village");

if (!villageElement) throw new Error("village root is missing");
const village: HTMLElement = villageElement;

let citizens = new Map<string, CitizenState>();
const retirementTimers = new Map<string, number>();
const renderer = new VillageRenderer(village, (regions: HitRegion[]) => {
  void invoke("set_hit_regions", { regions }).catch(() => {
    // The macOS hit-test bridge is optional on unsupported desktop targets.
  });
});

function render(): void {
  renderer.render(citizens, window.innerWidth);

  for (const citizen of citizens.values()) {
    if (!citizen.retiring || retirementTimers.has(citizen.id)) continue;
    const timer = window.setTimeout(() => {
      if (citizens.get(citizen.id)?.retiring) {
        citizens.delete(citizen.id);
        render();
      }
      retirementTimers.delete(citizen.id);
    }, RETIRE_ANIMATION_MS);
    retirementTimers.set(citizen.id, timer);
  }
}

async function poll(): Promise<void> {
  try {
    const snapshot = await invoke<AgentSnapshot>("list_agents");
    citizens = reconcileCitizens(citizens, snapshot.agents);
    render();
  } catch {
    citizens = reconcileCitizens(citizens, []);
    render();
  } finally {
    window.setTimeout(poll, POLL_INTERVAL_MS);
  }
}

window.addEventListener("resize", render);
render();
window.requestAnimationFrame(() => {
  void invoke("show_village");
});
void poll();
