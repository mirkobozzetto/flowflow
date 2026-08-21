import {
  startAuthentication,
  startRegistration,
} from "@simplewebauthn/browser";

interface BeginResp {
  ceremony_id: string;
  options: { publicKey?: unknown };
}

// webauthn-rs wraps its challenge in `publicKey`; @simplewebauthn wants the
// inner options object.
function unwrapOptions<T>(begin: BeginResp): T {
  return (begin.options.publicKey ?? begin.options) as T;
}

class ApiError extends Error {
  status: number;
  constructor(status: number) {
    super(`api ${status}`);
    this.status = status;
  }
}

async function api<T>(path: string, body?: unknown, csrf?: string): Promise<T> {
  const headers: Record<string, string> = {};
  const init: RequestInit = { method: body === undefined ? "GET" : "POST" };
  if (body !== undefined) {
    headers["content-type"] = "application/json";
    init.body = JSON.stringify(body);
  }
  if (csrf) headers["x-csrf-token"] = csrf;
  init.headers = headers;
  const res = await fetch(path, init);
  if (!res.ok) throw new ApiError(res.status);
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

function initAuthPage(): void {
  const root = document.getElementById("auth-root");
  if (!root) return;
  const mode = root.dataset.mode ?? "login";
  const home = root.dataset.home ?? "/";
  const err = document.getElementById("auth-err");
  const button = document.getElementById(
    "auth-go",
  ) as HTMLButtonElement | null;
  if (!button) return;

  const showError = (key: string): void => {
    if (err) err.textContent = root.dataset[key] ?? "";
  };

  const run = async (): Promise<void> => {
    showError("none");
    try {
      if (mode === "login") {
        const email = (
          document.getElementById("auth-email") as HTMLInputElement
        ).value;
        const begin = await api<BeginResp>("/v1/auth/login/begin", { email });
        const credential = await startAuthentication({
          optionsJSON: unwrapOptions(begin),
        });
        await api("/v1/auth/login/finish", {
          ceremony_id: begin.ceremony_id,
          credential,
        });
      } else if (mode === "register") {
        const email = (
          document.getElementById("auth-email") as HTMLInputElement
        ).value;
        const displayName = (
          document.getElementById("auth-name") as HTMLInputElement
        ).value;
        const begin = await api<BeginResp>("/v1/auth/register/begin", {
          email,
          display_name: displayName || null,
        });
        const credential = await startRegistration({
          optionsJSON: unwrapOptions(begin),
        });
        await api("/v1/auth/register/finish", {
          ceremony_id: begin.ceremony_id,
          credential,
        });
      } else {
        const code = (document.getElementById("auth-code") as HTMLInputElement)
          .value;
        const me = await api<{ csrf: string }>("/v1/auth/me");
        await api("/v1/account/link", { link_token: code.trim() }, me.csrf);
      }
      location.href = home;
    } catch (e) {
      if (e instanceof Error && e.name === "NotAllowedError") {
        showError("errNopasskey");
      } else if (e instanceof ApiError && e.status === 409) {
        showError(mode === "register" ? "errTaken" : "errGeneric");
      } else if (e instanceof ApiError && e.status === 401) {
        showError(mode === "login" ? "errNopasskey" : "errGeneric");
      } else {
        showError("errGeneric");
      }
    }
  };

  button.addEventListener("click", () => {
    void run();
  });
}

if (document.readyState !== "loading") {
  initAuthPage();
} else {
  addEventListener("DOMContentLoaded", initAuthPage);
}

export {};
