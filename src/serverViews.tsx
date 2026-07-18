import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { UiIcon } from "./icons";
import { ServerAvatar } from "./ServerAvatar";
import { serverIconCatalogUrls, useServerIconEntries } from "./serverIcons";
import type { AppLanguage, AppSettings, LinuxWindowDiagnostics, PlayResult, SavedServer } from "./types";
import { defaultAppSettings, withAppSettingsDefaults } from "./types";

export function ServerView({
  servers,
  serverIconCatalogUrls: iconCatalogUrls,
  onAdd,
  onImport,
  onExport,
  onActivate,
  onEdit,
  onDelete,
  onBack,
}: {
  servers: SavedServer[];
  serverIconCatalogUrls: string;
  onAdd: () => void;
  onImport: () => Promise<void>;
  onExport: () => Promise<void>;
  onActivate: (id: string) => Promise<void>;
  onEdit: (server: SavedServer) => void;
  onDelete: (id: string) => Promise<void>;
  onBack: () => void;
}) {
  const { t } = useTranslation();
  const serverIcons = useServerIconEntries(serverIconCatalogUrls(iconCatalogUrls));
  const sortedServers = [...servers].sort((left, right) => Number(right.active) - Number(left.active) || left.name.localeCompare(right.name));

  return (
    <div className="page narrow server-page">
      <div className="server-shell">
        <div className="server-topline">
          <button className="server-back" onClick={onBack} aria-label={t("common.back")}><UiIcon name="chevron-left" /></button>
          <h1>{t("server.manageTitle")}</h1>
        </div>
        <div className="server-heading">
          <div>
            <span className="eyebrow">{t("server.eyebrow")}</span>
            <p>{t("server.subtitle")}</p>
            <p className="server-export-note">{t("server.exportNote")}</p>
          </div>
          <div className="server-heading-actions">
            <button onClick={onAdd}>{t("server.add")}</button>
            <button onClick={() => void onImport()}>{t("server.import")}</button>
            <button onClick={() => void onExport()} disabled={!servers.length}>{t("server.export")}</button>
          </div>
        </div>
        <div className="server-list">
          {sortedServers.map((server) => (
            <article
              key={server.id}
              className={`server-card ${server.active ? "active" : ""} ${serverReachable(server) ? "reachable" : ""}`}
              role="button"
              tabIndex={0}
              onClick={() => void onActivate(server.id)}
              onKeyDown={(event) => {
                if (event.key !== "Enter" && event.key !== " ") return;
                event.preventDefault();
                void onActivate(server.id);
              }}
            >
              <ServerAvatar server={server} icons={serverIcons} className="server-logo" />
              <span className="server-status-dot" aria-hidden="true" />
              <div className="server-main">
                <div className="server-card-title">
                  <h3>{server.name}</h3>
                  <button
                    type="button"
                    className="server-card-icon"
                    onClick={(event) => {
                      event.stopPropagation();
                      onEdit(server);
                    }}
                    title={t("common.edit")}
                    aria-label={t("common.edit")}
                  >
                    <UiIcon name="settings" />
                  </button>
                  <button
                    type="button"
                    className="server-card-icon danger"
                    onClick={(event) => {
                      event.stopPropagation();
                      void onDelete(server.id);
                    }}
                    title={t("common.delete")}
                    aria-label={t("common.delete")}
                  >
                    <UiIcon name="x" />
                  </button>
                </div>
                <div className="server-counts">
                  <span>{t("server.movies")} <strong>{server.movieCount ?? "-"}</strong></span>
                  <span>{t("server.series")} <strong>{server.seriesCount ?? "-"}</strong></span>
                  <span>{t("server.episodes")} <strong>{server.episodeCount ?? "-"}</strong></span>
                </div>
              </div>
              <span className="server-watch-badge">{server.active ? t("server.today") : t("server.unwatched")}</span>
            </article>
          ))}
        </div>
      </div>
    </div>
  );
}

function serverReachable(server: SavedServer) {
  return [server.movieCount, server.seriesCount, server.episodeCount].some((count) => typeof count === "number");
}

export function SettingsView({
  settings,
  lastPlayResult,
  linuxWindowDiagnostics,
  onBack,
  onSaveSettings,
}: {
  settings: AppSettings;
  lastPlayResult: PlayResult | null;
  linuxWindowDiagnostics: LinuxWindowDiagnostics | null;
  onBack: () => void;
  onSaveSettings: (settings: AppSettings) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(() => withAppSettingsDefaults(settings));

  useEffect(() => {
    setDraft(withAppSettingsDefaults(settings));
  }, [settings]);

  function update<K extends keyof typeof draft>(key: K, value: (typeof draft)[K]) {
    setDraft((current) => ({ ...current, [key]: value }));
  }

  function save() {
    void onSaveSettings(draft);
  }

  function restore() {
    setDraft(defaultAppSettings);
    void onSaveSettings(defaultAppSettings);
  }

  return (
    <div className="page narrow settings-page">
      <button className="back" onClick={onBack} aria-label={t("common.back")}><UiIcon name="chevron-left" /></button>
      <span className="eyebrow">{t("settings.eyebrow")}</span>
      <h1>{t("settings.title")}</h1>
      <section className="settings-grid">
        <SettingsPanel title={t("settings.playbackTitle")} note={t("settings.playbackNote")}>
          <Stepper label={t("settings.defaultVolume")} value={draft.defaultVolume} min={0} max={100} step={5} unit="%" onChange={(value) => update("defaultVolume", value)} />
          <Stepper label={t("settings.seekBack")} value={draft.seekBackSeconds} min={5} max={60} step={5} unit={t("common.seconds")} onChange={(value) => update("seekBackSeconds", value)} />
          <Stepper label={t("settings.seekForward")} value={draft.seekForwardSeconds} min={5} max={180} step={5} unit={t("common.seconds")} onChange={(value) => update("seekForwardSeconds", value)} />
          <Toggle label={t("settings.autoplayNext")} checked={draft.autoplayNextEpisode} onChange={(checked) => update("autoplayNextEpisode", checked)} />
          <label className="settings-field">
            {t("settings.mpvPath")}
            <input className="settings-path-input" value={draft.mpvPath} onChange={(event) => update("mpvPath", event.target.value)} placeholder={t("settings.mpvPlaceholder")} />
          </label>
        </SettingsPanel>

        <SettingsPanel title={t("settings.subtitleTitle")} note={t("settings.subtitleNote")}>
          <SegmentedControl
            label={t("settings.defaultSubtitle")}
            value={draft.subtitleMode}
            options={[
              { value: "auto", label: t("settings.subtitleAuto") },
              { value: "off", label: t("settings.subtitleOff") },
            ]}
            onChange={(value) => update("subtitleMode", value)}
          />
        </SettingsPanel>

        <SettingsPanel title={t("settings.libraryTitle")} note={t("settings.libraryNote")}>
          <SegmentedControl
            label={t("settings.posterDensity")}
            value={draft.posterDensity}
            options={[
              { value: "comfortable", label: t("settings.densityComfortable") },
              { value: "compact", label: t("settings.densityCompact") },
            ]}
            onChange={(value) => update("posterDensity", value)}
          />
        </SettingsPanel>

        <SettingsPanel title={t("settings.cacheTitle")} note={t("settings.cacheNote")}>
          <Toggle label={t("settings.memoryCache")} checked={draft.metadataCacheEnabled} onChange={(checked) => update("metadataCacheEnabled", checked)} />
        </SettingsPanel>

        <SettingsPanel title={t("settings.tmdbTitle")} note={t("settings.tmdbNote")}>
          <label className="settings-field">
            {t("settings.tmdbToken")}
            <input
              className="settings-path-input"
              type="password"
              value={draft.tmdbApiKey}
              onChange={(event) => update("tmdbApiKey", event.target.value)}
              placeholder={t("settings.tmdbTokenPlaceholder")}
              autoComplete="off"
            />
          </label>
        </SettingsPanel>

        <SettingsPanel title={t("settings.serverIconsTitle")} note={t("settings.serverIconsNote")}>
          <label className="settings-field">
            {t("settings.serverIconCatalogUrls")}
            <textarea
              className="settings-path-input settings-textarea"
              value={draft.serverIconCatalogUrls}
              onChange={(event) => update("serverIconCatalogUrls", event.target.value)}
              placeholder={t("settings.serverIconCatalogUrlsPlaceholder")}
            />
          </label>
        </SettingsPanel>

        <SettingsPanel title={t("settings.themeTitle")} note={t("settings.themeNote")}>
          <SegmentedControl
            label={t("settings.themeTitle")}
            value={draft.theme}
            options={[
              { value: "dark", label: t("settings.themeDark") },
              { value: "midnight", label: t("settings.themeMidnight") },
            ]}
            onChange={(value) => update("theme", value)}
          />
        </SettingsPanel>

        <SettingsPanel title={t("settings.languageTitle")} note={t("settings.languageNote")}>
          <SegmentedControl<AppLanguage>
            label={t("settings.language")}
            value={draft.language}
            options={[
              { value: "auto", label: t("settings.languageAuto") },
              { value: "zh-CN", label: t("settings.languageZh") },
              { value: "en-US", label: t("settings.languageEn") },
            ]}
            onChange={(value) => update("language", value)}
          />
        </SettingsPanel>

        <SettingsPanel title={t("settings.diagnosticsTitle")} note={t("settings.diagnosticsNote")}>
          <Toggle label={t("settings.diagnostics")} checked={draft.diagnosticsEnabled} onChange={(checked) => update("diagnosticsEnabled", checked)} />
          {draft.diagnosticsEnabled && (
            <div className="diagnostics-box">
              <strong>{lastPlayResult?.logPath ?? t("settings.noLog")}</strong>
              <pre>{lastPlayResult?.logTail || t("settings.logHint")}</pre>
              {linuxWindowDiagnostics && (
                <pre>{[
                  `XDG_SESSION_TYPE: ${linuxWindowDiagnostics.xdgSessionType ?? t("common.unset")}`,
                  `WAYLAND_DISPLAY: ${linuxWindowDiagnostics.waylandDisplaySet ? "set" : t("common.unset")}`,
	                  `GDK_BACKEND: ${linuxWindowDiagnostics.gdkBackend ?? t("common.unset")}`,
	                  `WINIT_UNIX_BACKEND: ${linuxWindowDiagnostics.winitUnixBackend ?? t("common.unset")}`,
	                  `WEBKIT_DISABLE_DMABUF_RENDERER: ${linuxWindowDiagnostics.webkitDisableDmabufRenderer ? "1" : t("common.unset")}`,
	                  `ZPLAYER_RENDER_GPU: ${linuxWindowDiagnostics.renderGpuPreference ?? t("common.unset")}`,
	                  `NVIDIA driver: ${linuxWindowDiagnostics.nvidiaDriverAvailable ? t("common.yes") : t("common.no")}`,
	                  `NVIDIA render offload: ${linuxWindowDiagnostics.nvidiaPrimeRenderOffload ? "1" : t("common.unset")}`,
	                  `GLX vendor: ${linuxWindowDiagnostics.glxVendorLibraryName ?? t("common.unset")}`,
	                  `Vulkan Optimus: ${linuxWindowDiagnostics.vulkanOptimusLayer ?? t("common.unset")}`,
	                  `Wayland only: ${linuxWindowDiagnostics.waylandRequired ? t("common.yes") : t("common.no")}`,
                  `Wayland backend: ${linuxWindowDiagnostics.gdkBackendWayland ? t("common.yes") : t("common.no")}`,
                  `Tao backend: ${linuxWindowDiagnostics.winitBackendWayland ? "wayland" : t("common.no")}`,
                  `Native video layer: ${linuxWindowDiagnostics.nativeVideoOverlay ? t("common.yes") : t("common.no")}`,
                  `Native render context: ${linuxWindowDiagnostics.nativeVideoRenderContext ? t("common.yes") : t("common.no")}`,
                  `Native render count: ${linuxWindowDiagnostics.nativeVideoRenderCount}`,
                  `Native render size: ${linuxWindowDiagnostics.nativeVideoRenderWidth}x${linuxWindowDiagnostics.nativeVideoRenderHeight}`,
                  `Native render framebuffer: ${linuxWindowDiagnostics.nativeVideoRenderFramebuffer}`,
                  `Native render status: ${linuxWindowDiagnostics.nativeVideoRenderStatus}`,
                  `Opaque window: ${linuxWindowDiagnostics.opaqueWindow ? t("common.yes") : t("common.no")}`,
                ].join("\n")}</pre>
              )}
            </div>
          )}
        </SettingsPanel>

        <div className="settings-actions">
          <button onClick={save}>{t("common.save")}</button>
          <button onClick={restore}>{t("settings.restoreDefaults")}</button>
        </div>
      </section>
    </div>
  );
}

function SettingsPanel({ title, note, children }: { title: string; note: string; children: ReactNode }) {
  return (
    <section className="settings-panel">
      <div>
        <h2>{title}</h2>
        <p>{note}</p>
      </div>
      {children}
    </section>
  );
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function Stepper({
  label,
  value,
  min,
  max,
  step,
  unit,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  onChange: (value: number) => void;
}) {
  const { t } = useTranslation();
  const decrease = () => onChange(clamp(value - step, min, max));
  const increase = () => onChange(clamp(value + step, min, max));

  return (
    <div className="setting-control">
      <span>{label}</span>
      <div className="settings-stepper" role="group" aria-label={label}>
        <button type="button" onClick={decrease} disabled={value <= min} aria-label={`${t("common.decrease")} ${label}`}>
          -
        </button>
        <strong>
          {value}
          <small>{unit}</small>
        </strong>
        <button type="button" onClick={increase} disabled={value >= max} aria-label={`${t("common.increase")} ${label}`}>
          +
        </button>
      </div>
    </div>
  );
}

function SegmentedControl<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="setting-control">
      <span>{label}</span>
      <div className="settings-segmented" role="group" aria-label={label}>
        {options.map((option) => (
          <button
            key={option.value}
            type="button"
            className={option.value === value ? "active" : ""}
            onClick={() => onChange(option.value)}
            aria-pressed={option.value === value}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  const { t } = useTranslation();
  return (
    <div className="settings-toggle-row">
      <span>{label}</span>
      <button type="button" className={`settings-switch ${checked ? "on" : ""}`} onClick={() => onChange(!checked)} aria-pressed={checked}>
        <span className="settings-switch-text">{checked ? t("common.on") : t("common.off")}</span>
        <span className="settings-switch-knob" />
      </button>
    </div>
  );
}
