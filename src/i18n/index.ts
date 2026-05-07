import en from "./locales/en.json";
import fr from "./locales/fr.json";

export type Locale = "en" | "fr";
export type LanguagePref = "auto" | Locale;

const dictionaries: Record<Locale, unknown> = { en, fr };

export const SUPPORTED_LOCALES: Locale[] = ["en", "fr"];

export function detectLocale(): Locale {
  const lang = (typeof navigator !== "undefined" ? navigator.language : "en")
    .toLowerCase();
  if (lang.startsWith("fr")) return "fr";
  return "en";
}

export function resolveLocale(pref: LanguagePref): Locale {
  if (pref === "auto") return detectLocale();
  return pref;
}

function getNested(obj: unknown, path: string): string | undefined {
  return path.split(".").reduce<unknown>((acc, key) => {
    if (acc && typeof acc === "object" && key in (acc as Record<string, unknown>)) {
      return (acc as Record<string, unknown>)[key];
    }
    return undefined;
  }, obj) as string | undefined;
}

export function translate(
  locale: Locale,
  key: string,
  params?: Record<string, string | number>,
): string {
  const dict = dictionaries[locale] ?? dictionaries.en;
  let template = getNested(dict, key);
  if (typeof template !== "string") {
    template = getNested(dictionaries.en, key);
  }
  if (typeof template !== "string") {
    // unknown key — surface it visibly in dev rather than silently falling back
    return key;
  }
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      template = (template as string).replaceAll(`{${k}}`, String(v));
    }
  }
  return template;
}
