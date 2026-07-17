import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { UiIcon } from "./icons";
import type { AppLanguage, AppSettings, LinuxWindowDiagnostics, PlayResult, SavedServer } from "./types";
import { defaultAppSettings, withAppSettingsDefaults } from "./types";

export function ServerView({
  servers,
  onAdd,
  onExport,
  onActivate,
  onEdit,
  onDelete,
  onBack,
}: {
  servers: SavedServer[];
  onAdd: () => void;
  onExport: () => Promise<void>;
  onActivate: (id: string) => Promise<void>;
  onEdit: (server: SavedServer) => void;
  onDelete: (id: string) => Promise<void>;
  onBack: () => void;
}) {
  const { t } = useTranslation();
  const sortedServers = [...servers].sort((left, right) => Number(right.active) - Number(left.active) || left.name.localeCompare(right.name));

  return (
    <div className="page narrow server-page">
      <button className="back" onClick={onBack} aria-label={t("common.back")}><UiIcon name="chevron-left" /></button>
      <div className="server-heading">
        <div>
          <span className="eyebrow">{t("server.eyebrow")}</span>
          <h1>{t("server.title")}</h1>
          <p>{t("server.subtitle")}</p>
          <p className="server-export-note">{t("server.exportNote")}</p>
        </div>
        <div className="server-heading-actions">
          <button onClick={() => void onExport()} disabled={!servers.length}>{t("server.export")}</button>
          <button onClick={onAdd}>{t("server.add")}</button>
        </div>
      </div>
      <div className="server-list">
        {sortedServers.map((server) => (
          <button
            key={server.id}
            className={`server-card ${server.active ? "active" : ""}`}
            onClick={() => void onActivate(server.id)}
          >
            <span className="server-logo"><UiIcon name="server" /></span>
            <div className="server-main">
              <div className="server-card-title">
                <h3>{server.name}</h3>
                {server.active && <span>{t("common.current")}</span>}
              </div>
              <p>{server.url}</p>
              <small>{server.username}</small>
            </div>
            <div className="server-counts">
              <span><strong>{server.movieCount ?? "-"}</strong>{t("server.movies")}</span>
              <span><strong>{server.seriesCount ?? "-"}</strong>{t("server.series")}</span>
            </div>
            <span
              className="server-edit"
              role="button"
              tabIndex={0}
              onClick={(event) => {
                event.stopPropagation();
                onEdit(server);
              }}
              onKeyDown={(event) => {
                if (event.key !== "Enter" && event.key !== " ") return;
                event.preventDefault();
                event.stopPropagation();
                onEdit(server);
              }}
            >
              {t("common.edit")}
            </span>
            <span
              className="server-delete"
              role="button"
              tabIndex={0}
              onClick={(event) => {
                event.stopPropagation();
                void onDelete(server.id);
              }}
              onKeyDown={(event) => {
                if (event.key !== "Enter" && event.key !== " ") return;
                event.preventDefault();
                event.stopPropagation();
                void onDelete(server.id);
              }}
            >
              {t("common.delete")}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
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
