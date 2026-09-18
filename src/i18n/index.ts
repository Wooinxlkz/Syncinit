import { useCallback, useEffect, useState } from "react";

// One JSON file per locale under ./locales/<code>.json, loaded on demand via
// dynamic import so adding a translated locale later is just: drop the file
// here, flip `ready` to true below — no other code changes. Only en.json
// exists right now.
export const SUPPORTED_LOCALES = [
  { code: "en", label: "English", ready: true },
  { code: "fr", label: "Français", ready: false },
  { code: "de", label: "Deutsch", ready: false },
  { code: "es", label: "Español", ready: false },
  { code: "it", label: "Italiano", ready: false },
  { code: "pt-BR", label: "Português (BR)", ready: false },
  { code: "pt-PT", label: "Português (PT)", ready: false },
  { code: "nl", label: "Nederlands", ready: false },
  { code: "pl", label: "Polski", ready: false },
  { code: "ru", label: "Русский", ready: false },
  { code: "tr", label: "Türkçe", ready: false },
  { code: "ar", label: "العربية", ready: false },
  { code: "zh-CN", label: "中文(简体)", ready: false },
  { code: "ja", label: "日本語", ready: false },
  { code: "ko", label: "한국어", ready: false },
  { code: "id", label: "Bahasa Indonesia", ready: false },
  { code: "sv", label: "Svenska", ready: false },
  { code: "da", label: "Dansk", ready: false },
  { code: "ro", label: "Română", ready: false },
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
