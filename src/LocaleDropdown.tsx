import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { EASE_OUT } from "./ease";
import { SUPPORTED_LOCALES } from "./i18n";

interface Props {
  locale: string;
  onChange: (code: string) => void;
}

export function LocaleDropdown({ locale, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotion();

  useEffect(() => {
    if (!open) return;
    const onDocDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDocDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const current = SUPPORTED_LOCALES.find((l) => l.code === locale) ?? SUPPORTED_LOCALES[0];

  return (
    <div className="locale-dropdown" ref={rootRef}>
      <button type="button" className="locale-trigger" onClick={() => setOpen((o) => !o)}>
        {current.label}
        <span aria-hidden style={{ fontSize: 9, opacity: 0.6 }}>
          ▾
        </span>
      </button>

      <AnimatePresence>
        {open && (
          <motion.div
            role="listbox"
            className="locale-panel"
            initial={reduce ? { opacity: 0 } : { opacity: 0, y: -4, scale: 0.98 }}
            animate={reduce ? { opacity: 1 } : { opacity: 1, y: 0, scale: 1 }}
            exit={reduce ? { opacity: 0 } : { opacity: 0, y: -4, scale: 0.98 }}
            transition={{ duration: 0.15, ease: EASE_OUT }}
          >
            {SUPPORTED_LOCALES.map((l) => (
              <button
                key={l.code}
                type="button"
                role="option"
                aria-selected={l.code === locale}
                disabled={!l.ready}
                className="locale-option"
                onClick={() => {
                  if (!l.ready) return;
                  onChange(l.code);
                  setOpen(false);
                }}
              >
                <span>{l.label}</span>
                {!l.ready && <span className="locale-option-soon">soon</span>}
              </button>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
