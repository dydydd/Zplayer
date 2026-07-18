import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { UiIcon } from "./icons";
import { ServerAvatar } from "./ServerAvatar";
import { loadServerIconEntries, resolveServerIconUrl, serverIconCatalogUrls, type ServerIconEntry } from "./serverIcons";
import type { LoginResult, ServerForm } from "./types";

type ServerModalProps = {
  editingServerId: string;
  form: ServerForm;
  testedLogin: LoginResult | null;
  showPassword: boolean;
  canSaveWithoutLogin: boolean;
  iconCatalogUrls: string;
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
  canSaveWithoutLogin,
  iconCatalogUrls,
  onClose,
  onSubmit,
  onTestLogin,
  onAutoFetchServerName,
  onTogglePassword,
  onUpdateForm,
}: ServerModalProps) {
  const { t } = useTranslation();
  const [iconEntries, setIconEntries] = useState<ServerIconEntry[]>([]);
  const [iconSearch, setIconSearch] = useState("");
  const [iconLoading, setIconLoading] = useState(false);
  const [iconError, setIconError] = useState("");
  const catalogUrls = useMemo(() => serverIconCatalogUrls(iconCatalogUrls), [iconCatalogUrls]);
  const filteredIcons = useMemo(() => {
    const query = iconSearch.trim().toLowerCase();
    return query
      ? iconEntries.filter((icon) => icon.name.toLowerCase().includes(query))
      : iconEntries;
  }, [iconEntries, iconSearch]);
  const canSubmit = !!testedLogin || canSaveWithoutLogin;

  useEffect(() => {
    setIconEntries([]);
    setIconError("");
  }, [iconCatalogUrls]);

  async function loadIcons(autoMatch: boolean) {
    if (!catalogUrls.length) return;
    setIconLoading(true);
    setIconError("");
    try {
      const icons = await loadServerIconEntries(catalogUrls);
      setIconEntries(icons);
      if (autoMatch) {
        const iconUrl = resolveServerIconUrl(form.name, icons);
        const icon = icons.find((entry) => entry.url === iconUrl);
        if (icon) {
          onUpdateForm("iconUrl", icon.url);
          onUpdateForm("iconName", icon.name);
        } else {
          setIconError(t("modal.iconNoMatch"));
        }
      }
    } catch (err) {
      setIconError(String(err));
    } finally {
      setIconLoading(false);
    }
  }

  function chooseIcon(icon: ServerIconEntry) {
    onUpdateForm("iconUrl", icon.url);
    onUpdateForm("iconName", icon.name);
  }

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
          <UiIcon name="x" />
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
        <section className="server-icon-section">
          <div className="server-icon-header">
            <ServerAvatar server={{ name: form.name, iconUrl: form.iconUrl }} icons={iconEntries} className="server-modal-avatar" />
            <div>
              <strong>{t("modal.iconTitle")}</strong>
              <span>{form.iconName || t("modal.iconAuto")}</span>
              <small>{catalogUrls.length ? t("modal.iconSourceCount", { count: catalogUrls.length }) : t("modal.iconSourceMissing")}</small>
            </div>
          </div>
          <div className="icon-actions">
            <button type="button" onClick={() => void loadIcons(false)} disabled={!catalogUrls.length || iconLoading}>
              {iconLoading ? t("modal.iconLoading") : t("modal.loadIcons")}
            </button>
            <button type="button" onClick={() => void loadIcons(true)} disabled={!catalogUrls.length || iconLoading}>
              {t("modal.autoMatchIcon")}
            </button>
            <button
              type="button"
              onClick={() => {
                onUpdateForm("iconUrl", "");
                onUpdateForm("iconName", "");
              }}
              disabled={!form.iconUrl && !form.iconName}
            >
              {t("modal.clearIcon")}
            </button>
          </div>
          <label>
            {t("modal.iconUrl")}
            <input
              value={form.iconUrl}
              onChange={(event) => {
                onUpdateForm("iconUrl", event.target.value);
                if (form.iconName) onUpdateForm("iconName", "");
              }}
              placeholder={t("modal.iconUrlPlaceholder")}
            />
          </label>
          {iconEntries.length ? (
            <div className="icon-picker">
              <label>
                {t("modal.iconSearch")}
                <input
                  value={iconSearch}
                  onChange={(event) => setIconSearch(event.target.value)}
                  placeholder={t("modal.iconSearchPlaceholder")}
                />
              </label>
              <div className="icon-grid" aria-label={t("modal.iconTitle")}>
                {filteredIcons.map((icon) => (
                  <button
                    type="button"
                    key={`${icon.name}:${icon.url}`}
                    className={icon.url === form.iconUrl ? "selected" : ""}
                    onClick={() => chooseIcon(icon)}
                    title={icon.name}
                  >
                    <img src={icon.url} alt="" loading="lazy" decoding="async" />
                    <span>{icon.name}</span>
                  </button>
                ))}
              </div>
            </div>
          ) : null}
          {iconEntries.length ? <span className="icon-count">{t("modal.iconCount", { count: iconEntries.length })}</span> : null}
          {iconError ? <span className="icon-error">{iconError}</span> : null}
        </section>
        <div className="modal-actions">
          <span>{testedLogin ? t("modal.loginPassed") : canSaveWithoutLogin ? t("modal.iconOnlySaveHint") : t("modal.loginRequired")}</span>
          <button type="button" onClick={onTestLogin}>
            {t("modal.testLogin")}
          </button>
          <button type="submit" disabled={!canSubmit}>
            {t("common.save")}
          </button>
        </div>
      </form>
    </div>
  );
}
