export const defaultLang = "en" as const;

export const languages = {
  en: "English",
  fr: "Français",
} as const;

export type Lang = keyof typeof languages;

export const meta = {
  en: {
    title: "FlowFlow Account",
    description:
      "Your FlowFlow account: devices, services, subscription and security.",
    locale: "en_US",
  },
  fr: {
    title: "FlowFlow Compte",
    description:
      "Ton compte FlowFlow : appareils, services, abonnement et sécurité.",
    locale: "fr_FR",
  },
} as const;

export function getMeta(lang: string) {
  return lang === "fr" ? meta.fr : meta.en;
}
