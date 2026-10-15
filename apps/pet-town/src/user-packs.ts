import { invoke } from "@tauri-apps/api/core";
import {
  allCharacterIds,
  installUserPacks,
  type CharacterId,
  type PetExtensionPayload,
  type UserPackPayload,
} from "./character-packs";
import { applyActiveCast, type CitizenState } from "./village";
import {
  applyOrchestratorCitizen,
  validOrchestratorView,
  type OrchestratorView,
} from "./orchestrator-pet";
import type { PreferencesFile } from "./preferences-types";

type PetExtensionCatalog = { extensions: PetExtensionPayload[]; warnings: string[] };

export interface ReloadedPacks {
  citizens: Map<string, CitizenState>;
  view: OrchestratorView | null;
  activeIds: readonly CharacterId[];
}

/** Load user packs and compute the next village state. Returns null when packs
 * are invalid or unavailable, and the caller keeps its current citizens. */
export async function reloadUserPacks(
  current: ReadonlyMap<string, CitizenState>,
  pendingView: OrchestratorView | null,
  preferences: PreferencesFile | null,
): Promise<ReloadedPacks | null> {
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
    const view = validOrchestratorView(pendingView);
    const activeIds = allCharacterIds().filter(
      (id) => preferences?.pets[id]?.includedInRandomCast ?? true,
    );
    let citizens = applyActiveCast(current, activeIds);
    citizens = applyOrchestratorCitizen(citizens, view);
    return { citizens, view, activeIds };
  } catch {
    // Invalid or unavailable user packs never prevent bundled pets from loading.
    return null;
  }
}
