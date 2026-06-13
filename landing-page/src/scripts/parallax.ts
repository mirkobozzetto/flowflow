const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
const finePointer = matchMedia("(pointer: fine)").matches;

export function initParallax(sceneId = "scene"): void {
  const scene = document.getElementById(sceneId);
  if (!scene || reduced || !finePointer) return;

  let frame = 0;
  let targetX = 0;
  let targetY = 0;

  const onMove = (e: MouseEvent) => {
    targetX = e.clientX / innerWidth - 0.5;
    targetY = e.clientY / innerHeight - 0.5;
    if (frame) return;
    frame = requestAnimationFrame(() => {
      const ry = targetX * 7;
      const rx = -targetY * 5;
      scene.style.transform = `rotateY(${ry}deg) rotateX(${rx}deg)`;
      frame = 0;
    });
  };

  addEventListener("mousemove", onMove, { passive: true });
}
