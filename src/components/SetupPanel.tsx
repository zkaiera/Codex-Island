import type { HookInstallReport } from "../App";

type SetupPanelProps = {
  windowsSnippet: string;
  wslSnippet: string;
  stateDir: string;
  isBrowserPreview: boolean;
  isInstallingHooks: boolean;
  hookInstallReport: HookInstallReport | null;
  hookInstallError: string | null;
  onInstallHooks: () => void;
  onPreviewDemo: () => void;
};

export function SetupPanel({
  windowsSnippet,
  wslSnippet,
  stateDir,
  isBrowserPreview,
  isInstallingHooks,
  hookInstallReport,
  hookInstallError,
  onInstallHooks,
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
        <span className="setup-panel__label">安装设置</span>
        <button
          type="button"
          className="setup-panel__install"
          disabled={isBrowserPreview || isInstallingHooks}
          onClick={onInstallHooks}
        >
          {isInstallingHooks ? "正在配置 hooks..." : "自动配置 Windows 和 WSL hooks"}
        </button>
        <p className="setup-panel__note">
          自动配置只会合并 hooks 配置并保留备份；Codex 的 hook 信任仍需要你手动确认。
        </p>
      </div>

      {hookInstallError ? (
        <div className="setup-panel__result setup-panel__result--failed">
          自动配置失败：{hookInstallError}
        </div>
      ) : null}

      {hookInstallReport ? <HookInstallResult report={hookInstallReport} /> : null}

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

function HookInstallResult({ report }: { report: HookInstallReport }) {
  return (
    <div className="setup-panel__result" aria-label="hook 配置结果">
      <TargetResult target={report.windows} />
      <TargetResult target={report.wsl} />
      <div className="setup-panel__trust">
        <span className="setup-panel__label">下一步</span>
        <ol>
          {report.trust_steps.map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ol>
      </div>
    </div>
  );
}

function TargetResult({ target }: { target: HookInstallReport["windows"] }) {
  return (
    <div className={`setup-panel__target setup-panel__target--${target.status}`}>
      <div className="setup-panel__target-heading">
        <span>{target.label}</span>
        <span>{statusText[target.status]}</span>
      </div>
      <p>{target.message}</p>
      {target.path ? <code>{target.path}</code> : null}
      {target.backup_path ? <code>备份：{target.backup_path}</code> : null}
    </div>
  );
}

const statusText = {
  installed: "已写入",
  already_installed: "已存在",
  unavailable: "不可用",
  failed: "失败",
};
