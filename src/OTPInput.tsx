// Adapted from Xuro's src/components/blocks/otp-input.tsx (same project,
// same author) for use as Syncinit's PIN entry. Kept the core mechanics as-is
// (fixed-length slot array so a cleared middle slot stays a hole, single
// hidden-input-drives-everything approach, keyboard/paste/autofill
// handling, caret blink, per-digit enter/exit animation, error shake) and
// changed: Tailwind classes → Syncinit's CSS variables (src/OTPInput.css,
// since this project has no Tailwind config), slot size smaller (Syncinit asked
// for "smaller" — 40px instead of Xuro's 56px), dropped the success
// checkmark glyph since Syncinit's PIN entry never has a "success" state to
// show it in.
import { AnimatePresence, animate, motion, useReducedMotion } from "motion/react";
import { useEffect, useId, useRef, useState } from "react";
import { EASE_OUT } from "./ease";
import { cn } from "./cn";
import "./OTPInput.css";

export type OTPStatus = "idle" | "error";

export interface OTPInputProps {
  length?: number;
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  onComplete?: (value: string) => void;
  label?: string;
  hint?: string;
  errorMessage?: string;
  status?: OTPStatus;
  mask?: boolean;
  disabled?: boolean;
  autoFocus?: boolean;
  "aria-label"?: string;
  className?: string;
}

export function OTPInput({
  length = 6,
  value: controlledValue,
  defaultValue = "",
  onChange,
  onComplete,
  label,
  hint,
  errorMessage,
  status = "idle",
  mask = true,
  disabled = false,
  autoFocus = false,
  "aria-label": ariaLabel = "PIN",
  className,
}: OTPInputProps) {
  const uid = useId();
  const reduce = useReducedMotion();
  const inputRef = useRef<HTMLInputElement>(null);
  const slotsRef = useRef<HTMLDivElement>(null);

  const controlled = controlledValue !== undefined;

  const [slots, setSlots] = useState<string[]>(() =>
    toSlots(controlled ? controlledValue : defaultValue, length)
  );
  const [focused, setFocused] = useState(false);
  const [active, setActive] = useState(0);

  const joined = slots.join("");
  const joinedRef = useRef(joined);

  useEffect(() => {
    joinedRef.current = joined;
  }, [joined]);

  useEffect(() => {
    if (!controlled) return;
    const incoming = sanitize(controlledValue, length);
    if (incoming !== joinedRef.current) setSlots(toSlots(incoming, length));
  }, [controlled, controlledValue, length]);

  const commit = (next: string[]) => {
    const wasComplete = slots.every((c) => c !== "");
    setSlots(next);
    const str = next.join("");
    onChange?.(str);
    if (!wasComplete && next.every((c) => c !== "")) onComplete?.(str);
  };

  const clearSlot = (idx: number) => {
    const next = [...slots];
    next[idx] = "";
    commit(next);
  };

  const slotFromClientX = (clientX: number) => {
    const els = slotsRef.current?.children;
    if (!els) return 0;
    for (let i = 0; i < els.length; i++) {
      if (clientX < els[i].getBoundingClientRect().right) return i;
    }
    return length - 1;
  };

  const insert = (raw: string, from = active) => {
    const digits = raw.replace(/\D/g, "");
    if (!digits) return;
    const next = [...slots];
    let i = from;
    for (const ch of digits) {
      if (i >= length) break;
      next[i] = ch;
      i++;
    }
    commit(next);
    setActive(Math.min(i, length - 1));
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (disabled || e.metaKey || e.ctrlKey || e.altKey) return;
    const k = e.key;
    if (/^[0-9]$/.test(k)) {
      e.preventDefault();
      insert(k);
    } else if (k === "Backspace") {
      e.preventDefault();
      if (slots[active]) {
        clearSlot(active);
      } else if (active > 0) {
        clearSlot(active - 1);
        setActive((current) => Math.max(current - 1, 0));
      }
    } else if (k === "Delete") {
      e.preventDefault();
      clearSlot(active);
    } else if (k === "ArrowLeft") {
      e.preventDefault();
      setActive((a) => Math.max(a - 1, 0));
    } else if (k === "ArrowRight") {
      e.preventDefault();
      setActive((a) => Math.min(a + 1, length - 1));
    } else if (k === "Home") {
      e.preventDefault();
      setActive(0);
    } else if (k === "End") {
      e.preventDefault();
      setActive(length - 1);
    }
  };

  const onPaste = (e: React.ClipboardEvent<HTMLInputElement>) => {
    if (disabled) return;
    e.preventDefault();
    insert(e.clipboardData.getData("text"), active);
  };

  const onChangeNative = (e: React.ChangeEvent<HTMLInputElement>) => {
    const digits = sanitize(e.target.value, length);
    if (!digits) return;
    commit(toSlots(digits, length));
    setActive(Math.min(digits.length, length - 1));
  };

  useEffect(() => {
    if (status !== "error" || reduce || !slotsRef.current) return;
    const controls = animate(
      slotsRef.current,
      { x: [0, -5, 5, -3, 3, -1, 0] },
      { duration: 0.45, ease: EASE_OUT }
    );
    // Stop the animation on unmount instead of leaving it running against a
    // detached node — a dialog can close (Cancel) mid-shake since this
    // fires while the person is still typing a partial PIN.
    return () => controls.stop();
  }, [status, reduce]);

  const activeIndex = focused ? active : -1;
  const message = status === "error" ? errorMessage : hint;

  return (
    <div className={cn("otp-root", className)}>
      {label ? (
        <label htmlFor={`${uid}-input`} className="otp-label">
          {label}
        </label>
      ) : null}
      <fieldset
        className="otp-fieldset"
        onMouseDown={(e) => {
          if (disabled) return;
          e.preventDefault();
          const firstEmpty = slots.indexOf("");
          const cap = firstEmpty === -1 ? length - 1 : firstEmpty;
          setActive(Math.min(slotFromClientX(e.clientX), cap));
          inputRef.current?.focus();
        }}
      >
        <input
          ref={inputRef}
          id={`${uid}-input`}
          inputMode="numeric"
          autoComplete="one-time-code"
          autoFocus={autoFocus}
          disabled={disabled}
          aria-label={ariaLabel}
          aria-invalid={status === "error"}
          value=""
          maxLength={length}
          onKeyDown={onKeyDown}
          onChange={onChangeNative}
          onPaste={onPaste}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          className="otp-hidden-input"
        />

        <div ref={slotsRef} className="otp-slots">
          {Array.from({ length }, (_, i) => {
            const char = slots[i] ?? "";
            const isActive = i === activeIndex;
            return (
              <div
                key={`${uid}-${i}`}
                className={cn(
                  "otp-slot",
                  status === "error" && "otp-slot-error",
                  char && status !== "error" && "otp-slot-filled",
                  isActive && status !== "error" && "otp-slot-active",
                  disabled && "otp-slot-disabled"
                )}
              >
                {isActive ? (
                  <motion.span
                    aria-hidden
                    animate={reduce ? undefined : { opacity: [1, 1, 0, 0] }}
                    transition={
                      reduce ? undefined : { duration: 1, repeat: Infinity, ease: "linear" }
                    }
                    className={cn("otp-caret", char ? "otp-caret-trailing" : "otp-caret-centered")}
                  />
                ) : null}

                <AnimatePresence initial={false}>
                  {char ? (
                    <motion.span
                      key={char}
                      initial={reduce ? { opacity: 0 } : { y: 10, opacity: 0, filter: "blur(3px)" }}
                      animate={reduce ? { opacity: 1 } : { y: 0, opacity: 1, filter: "blur(0px)" }}
                      exit={reduce ? { opacity: 0 } : { y: -10, opacity: 0, filter: "blur(3px)" }}
                      transition={reduce ? { duration: 0 } : { duration: 0.2, ease: EASE_OUT }}
                      className="otp-digit"
                    >
                      {mask ? "•" : char}
                    </motion.span>
                  ) : null}
                </AnimatePresence>
              </div>
            );
          })}
        </div>
      </fieldset>

      {message ? (
        <p aria-live="polite" className={cn("otp-message", status === "error" && "otp-message-error")}>
          {message}
        </p>
      ) : null}
    </div>
  );
}

function sanitize(raw: string, length: number) {
  return raw.replace(/\D/g, "").slice(0, length);
}

function toSlots(raw: string, length: number) {
  const digits = sanitize(raw, length);
  return Array.from({ length }, (_, i) => digits[i] ?? "");
}
