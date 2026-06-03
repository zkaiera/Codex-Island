export type SessionState = "running" | "completed" | "waiting" | "error" | "stale";
export type SessionSource = "wsl" | "windows";

export type SessionView = {
  sessionId: string;
  title: string;
  status: SessionState;
  source: SessionSource;
  createdAt: string;
  updatedAt: string;
};

export const STATUS_LABELS: Record<SessionState, string> = {
  running: "运行中",
  completed: "已完成",
  waiting: "等待确认",
  error: "异常",
  stale: "已过期",
};

export const SOURCE_LABELS: Record<SessionSource, string> = {
  wsl: "WSL",
  windows: "Windows",
};

export function formatRelativeTime(isoTime: string, now = new Date()): string {
  const deltaMs = now.getTime() - new Date(isoTime).getTime();
  const seconds = Math.max(0, Math.round(deltaMs / 1000));

  if (seconds < 60) {
    return `${seconds} 秒前`;
  }

  const minutes = Math.round(seconds / 60);
  if (minutes < 60) {
    return `${minutes} 分钟前`;
  }

  const hours = Math.round(minutes / 60);
  return `${hours} 小时前`;
}
