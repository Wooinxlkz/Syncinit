import { AnimatePresence, motion } from "motion/react";
import { X, Globe, Info, Package, Sun, Moon } from "lucide-react";
import { Modal } from "./Modal";
import { EASE_OUT } from "./ease";
import { SUPPORTED_LOCALES } from "./i18n";
import appIcon from "../src-tauri/icons/128x128.png";
import { version as APP_VERSION } from "../package.json";

export type SettingsPage = "general" | "about";
export type Theme = "dark" | "light";

interface Props {
  open: boolean;
  page: SettingsPage;
  onPageChange: (page: SettingsPage) => void;
  onClose: () => void;
  locale: string;
  onLocaleChange: (code: string) => void;
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
}

const NAV: { id: SettingsPage; label: string; icon: typeof Globe }[] = [
  { id: "general", label: "General", icon: Globe },
  { id: "about", label: "About", icon: Info },
];

const PAGE_META: Record<SettingsPage, { title: string; description: string }> = {
  general: { title: "General", description: "Language, theme and app behavior." },
  about: { title: "About", description: "Version and app info." },
};

export function SettingsModal({
  open,
  page,
  onPageChange,
  onClose,
  locale,
  onLocaleChange,
  theme,
  onThemeChange,
}: Props) {
  return (
    <Modal open={open} onClose={onClose} className="settings-modal" ariaLabel="Settings">
      <div className="settings-body">
        <aside className="settings-sidebar">
          <div className="settings-sidebar-header">
            <span className="icon-chip">
              <Package size={16} strokeWidth={1.75} />
            </span>
            <div>
              <h2>Syncinit</h2>
              <p>Settings</p>
            </div>
          </div>
          <nav className="settings-nav">
            {NAV.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.id}
                  className={`settings-nav-item ${page === item.id ? "active" : ""}`}
                  onClick={() => onPageChange(item.id)}
                >
                  <Icon size={15} strokeWidth={1.75} />
                  {item.label}
                </button>
              );
            })}
          </nav>
          <div className="settings-sidebar-footer">
            No telemetry, no network calls.
            <div className="settings-version">v{APP_VERSION}</div>
          </div>
        </aside>

        <div className="settings-content">
          <div className="settings-content-header">
            <div>
              <h3>{PAGE_META[page].title}</h3>
              <p>{PAGE_META[page].description}</p>
            </div>
            <button className="icon-btn" onClick={onClose}>
              <X size={14} />
            </button>
          </div>

          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={page}
              className="settings-page"
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.15, ease: EASE_OUT }}
            >
              {page === "general" && (
                <>
                  <section>
                    <h4 className="settings-group-title">Appearance</h4>
                    <p className="settings-group-desc">Switch between dark and light.</p>
                    <div className="theme-switch" role="radiogroup" aria-label="Theme">
                      <button
                        type="button"
                        role="radio"
                        aria-checked={theme === "dark"}
                        className={`theme-switch-btn ${theme === "dark" ? "active" : ""}`}
                        onClick={() => onThemeChange("dark")}
                      >
                        <Moon size={14} strokeWidth={1.75} />
                        Dark
                      </button>
                      <button
                        type="button"
                        role="radio"
                        aria-checked={theme === "light"}
                        className={`theme-switch-btn ${theme === "light" ? "active" : ""}`}
                        onClick={() => onThemeChange("light")}
                      >
                        <Sun size={14} strokeWidth={1.75} />
                        Light
                      </button>
                    </div>
                  </section>

                  <section>
                    <h4 className="settings-group-title">Language</h4>
                    <p className="settings-group-desc">
                      Only English ships translated right now — the rest are on the way.
                    </p>
                    <select
                      className="settings-select"
                      value={locale}
                      onChange={(e) => onLocaleChange(e.target.value)}
                    >
                      {SUPPORTED_LOCALES.map((l) => (
                        <option key={l.code} value={l.code} disabled={!l.ready}>
                          {l.label}
                          {l.ready ? "" : " (soon)"}
                        </option>
                      ))}
                    </select>
                  </section>
                </>
              )}

              {page === "about" && (
                <section>
                  <div className="settings-card">
                    <div className="about-card">
                      <img src={appIcon} alt="" />
                      <div>
                        <div className="about-card-name">Syncinit</div>
                        <div className="about-card-version">v{APP_VERSION}</div>
                      </div>
                    </div>
                    <ul className="about-features">
                      <li>A fast, modern archive manager for Windows — a practical WinRAR alternative.</li>
                      <li>No telemetry, no network calls. See PRIVACY.md / TERMS.md / LICENSE in the install folder for the full policies.</li>
                    </ul>
                  </div>
                </section>
              )}
            </motion.div>
          </AnimatePresence>
        </div>
      </div>
    </Modal>
  );
}
