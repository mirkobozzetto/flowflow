const REPO = "mirkobozzetto/flowflow";
export const STARS_FALLBACK = "—";

export function formatStars(count: number): string {
  if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
  return String(count);
}

// GitHub allows 60 anonymous reads an hour per IP: one read per process
// (or page) keeps dev renders from burning the quota.
let pending: Promise<string> | undefined;

export function getStarCount(): Promise<string> {
  pending ??= readStarCount();
  return pending;
}

async function readStarCount(): Promise<string> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return STARS_FALLBACK;
    const data = (await res.json()) as { stargazers_count?: number };
    if (typeof data.stargazers_count !== "number") return STARS_FALLBACK;
    return formatStars(data.stargazers_count);
  } catch {
    return STARS_FALLBACK;
  }
}
