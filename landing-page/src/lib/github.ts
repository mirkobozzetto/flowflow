const REPO = "mirkobozzetto/flowflow";
const FALLBACK = "★";

export function formatStars(count: number): string {
  if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
  return String(count);
}

export async function getStarCount(): Promise<string> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return FALLBACK;
    const data = (await res.json()) as { stargazers_count?: number };
    if (typeof data.stargazers_count !== "number") return FALLBACK;
    return formatStars(data.stargazers_count);
  } catch {
    return FALLBACK;
  }
}
