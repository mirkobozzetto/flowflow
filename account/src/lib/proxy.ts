// Same-origin proxy to the Rust API: the browser only ever talks to
// account.flowflow.be, so the backend's httpOnly SameSite=Strict cookie stays
// first-party and host-only. Four narrow prefixes are mounted under
// src/pages/v1/ - NEVER a /v1 catch-all, which would expose /v1/admin/* and
// the device endpoints from the customer origin.

const BACKEND_URL =
  import.meta.env.BACKEND_URL ??
  process.env.BACKEND_URL ??
  "http://localhost:8080";

const FORWARD_IN = ["cookie", "x-csrf-token", "content-type"];

export async function forward(
  request: Request,
  clientAddress: string,
): Promise<Response> {
  const incoming = new URL(request.url);
  const target = `${BACKEND_URL}${incoming.pathname}${incoming.search}`;

  const headers = new Headers();
  for (const name of FORWARD_IN) {
    const value = request.headers.get(name);
    if (value) headers.set(name, value);
  }
  const upstreamXff = request.headers.get("x-forwarded-for");
  headers.set(
    "x-forwarded-for",
    upstreamXff ? `${upstreamXff}, ${clientAddress}` : clientAddress,
  );

  const init: RequestInit = { method: request.method, headers };
  if (request.method !== "GET" && request.method !== "HEAD") {
    init.body = await request.arrayBuffer();
  }

  const res = await fetch(target, init);

  const out = new Headers();
  const contentType = res.headers.get("content-type");
  if (contentType) out.set("content-type", contentType);
  for (const cookie of res.headers.getSetCookie()) {
    out.append("set-cookie", cookie);
  }
  if (incoming.pathname.startsWith("/v1/me")) {
    out.set("cache-control", "no-store");
  }
  return new Response(res.body, { status: res.status, headers: out });
}
