import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import type { AppSettings, LinuxWindowDiagnostics, PlayResult, SavedServer } from "./types";
import { defaultAppSettings, withAppSettingsDefaults } from "./types";

export function ServerView({
  servers,
  onAdd,
  onActivate,
  onEdit,
  onDelete,
  onBack,
}: {
  servers: SavedServer[];
  onAdd: () => void;
  onActivate: (id: string) => Promise<void>;
  onEdit: (server: SavedServer) => void;
  onDelete: (id: string) => Promise<void>;
  onBack: () => void;
}) {
  const sortedServers = [...servers].sort((left, right) => Number(right.active) - Number(left.active) || left.name.localeCompare(right.name));

  return (
    <div className="page narrow server-page">
      <button className="back" onClick={onBack} aria-label="返回" />
      <div className="server-heading">
        <div>
          <span className="eyebrow">连接中心</span>
          <h1>媒体服务器</h1>
          <p>管理 Emby / Jellyfin 连接和媒体概览。</p>
        </div>
        <div className="server-heading-actions">
          <button onClick={onAdd}>添加服务器</button>
        </div>
      </div>
      <div className="server-list">
        {sortedServers.map((server) => (
          <button
            key={server.id}
            className={`server-card ${server.active ? "active" : ""}`}
            onClick={() => void onActivate(server.id)}
          >
            <span className="server-logo play-icon" />
            <div className="server-main">
              <div className="server-card-title">
                <h3>{server.name}</h3>
                {server.active && <span>当前</span>}
              </div>
              <p>{server.url}</p>
              <small>{server.username}</small>
            </div>
            <div className="server-counts">
              <span><strong>{server.movieCount ?? "-"}</strong>电影</span>
              <span><strong>{server.seriesCount ?? "-"}</strong>剧集</span>
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
              编辑
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
              删除
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
      <button className="back" onClick={onBack} aria-label="返回" />
      <span className="eyebrow">专业设置</span>
      <h1>设置</h1>
      <section className="settings-grid">
        <SettingsPanel title="播放体验" note="这些选项会直接写入 mpv 启动参数和播放器控制。">
          <Stepper label="默认音量" value={draft.defaultVolume} min={0} max={100} step={5} unit="%" onChange={(value) => update("defaultVolume", value)} />
          <Stepper label="快退秒数" value={draft.seekBackSeconds} min={5} max={60} step={5} unit="秒" onChange={(value) => update("seekBackSeconds", value)} />
          <Stepper label="快进秒数" value={draft.seekForwardSeconds} min={5} max={180} step={5} unit="秒" onChange={(value) => update("seekForwardSeconds", value)} />
          <Toggle label="自动下一集" checked={draft.autoplayNextEpisode} onChange={(checked) => update("autoplayNextEpisode", checked)} />
          <label className="settings-field">
            mpv 路径
            <input className="settings-path-input" value={draft.mpvPath} onChange={(event) => update("mpvPath", event.target.value)} placeholder="默认：mpv/mpv.exe" />
          </label>
        </SettingsPanel>

        <SettingsPanel title="字幕偏好" note="播放时会把默认字幕策略传给服务端和 mpv。">
          <SegmentedControl
            label="默认字幕"
            value={draft.subtitleMode}
            options={[
              { value: "auto", label: "自动选择" },
              { value: "off", label: "默认关闭" },
            ]}
            onChange={(value) => update("subtitleMode", value)}
          />
        </SettingsPanel>

        <SettingsPanel title="媒体库显示" note="影响媒体库和搜索页的海报网格密度。">
          <SegmentedControl
            label="海报密度"
            value={draft.posterDensity}
            options={[
              { value: "comfortable", label: "舒适" },
              { value: "compact", label: "紧凑" },
            ]}
            onChange={(value) => update("posterDensity", value)}
          />
        </SettingsPanel>

        <SettingsPanel title="网络 / 缓存" note="关闭后首页、详情、媒体库每次都会重新请求服务端。">
          <Toggle label="内存缓存" checked={draft.metadataCacheEnabled} onChange={(checked) => update("metadataCacheEnabled", checked)} />
        </SettingsPanel>

        <SettingsPanel title="外观主题" note="立即切换应用外观。">
          <SegmentedControl
            label="主题"
            value={draft.theme}
            options={[
              { value: "dark", label: "深色影院" },
              { value: "midnight", label: "午夜蓝" },
            ]}
            onChange={(value) => update("theme", value)}
          />
        </SettingsPanel>

        <SettingsPanel title="日志 / 诊断" note="开启后保留最近一次播放的 mpv 日志摘要，方便排查播放失败。">
          <Toggle label="播放诊断" checked={draft.diagnosticsEnabled} onChange={(checked) => update("diagnosticsEnabled", checked)} />
          {draft.diagnosticsEnabled && (
            <div className="diagnostics-box">
              <strong>{lastPlayResult?.logPath ?? "还没有播放日志"}</strong>
              <pre>{lastPlayResult?.logTail || "启动一次播放后，这里会显示 mpv 日志尾部。"}</pre>
              {linuxWindowDiagnostics && (
                <pre>{[
                  `XDG_SESSION_TYPE: ${linuxWindowDiagnostics.xdgSessionType ?? "-"}`,
                  `WAYLAND_DISPLAY: ${linuxWindowDiagnostics.waylandDisplaySet ? "set" : "-"}`,
                  `GDK_BACKEND: ${linuxWindowDiagnostics.gdkBackend ?? "-"}`,
                  `Opaque window: ${linuxWindowDiagnostics.opaqueWindow ? "yes" : "no"}`,
                ].join("\n")}</pre>
              )}
            </div>
          )}
        </SettingsPanel>

        <div className="settings-actions">
          <button onClick={save}>保存</button>
          <button onClick={restore}>恢复默认</button>
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
  const decrease = () => onChange(clamp(value - step, min, max));
  const increase = () => onChange(clamp(value + step, min, max));

  return (
    <div className="setting-control">
      <span>{label}</span>
      <div className="settings-stepper" role="group" aria-label={label}>
        <button type="button" onClick={decrease} disabled={value <= min} aria-label={`${label}减少`}>
          -
        </button>
        <strong>
          {value}
          <small>{unit}</small>
        </strong>
        <button type="button" onClick={increase} disabled={value >= max} aria-label={`${label}增加`}>
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
  return (
    <div className="settings-toggle-row">
      <span>{label}</span>
      <button type="button" className={`settings-switch ${checked ? "on" : ""}`} onClick={() => onChange(!checked)} aria-pressed={checked}>
        <span className="settings-switch-text">{checked ? "开启" : "关闭"}</span>
        <span className="settings-switch-knob" />
      </button>
    </div>
  );
}
