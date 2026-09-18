import { useEffect, useState } from "react";
import type { Format } from "./api";
import { OTPInput } from "./OTPInput";
import { Modal } from "./Modal";

export interface ArchiveDialogResult {
  name: string;
  format: Format;
  level: number;
  password: string;
  deleteAfter: boolean;
  testAfter: boolean;
  comment: string;
  smartStore: boolean;
}

interface Props {
  open: boolean;
  defaultName: string;
  onCancel: () => void;
  onConfirm: (result: ArchiveDialogResult) => void;
}

type Tab = "general" | "advanced" | "options" | "comment";

const TABS: { id: Tab; label: string }[] = [
  { id: "general", label: "General" },
  { id: "advanced", label: "Advanced" },
  { id: "options", label: "Options" },
  { id: "comment", label: "Comment" },
];

export default function AddArchiveDialog({ open, defaultName, onCancel, onConfirm }: Props) {
  const [tab, setTab] = useState<Tab>("general");
  const [name, setName] = useState(defaultName);
  const [format, setFormat] = useState<Format>("zip");
  const [level, setLevel] = useState(6);
  const [pinEnabled, setPinEnabled] = useState(false);
  const [pin, setPin] = useState("");
  const [deleteAfter, setDeleteAfter] = useState(false);
  const [testAfter, setTestAfter] = useState(false);
  const [comment, setComment] = useState("");
  const [smartStore, setSmartStore] = useState(true);

  // Reset the form to a clean slate every time the dialog is (re)opened,
  // since it now stays mounted (for the Modal exit animation) instead of
  // being thrown away and recreated each time.
  useEffect(() => {
    if (!open) return;
    setTab("general");
    setName(defaultName);
    setFormat("zip");
    setLevel(6);
    setPinEnabled(false);
    setPin("");
    setDeleteAfter(false);
    setTestAfter(false);
    setComment("");
    setSmartStore(true);
  }, [open, defaultName]);

  const pinIncomplete = pinEnabled && pin.length > 0 && pin.length < 6;

  function submit() {
    if (pinIncomplete) return;
    onConfirm({
      name,
      format,
      level,
      password: pinEnabled ? pin : "",
      deleteAfter,
      testAfter,
      comment,
      smartStore,
    });
  }

  return (
    <Modal open={open} onClose={onCancel} ariaLabel="Archive name and parameters">
      <div className="modal-header">
        <span>Archive name and parameters</span>
        <button className="icon-btn" onClick={onCancel}>
          ✕
        </button>
      </div>

      <div className="modal-tabs">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`modal-tab ${tab === t.id ? "active" : ""}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div className="modal-body">
        {tab === "general" && (
          <>
            <label className="field">
              <span>Archive name</span>
              <input value={name} onChange={(e) => setName(e.target.value)} />
            </label>

            <div className="field-row">
              <fieldset className="fieldset">
                <legend>Archive format</legend>
                {(["init", "zip", "tar", "targz", "tarzst"] as Format[]).map((f) => (
                  <label key={f} className="radio">
                    <input
                      type="radio"
                      checked={format === f}
                      onChange={() => setFormat(f)}
                    />
                    {f === "init" ? "SYNCINIT (.INIT)" : f.toUpperCase()}
                  </label>
                ))}
              </fieldset>

              <label className="field">
                <span>Compression level ({level === 0 ? "Store" : level})</span>
                <input
                  type="range"
                  min={0}
                  max={9}
                  value={level}
                  onChange={(e) => setLevel(Number(e.target.value))}
                />
              </label>
            </div>
          </>
        )}

        {tab === "advanced" && (
          <>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={pinEnabled}
                onChange={(e) => setPinEnabled(e.target.checked)}
              />
              Protect with a PIN (AES-256)
            </label>
            {pinEnabled && (
              <OTPInput
                length={6}
                label="6-digit PIN"
                value={pin}
                onChange={setPin}
                status={pinIncomplete ? "error" : "idle"}
                errorMessage={pinIncomplete ? "Enter all 6 digits" : undefined}
                autoFocus
              />
            )}
            <p className="hint">
              The PIN is used directly as the AES-256 password — since an archive has to
              open on any machine, there's no OS keychain to fall back on like Xuro's
              note-lock PINs have, so the 6 digits carry all the protection themselves.
              Zip password protection uses real AES-256, not legacy ZipCrypto.
            </p>
          </>
        )}

        {tab === "options" && (
          <>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={testAfter}
                onChange={(e) => setTestAfter(e.target.checked)}
              />
              Test archived files after creation
            </label>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={deleteAfter}
                onChange={(e) => setDeleteAfter(e.target.checked)}
              />
              Delete files after archiving
            </label>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={smartStore}
                onChange={(e) => setSmartStore(e.target.checked)}
              />
              Smart store for already-compressed files
            </label>
            <p className="hint">
              Smart store avoids spending CPU recompressing JPEG, MP4, ZIP, PDF and other formats that
              are already compressed.
            </p>
          </>
        )}

        {tab === "comment" && (
          <label className="field">
            <span>Archive comment</span>
            <textarea
              rows={6}
              value={comment}
              onChange={(e) => setComment(e.target.value)}
              placeholder="Optional comment stored inside the archive"
            />
          </label>
        )}
      </div>

      <div className="modal-footer">
        <button className="btn-secondary" onClick={onCancel}>
          Cancel
        </button>
        <button className="btn-primary" onClick={submit} disabled={pinIncomplete || !name.trim()}>
          OK
        </button>
      </div>
    </Modal>
  );
}
