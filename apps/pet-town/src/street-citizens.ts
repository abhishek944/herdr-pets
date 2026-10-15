import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CitizenState } from "./village";

export interface StreetMembership {
  fetch(): Promise<void>;
  retain(citizens: Map<string, CitizenState>): void;
  isPrimary(): boolean;
}

/** Which street this window serves. Every window computes the same split:
 * sorted citizen ids dealt round-robin across streets. The orchestrator pet
 * stays on the primary street instead of duplicating on every display. */
export function createStreetMembership(): StreetMembership {
  let index = 0;
  let total = 1;
  return {
    async fetch(): Promise<void> {
      try {
        const assignment = await invoke<{ index: number; total: number }>("street_assignment");
        index = assignment.index;
        total = Math.max(1, assignment.total);
      } catch {
        index = 0;
        total = 1;
      }
    },
    retain(citizens: Map<string, CitizenState>): void {
      const regular = [...citizens.keys()]
        .filter((id) => citizens.get(id)?.source !== "orchestrator")
        .sort();
      for (const [position, id] of regular.entries()) {
        if (position % total !== index) citizens.delete(id);
      }
      if (index !== 0) {
        for (const [id, citizen] of citizens) {
          if (citizen.source === "orchestrator") citizens.delete(id);
        }
      }
    },
    isPrimary(): boolean {
      return index === 0;
    },
  };
}

/** Re-partition when displays change, then let the caller render. */
export async function watchStreetChanges(
  membership: StreetMembership,
  onChange: () => void,
): Promise<void> {
  await listen("streets-changed", async () => {
    await membership.fetch();
    onChange();
  });
}
