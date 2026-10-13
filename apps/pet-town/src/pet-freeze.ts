/**
 * Dedicated blocked and done loops act in place, so they keep playing unless a real policy asks
 * for a frozen frame. Pets without accepted sidecar art keep the legacy still frame because their
 * blocked and done fallbacks are locomotion clips that would otherwise walk in place.
 */
export function freezeActivityFrame(
  frozen: boolean,
  theme: string,
  state: string,
  dedicated: boolean,
): boolean {
  if (frozen) return true;
  return !dedicated && (state === "blocked" || (state === "done" && theme !== "railway"));
}

export function freezePetFrame(element: HTMLElement): void {
  const image = element.querySelector<HTMLImageElement>("img.pet");
  if (!image || image.hidden || !image.complete || image.naturalWidth === 0) return;
  const existing = element.querySelector<HTMLCanvasElement>("canvas.pet-freeze");
  const key = `${image.dataset.assetReadyKey ?? image.dataset.assetKey ?? ""}:${image.className}`;
  if (existing?.dataset.freezeKey === key) return;
  existing?.remove();
  const canvas = document.createElement("canvas");
  canvas.width = image.naturalWidth;
  canvas.height = image.naturalHeight;
  canvas.className = `${image.className} pet-freeze`;
  canvas.dataset.freezeKey = key;
  canvas.setAttribute("aria-hidden", "true");
  canvas.style.setProperty("--clip-scale", image.style.getPropertyValue("--clip-scale"));
  const context = canvas.getContext("2d");
  if (!context) return;
  context.drawImage(image, 0, 0);
  // Keep the image in the hit-test tree while the canvas supplies the frozen
  // pixels. Replacing the pointer target between two clicks prevents browsers
  // from recognizing a double-click.
  image.style.opacity = "0";
  image.insertAdjacentElement("afterend", canvas);
}

export function unfreezePetFrame(element: HTMLElement): void {
  element.querySelector<HTMLCanvasElement>("canvas.pet-freeze")?.remove();
  const image = element.querySelector<HTMLImageElement>("img.pet");
  if (image) image.style.opacity = "";
}
