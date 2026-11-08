export const repoUrl = "https://github.com/mirkobozzetto/flowflow";

export const licenseUrl = `${repoUrl}/blob/main/LICENSE`;

export const macDownloadUrl = `${repoUrl}/releases/latest/download/FlowFlow-macos-arm64.dmg`;

const appStore = {
  en: "https://apps.apple.com/be/app/flowflow/id6773033233?l=en-GB",
  fr: "https://apps.apple.com/be/app/flowflow/id6773033233?l=fr-FR",
} as const;

// Replaced by the Cal.com page before the PR leaves draft.
export const bookingUrl =
  "mailto:mirko.prodev@gmail.com?subject=FlowFlow%20-%2030%20minutes";

export const hermesUrl = "/hermes";

export const hermesSpacesUrl = "/hermes/espaces";

export const hermesKitUrl = "/downloads/FlowFlow-Hermes-kit-2026-10-04.zip";

export function appStoreUrl(lang: string): string {
  return lang === "fr" ? appStore.fr : appStore.en;
}

export const externalLinkAttrs = {
  target: "_blank",
  rel: "noopener noreferrer",
} as const;
