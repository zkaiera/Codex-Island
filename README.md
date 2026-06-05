# Codex Island hooks 自动配置手册

这份 README 是给 Codex Agent 执行的，不是给最终用户阅读的产品介绍。

当用户把这个仓库地址或 README 内容交给你时，你的任务是：在当前 Windows 电脑上找到已安装的 Codex Island，自动为 Windows 原生 Codex 和 WSL 内的 Codex 配好 hooks，让后续 Codex 会话能把状态写入 Codex Island。不要要求用户安装 Node、Rust、pnpm 或从源码构建；正常路径是用户先安装已经打包好的 Windows 安装包。

## 目标状态

- Windows 已安装 `Codex Island_*_x64-setup.exe` 生成的应用。
- 安装目录中存在 `codex-island-hook.exe`。
- Windows 原生 Codex 的用户级 hooks 指向这个 helper。
- 每个需要接入的 WSL 发行版内，Codex 的用户级 hooks 也指向同一个 Windows helper 的 `/mnt/c/...` 路径。
- hooks 配置保留用户原有内容，只添加或更新 Codex Island 自己的 command hook。
- hooks 写好后，用户只需要在 Codex 的 `/hooks` 页面信任这些新增 hook。不要伪造或手动写入信任 hash。

## 安全边界

- 不要覆盖用户已有 hooks；必须解析、合并、去重。
- 写配置前必须创建同目录备份，文件名建议为 `<原文件名>.codex-island.<YYYYMMDD-HHMMSS>.bak`。
- 不要删除用户配置、不要清空 `config.toml`、不要修改与 Codex Island 无关的 hook。
- 不要使用 `--dangerously-bypass-hook-trust` 做持久安装，除非用户明确要求一次性绕过信任。
- 如果发现企业托管配置、只读配置或权限不足，停止自动写入并向用户说明需要管理员或设备策略处理。

## 需要配置的事件

给下面 6 个事件都添加同一个 command hook：

```text
SessionStart
UserPromptSubmit
PermissionRequest
PreToolUse
PostToolUse
Stop
```

hook handler 统一使用：

```json
{
  "type": "command",
  "command": "<按当前环境生成的 codex-island-hook.exe 绝对路径命令>",
  "timeout": 5,
  "statusMessage": "Updating Codex Island"
}
```

不要设置 `matcher`。Codex Island 需要接收这些事件的全部输入；省略 `matcher` 表示匹配该事件的所有触发。

## 1. 确认 Codex Island 已安装

优先查找当前用户安装目录：

```powershell
$hook = Join-Path $env:LOCALAPPDATA "Codex Island\codex-island-hook.exe"
Test-Path $hook
```

如果不存在：

- 如果用户提供了安装包路径，先安装该安装包，再重新检查。
- 如果用户只提供仓库地址或 README 内容，告诉用户需要先安装打包好的 Windows exe 安装包。
- 不要默认从源码构建安装包。源码构建只用于开发者明确要求的场景。

Windows command hook 使用完整 Windows 路径，并给带空格的路径加外层引号：

```text
"C:\Users\<User>\AppData\Local\Codex Island\codex-island-hook.exe"
```

WSL command hook 使用同一个文件的 WSL 挂载路径，并给带空格的路径加外层引号：

```text
"/mnt/c/Users/<User>/AppData/Local/Codex Island/codex-island-hook.exe"
```

在 WSL 内生成该路径时，优先从 Windows 环境读取真实路径：

```bash
win_local_app_data="$(cmd.exe /c 'echo %LOCALAPPDATA%' | tr -d '\r')"
hook_path="$(wslpath -u "$win_local_app_data")/Codex Island/codex-island-hook.exe"
test -f "$hook_path"
```

## 2. 选择 Codex hooks 配置位置

Codex 会从用户级和项目级配置中加载 hooks。本项目要做全局状态展示，优先写用户级配置，不写项目级配置。

Windows 原生 Codex：

```text
%USERPROFILE%\.codex\hooks.json
%USERPROFILE%\.codex\config.toml
```

WSL Codex：

```text
~/.codex/hooks.json
~/.codex/config.toml
```

选择规则：

- 如果同一层已有 `hooks.json`，修改 `hooks.json`。
- 如果没有 `hooks.json`，但 `config.toml` 里已经有 inline `[hooks]` 或 `[[hooks.<Event>]]`，修改 `config.toml`，不要额外创建 `hooks.json`，避免同一层混用两种表示。
- 如果两者都没有 hooks，创建 `hooks.json`。
- 如果同一层已经同时存在 `hooks.json` 和 inline hooks，优先修改已有 `hooks.json`，并在最终报告里提醒用户 Codex 可能会提示混用警告。

## 3. 合并 `hooks.json`

目标 JSON 结构如下。实际写入时必须保留文件中的其他事件、其他 matcher group 和其他 hook handler。

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"C:\\Users\\<User>\\AppData\\Local\\Codex Island\\codex-island-hook.exe\"",
            "timeout": 5,
            "statusMessage": "Updating Codex Island"
          }
        ]
      }
    ]
  }
}
```

合并算法：

1. 读取现有 JSON；不存在时使用 `{ "hooks": {} }`。
2. 校验顶层是对象，`hooks` 是对象；格式不合法时停止并报告，不要盲写。
3. 对每个目标事件，确保 `hooks[event]` 是数组。
4. 在该事件数组中查找已有 Codex Island handler。判断方式：`command` 里包含 `codex-island-hook.exe`。
5. 如果找到旧 handler，原地更新它的 `command`、`timeout`、`statusMessage` 和 `type`。
6. 如果没有找到，追加一个无 `matcher` 的 matcher group，里面只包含 Codex Island handler。
7. 不改变其他 handler 的顺序和内容。
8. 写入前备份原文件。
9. 写入后重新解析 JSON，确认格式有效。

Windows hooks 中的 `command` 应使用 Windows 路径；WSL hooks 中的 `command` 应使用 `/mnt/c/...` 路径。

## 4. 合并 inline TOML hooks

只有在当前配置层已经使用 inline hooks 且没有 `hooks.json` 时才走这条路径。

需要为每个事件追加或更新等价配置：

```toml
[[hooks.SessionStart]]

[[hooks.SessionStart.hooks]]
type = "command"
command = '"C:\Users\<User>\AppData\Local\Codex Island\codex-island-hook.exe"'
timeout = 5
statusMessage = "Updating Codex Island"
```

处理规则：

- 如果已存在 `command` 包含 `codex-island-hook.exe` 的 handler，更新它。
- 如果不存在，追加新的 event table 和 handler table。
- 不重排用户已有 TOML。
- 写入前备份，写入后用 TOML 解析器或 `codex` 启动检查确认没有语法错误。

如果你没有可靠 TOML 编辑能力，停止并告诉用户需要改 `hooks.json` 或让 Agent 使用 TOML parser。不要用脆弱的纯字符串替换破坏配置。

## 5. 配置 Windows 原生 Codex

在 PowerShell 中定位 Windows 用户级 Codex 配置：

```powershell
$codexDir = Join-Path $env:USERPROFILE ".codex"
$hooksJson = Join-Path $codexDir "hooks.json"
$configToml = Join-Path $codexDir "config.toml"
New-Item -ItemType Directory -Force $codexDir | Out-Null
```

然后按第 2 到第 4 节合并配置。Windows command 值必须指向：

```powershell
$command = '"' + (Join-Path $env:LOCALAPPDATA "Codex Island\codex-island-hook.exe") + '"'
```

配置完成后做 smoke test：

```powershell
$payload = '{"session_id":"codex-island-smoke-windows","cwd":"C:\\","hook_event_name":"UserPromptSubmit"}'
$payload | & (Join-Path $env:LOCALAPPDATA "Codex Island\codex-island-hook.exe")
Test-Path (Join-Path $env:LOCALAPPDATA "CodexIsland\sessions\codex-island-smoke-windows.json")
```

## 6. 配置 WSL Codex

如果当前会话已经在 WSL 内，先配置当前发行版。若你从 Windows 原生 Codex 运行，并且用户希望覆盖多个 WSL 发行版，可以用 `wsl.exe -l -q` 枚举发行版，再逐个执行相同逻辑。

在每个目标 WSL 发行版内：

```bash
mkdir -p ~/.codex
win_local_app_data="$(cmd.exe /c 'echo %LOCALAPPDATA%' | tr -d '\r')"
hook_path="$(wslpath -u "$win_local_app_data")/Codex Island/codex-island-hook.exe"
test -f "$hook_path"
```

WSL hooks command 值：

```bash
command="\"$hook_path\""
```

然后按第 2 到第 4 节合并 `~/.codex/hooks.json` 或 `~/.codex/config.toml`。

配置完成后做 smoke test：

```bash
printf '%s' '{"session_id":"codex-island-smoke-wsl","cwd":"/home","hook_event_name":"UserPromptSubmit"}' | "$hook_path"
test -f "$hook_path"
test -f "$(wslpath -u "$win_local_app_data")/CodexIsland/sessions/codex-island-smoke-wsl.json"
```

## 7. 让用户信任 hooks

配置写入后，不要直接宣称 hooks 已经生效。Codex 对非托管 command hooks 有信任机制，新 hook 或修改过的 hook 需要用户审查。

告诉用户在 Windows Codex 和每个 WSL Codex 中分别执行：

```text
/hooks
```

用户需要检查并信任包含 `codex-island-hook.exe` 的 6 个事件 hook。信任完成前，Codex 可能会跳过这些 hook，这是正常安全行为。

## 8. 最终验证

只有完成下面检查后，才可以报告配置完成：

- `codex-island-hook.exe` 存在。
- Windows 用户级 hooks 已写入或明确说明 Windows 原生 Codex 不在本机使用。
- WSL 用户级 hooks 已写入或明确说明本机没有需要接入的 WSL Codex。
- 每个修改过的配置文件都有备份。
- JSON 或 TOML 配置能重新解析。
- smoke test 能写出 `%LOCALAPPDATA%\CodexIsland\sessions\*.json`。
- 已提示用户进入 `/hooks` 信任新增 hook。

最终回复用户时，说明实际写入了哪些配置路径、哪些 WSL 发行版已配置、备份文件在哪里，以及是否还等待 `/hooks` 信任。

## 当前产品行为

Codex Island 只读取状态文件并显示状态，不再提供内置 hook 自动配置界面。hook 自动配置由拿到本 README 的 Codex Agent 完成。

状态目录：

```text
%LOCALAPPDATA%\CodexIsland\sessions\
```

状态含义：

```text
running   蓝色   正在运行
completed 绿色   当前轮完成
waiting   黄色   需要人工确认
error     红色   状态文件解析异常或字段缺失
stale     灰色   运行中但超过 10 分钟没有更新
```

运行逻辑：

- 默认收起，只显示呼吸灯。
- 鼠标悬浮后展开详情列表。
- 会话按首次加入时间排序，状态变化不会改变位置。
- 完成、异常和过期会话会保留，直到用户手动隐藏。
- 隐藏只影响 UI，不删除状态文件；同一会话后续有新事件时会重新出现。

## 开发者补充

普通使用者不需要本节。只有维护安装包时才需要源码命令。

```bash
pnpm install
pnpm test
pnpm build
cd src-tauri && cargo test
```

构建 Windows 安装包：

```bash
pnpm bundle:windows
```

如果在 WSL/Linux 中交叉构建 Windows 安装包：

```bash
pnpm bundle:windows:wsl
```

安装包输出目录：

```text
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/
```
