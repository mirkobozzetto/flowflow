const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
const TAU = Math.PI * 2;

type Kind = "sine" | "voice" | "bars" | "tone";

function resize(canvas: HTMLCanvasElement): CanvasRenderingContext2D | null {
  const ratio = Math.min(devicePixelRatio || 1, 2);
  const width = canvas.offsetWidth;
  const height = canvas.offsetHeight;
  if (canvas.width !== Math.round(width * ratio)) {
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
  }
  const context = canvas.getContext("2d");
  context?.setTransform(ratio, 0, 0, ratio, 0, 0);
  return context;
}

function envelope(x: number, t: number): number {
  const edge = Math.sin(Math.PI * x);
  const speech =
    0.45 +
    0.55 * Math.abs(Math.sin(x * 9 + t * 0.7) * Math.sin(x * 4 - t * 0.4));
  return edge * speech;
}

function stroke(
  context: CanvasRenderingContext2D,
  width: number,
  mid: number,
  amplitude: number,
  phase: number,
  voice: boolean,
): void {
  const cycles = voice ? 7 : 3;
  context.beginPath();
  for (let px = 0; px <= width; px += 2) {
    const x = px / width;
    const shape = voice ? envelope(x, phase) : Math.sin(Math.PI * x);
    const y =
      mid +
      shape *
        amplitude *
        Math.sin(x * TAU * cycles + phase) *
        (voice ? 1 : 0.8);
    if (px === 0) context.moveTo(px, y);
    else context.lineTo(px, y);
  }
  context.stroke();
}

function draw(canvas: HTMLCanvasElement, time: number): void {
  const context = resize(canvas);
  if (!context) return;
  const width = canvas.offsetWidth;
  const height = canvas.offsetHeight;
  const kind = (canvas.dataset.wave ?? "sine") as Kind;
  const mid = height / 2;
  const color = canvas.dataset.color ?? "#ff5c00";
  const phase = time * Number(canvas.dataset.speed ?? 1);
  context.clearRect(0, 0, width, height);
  context.strokeStyle = color;
  context.fillStyle = color;
  context.lineCap = "round";
  context.lineWidth = Number(canvas.dataset.line ?? 1.5);

  if (kind === "bars") {
    const count = Math.floor(width / 7);
    for (let i = 0; i < count; i += 1) {
      const x = i / count;
      const size = Math.max(1.5, envelope(x, phase) * height * 0.9);
      context.beginPath();
      context.roundRect(i * 7, mid - size / 2, 2.5, size, 2);
      context.fill();
    }
    return;
  }

  if (kind === "tone") {
    context.globalAlpha = 0.26;
    for (let i = 0; i < 7; i += 1) {
      stroke(
        context,
        width,
        height * (0.22 + i * 0.09),
        height * 0.1,
        phase * 0.5 + i * 0.5,
        false,
      );
    }
    context.globalAlpha = 1;
    return;
  }

  stroke(context, width, mid, height * 0.42, phase, kind === "voice");
}

function initWaves(): void {
  const canvases = [
    ...document.querySelectorAll<HTMLCanvasElement>("[data-wave]"),
  ];
  const visible = new Set<HTMLCanvasElement>();
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      const canvas = entry.target as HTMLCanvasElement;
      if (entry.isIntersecting) visible.add(canvas);
      else visible.delete(canvas);
    }
  });

  for (const canvas of canvases) {
    observer.observe(canvas);
    draw(canvas, 1);
  }
  if (reduced || canvases.length === 0) return;

  const tick = (now: number): void => {
    for (const canvas of visible) draw(canvas, now / 650);
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

initWaves();

export {};
