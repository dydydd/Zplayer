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
  return (
    <div className="modal-backdrop">
      <form
        className="server-modal"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
        }}
      >
        <button type="button" className="close" onClick={onClose} title="关闭">
          x
        </button>
        <h2>{editingServerId ? "重新登录服务器" : "添加服务器"}</h2>
        <label>
          名称（选填）
          <input
            value={form.name}
            onChange={(event) => onUpdateForm("name", event.target.value)}
            placeholder="登录检测后自动获取"
          />
        </label>
        <label>
          服务器类型
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
          Emby/Jellyfin 服务器
          <input
            value={form.url}
            onChange={(event) => onUpdateForm("url", event.target.value)}
            onBlur={onAutoFetchServerName}
            placeholder="http://127.0.0.1:8096"
          />
        </label>
        <label>
          用户
          <input value={form.username} onChange={(event) => onUpdateForm("username", event.target.value)} />
        </label>
        <label>
          密码
          <div className="password-row">
            <input
              type={showPassword ? "text" : "password"}
              value={form.password}
              onChange={(event) => onUpdateForm("password", event.target.value)}
            />
            <button type="button" onClick={onTogglePassword}>
              {showPassword ? "隐藏" : "显示"}
            </button>
          </div>
        </label>
        <label className="check-row">
          <input
            type="checkbox"
            checked={form.useSystemProxy}
            onChange={(event) => onUpdateForm("useSystemProxy", event.target.checked)}
          />
          启用系统代理
        </label>
        <div className="modal-actions">
          <span>{testedLogin ? "登录检测已通过" : "请先通过登录检测"}</span>
          <button type="button" onClick={onTestLogin}>
            登录检测
          </button>
          <button type="submit" disabled={!testedLogin}>
            保存
          </button>
        </div>
      </form>
    </div>
  );
}
