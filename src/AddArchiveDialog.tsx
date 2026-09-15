import { useState } from "react";
import type { Format } from "./api";

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

export default function AddArchiveDialog({ defaultName, onCancel, onConfirm }: Props) {
  const [tab, setTab] = useState<Tab>("general");
  const [name, setName] = useState(defaultName);
  const [format, setFormat] = useState<Format>("zip");
  const [level, setLevel] = useState(6);
  const [password, setPassword] = useState("");
  const [confirmPw, setConfirmPw] = useState("");
  const [deleteAfter, setDeleteAfter] = useState(false);
  const [testAfter, setTestAfter] = useState(false);
  const [comment, setComment] = useState("");
  const [smartStore, setSmartStore] = useState(true);

  const pwMismatch = password.length > 0 && password !== confirmPw;

  function submit() {
    if (pwMismatch) return;
    onConfirm({ name, format, level, password, deleteAfter, testAfter, comment, smartStore });
  }

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
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
                  {(["arc", "zip", "tar", "targz", "tarzst"] as Format[]).map((f) => (
                    <label key={f} className="radio">
                      <input
                        type="radio"
                        checked={format === f}
                        onChange={() => setFormat(f)}
                      />
                      {f === "arc" ? "ZARC (.ARC)" : f.toUpperCase()}
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
              <label className="field">
                <span>Password (AES-256, optional)</span>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </label>
              <label className="field">
                <span>Confirm password</span>
                <input
                  type="password"
                  value={confirmPw}
                  onChange={(e) => setConfirmPw(e.target.value)}
                />
              </label>
              {pwMismatch && <div className="field-error">Passwords don't match</div>}
              <p className="hint">
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
          <button className="btn-primary" onClick={submit} disabled={pwMismatch || !name.trim()}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}
