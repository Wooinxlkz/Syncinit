// Ported from Xuro's src/components/ui/ContextMenu.tsx (same author, same
// project family): portal to document.body, clamp to the viewport so it
// never renders off-screen, roving keyboard focus (arrows/Home/End),
// Escape and outside-click to dismiss, optional danger styling per item.
// Xuro's own version has no open/close animation — added a fade+scale one
// here using the same EASE_OUT curve the rest of Syncinit's ported-from-Xuro
// surfaces (the modal, the locale dropdown) already use, so this doesn't
// stand out as the one static menu in an otherwise-animated app.
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import type { LucideIcon } from "lucide-react";
import { EASE_OUT } from "./ease";
import { cn } from "./cn";

export interface MenuItem {
  label: string;
  icon?: LucideIcon;
  danger?: boolean;
  disabled?: boolean;
  onSelect: () => void;
}

export interface MenuPosition {
  x: number;
  y: number;
}

export function ContextMenu({
  position,
  items,
  onClose,
}: {
  position: MenuPosition;
  items: MenuItem[];
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [adjusted, setAdjusted] = useState(position);
  const reduce = useReducedMotion();

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    setAdjusted({
      x: Math.min(position.x, window.innerWidth - rect.width - 8),
      y: Math.min(position.y, window.innerHeight - rect.height - 8),
    });
  }, [position]);

  useEffect(() => {
    requestAnimationFrame(() => {
      ref.current?.querySelector<HTMLElement>('[role="menuitem"]:not([disabled])')?.focus();
    });
    const close = (event: MouseEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
        return;
      }
      const focusable = Array.from(
        ref.current?.querySelectorAll<HTMLElement>('[role="menuitem"]:not([disabled])') ?? []
      );
      const index = focusable.indexOf(document.activeElement as HTMLElement);
      let next = index;
      if (event.key === "ArrowDown") next = (index + 1) % focusable.length;
      else if (event.key === "ArrowUp") next = (index - 1 + focusable.length) % focusable.length;
      else if (event.key === "Home") next = 0;
      else if (event.key === "End") next = focusable.length - 1;
      else return;
      event.preventDefault();
      focusable[next]?.focus();
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("contextmenu", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("contextmenu", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return createPortal(
    <AnimatePresence>
      <motion.div
        ref={ref}
        role="menu"
        className="ctx-menu"
        style={{ left: adjusted.x, top: adjusted.y }}
        initial={reduce ? { opacity: 0 } : { opacity: 0, scale: 0.96, y: -4 }}
        animate={reduce ? { opacity: 1 } : { opacity: 1, scale: 1, y: 0 }}
        exit={reduce ? { opacity: 0 } : { opacity: 0, scale: 0.96, y: -4 }}
        transition={{ duration: 0.12, ease: EASE_OUT }}
      >
        {items.map((item) => (
          <button
            key={item.label}
            type="button"
            role="menuitem"
            disabled={item.disabled}
            className={cn("ctx-menu-item", item.danger && "ctx-menu-item-danger")}
            onClick={() => {
              onClose();
              item.onSelect();
            }}
          >
            {item.icon ? <item.icon size={14} strokeWidth={1.75} /> : null}
            {item.label}
          </button>
        ))}
      </motion.div>
    </AnimatePresence>,
    document.body
  );
}
