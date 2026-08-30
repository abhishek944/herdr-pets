export type HerdrStatus = "working" | "blocked" | "idle" | "done" | "unknown";

export interface AgentView {
  id: string;
  status: HerdrStatus | string;
  label: string;
}

export interface AgentSnapshot {
  available: boolean;
  agents: AgentView[];
}

export const CHARACTER_IDS = ["human-male", "dog", "cat", "viking"] as const;

export type CharacterId = (typeof CHARACTER_IDS)[number];

export interface CitizenState extends AgentView {
  sprite: CharacterId;
  missedPolls: number;
  retiring: boolean;
}

export function fnv1a(value: string): number {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash;
}

export function spriteForAgent(
  id: string,
  used: ReadonlySet<CharacterId> = new Set(),
): CharacterId {
  const preferred = fnv1a(id) % CHARACTER_IDS.length;
  for (let offset = 0; offset < CHARACTER_IDS.length; offset += 1) {
    const candidate = CHARACTER_IDS[(preferred + offset) % CHARACTER_IDS.length];
    if (!used.has(candidate)) return candidate;
  }
  return CHARACTER_IDS[preferred];
}

export function reconcileCitizens(
  current: ReadonlyMap<string, CitizenState>,
  agents: readonly AgentView[],
): Map<string, CitizenState> {
  const next = new Map<string, CitizenState>();
  const seen = new Set<string>();
  const usedSprites = new Set<CharacterId>();

  for (const citizen of current.values()) {
    if (agents.some((agent) => agent.id === citizen.id) && !usedSprites.has(citizen.sprite)) {
      usedSprites.add(citizen.sprite);
    }
  }

  for (const agent of agents) {
    if (!agent.id || seen.has(agent.id)) continue;
    seen.add(agent.id);
    const existing = current.get(agent.id)?.sprite;
    const sprite = existing ?? spriteForAgent(agent.id, usedSprites);
    usedSprites.add(sprite);
    next.set(agent.id, {
      ...agent,
      sprite,
      missedPolls: 0,
      retiring: false,
    });
  }

  for (const [id, citizen] of current) {
    if (seen.has(id)) continue;
    const missedPolls = citizen.missedPolls + 1;
    next.set(id, {
      ...citizen,
      missedPolls,
      retiring: missedPolls >= 3,
    });
  }

  return next;
}

export function citizenSize(count: number, width: number): number {
  if (count <= 0) return 44;
  const usableWidth = Math.max(240, width - 24);
  return Math.max(24, Math.min(44, Math.floor(usableWidth / count) - 6));
}
