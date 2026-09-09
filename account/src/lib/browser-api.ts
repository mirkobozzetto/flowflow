export class ApiError extends Error {
  constructor(public status: number, public code: string) {
    super(`api ${status}`);
  }
}

export async function api<T>(path: string, body?: unknown, csrf?: string): Promise<T> {
  const headers: Record<string, string> = {};
  const init: RequestInit = { method: body === undefined ? "GET" : "POST", cache: "no-store" };
  if (body !== undefined) {
    headers["content-type"] = "application/json";
    init.body = JSON.stringify(body);
  }
  if (csrf) headers["x-csrf-token"] = csrf;
  init.headers = headers;
  const response = await fetch(path, init);
  if (!response.ok) {
    const error = await response.json().catch(() => null);
    throw new ApiError(response.status, typeof error?.error === "string" ? error.error : "unknown");
  }
  if (response.status === 204) return undefined as T;
  const text = await response.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

// Only first-party onboarding destinations survive a passkey ceremony.
export function continuation(value: string | null, fallback: string): string {
  return value && /^(\/fr)?\/(onboarding|link|invite|verify-email)$/.test(value)
    ? value
    : fallback;
}
