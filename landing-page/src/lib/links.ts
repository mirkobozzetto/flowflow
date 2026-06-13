export const repoUrl = "https://github.com/mirkobozzetto/flowflow";

export const licenseUrl = `${repoUrl}/blob/main/LICENSE`;

const dmgFile = "FlowFlow-0.1.1-macos-arm64.dmg";

export const macDownloadUrl = `${repoUrl}/releases/download/v0.1.1/${dmgFile}`;

const appStore = {
  en: "https://apps.apple.com/be/app/flowflow/id6773033233?l=en-GB",
  fr: "https://apps.apple.com/be/app/flowflow/id6773033233?l=fr-FR",
} as const;

export function appStoreUrl(lang: string): string {
  return lang === "fr" ? appStore.fr : appStore.en;
}

export const externalLinkAttrs = {
  target: "_blank",
  rel: "noopener noreferrer",
} as const;
