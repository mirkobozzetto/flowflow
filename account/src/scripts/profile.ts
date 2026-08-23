// Profile pane behavior (proposal 0001 T10/T11): visibility pills, field save
// through PUT /v1/me/profile (csrf from /v1/auth/me), and the tap-to-change
// avatar upload (bounded client-side, re-checked server-side).

const MAX_AVATAR_BYTES = 256 * 1024;
// Client-side recompress: any photo shrinks to a 512px JPEG before upload
// (the server re-encodes to 512 anyway), so real-world photos never hit the
// 256 KB refusal.
const AVATAR_MAX_DIM = 512;
const JPEG_QUALITY = 0.85;

async function compressToJpegBase64(file: File): Promise<string> {
  const bmp = await createImageBitmap(file);
  const scale = Math.min(1, AVATAR_MAX_DIM / Math.max(bmp.width, bmp.height));
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(bmp.width * scale));
  canvas.height = Math.max(1, Math.round(bmp.height * scale));
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("canvas unavailable");
  ctx.drawImage(bmp, 0, 0, canvas.width, canvas.height);
  bmp.close();
  const url = canvas.toDataURL("image/jpeg", JPEG_QUALITY);
  return url.slice(url.indexOf(",") + 1);
}

async function csrf(): Promise<string | null> {
  const me = await fetch("/v1/auth/me").then((r) =>
    r.ok ? (r.json() as Promise<{ csrf: string }>) : null,
  );
  return me?.csrf ?? null;
}

function show(el: HTMLElement | null, message?: string): void {
  if (!el) return;
  if (message !== undefined) el.textContent = message;
  el.hidden = false;
}

function initPills(): void {
  for (const pill of document.querySelectorAll<HTMLButtonElement>(
    ".b-pfield .b-pill",
  )) {
    pill.addEventListener("click", () => {
      // Groups is visible but inert in v1: it lights up with shared folders.
      if (pill.classList.contains("soon")) return;
      const group = pill.closest<HTMLElement>(".b-pills");
      if (!group) return;
      for (const other of group.querySelectorAll(".b-pill")) {
        other.classList.toggle("on", other === pill);
      }
      group.dataset.visibility = pill.dataset.vis;
    });
  }
}

function initSave(): void {
  const btn = document.getElementById("profile-save");
  const err = document.getElementById("profile-error");
  const ok = document.getElementById("profile-saved");
  btn?.addEventListener("click", () => {
    void (async () => {
      if (err) err.hidden = true;
      if (ok) ok.hidden = true;
      const body: Record<string, { value: string; visibility: string }> = {};
      for (const row of document.querySelectorAll<HTMLElement>(".b-pfield")) {
        const field = row.dataset.field;
        const input = row.querySelector<HTMLInputElement>("input.val");
        const pills = row.querySelector<HTMLElement>(".b-pills");
        if (!field || !input || !pills) continue;
        body[field] = {
          value: input.value.trim(),
          visibility: pills.dataset.visibility ?? "private",
        };
      }
      const token = await csrf();
      if (!token) {
        show(err, "Session expired - reload the page.");
        return;
      }
      const res = await fetch("/v1/me/profile", {
        method: "PUT",
        headers: {
          "content-type": "application/json",
          "x-csrf-token": token,
        },
        body: JSON.stringify(body),
      });
      if (!res.ok) {
        const detail = (await res.json().catch(() => null)) as {
          error?: string;
        } | null;
        show(err, detail?.error ?? `Error ${res.status}`);
        return;
      }
      show(ok);
    })();
  });
}

function initAvatar(): void {
  const shot = document.getElementById("avatar-shot");
  const file = document.getElementById("avatar-file") as HTMLInputElement | null;
  const err = document.getElementById("avatar-error");
  if (!shot || !file) return;

  shot.addEventListener("click", () => file.click());
  file.addEventListener("change", () => {
    void (async () => {
      if (err) err.hidden = true;
      const picked = file.files?.[0];
      file.value = "";
      if (!picked) return;
      let data: string;
      try {
        data = await compressToJpegBase64(picked);
      } catch {
        show(err, "Could not read this image.");
        return;
      }
      // post-compression backstop; a 512px JPEG never gets near it
      if (data.length > (MAX_AVATAR_BYTES * 4) / 3) {
        show(err, "256 KB max.");
        return;
      }
      const token = await csrf();
      if (!token) {
        show(err, "Session expired - reload the page.");
        return;
      }
      const res = await fetch("/v1/me/profile/avatar", {
        method: "PUT",
        headers: {
          "content-type": "application/json",
          "x-csrf-token": token,
        },
        body: JSON.stringify({ data_base64: data, mime: "image/jpeg" }),
      });
      if (!res.ok) {
        const detail = (await res.json().catch(() => null)) as {
          error?: string;
        } | null;
        show(err, detail?.error ?? `Error ${res.status}`);
        return;
      }
      location.reload();
    })();
  });
}

function initProfile(): void {
  initPills();
  initSave();
  initAvatar();
}

if (document.readyState !== "loading") {
  initProfile();
} else {
  addEventListener("DOMContentLoaded", initProfile);
}

export {};
