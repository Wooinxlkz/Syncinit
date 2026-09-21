import { useCallback, useEffect, useState } from "react";

// One JSON file per locale under ./locales/<code>.json, loaded on demand via
// dynamic import. All 12 are translated now (20 keys each, everything the
// UI currently has strings for) — Arabic is translated but the app layout
// itself isn't RTL-aware yet (no dir="rtl" flip), so Arabic text reads
// correctly but the overall toolbar/panel layout stays left-to-right.
export const SUPPORTED_LOCALES = [
  { code: "en", label: "English", ready: true },
  { code: "fr", label: "Français", ready: true },
  { code: "de", label: "Deutsch", ready: true },
  { code: "es", label: "Español", ready: true },
  { code: "nl", label: "Nederlands", ready: true },
  { code: "no", label: "Norsk", ready: true },
  { code: "pl", label: "Polski", ready: true },
  { code: "ru", label: "Русский", ready: true },
  { code: "ar", label: "العربية", ready: true },
  { code: "id", label: "Bahasa Indonesia", ready: true },
  { code: "zh-CN", label: "中文(简体)", ready: true },
  { code: "ja", label: "日本語", ready: true },
] as const;

type Dict = Record<string, string>;
const cache = new Map<string, Dict>();
const STORAGE_KEY = "syncinit:locale";

async function loadLocale(code: string): Promise<Dict> {
  if (cache.has(code)) return cache.get(code)!;
  try {
    const mod = await import(`./locales/${code}.json`);
    const dict = (mod.default ?? mod) as Dict;
    cache.set(code, dict);
    return dict;
  } catch {
    // Locale has no JSON file yet (everything but "en" right now) — fall
    // back to English rather than a wall of raw keys.
    return loadLocale("en");
  }
}

function interpolate(str: string, vars?: Record<string, string | number>) {
  if (!vars) return str;
  return Object.entries(vars).reduce((acc, [k, v]) => acc.replaceAll(`{${k}}`, String(v)), str);
}

export function useI18n() {
  const [locale, setLocaleState] = useState(
    () => localStorage.getItem(STORAGE_KEY) ?? "en"
  );
  const [dict, setDict] = useState<Dict>({});
  const [enDict, setEnDict] = useState<Dict>({});

  useEffect(() => {
    loadLocale("en").then(setEnDict);
  }, []);

  useEffect(() => {
    let cancelled = false;
    loadLocale(locale).then((d) => {
      if (!cancelled) setDict(d);
    });
    return () => {
      cancelled = true;
    };
  }, [locale]);

  const setLocale = useCallback((code: string) => {
    localStorage.setItem(STORAGE_KEY, code);
    setLocaleState(code);
  }, []);

  const t = useCallback(
    (key: string, vars?: Record<string, string | number>) => {
      return interpolate(dict[key] ?? enDict[key] ?? key, vars);
    },
    [dict, enDict]
  );

  return { locale, setLocale, t };
}
