export function agentIdForPetClick(target: EventTarget | null): string | null {
  if (!(target instanceof Element)) return null;
  const pet = target.closest(".pet");
  const citizen = pet?.closest<HTMLElement>(".citizen");
  return citizen?.dataset.agentId || null;
}

export function installPetFocus(
  root: HTMLElement,
  focusAgent: (id: string) => void,
): () => void {
  const handleClick = (event: MouseEvent): void => {
    const id = agentIdForPetClick(event.target);
    if (id) focusAgent(id);
  };
  root.addEventListener("click", handleClick);
  return () => root.removeEventListener("click", handleClick);
}
