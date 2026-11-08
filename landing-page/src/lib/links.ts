export const repoUrl = "https://github.com/mirkobozzetto/flowflow";

export const licenseUrl = `${repoUrl}/blob/main/LICENSE`;

export const macDownloadUrl = `${repoUrl}/releases/latest/download/FlowFlow-macos-arm64.dmg`;

const appStore = {
  en: "https://apps.apple.com/be/app/flowflow/id6773033233?l=en-GB",
  fr: "https://apps.apple.com/be/app/flowflow/id6773033233?l=fr-FR",
} as const;

const BOOKING_SUBJECT = "FlowFlow%20-%2030%20minutes";

// The address comes from the CONTACT_EMAIL build variable, never from the
// repo, and reaches the page only reversed and base64-encoded, so the HTML
// holds no address to scrape. scripts/contact.ts rebuilds it on click.
export function bookingCode(): string {
  const email = import.meta.env.CONTACT_EMAIL ?? "";
  if (!email) return "";
  const target = `mailto:${email}?subject=${BOOKING_SUBJECT}`;
  return btoa([...target].reverse().join(""));
}

export const hermesUrl = "/hermes";

export const rustUrl = "https://www.rust-lang.org";

export const dioxusUrl = "https://dioxuslabs.com";

export const hermesSpacesUrl = "/hermes/espaces";

export const hermesKitUrl = "/downloads/FlowFlow-Hermes-kit-2026-10-04.zip";

export function appStoreUrl(lang: string): string {
  return lang === "fr" ? appStore.fr : appStore.en;
}

export const externalLinkAttrs = {
  target: "_blank",
  rel: "noopener noreferrer",
} as const;
