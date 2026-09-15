import { useCallback, useState } from "react";
import en from "./languages/en.json";

// Only English ships with real translations in v0.1.0. The rest of this list
// mirrors the locales Zarc intends to support — they render in the dropdown
// as disabled/"coming soon" until their JSON files are filled in, following
// the same /languages/<code>.json convention as our other apps.
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
const DICTS: Record<string, Dict> = { en };

function interpolate(str: string, vars?: Record<string, string | number>) {
  if (!vars) return str;
  return Object.entries(vars).reduce(
    (acc, [k, v]) => acc.replaceAll(`{${k}}`, String(v)),
    str
  );
}

export function useI18n() {
  const [locale, setLocale] = useState("en");

  const t = useCallback(
    (key: string, vars?: Record<string, string | number>) => {
      const dict = DICTS[locale] ?? DICTS.en;
      return interpolate(dict[key] ?? DICTS.en[key] ?? key, vars);
    },
    [locale]
  );

  return { locale, setLocale, t };
}
