import { motion, AnimatePresence } from "motion/react";
import { Download, X } from "lucide-react";
import { invoke } from "./invokeSafe";
import { EASE_OUT } from "./ease";
import type { UpdateInfo } from "./updateCheck";

export function UpdateBanner({
  info,
  currentVersion,
  onDismiss,
}: {
  info: UpdateInfo | null;
  currentVersion: string;
  onDismiss: () => void;
}) {
  return (
    <AnimatePresence>
      {info && (
        <motion.div
          className="update-banner"
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -10 }}
          transition={{ duration: 0.18, ease: EASE_OUT }}
        >
          <Download size={15} strokeWidth={1.75} />
          <span>
            Syncinit v{info.version} is available — you're on v{currentVersion}.
          </span>
          <button
            className="update-banner-link"
            onClick={() => invoke("open_url", { url: info.url })}
          >
            View release
          </button>
          <button className="update-banner-close" onClick={onDismiss} aria-label="Dismiss">
            <X size={13} />
          </button>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
