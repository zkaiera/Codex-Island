# Codex Island 第一版设计规格

## 背景

Codex Island 是一个 Windows 桌面悬浮状态岛，用来观察多个 Codex 会话的当前状态。目标用户是在 Windows 主系统中同时使用 WSL 和 Windows 原生 Codex 的开发者。

第一版定位为本机自用工具，优先跑通端到端最小闭环，不做完整 Agent 管理平台、历史日志、任务恢复、通知中心或复杂进程扫描。

## 目标

第一版必须跑通这条链路：

```text
Codex hook 事件
-> hook 脚本写入状态文件
-> Tauri Rust 后端监听和归一化
-> Tauri 前端显示呼吸灯
-> 鼠标悬浮展开详情
-> 用户手动隐藏某个灯
```

第一版同时支持 WSL 内 Codex 和 Windows 原生 Codex。状态文件统一写到 Windows 用户本地目录，例如：

```text
%LOCALAPPDATA%/CodexIsland/sessions/
```

任务标题默认只取项目目录名，不记录 prompt 正文。

## 非目标

第一版不实现以下能力：

- 完整进程扫描。
- 任务历史和会话恢复。
- 通知中心。
- 托盘菜单。
- 开机自启。
- 固定展开、拖拽定位、动画配置、主题设置。
- 自动覆盖或强行改写已有 Codex hook 配置。

## 架构

第一版采用 Tauri 后端主导型架构：

```text
Codex hooks
  -> hook 脚本
  -> 状态文件目录
  -> Tauri Rust 后端
  -> Tauri 前端悬浮窗
```

### hook 脚本

hook 脚本只负责采集和写入状态：

- 从标准输入读取 Codex hook 事件 JSON。
- 提取会话 ID、项目路径、事件名、运行环境、更新时间等字段。
- 为每个会话写入独立状态文件。
- 使用临时文件替换目标文件，避免读取到半截 JSON。
- 不扫描进程。
- 不做 UI 排序或展示判断。
- 不写入 prompt 正文。

### Tauri Rust 后端

Rust 后端负责状态中心逻辑：

- 监听状态目录。
- 读取并解析多个会话状态文件。
- 把 hook 事件转换为统一 UI 状态。
- 按首次加入时间维护稳定排序。
- 每隔一段时间检查 stale 状态。
- 维护用户已隐藏会话集合。
- 在会话出现新事件后重新显示此前隐藏的会话。
- 把当前可见会话列表推送给前端。

### Tauri 前端

前端只负责渲染和交互：

- 默认收起为一排呼吸灯。
- 鼠标悬浮后展开为实用状态列表。
- 显示项目目录名、状态、运行位置、最后更新和关闭按钮。
- 关闭按钮通知 Rust 后端隐藏对应会话。

## 状态文件

每个 Codex 会话一个状态文件，文件名使用会话 ID：

```text
%LOCALAPPDATA%/CodexIsland/sessions/<session_id>.json
```

状态文件保存最小展示数据：

```json
{
  "session_id": "abc123",
  "turn_id": "turn-456",
  "cwd": "/home/zkai/project/web3-agent-research",
  "title": "web3-agent-research",
  "source": "wsl",
  "distro": "Ubuntu-24.04",
  "last_event": "PreToolUse",
  "last_tool": "Bash",
  "ui_state": "running",
  "created_at": "2026-06-03T10:21:00+08:00",
  "updated_at": "2026-06-03T10:23:18+08:00"
}
```

`source` 的取值为：

```text
wsl
windows
```

## UI 状态

第一版 UI 状态收敛为五类：

```text
running：蓝色，正在运行
completed：绿色，当前轮完成
waiting：黄色，需要人工介入
error：红色，状态文件解析异常或字段缺失
stale：灰色，运行中但 10 分钟没有更新
```

事件映射规则：

```text
SessionStart / UserPromptSubmit / PreToolUse / PostToolUse -> running
PermissionRequest -> waiting
Stop -> completed
状态文件解析失败或字段缺失 -> error
running / waiting 超过 10 分钟无更新 -> stale
```

`PostToolUse` 不直接映射为红色。测试失败、构建失败或单次命令失败可能是 Codex 正在修复问题的一部分，第一版不把它视为整个会话异常。

## 悬浮窗交互

默认状态是顶部居中的收起岛，只显示呼吸灯：

- 每个可见会话一个灯。
- 呼吸灯按首次加入时间从左到右排列，优先使用状态文件中的 `created_at`。
- 状态变化只改变颜色和动画，不改变位置。
- 当宽度接近屏幕最大限制时，尾部显示 `+N`，避免窗口溢出。

鼠标悬浮到岛上后，窗口向下展开为小列表。展开态每一行显示：

```text
项目目录名
状态文字
运行位置：WSL / Windows
最后更新：例如 12 秒前、8 分钟前
关闭按钮
```

关闭按钮只在 UI 中隐藏当前会话，不删除状态文件。后端记录该会话的隐藏时间；如果这个会话后续出现晚于隐藏时间的新 hook 事件，后端会把它从隐藏集合中移除并重新展示。重新展示后保持原有加入顺序。

非运行任务不会自动消失。`completed`、`error` 和 `stale` 都会保留，直到用户手动关闭对应呼吸灯。

## 隐私

第一版默认不写入 prompt 正文，也不在 UI 中展示用户输入内容。

任务标题来自 `cwd` 的最后一级目录名。例如：

```text
/home/zkai/project/web3-agent-research -> web3-agent-research
```

状态文件可以保留完整 `cwd` 用于本地判断，但 UI 默认不展示完整路径。

## 错误处理

hook 脚本写状态文件时必须使用原子写入：

```text
写临时文件 -> 替换目标文件
```

hook 写入失败时，应把错误写入本地日志文件，不能静默失败。由于主状态文件可能没有成功写入，第一版不要求 UI 一定能展示这类 hook 自身写入失败。

Tauri 后端读取状态文件时，如果遇到 JSON 损坏或字段缺失，应把该会话标为 `error`，并在展开态显示简短错误原因。解析错误不能导致应用崩溃。

stale 规则：

- `running` 或 `waiting` 超过 10 分钟没有更新后变为 `stale`。
- `completed` 不会因为时间变为 `stale`。
- 第一版不通过进程扫描判断 crashed。

## hook 安装

第一版采用半自动安装：

- Tauri 应用生成 Windows 和 WSL 两套 hook 配置建议。
- 经用户确认后，可以写入目标配置文件。
- 写入前必须提示用户已有配置的合并风险。
- 用户仍需要在 Codex 中信任 hook。
- 应避免偷偷覆盖已有 hook 配置。

## 测试

hook 脚本需要覆盖：

- 能从 stdin 读取 Codex hook JSON。
- 能正确提取 session_id、cwd、source、event。
- 能为新会话写入 created_at，并在后续事件中保留原 created_at。
- 能原子写入 `sessions/<session_id>.json`。
- 不写入 prompt 正文。

Rust 后端需要覆盖：

- 能读取多个 session 状态文件。
- 能按 created_at 保持首次加入顺序。
- 能把事件映射为 running、completed、waiting、error、stale。
- 能在 10 分钟无更新后标记 stale。
- 能处理损坏 JSON，不让应用崩溃。
- 能隐藏单个会话，并在该会话有新事件后重新显示。

前端需要覆盖：

- 默认收起，只显示呼吸灯。
- hover 后展开列表。
- 展示项目目录名、状态、运行位置、最后更新、关闭按钮。
- 状态变化只改变颜色，不改变顺序。
- 灯太多时出现 `+N`，窗口不溢出屏幕。

## 手工验收

第一版完成后，需要通过以下场景：

1. 在 WSL 中启动一个 Codex 会话，岛上出现蓝灯。
2. Codex 完成后蓝灯变绿，位置不变。
3. 触发权限请求时变黄。
4. 模拟 10 分钟无更新后变灰。
5. 打开 Windows 原生 Codex，会出现另一个灯。
6. hover 展开后能看到两个会话来源分别为 WSL 和 Windows。
7. 点击关闭某个灯后，它从 UI 消失。
8. 给被关闭的会话写入新事件后，它重新出现。
9. 重启应用后，会话顺序仍按 created_at 保持稳定。
