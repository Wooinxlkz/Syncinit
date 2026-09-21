import { useEffect, useState } from "react";
import { OTPInput } from "./OTPInput";
import { Modal } from "./Modal";

interface Props {
  open: boolean;
  error?: string;
  purpose?: "open" | "extract";
  onCancel: () => void;
  onSubmit: (password: string, remember: boolean) => void;
}

export default function PasswordDialog({ open, error, purpose = "open", onCancel, onSubmit }: Props) {
  const [mode, setMode] = useState<"pin" | "password">("pin");
  const [pin, setPin] = useState("");
  const [password, setPassword] = useState("");
  const [remember, setRemember] = useState(false);

  useEffect(() => {
    if (!open) return;
    setMode("pin");
    setPin("");
    setPassword("");
    setRemember(false);
  }, [open]);

  return (
    <Modal open={open} onClose={onCancel} className="password-modal" ariaLabel="Encrypted archive">
      <div className="modal-header">
        <span>Encrypted archive</span>
        <button className="icon-btn" type="button" onClick={onCancel}>
          ✕
        </button>
      </div>
      <div className="modal-body">
        <p className="password-copy">
          Enter the {mode === "pin" ? "PIN" : "password"} to{" "}
          {purpose === "extract" ? "extract this archive" : "open this archive"}.
        </p>

        {mode === "pin" ? (
          <OTPInput
            length={6}
            value={pin}
            onChange={setPin}
            onComplete={(value) => onSubmit(value, remember)}
            status={error ? "error" : "idle"}
            errorMessage={error}
            autoFocus
          />
        ) : (
          <label className="field">
            <span>Password</span>
            <input
              autoFocus
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder="Archive password"
            />
            {error && <div className="field-error">{error}</div>}
          </label>
        )}

        <button
          type="button"
          className="link-btn"
          onClick={() => setMode((m) => (m === "pin" ? "password" : "pin"))}
        >
          {mode === "pin"
            ? "This archive uses a full password instead"
            : "This archive uses a PIN instead"}
        </button>

        <label className="checkbox">
          <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
          Remember this password (stored in Windows Credential Manager, not on disk)
        </label>
      </div>
      <div className="modal-footer">
        <button className="btn-secondary" type="button" onClick={onCancel}>
          Cancel
        </button>
        <button
          className="btn-primary"
          type="button"
          onClick={() => onSubmit(mode === "pin" ? pin : password, remember)}
          disabled={mode === "pin" ? pin.length < 6 : password.length === 0}
        >
          Unlock
        </button>
      </div>
    </Modal>
  );
}
