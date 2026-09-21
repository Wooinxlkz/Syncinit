import { useEffect, useState } from "react";
import { Modal } from "./Modal";
import { OTPInput } from "./OTPInput";

interface Props {
  open: boolean;
  isEncrypted: boolean;
  onCancel: () => void;
  onSubmit: (oldPassword: string | undefined, newPassword: string | undefined) => Promise<void> | void;
}

type Mode = "pin" | "password";

export default function ChangePasswordDialog({ open, isEncrypted, onCancel, onSubmit }: Props) {
  const [oldMode, setOldMode] = useState<Mode>("pin");
  const [oldPin, setOldPin] = useState("");
  const [oldPassword, setOldPassword] = useState("");
  const [action, setAction] = useState<"set" | "remove">("set");
  const [newMode, setNewMode] = useState<Mode>("pin");
  const [newPin, setNewPin] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!open) return;
    setOldMode("pin");
    setOldPin("");
    setOldPassword("");
    setAction(isEncrypted ? "remove" : "set");
    setNewMode("pin");
    setNewPin("");
    setNewPassword("");
    setBusy(false);
    setError("");
  }, [open, isEncrypted]);

  const oldValue = oldMode === "pin" ? oldPin : oldPassword;
  const oldReady = !isEncrypted || oldValue.length > 0;
  const newValue = newMode === "pin" ? newPin : newPassword;
  const canSubmit = oldReady && (action === "remove" || newValue.length > 0) && !busy;

  async function submit() {
    if (!canSubmit) return;
    setBusy(true);
    setError("");
    try {
      await onSubmit(isEncrypted ? oldValue : undefined, action === "set" ? newValue : undefined);
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }

  return (
    <Modal open={open} onClose={onCancel} className="password-modal" ariaLabel="Set/change password">
      <div className="modal-header">
        <span>{isEncrypted ? "Change password" : "Set password"}</span>
        <button className="icon-btn" type="button" onClick={onCancel}>
          ✕
        </button>
      </div>
      <div className="modal-body">
        {isEncrypted && (
          <>
            <p className="password-copy">Enter the current PIN or password.</p>
            {oldMode === "pin" ? (
              <OTPInput length={6} value={oldPin} onChange={setOldPin} autoFocus />
            ) : (
              <label className="field">
                <span>Current password</span>
                <input
                  autoFocus
                  type="password"
                  value={oldPassword}
                  onChange={(e) => setOldPassword(e.target.value)}
                />
              </label>
            )}
            <button
              type="button"
              className="link-btn"
              onClick={() => setOldMode((m) => (m === "pin" ? "password" : "pin"))}
            >
              {oldMode === "pin" ? "It's a full password instead" : "It's a PIN instead"}
            </button>

            <div className="theme-switch" role="radiogroup" aria-label="Action" style={{ marginTop: 6 }}>
              <button
                type="button"
                role="radio"
                aria-checked={action === "remove"}
                className={`theme-switch-btn ${action === "remove" ? "active" : ""}`}
                onClick={() => setAction("remove")}
              >
                Remove protection
              </button>
              <button
                type="button"
                role="radio"
                aria-checked={action === "set"}
                className={`theme-switch-btn ${action === "set" ? "active" : ""}`}
                onClick={() => setAction("set")}
              >
                Set new one
              </button>
            </div>
          </>
        )}

        {action === "set" && (
          <>
            <p className="password-copy">
              {isEncrypted ? "Enter the new PIN or password." : "This archive isn't protected yet — set a PIN or password."}
            </p>
            {newMode === "pin" ? (
              <OTPInput length={6} value={newPin} onChange={setNewPin} autoFocus={!isEncrypted} />
            ) : (
              <label className="field">
                <span>New password</span>
                <input
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                />
              </label>
            )}
            <button
              type="button"
              className="link-btn"
              onClick={() => setNewMode((m) => (m === "pin" ? "password" : "pin"))}
            >
              {newMode === "pin" ? "Use a full password instead" : "Use a PIN instead"}
            </button>
          </>
        )}

        {error && <div className="field-error">{error}</div>}
      </div>
      <div className="modal-footer">
        <button className="btn-secondary" type="button" onClick={onCancel}>
          Cancel
        </button>
        <button className="btn-primary" type="button" onClick={submit} disabled={!canSubmit}>
          {action === "remove" ? "Remove password" : "Save"}
        </button>
      </div>
    </Modal>
  );
}
