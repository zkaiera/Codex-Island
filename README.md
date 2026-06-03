# Codex Island

Codex Island 是一个基于 Tauri 的 Windows 桌面悬浮状态岛，用来观察多个 Codex 会话的当前状态。

当前第一版能力：

- 监听 Windows 本地状态目录中的会话 JSON。
- 用收起态呼吸灯展示会话状态。
- 鼠标悬浮后展开会话列表。
- 支持手动隐藏单个会话。
- 支持 WSL 与 Windows 原生 Codex 的 hook 自动配置。

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

## Windows 安装与 hook 配置

安装包生成后，双击 `Codex Island_*_x64-setup.exe` 安装。当前安装包使用当前用户安装模式，不要求管理员权限。

首次打开应用时，如果还没有检测到会话状态文件，会显示设置面板：

1. 点击“自动配置 Windows 和 WSL hooks”。
2. 应用会合并写入 Windows Codex 的 `%USERPROFILE%\.codex\hooks.json`。
3. 应用会通过 `wsl.exe` 合并写入默认 WSL 发行版中的 `~/.codex/hooks.json`。
4. 写入前会保留已有 hook，并把被修改的配置备份为 `hooks.json.codex-island.bak`。
5. 新开或重启 Codex 会话。
6. 当 Codex 提示 hook 需要信任时，手动选择信任或允许。

应用不会绕过 Codex 的 hook 信任机制，也不会使用 `--dangerously-bypass-hook-trust`。如果 WSL 未安装或 `wsl.exe` 不可用，Windows hooks 仍会照常配置，WSL 配置结果会显示为不可用。

设置面板仍保留 Windows 和 WSL hook 片段，便于你手动核对最终配置。

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
