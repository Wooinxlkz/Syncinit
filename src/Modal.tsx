import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { EASE_OUT, SPRING_PANEL } from "./ease";
import { cn } from "./cn";

type Align = "center" | "top";

/**
 * Backdrop + spring-entered panel, ported from Xuro's src/components/ui/Modal.tsx
 * so every dialog in Syncinit (Add to archive, password prompt, archive info,
 * about, settings) opens with the same motion and behaves the same way:
 * rendered into document.body via a portal (so it's never clipped by an
 * ancestor's overflow/z-index), traps focus while open, closes on Escape or
 * a click on the backdrop, and restores focus to whatever triggered it.
 */
export function Modal({
  open,
  onClose,
  children,
  align = "center",
  className,
  ariaLabel,
}: {
  open: boolean;
  onClose: () => void;
  children: React.ReactNode;
  align?: Align;
  className?: string;
  ariaLabel?: string;
}) {
  const reduce = useReducedMotion();
  const enterY = reduce ? 0 : align === "top" ? -10 : 10;
  const panelRef = useRef<HTMLDivElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    if (!open) return;
    previousFocus.current = document.activeElement as HTMLElement | null;
    const focusable = () =>
      Array.from(
        panelRef.current?.querySelectorAll<HTMLElement>(
          'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
        ) ?? []
      );
    requestAnimationFrame(() => focusable()[0]?.focus());

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onCloseRef.current();
        return;
      }
      if (event.key !== "Tab") return;
      const items = focusable();
      if (items.length === 0) {
        event.preventDefault();
        panelRef.current?.focus();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      previousFocus.current?.focus();
    };
  }, [open]);

  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0, transition: { duration: 0.1, ease: EASE_OUT } }}
          transition={{ duration: 0.2, ease: EASE_OUT }}
          className={cn("modal-backdrop", align === "top" ? "modal-backdrop-top" : "")}
          onMouseDown={onClose}
        >
          <motion.div
            ref={panelRef}
            role="dialog"
            aria-modal="true"
            aria-label={ariaLabel}
            tabIndex={-1}
            initial={{ opacity: 0, y: enterY, scale: reduce ? 1 : 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{
              opacity: 0,
              y: enterY * 0.6,
              scale: reduce ? 1 : 0.96,
              transition: { duration: 0.14, ease: EASE_OUT },
            }}
            transition={SPRING_PANEL}
            className={cn("modal", className)}
            onMouseDown={(event) => event.stopPropagation()}
          >
            {children}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>,
    document.body
  );
}
