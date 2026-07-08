import { useTranslation } from "react-i18next";
import type { LoginResult, ServerForm } from "./types";

type ServerModalProps = {
  editingServerId: string;
  form: ServerForm;
  testedLogin: LoginResult | null;
  showPassword: boolean;
  onClose: () => void;
  onSubmit: () => void;
  onTestLogin: () => void;
  onAutoFetchServerName: () => void;
  onTogglePassword: () => void;
  onUpdateForm: <K extends keyof ServerForm>(key: K, value: ServerForm[K]) => void;
};

export function ServerModal({
  editingServerId,
  form,
  testedLogin,
  showPassword,
  onClose,
  onSubmit,
  onTestLogin,
  onAutoFetchServerName,
  onTogglePassword,
  onUpdateForm,
}: ServerModalProps) {
  const { t } = useTranslation();

  return (
    <div className="modal-backdrop">
      <form
        className="server-modal"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
        }}
      >
        <button type="button" className="close" onClick={onClose} title={t("common.close")}>
          x
        </button>
        <h2>{editingServerId ? t("modal.editTitle") : t("modal.addTitle")}</h2>
        <label>
          {t("modal.name")}
          <input
            value={form.name}
            onChange={(event) => onUpdateForm("name", event.target.value)}
            placeholder={t("modal.namePlaceholder")}
          />
        </label>
        <label>
          {t("modal.serverType")}
          <div className="server-type-row">
            <button
              type="button"
              className={`type-btn ${form.serverType === "emby" ? "active" : ""}`}
              onClick={() => onUpdateForm("serverType", "emby")}
            >
              Emby
            </button>
            <button
              type="button"
              className={`type-btn ${form.serverType === "jellyfin" ? "active" : ""}`}
              onClick={() => onUpdateForm("serverType", "jellyfin")}
            >
              Jellyfin
            </button>
          </div>
        </label>
        <label>
          {t("modal.serverUrl")}
          <input
            value={form.url}
            onChange={(event) => onUpdateForm("url", event.target.value)}
            onBlur={onAutoFetchServerName}
            placeholder="http://127.0.0.1:8096"
          />
        </label>
        <label>
          {t("modal.username")}
          <input value={form.username} onChange={(event) => onUpdateForm("username", event.target.value)} />
        </label>
        <label>
          {t("modal.password")}
          <div className="password-row">
            <input
              type={showPassword ? "text" : "password"}
              value={form.password}
              onChange={(event) => onUpdateForm("password", event.target.value)}
            />
            <button type="button" onClick={onTogglePassword}>
              {showPassword ? t("modal.hidePassword") : t("modal.showPassword")}
            </button>
          </div>
        </label>
        <label className="check-row">
          <input
            type="checkbox"
            checked={form.useSystemProxy}
            onChange={(event) => onUpdateForm("useSystemProxy", event.target.checked)}
          />
          {t("modal.systemProxy")}
        </label>
        <div className="modal-actions">
          <span>{testedLogin ? t("modal.loginPassed") : t("modal.loginRequired")}</span>
          <button type="button" onClick={onTestLogin}>
            {t("modal.testLogin")}
          </button>
          <button type="submit" disabled={!testedLogin}>
            {t("common.save")}
          </button>
        </div>
      </form>
    </div>
  );
}
