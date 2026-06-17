type CaptureResult = { ok: boolean };

async function submitEmail(email: string): Promise<CaptureResult> {
  const form = document.querySelector<HTMLFormElement>("[data-capture]");
  const endpoint = form?.dataset.endpoint;
  if (!endpoint) {
    return { ok: true };
  }
  try {
    const res = await fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email }),
    });
    return { ok: res.ok };
  } catch {
    return { ok: false };
  }
}

function initCapture(): void {
  const form = document.querySelector<HTMLFormElement>("[data-capture]");
  if (!form) return;
  const input = form.querySelector<HTMLInputElement>("input[type=email]");
  const ok =
    form.parentElement?.querySelector<HTMLElement>("[data-capture-ok]");

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const email = input?.value.trim();
    if (!email) return;
    const result = await submitEmail(email);
    if (!result.ok) return;
    form.style.display = "none";
    if (ok) ok.style.display = "inline-flex";
  });
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initCapture);
} else {
  initCapture();
}

export { initCapture, submitEmail };
