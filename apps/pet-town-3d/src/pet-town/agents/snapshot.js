export const MAYOR_ID = "pet-town-mayor";

/** The desktop broker already removes the Mayor-owned Firstmate primary session. */
export function visibleTownAgents(snapshot) {
  if (snapshot?.v !== 1 || snapshot.type !== "snapshot") return null;
  const next = new Map();
  const mayor = snapshot.mayor ?? {};
  if (mayor.active) {
    const status = mayor.speaking
      ? "speaking"
      : mayor.listening
        ? "listening"
        : mayor.working
          ? "working"
          : "idle";
    next.set(MAYOR_ID, {
      id: MAYOR_ID,
      label: String(mayor.name || "Mayor").trim() || "Mayor",
      status,
      source: "mayor",
      supportsTerminal: false,
      remoteMachine: null,
      isMayor: true,
      conversationActive: Boolean(
        mayor.conversationActive || mayor.listening || mayor.speaking || mayor.working,
      ),
    });
  }
  if (snapshot.available && Array.isArray(snapshot.agents)) {
    for (const candidate of snapshot.agents) {
      if (!candidate || typeof candidate !== "object") continue;
      const id = typeof candidate.id === "string" ? candidate.id.trim() : "";
      const status = String(candidate.status ?? "unknown").toLowerCase();
      if (!id || id === MAYOR_ID || status === "idle" || status === "unknown") continue;
      const remoteMachine =
        typeof candidate.remoteMachine === "string" ? candidate.remoteMachine : null;
      next.set(id, {
        id,
        label: String(candidate.label || "Agent"),
        status,
        source: String(candidate.source || ""),
        remoteMachine,
        supportsTerminal:
          !remoteMachine && (candidate.supportsTerminal ?? candidate.source === "herdr"),
        isMayor: false,
        conversationActive: false,
      });
    }
  }
  return next;
}
