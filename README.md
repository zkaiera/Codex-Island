# Codex Island

Codex Island 是一个基于 Tauri 的 Windows 桌面悬浮状态岛，用来观察多个 Codex 会话的当前状态。

当前第一版能力：

- 监听 Windows 本地状态目录中的会话 JSON。
- 用收起态呼吸灯展示会话状态。
- 鼠标悬浮后展开会话列表。
- 支持手动隐藏单个会话。
- 支持 WSL 与 Windows 原生 Codex 写入的状态文件。

## 本地开发

先安装依赖：

```bash
pnpm install
```

前端构建与测试：

```bash
pnpm test
pnpm build
```

Rust 测试：

```bash
cd src-tauri
cargo test
```

构建 Windows 安装包：

```bash
pnpm bundle:windows
```

如果在 WSL/Linux 中交叉构建 Windows 安装包：

```bash
pnpm bundle:windows:wsl
```

安装包输出在：

```text
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/
```

启动前端开发服务器：

```bash
pnpm dev -- --host 127.0.0.1 --port 1420
```

普通网页预览无法读取本机状态文件。要直接查看灵动岛示例效果，可以打开：

```text
http://127.0.0.1:1420/?demo=1
```

启动 Tauri 桌面应用：

```bash
pnpm tauri dev
```

## 状态目录

默认状态目录：

```text
%LOCALAPPDATA%\CodexIsland\sessions\
```

可以通过环境变量覆盖：

```text
CODEX_ISLAND_STATE_DIR
```

每个会话一个 JSON 文件，文件名使用 `session_id`。

## Windows 安装

安装包生成后，双击 `Codex Island_*_x64-setup.exe` 安装。当前安装包使用当前用户安装模式，不要求管理员权限。

启动后应用只显示悬浮状态岛，并直接监听状态目录。没有状态文件时只显示空闲呼吸灯，不再提供设置流程或 hook 自动配置。

hook 配置由用户手动维护。确保 Windows Codex 和 WSL Codex 的 hook 命令都指向安装目录中的：

```text
codex-island-hook.exe
```

WSL 中可以通过 `/mnt/c/.../codex-island-hook.exe` 调用同一个 Windows helper。Codex 的 hook 信任仍由 Codex 自身管理，需要你手动确认。

## 状态说明

```text
running   蓝色   正在运行
completed 绿色   当前轮完成
waiting   黄色   需要人工确认
error     红色   状态文件解析异常或字段缺失
stale     灰色   运行中但超过 10 分钟没有更新
```

## 运行逻辑

- 默认收起，只显示呼吸灯。
- 鼠标悬浮后展开详情列表。
- 会话按首次加入时间排序，状态变化不会改变位置。
- 完成、异常和过期会话会保留，直到用户手动隐藏。
- 隐藏只影响 UI，不删除状态文件；同一会话后续有新事件时会重新出现。
