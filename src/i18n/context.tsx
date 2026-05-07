import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  type ReactNode,
} from "react";
import {
  resolveLocale,
  translate,
  type LanguagePref,
  type Locale,
} from "./index";

interface LocaleValue {
  locale: Locale;
  pref: LanguagePref;
  setPref: (pref: LanguagePref) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const LocaleContext = createContext<LocaleValue | null>(null);

interface ProviderProps {
  pref: LanguagePref;
  setPref: (pref: LanguagePref) => void;
  children: ReactNode;
}

export function LocaleProvider({ pref, setPref, children }: ProviderProps) {
  const locale = useMemo(() => resolveLocale(pref), [pref]);
  const t = useCallback(
    (key: string, params?: Record<string, string | number>) =>
      translate(locale, key, params),
    [locale],
  );
  const value = useMemo<LocaleValue>(
    () => ({ locale, pref, setPref, t }),
    [locale, pref, setPref, t],
  );
  return <LocaleContext.Provider value={value}>{children}</LocaleContext.Provider>;
}

export function useT() {
  const ctx = useContext(LocaleContext);
  if (!ctx) {
    // graceful fallback during early render or tests
    return {
      locale: "en" as Locale,
      pref: "auto" as LanguagePref,
      setPref: () => {},
      t: (key: string) => key,
    };
  }
  return ctx;
}
