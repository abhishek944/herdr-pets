export function createAgentFocus(
  root: HTMLElement,
  focus: (id: string) => Promise<unknown>,
): (id: string) => Promise<void> {
  const notice = document.createElement("div");
  notice.className = "agent-focus-notice";
  notice.setAttribute("role", "status");
  notice.hidden = true;
  root.append(notice);
  const pending = new Set<string>();
  let sequence = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  function show(message: string): void {
    clearTimeout(timer);
    notice.textContent = message;
    notice.hidden = false;
    timer = setTimeout(() => {
      notice.hidden = true;
    }, 10_000);
  }
  return async (id) => {
    if (pending.has(id)) return;
    pending.add(id);
    const request = ++sequence;
    clearTimeout(timer);
    notice.hidden = true;
    try {
      await focus(id);
    } catch (error) {
      if (request === sequence) show(String(error));
    } finally {
      pending.delete(id);
    }
  };
}
