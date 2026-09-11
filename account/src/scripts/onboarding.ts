import { api, ApiError } from "../lib/browser-api";

type Onboarding = {
  linked: boolean;
  account_id: string | null;
  email_verified: boolean | null;
  premium: boolean;
  premium_status: "active" | "expired" | "inactive";
  premium_expires_at: string | null;
  request: { status: "pending" | "approved" | "denied" } | null;
};
type Session = { csrf: string; email: string };
type Messages = Record<string, string | Record<string, string>>;

const root = document.getElementById("onboarding-root");
if (root) {
  const mode = root.dataset.mode ?? "onboarding";
  const lang = root.dataset.lang === "fr" ? "fr" : "en";
  const messages = JSON.parse(root.dataset.messages ?? "{}") as Messages;
  const text = (key: string): string => typeof messages[key] === "string" ? messages[key] as string : "";
  const status = document.getElementById("onboarding-status")!;
  const guest = document.getElementById("onboarding-guest")!;
  const account = document.getElementById("onboarding-account")!;
  const button = (id: string) => document.getElementById(`onboarding-${id}`) as HTMLButtonElement | null;
  const set = (id: string, value: string) => { document.getElementById(`onboarding-${id}`)!.textContent = value; };
  const tokenKey = `flowflow:onboarding:${mode}`;
  let token: string | null = null;
  let session: Session | null = null;
  let state: Onboarding | null = null;
  let busy = false;
  let loading = false;
  let storageFailed = false;

  if (mode === "invite" || mode === "verify-email") {
    const fragment = new URLSearchParams(location.hash.slice(1));
    const incoming = fragment.get("token");
    history.replaceState(null, "", location.pathname + location.search);
    try {
      if (incoming && /^[A-Za-z0-9_-]{16,512}$/.test(incoming)) {
        token = incoming;
        sessionStorage.setItem(tokenKey, JSON.stringify({ token, savedAt: Date.now() }));
      } else {
        const saved = JSON.parse(sessionStorage.getItem(tokenKey) ?? "null");
        // Browser retention is bounded independently of the server's token TTL.
        if (saved && typeof saved.token === "string" && /^[A-Za-z0-9_-]{16,512}$/.test(saved.token)
          && typeof saved.savedAt === "number" && Date.now() - saved.savedAt < 24 * 60 * 60 * 1000) {
          token = saved.token;
        } else {
          sessionStorage.removeItem(tokenKey);
        }
      }
    } catch {
      storageFailed = true;
    }
  }

  function showError(error: unknown): void {
    const errors = messages.errors as Record<string, string>;
    const key = error instanceof ApiError ? error.code : "unknown";
    status.textContent = errors[key] ?? (error instanceof ApiError && error.status === 429
      ? errors.rateLimited : error instanceof ApiError && error.status === 401
        ? errors.unauthorized : errors.unknown);
  }

  function controls(): void {
    const unavailable = busy || loading || !session || !state;
    const request = button("request");
    if (request) {
      request.hidden = state?.premium === true;
      request.disabled = unavailable || !state?.linked || state.premium || state.request?.status === "pending";
    }
    const redeem = button("redeem");
    if (redeem) redeem.disabled = unavailable || !token;
    button("refresh")!.disabled = busy || loading;
    button("signout")!.disabled = busy || loading;
  }

  function render(value: Onboarding): void {
    state = value;
    set("link-state", text(value.linked ? "linked" : "unlinked"));
    set("account-id", value.account_id ?? "");
    set("premium-state", text(`premium_${value.premium_status}`));
    const expiry = value.premium_expires_at ? new Date(value.premium_expires_at) : null;
    set("expiry", expiry && Number.isFinite(expiry.getTime())
      ? `${text("expiry")} ${new Intl.DateTimeFormat(lang, { dateStyle: "medium", timeStyle: "short" }).format(expiry)}` : "");
    set("request-state", value.premium ? "" : text(`request_${value.request?.status ?? "none"}`));
    document.getElementById("onboarding-premium-hint")!.hidden = value.premium;
    document.getElementById("onboarding-link-instructions")!.hidden = value.linked;
  }

  async function refresh(): Promise<void> {
    if (loading) return;
    loading = true;
    controls();
    try {
      session = await api<Session>("/v1/auth/me");
      set("email", session.email);
      render(await api<Onboarding>("/v1/me/onboarding"));
      guest.hidden = true;
      account.hidden = false;
      status.textContent = storageFailed ? text("storageUnavailable")
        : mode !== "onboarding" && !token ? text("missingToken") : "";
    } catch (error) {
      state = null;
      if (error instanceof ApiError && error.status === 401) {
        session = null;
        guest.hidden = false;
        account.hidden = true;
        status.textContent = storageFailed ? text("storageUnavailable") : "";
      } else {
        showError(error);
      }
    } finally {
      loading = false;
      controls();
    }
  }

  async function action(kind: "request" | "redeem" | "signout"): Promise<void> {
    if (busy || loading || !session || !state) return;
    busy = true;
    controls();
    status.textContent = text("working");
    try {
      // Re-read CSRF so an account switch in another tab cannot reuse stale auth.
      const current = await api<Session>("/v1/auth/me");
      if (current.email !== session.email) { await refresh(); return; }
      if (kind === "signout") {
        await api("/v1/auth/logout", {}, current.csrf);
        session = null;
        await refresh();
      } else if (kind === "request") {
        render(await api<Onboarding>("/v1/me/premium-requests", {}, current.csrf));
        status.textContent = text("request_pending");
      } else if (token) {
        const endpoint = mode === "invite" ? "/v1/me/invitations/accept" : "/v1/me/email-verification/confirm";
        render(await api<Onboarding>(endpoint, { token }, current.csrf));
        token = null;
        try { sessionStorage.removeItem(tokenKey); } catch { /* The token is also cleared in memory. */ }
        status.textContent = text(mode === "invite" ? "invitationAccepted" : "emailVerified");
      }
    } catch (error) {
      if (error instanceof ApiError && error.code === "token_invalid_or_expired") {
        token = null;
        try { sessionStorage.removeItem(tokenKey); } catch { /* Do not retain an unusable capability. */ }
        await refresh();
      }
      showError(error);
    } finally {
      busy = false;
      controls();
    }
  }

  for (const kind of ["request", "redeem", "signout"] as const) {
    button(kind)?.addEventListener("click", () => { void action(kind); });
  }
  button("refresh")?.addEventListener("click", () => { void refresh(); });
  addEventListener("hashchange", () => {
    if (new URLSearchParams(location.hash.slice(1)).has("token")) location.reload();
  });
  const resume = () => { if (!busy && document.visibilityState === "visible") void refresh(); };
  addEventListener("focus", resume);
  document.addEventListener("visibilitychange", resume);
  void refresh();
}
