import { defineMiddleware } from "astro:middleware";

const pages = new Set(["/", "/login", "/register", "/link", "/onboarding", "/invite", "/verify-email"]);

export const onRequest = defineMiddleware(async (context, next) => {
  const { url, cookies, request } = context;
  const french = url.pathname === "/fr" || url.pathname.startsWith("/fr/");
  const path = (french ? url.pathname.slice(3) : url.pathname).replace(/\/$/, "") || "/";
  if (!pages.has(path)) return next();
  const choice = url.searchParams.get("lang");
  const saved = cookies.get("flowflow_lang")?.value;
  const preferred = (request.headers.get("accept-language") ?? "")
    .split(",")
    .map((part) => {
      const [tag, quality] = part.trim().toLowerCase().split(";q=");
      return { lang: tag.split("-")[0], quality: quality === undefined ? 1 : Number(quality) };
    })
    .filter((entry) => ["fr", "en"].includes(entry.lang) && entry.quality > 0)
    .sort((a, b) => b.quality - a.quality)[0]?.lang;
  const lang = choice === "en" || choice === "fr" ? choice
    : french ? "fr" : saved === "fr" || saved === "en" ? saved : preferred ?? "en";
  cookies.set("flowflow_lang", lang, {
    path: "/", httpOnly: true, sameSite: "lax", secure: url.protocol === "https:", maxAge: 31536000,
  });
  const target = lang === "fr" ? `/fr${path === "/" ? "" : path}` : path;
  if (target !== url.pathname || choice !== null) {
    const params = new URLSearchParams(url.search);
    params.delete("lang");
    const continuation = params.get("next");
    if (continuation && /^(\/fr)?\/(onboarding|link|invite|verify-email)$/.test(continuation)) {
      params.set("next", `${lang === "fr" ? "/fr" : ""}${continuation.replace(/^\/fr/, "")}`);
    }
    const response = context.redirect(`${target}${params.size ? `?${params}` : ""}`, 302);
    response.headers.set("Cache-Control", "no-store");
    return response;
  }
  const response = await next();
  response.headers.set("Cache-Control", "no-store");
  return response;
});
