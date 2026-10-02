import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { installPetInteractions } from "./pet-interactions";
import { createAgentFocus } from "./agent-focus";
import { VillageRenderer, type HitRegion } from "./renderer";
import {
  allCharacterIds,
  behaviorPackForCharacter,
  bundledBehaviorPackForCharacter,
  CHARACTER_IDS,
  installUserPacks,
  type CharacterId,
  type PetExtensionPayload,
  type UserPackPayload,
} from "./character-packs";
type PetExtensionCatalog = { extensions: PetExtensionPayload[]; warnings: string[] };
import {
  applyActiveCast,
  reconcileCitizens,
  type AgentSnapshot,
  type CitizenState,
} from "./village";
import { installVillagePreferences } from "./village-preferences";
import type { PreferencesFile } from "./preferences-types";
import {
  applyOrchestratorCitizen,
  orchestratorPack,
  validOrchestratorView,
  type OrchestratorView,
} from "./orchestrator-pet";

import { installOnboardingPresence } from "./onboarding/presence";

const POLL_INTERVAL_MS = 1_000;
const RETIRE_ANIMATION_MS = 520;
const villageElement = document.querySelector<HTMLElement>("#village");

if (!villageElement) throw new Error("village root is missing");
const village: HTMLElement = villageElement;
void invoke("trace_pet_input", { stage: "ready" });
for (const stage of ["pointerdown", "pointerup", "pointercancel", "click", "contextmenu"]) {
  village.addEventListener(
    stage,
    () => {
      void invoke("trace_pet_input", { stage });
    },
    true,
  );
}

let citizens = new Map<string, CitizenState>();
let activePetIds: readonly CharacterId[] = CHARACTER_IDS;
let latestPreferences: PreferencesFile | null = null;
const retirementTimers = new Map<string, number>();
let villagePaused = false;
let hasRenderedAgentSnapshot = false;
let orchestratorView: OrchestratorView | null = null;
let pendingOrchestratorView: OrchestratorView | null = null;
const renderer = new VillageRenderer(
  village,
  (regions: HitRegion[]) => {
    void invoke("set_hit_regions", { regions }).catch(() => {
      // The macOS hit-test bridge is optional on unsupported desktop targets.
    });
  },
  (citizen) => {
    const pack =
      citizen.source === "orchestrator"
        ? bundledBehaviorPackForCharacter(citizen.sprite)
        : behaviorPackForCharacter(citizen.sprite);
    return citizen.source === "orchestrator" ? orchestratorPack(pack) : pack;
  },
);
const systemReducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
renderer.setSystemReducedMotion(systemReducedMotion.matches);
systemReducedMotion.addEventListener("change", (event) => {
  renderer.setSystemReducedMotion(event.matches);
});
installPetInteractions(village, {
  focusAgent: createAgentFocus(village, (id) => invoke("focus_agent", { id })),
  openPreferences: (id) => {
    const petId = citizens.get(id)?.sprite;
    if (petId) void invoke("open_preferences", { petId });
  },
  beginDrag: (id, clientX) => renderer.beginDrag(id, clientX),
  moveDrag: (id, clientX) => renderer.moveDrag(id, clientX),
  endDrag: (id) => renderer.endDrag(id),
  geometryChanged: () => renderer.refreshHitRegions(),
});

function render(whilePaused = false): void {
  if (villagePaused && !whilePaused) return;
  renderer.render(citizens, window.innerWidth);
  if (villagePaused) renderer.setPaused(true);

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

function renderPolledSnapshot(): void {
  const forceInitialRender = !hasRenderedAgentSnapshot;
  hasRenderedAgentSnapshot = true;
  render(forceInitialRender);
}

async function poll(): Promise<void> {
  try {
    const snapshot = await invoke<AgentSnapshot>("list_agents");
    citizens = reconcileCitizens(citizens, snapshot.agents, activePetIds);
    citizens = applyOrchestratorCitizen(citizens, orchestratorView);
    renderPolledSnapshot();
  } catch {
    citizens = reconcileCitizens(citizens, [], activePetIds);
    renderPolledSnapshot();
  } finally {
    window.setTimeout(poll, POLL_INTERVAL_MS);
  }
}

window.addEventListener("resize", () => render());
async function reloadUserPacks(): Promise<void> {
  try {
    const packs = await invoke<UserPackPayload[]>("list_user_pet_packs");
    let extensions: PetExtensionPayload[] = [];
    try {
      const catalog = await invoke<PetExtensionCatalog>("list_pet_extensions");
      extensions = catalog.extensions;
    } catch {
      /* Optional extensions never prevent base pets from loading. */
    }
    installUserPacks(packs, extensions);
    orchestratorView = validOrchestratorView(pendingOrchestratorView);
    activePetIds = allCharacterIds().filter(
      (id) => latestPreferences?.pets[id]?.includedInRandomCast ?? true,
    );
    citizens = applyActiveCast(citizens, activePetIds);
    citizens = applyOrchestratorCitizen(citizens, orchestratorView);
    render(true);
  } catch {
    // Invalid or unavailable user packs never prevent bundled pets from loading.
  }
}

async function start(): Promise<void> {
  await installOnboardingPresence(village);
  await listen("user-packs-changed", () => {
    void reloadUserPacks();
  });
  await listen<OrchestratorView>("orchestrator-pet-state", (event) => {
    pendingOrchestratorView = event.payload;
    orchestratorView = validOrchestratorView(event.payload);
    citizens = applyOrchestratorCitizen(citizens, orchestratorView);
    render(true);
  });
  await reloadUserPacks();
  try {
    pendingOrchestratorView = await invoke<OrchestratorView>("get_orchestrator_pet_state");
    orchestratorView = validOrchestratorView(pendingOrchestratorView);
  } catch {
    pendingOrchestratorView = null;
    orchestratorView = null;
  }
  citizens = applyOrchestratorCitizen(citizens, orchestratorView);
  await installVillagePreferences(
    renderer,
    () => {
      village.dispatchEvent(new Event("village-pause"));
      villagePaused = true;
    },
    () => {
      villagePaused = false;
      render();
    },
    (preferences) => {
      latestPreferences = preferences;
      activePetIds = allCharacterIds().filter(
        (id) => preferences.pets[id]?.includedInRandomCast ?? true,
      );
      citizens = applyActiveCast(citizens, activePetIds);
      render(true);
    },
  );
  render();
  void invoke("renderer_ready");
  void poll();
}
void start();
