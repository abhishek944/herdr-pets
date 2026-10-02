import { CHARACTER_IDS, type CharacterId } from "./character-packs";

export type AgentStatus =
  "working" | "blocked" | "idle" | "done" | "unknown" | "listening" | "speaking";

export interface TokenActivity {
  outputTokensPerMinute: number;
  observedAtMs: number;
  measurement: "estimated" | "reported";
}

export interface AgentView {
  id: string;
  tokenActivity?: TokenActivity;
  status: AgentStatus | string;
  label: string;
  source: string;
  supportsTerminal?: boolean;
  remoteMachine?: string | null;
}

export interface AgentSnapshot {
  available: boolean;
  agents: AgentView[];
}

export interface CitizenState extends AgentView {
  sprite: CharacterId;
  missedPolls: number;
  retiring: boolean;
  doneSinceMs: number | null;
}

export function fnv1a(value: string): number {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash;
}

function availableCharacters(ids: readonly CharacterId[]): readonly CharacterId[] {
  return ids.length > 0 ? ids : CHARACTER_IDS;
}

export function spriteForAgent(
  id: string,
  used: ReadonlySet<CharacterId> = new Set(),
  available: readonly CharacterId[] = CHARACTER_IDS,
): CharacterId {
  const candidates = availableCharacters(available);
  const preferred = fnv1a(id) % candidates.length;
  for (let offset = 0; offset < candidates.length; offset += 1) {
    const candidate = candidates[(preferred + offset) % candidates.length];
    if (!used.has(candidate)) return candidate;
  }
  return candidates[preferred];
}

export function applyActiveCast(
  current: ReadonlyMap<string, CitizenState>,
  available: readonly CharacterId[],
): Map<string, CitizenState> {
  const candidates = availableCharacters(available);
  const allowed = new Set(candidates);
  const used = new Set<CharacterId>();
  for (const citizen of current.values()) {
    if (allowed.has(citizen.sprite)) used.add(citizen.sprite);
  }
  return new Map(
    [...current.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([id, citizen]) => {
        if (allowed.has(citizen.sprite)) return [id, citizen];
        const sprite = spriteForAgent(id, used, candidates);
        used.add(sprite);
        return [id, { ...citizen, sprite }];
      }),
  );
}

export function reconcileCitizens(
  current: ReadonlyMap<string, CitizenState>,
  agents: readonly AgentView[],
  available: readonly CharacterId[] = CHARACTER_IDS,
  nowMs = Date.now(),
): Map<string, CitizenState> {
  const candidates = availableCharacters(available);
  const allowed = new Set(candidates);
  const next = new Map<string, CitizenState>();
  const seen = new Set<string>();
  const usedSprites = new Set<CharacterId>();

  for (const citizen of current.values()) {
    if (allowed.has(citizen.sprite)) usedSprites.add(citizen.sprite);
  }

  for (const agent of agents) {
    if (!agent.id || seen.has(agent.id)) continue;
    seen.add(agent.id);
    const previous = current.get(agent.id);
    const existing = previous?.sprite;
    const sprite =
      existing && allowed.has(existing)
        ? existing
        : spriteForAgent(agent.id, usedSprites, candidates);
    usedSprites.add(sprite);
    const doneSinceMs =
      agent.status === "done"
        ? previous?.status === "done"
          ? (previous.doneSinceMs ?? nowMs)
          : nowMs
        : null;
    next.set(agent.id, {
      ...agent,
      sprite,
      missedPolls: 0,
      retiring: false,
      doneSinceMs,
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

export function citizenSize(count: number, width: number, maximum = 44): number {
  if (count <= 0) return maximum;
  const usableWidth = Math.max(240, width - 24);
  return Math.max(24, Math.min(maximum, Math.floor(usableWidth / count) - 6));
}
