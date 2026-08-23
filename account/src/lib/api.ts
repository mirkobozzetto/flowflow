// Server-side reads: the SSR page calls the backend directly (same network,
// no browser in the loop) with the visitor's cookie. ACCOUNT_PREVIEW=1 swaps
// in fixture data so the design can be reviewed with no backend running.

const BACKEND_URL =
  import.meta.env.BACKEND_URL ??
  process.env.BACKEND_URL ??
  "http://localhost:8080";

const PREVIEW = process.env.ACCOUNT_PREVIEW === "1";

export interface Me {
  web_user_id: string;
  email: string;
  display_name: string | null;
  role: string;
}

export interface DeviceRow {
  device_id: string;
  created_at: string;
  last_seen: string | null;
}

export interface PlanRow {
  plan: string;
  status: string;
  expires_at: string | null;
}

// GET /v1/me/entitlements: the raw plan rows plus the resolved catalog item
// ids (account-level, not per plan row).
interface EntitlementsResp {
  plans: PlanRow[];
  items: string[];
}

export interface ConnectionRow {
  device_id: string;
  provider: string;
  access_expires_at: string | null;
  scopes: string | null;
}

// Backend shape of GET /v1/me/requests; the UI shows target_id as the item.
interface BackendRequestRow {
  target_kind: string;
  target_id: string;
  status: string;
  created_at: string;
  decided_at: string | null;
}

export interface RequestRow {
  item: string;
  status: string;
  decided_at: string | null;
}

export interface LoginEventRow {
  kind: string;
  at: string;
}

export interface AccountData {
  linked: boolean;
  devices: DeviceRow[];
  plans: PlanRow[];
  items: string[];
  connections: ConnectionRow[];
  requests: RequestRow[];
  loginEvents: LoginEventRow[];
}

async function backendJson<T>(path: string, cookie: string): Promise<T | null> {
  const res = await fetch(`${BACKEND_URL}${path}`, {
    headers: { cookie },
  });
  if (!res.ok) return null;
  return (await res.json()) as T;
}

export async function fetchMe(cookie: string): Promise<Me | null> {
  if (PREVIEW) {
    return {
      web_user_id: "preview",
      email: "mirko@flowflow.be",
      display_name: "Mirko Bozzetto",
      role: "user",
    };
  }
  if (!cookie) return null;
  return backendJson<Me>("/v1/auth/me", cookie);
}

// Profile KV (proposal 0001): field -> value + per-field visibility.
export interface ProfileField {
  value: string;
  visibility: "private" | "groups" | "public";
}

export type ProfileData = Record<string, ProfileField>;

export async function fetchProfile(cookie: string): Promise<ProfileData> {
  if (PREVIEW) {
    return {
      display_name: { value: "Mirko Bozzetto", visibility: "public" },
      bio: {
        value: "Full-stack developer, Brussels. Rust, voice notes, local-first.",
        visibility: "groups",
      },
      website: { value: "https://mirkobozzetto.com", visibility: "public" },
    };
  }
  return (await backendJson<ProfileData>("/v1/me/profile", cookie)) ?? {};
}

export async function fetchAccountData(cookie: string): Promise<AccountData> {
  if (PREVIEW) return previewData();
  const [devices, entitlements, connections, requests, loginEvents] =
    await Promise.all([
      backendJson<DeviceRow[]>("/v1/me/devices", cookie),
      backendJson<EntitlementsResp>("/v1/me/entitlements", cookie),
      backendJson<ConnectionRow[]>("/v1/me/connections", cookie),
      backendJson<BackendRequestRow[]>("/v1/me/requests", cookie),
      backendJson<LoginEventRow[]>("/v1/me/login-events", cookie),
    ]);
  return {
    linked: (devices ?? []).length > 0,
    devices: devices ?? [],
    plans: entitlements?.plans ?? [],
    items: entitlements?.items ?? [],
    connections: connections ?? [],
    requests: (requests ?? []).map((r) => ({
      item: r.target_id,
      status: r.status,
      decided_at: r.decided_at,
    })),
    loginEvents: loginEvents ?? [],
  };
}

export function shortId(id: string): string {
  if (id.length <= 9) return id;
  return `${id.slice(0, 4)}…${id.slice(-4)}`;
}

export function fmtDate(iso: string | null, lang: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat(lang === "fr" ? "fr-BE" : "en-GB", {
    day: "numeric",
    month: "short",
    year: "numeric",
  }).format(d);
}

export function fmtDateTime(iso: string | null, lang: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat(lang === "fr" ? "fr-BE" : "en-GB", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  }).format(d);
}

function previewData(): AccountData {
  const now = new Date();
  const daysAgo = (n: number) =>
    new Date(now.getTime() - n * 86400_000).toISOString();
  return {
    linked: true,
    devices: [
      {
        device_id: "d4b17c02-91ce-4f7d-a2b8-33e05c1e7a02",
        created_at: daysAgo(107),
        last_seen: daysAgo(0),
      },
      {
        device_id: "91ce3f7d-55aa-4e21-9c47-8b12ef043f7d",
        created_at: daysAgo(107),
        last_seen: daysAgo(1),
      },
    ],
    plans: [
      {
        plan: "premium",
        status: "active",
        expires_at: null,
      },
    ],
    items: ["agent-crm", "sheets", "gmail", "exa-search"],
    connections: [
      {
        device_id: "d4b17c02-91ce-4f7d-a2b8-33e05c1e7a02",
        provider: "google-sheets",
        access_expires_at: daysAgo(-30),
        scopes: "spreadsheets drive",
      },
      {
        device_id: "d4b17c02-91ce-4f7d-a2b8-33e05c1e7a02",
        provider: "gmail",
        access_expires_at: daysAgo(-30),
        scopes: "gmail.readonly",
      },
    ],
    requests: [
      {
        item: "agent-veille",
        status: "pending",
        decided_at: null,
      },
    ],
    loginEvents: [
      { kind: "login", at: daysAgo(0) },
      { kind: "login", at: daysAgo(1) },
      { kind: "register", at: daysAgo(107) },
    ],
  };
}
