import enAgentic from "../i18n/en/agentic.json";
import enBeta from "../i18n/en/beta.json";
import enBento from "../i18n/en/bento.json";
import enFaq from "../i18n/en/faq.json";
import enFooter from "../i18n/en/footer.json";
import enHero from "../i18n/en/hero.json";
import enShowcase from "../i18n/en/showcase.json";
import frAgentic from "../i18n/fr/agentic.json";
import frBeta from "../i18n/fr/beta.json";
import frBento from "../i18n/fr/bento.json";
import frFaq from "../i18n/fr/faq.json";
import frFooter from "../i18n/fr/footer.json";
import frHero from "../i18n/fr/hero.json";
import frShowcase from "../i18n/fr/showcase.json";
import { defaultLang, type Lang } from "../i18n/ui";

export type Namespace =
  | "hero"
  | "showcase"
  | "agentic"
  | "bento"
  | "faq"
  | "footer"
  | "beta";

const sections = {
  en: {
    hero: enHero,
    showcase: enShowcase,
    agentic: enAgentic,
    bento: enBento,
    faq: enFaq,
    footer: enFooter,
    beta: enBeta,
  },
  fr: {
    hero: frHero,
    showcase: frShowcase,
    agentic: frAgentic,
    bento: frBento,
    faq: frFaq,
    footer: frFooter,
    beta: frBeta,
  },
} as const;

export const supportedLangs = ["en", "fr"] as const;

export function isLang(value: string): value is Lang {
  return (supportedLangs as readonly string[]).includes(value);
}

function resolveLang(lang: string): Lang {
  return isLang(lang) ? lang : defaultLang;
}

function readPath(source: unknown, path: string): unknown {
  return path.split(".").reduce<unknown>((acc, key) => {
    if (acc && typeof acc === "object" && key in acc) {
      return (acc as Record<string, unknown>)[key];
    }
    return undefined;
  }, source);
}

export function section<N extends Namespace>(
  lang: string,
  namespace: N,
): (typeof sections)["en"][N] {
  const resolved = resolveLang(lang);
  return sections[resolved][namespace] as (typeof sections)["en"][N];
}

export function useTranslations(lang: string, namespace: Namespace) {
  const resolved = resolveLang(lang);
  const pack = sections[resolved][namespace];
  const fallback = sections[defaultLang][namespace];
  return function t<T = string>(key: string): T {
    const value = readPath(pack, key) ?? readPath(fallback, key);
    return value as T;
  };
}

export function localizePath(path: string, lang: string): string {
  const resolved = resolveLang(lang);
  const clean = path.startsWith("/") ? path : `/${path}`;
  if (resolved === defaultLang) return clean;
  return `/${resolved}${clean === "/" ? "" : clean}`;
}
