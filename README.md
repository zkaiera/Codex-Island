# Codex Island

Codex Island 是一个基于 Tauri 的 Windows 桌面悬浮状态岛，用来观察多个 Codex 会话的当前状态。

当前第一版能力：

- 监听 Windows 本地状态目录中的会话 JSON。
- 用收起态呼吸灯展示会话状态。
- 鼠标悬浮后展开会话列表。
- 支持手动隐藏单个会话。
- 支持 WSL 与 Windows 原生 Codex 的 hook 配置片段生成。

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

## hook 安装

没有检测到会话时，应用会显示设置面板，提供两段可复制的 hook 片段：

- Windows hook 片段
- WSL hook 片段

使用方式：

1. 把对应片段合并进你的 Codex hook 配置。
2. 确认 hook 指向 `codex-island-hook` 可执行文件。
3. 在 Codex 中显式信任这些 hooks。

当前版本不会自动覆盖已有 hook 配置。

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
