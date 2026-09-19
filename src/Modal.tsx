import * as Dialog from "@radix-ui/react-dialog";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { EASE_OUT, SPRING_PANEL } from "./ease";
import { cn } from "./cn";

type Align = "center" | "top";

/**
 * Backdrop + spring-entered panel, built on Radix's Dialog primitive
 * (@radix-ui/react-dialog) instead of a hand-rolled focus trap — every
 * dialog in Syncinit (Add to archive, password prompt, settings, archive
 * info) goes through this. Radix owns focus-trapping, Escape-to-close,
 * outside-click, portal mounting and aria wiring — all things a hand-rolled
 * version is easy to get subtly wrong in a way that only shows up on
 * specific close paths (a freeze on Cancel was traced to exactly that).
 * Framer Motion only drives the visual animation on top: `layout` on the
 * panel means its height smoothly animates to fit whatever content is
 * inside it (e.g. switching tabs in Add-to-archive) instead of snapping.
 *
 * Structural note: Radix renders Dialog.Overlay and Dialog.Content as
 * *siblings* inside the portal, not nested — so Content gets its own
 * full-viewport centering layer (.modal-layer, pointer-events: none) with
 * the actual visible card (.modal, pointer-events: auto) inside it. That
 * split is what lets a click on the empty space *around* the card fall
 * through to the Overlay underneath and close the dialog, while a click on
 * the card itself doesn't.
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

  return (
    <Dialog.Root open={open} onOpenChange={(next) => !next && onClose()}>
      <AnimatePresence>
        {open && (
          <Dialog.Portal forceMount>
            <Dialog.Overlay asChild forceMount>
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0, transition: { duration: 0.1, ease: EASE_OUT } }}
                transition={{ duration: 0.2, ease: EASE_OUT }}
                className="modal-backdrop"
              />
            </Dialog.Overlay>
            <Dialog.Content asChild forceMount aria-describedby={undefined}>
              <div className={cn("modal-layer", align === "top" ? "modal-layer-top" : "")}>
                <motion.div
                  layout
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
                >
                  <Dialog.Title className="sr-only">{ariaLabel}</Dialog.Title>
                  {children}
                </motion.div>
              </div>
            </Dialog.Content>
          </Dialog.Portal>
        )}
      </AnimatePresence>
    </Dialog.Root>
  );
}
