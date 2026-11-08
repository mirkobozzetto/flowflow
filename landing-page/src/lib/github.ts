const REPO = "mirkobozzetto/flowflow";
// Shields caches GitHub and answers with CORS; the anonymous GitHub API
// is capped at 60 reads an hour per IP and sometimes answers 5xx.
const SHIELDS_URL = `https://img.shields.io/github/stars/${REPO}.json`;
const GITHUB_URL = `https://api.github.com/repos/${REPO}`;
export const STARS_FALLBACK = "—";

export function formatStars(count: number): string {
  if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
  return String(count);
}

// One read per process (or page) keeps dev renders from burning quotas.
let pending: Promise<string> | undefined;

export function getStarCount(): Promise<string> {
  pending ??= readStarCount();
  return pending;
}

async function readStarCount(): Promise<string> {
  return (await fromShields()) ?? (await fromGithub()) ?? STARS_FALLBACK;
}

async function fromShields(): Promise<string | undefined> {
  try {
    const res = await fetch(SHIELDS_URL);
    if (!res.ok) return undefined;
    const data = (await res.json()) as { value?: string; message?: string };
    const value = data.value ?? data.message;
    return value && /\d/.test(value) ? value : undefined;
  } catch {
    return undefined;
  }
}

async function fromGithub(): Promise<string | undefined> {
  try {
    const res = await fetch(GITHUB_URL, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return undefined;
    const data = (await res.json()) as { stargazers_count?: number };
    if (typeof data.stargazers_count !== "number") return undefined;
    return formatStars(data.stargazers_count);
  } catch {
    return undefined;
  }
}
