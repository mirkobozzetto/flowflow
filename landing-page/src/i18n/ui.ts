export const defaultLang = "en" as const;

export const languages = {
  en: "English",
  fr: "Français",
} as const;

export type Lang = keyof typeof languages;

export const meta = {
  en: {
    title: "FlowFlow - A word away",
    description:
      "You talk, FlowFlow handles the rest. Record, transcribe, " +
      "organize and chat with your voice notes. 100% Rust, " +
      "local-first, your keys and your data.",
    locale: "en_US",
  },
  fr: {
    title: "FlowFlow - À portée de voix",
    description:
      "Tu parles, FlowFlow s'occupe du reste. Enregistre, transcris, " +
      "range et discute avec tes notes vocales. 100% Rust, " +
      "local-first, tes clés et tes données.",
    locale: "fr_FR",
  },
} as const;

export function getMeta(lang: string) {
  return lang === "fr" ? meta.fr : meta.en;
}
