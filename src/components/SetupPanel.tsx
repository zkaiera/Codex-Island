type SetupPanelProps = {
  windowsSnippet: string;
  wslSnippet: string;
  stateDir: string;
  isBrowserPreview: boolean;
  onPreviewDemo: () => void;
};

export function SetupPanel({
  windowsSnippet,
  wslSnippet,
  stateDir,
  isBrowserPreview,
  onPreviewDemo,
}: SetupPanelProps) {
  return (
    <section className="setup-panel" aria-label="Codex Island 设置">
      <div className="setup-panel__header">
        <h1 className="setup-panel__title">Codex Island</h1>
        <p className="setup-panel__hint">当前还没有检测到会话状态文件。</p>
      </div>

      {isBrowserPreview ? (
        <div className="setup-panel__notice">
          当前是在普通网页预览，不能读取本机状态目录，也不会收到 Tauri 后端事件。
        </div>
      ) : null}

      <button type="button" className="setup-panel__preview" onClick={onPreviewDemo}>
        预览示例状态岛
      </button>

      <div className="setup-panel__section">
        <span className="setup-panel__label">状态目录</span>
        <code className="setup-panel__path">{stateDir}</code>
      </div>

      <div className="setup-panel__section">
        <span className="setup-panel__label">Windows hooks 片段</span>
        <textarea className="setup-panel__code" readOnly value={windowsSnippet} />
      </div>

      <div className="setup-panel__section">
        <span className="setup-panel__label">WSL hooks 片段</span>
        <textarea className="setup-panel__code" readOnly value={wslSnippet} />
      </div>

      <p className="setup-panel__note">
        把对应片段合并到 Codex hook 配置后，还需要在 Codex 里显式信任这些 hooks。
      </p>
    </section>
  );
}
