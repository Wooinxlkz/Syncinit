import { FormEvent, useState } from "react";

interface Props {
  error?: string;
  purpose?: "open" | "extract";
  onCancel: () => void;
  onSubmit: (password: string) => void;
}

export default function PasswordDialog({ error, purpose = "open", onCancel, onSubmit }: Props) {
  const [password, setPassword] = useState("");

  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit(password);
  }

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <form className="modal password-modal" onClick={(event) => event.stopPropagation()} onSubmit={submit}>
        <div className="modal-header">
          <span>Encrypted archive</span>
          <button className="icon-btn" type="button" onClick={onCancel}>✕</button>
        </div>
        <div className="modal-body">
          <p className="password-copy">
            Enter the password to {purpose === "extract" ? "extract this archive" : "open this archive"}.
          </p>
          <label className="field">
            <span>Password</span>
            <input
              autoFocus
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder="Archive password"
            />
          </label>
          {error && <div className="field-error">{error}</div>}
        </div>
        <div className="modal-footer">
          <button className="btn-secondary" type="button" onClick={onCancel}>Cancel</button>
          <button className="btn-primary" type="submit">Unlock</button>
        </div>
      </form>
    </div>
  );
}