import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import type { AppSettings, PlayResult, SavedServer } from "./types";
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
  onBack,
  onSaveSettings,
}: {
  settings: AppSettings;
  lastPlayResult: PlayResult | null;
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
          <label>
            默认音量
            <input type="number" min="0" max="100" value={draft.defaultVolume} onChange={(event) => update("defaultVolume", Number(event.target.value))} />
          </label>
          <label>
            快退秒数
            <input type="number" min="5" max="60" value={draft.seekBackSeconds} onChange={(event) => update("seekBackSeconds", Number(event.target.value))} />
          </label>
          <label>
            快进秒数
            <input type="number" min="5" max="180" value={draft.seekForwardSeconds} onChange={(event) => update("seekForwardSeconds", Number(event.target.value))} />
          </label>
          <label>
            mpv 路径
            <input value={draft.mpvPath} onChange={(event) => update("mpvPath", event.target.value)} placeholder="默认：mpv/mpv.exe" />
          </label>
        </SettingsPanel>

        <SettingsPanel title="字幕偏好" note="播放时会把默认字幕策略传给服务端和 mpv。">
          <label>
            默认字幕
            <select value={draft.subtitleMode} onChange={(event) => update("subtitleMode", event.target.value as typeof draft.subtitleMode)}>
              <option value="auto">自动选择</option>
              <option value="off">默认关闭</option>
            </select>
          </label>
        </SettingsPanel>

        <SettingsPanel title="媒体库显示" note="影响媒体库和搜索页的海报网格密度。">
          <label>
            海报密度
            <select value={draft.posterDensity} onChange={(event) => update("posterDensity", event.target.value as typeof draft.posterDensity)}>
              <option value="comfortable">舒适</option>
              <option value="compact">紧凑</option>
            </select>
          </label>
        </SettingsPanel>

        <SettingsPanel title="网络 / 缓存" note="关闭后首页、详情、媒体库每次都会重新请求服务端。">
          <Toggle label="内存缓存" checked={draft.metadataCacheEnabled} onChange={(checked) => update("metadataCacheEnabled", checked)} />
        </SettingsPanel>

        <SettingsPanel title="外观主题" note="立即切换应用外观。">
          <label>
            主题
            <select value={draft.theme} onChange={(event) => update("theme", event.target.value as typeof draft.theme)}>
              <option value="dark">深色影院</option>
              <option value="midnight">午夜蓝</option>
            </select>
          </label>
        </SettingsPanel>

        <SettingsPanel title="日志 / 诊断" note="开启后保留最近一次播放的 mpv 日志摘要，方便排查播放失败。">
          <Toggle label="播放诊断" checked={draft.diagnosticsEnabled} onChange={(checked) => update("diagnosticsEnabled", checked)} />
          {draft.diagnosticsEnabled && (
            <div className="diagnostics-box">
              <strong>{lastPlayResult?.logPath ?? "还没有播放日志"}</strong>
              <pre>{lastPlayResult?.logTail || "启动一次播放后，这里会显示 mpv 日志尾部。"}</pre>
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

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  return (
    <label className="settings-toggle">
      {label}
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <span>{checked ? "开启" : "关闭"}</span>
    </label>
  );
}
