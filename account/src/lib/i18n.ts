import enAccount from "../i18n/en/account.json";
import frAccount from "../i18n/fr/account.json";
import { defaultLang, type Lang } from "../i18n/ui";

export type Namespace = "account";

const sections = {
  en: { account: enAccount },
  fr: { account: frAccount },
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
