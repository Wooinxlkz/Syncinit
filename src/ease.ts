// Same curve Xuro uses everywhere (its src/lib/ease.ts EASE_OUT), duplicated
// here so Syncinit's motion matches without depending on Xuro's codebase.
export const EASE_OUT = [0.16, 1, 0.3, 1] as const;

// Xuro's panel spring (src/lib/ease.ts SPRING_PANEL) — the "snap into place"
// feel on dialogs/modals, duplicated here for the same reason as EASE_OUT.
export const SPRING_PANEL = {
  type: "spring",
  stiffness: 420,
  damping: 34,
  mass: 0.9,
} as const;
