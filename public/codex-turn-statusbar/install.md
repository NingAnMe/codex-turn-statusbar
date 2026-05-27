# Codex Turn Status Bar Agent 安装指南

你正在为用户安装 `Codex Turn Status Bar`。这是一个 macOS 菜单栏工具，用来显示 Codex Desktop 是否有未处理或未读的任务轮次。

发布地址：`http://10.1.111.12:18000/codex-turn-statusbar/`。

## 目标

- 安装 macOS universal 版 `CodexTurnStatusBar.app`。
- 启动 App，让它自动安装 `codex-turn-notify` helper 并更新 `~/.codex/config.toml`。
- 不要要求用户提供 `SKILL_INSTALL_DIR`。这个工具不是 Agent skill，不安装到 skills 目录。
- 安装后提醒用户重启 Codex Desktop，或新开 Codex 会话，让 Codex 重新加载 `notify` 配置。

## 支持平台

- macOS：已发布 `.dmg`，支持 Apple Silicon 和 Intel。
- Windows：当前页面未发布 Windows 安装包。不要伪造 Windows 安装步骤。
- Linux：不适用；这是 macOS 菜单栏应用。

## Agent 安装命令

macOS：

```bash
curl -fsSL http://10.1.111.12:18000/codex-turn-statusbar/install.sh | sh
```

Windows PowerShell 当前只会说明未发布：

```powershell
irm http://10.1.111.12:18000/codex-turn-statusbar/install.ps1 | iex
```

## 覆盖策略

安装脚本会：

- 下载 `codex-turn-statusbar-latest.dmg`。
- 校验 SHA256。
- 挂载 dmg。
- 默认复制 `CodexTurnStatusBar.app` 到 `/Applications`。
- 如果 `/Applications` 不可写，回退到 `$HOME/Applications`。
- 覆盖已有 App 前先备份为 `CodexTurnStatusBar.app.backup.<timestamp>`。
- 启动 App。

App 首次启动会：

- 从自身 bundle 的 `Contents/Resources/codex-turn-notify` 复制 helper 到 `~/.codex/bin/codex-turn-notify`。
- 更新 `~/.codex/config.toml` 中的 `notify = ["..."]`。
- 如果已有 config，会先备份。

## 验证安装

安装后运行：

```bash
test -d /Applications/CodexTurnStatusBar.app || test -d "$HOME/Applications/CodexTurnStatusBar.app"
test -x "$HOME/.codex/bin/codex-turn-notify"
grep '^notify = ' "$HOME/.codex/config.toml"
```

也可以确认 App 是 universal：

```bash
APP="/Applications/CodexTurnStatusBar.app"
if [ ! -d "$APP" ]; then APP="$HOME/Applications/CodexTurnStatusBar.app"; fi
file "$APP/Contents/MacOS/CodexTurnStatusBar"
file "$APP/Contents/Resources/codex-turn-notify"
```

期望看到 `x86_64` 和 `arm64`。

## 首次使用时要告诉用户

- 打开 App 后，它只显示在菜单栏，不会出现在 Dock。
- 白色圆环表示没有已知未读任务。
- 绿色图标表示有未处理或未读 Codex 活动。
- 重启 Codex Desktop，或新开 Codex 会话后，`notify` 才会被 Codex 重新加载。
- 如果 macOS Gatekeeper 提示无法验证开发者，让用户在 Finder 中右键 App 选择“打开”。

## 安全边界

- 不要把用户的 Codex 配置内容贴到聊天里。
- 不要修改除 `~/.codex/bin/codex-turn-notify` 和 `~/.codex/config.toml` 以外的 Codex 文件。
- 不要删除用户已有配置；覆盖前必须备份。
- 不要要求用户提供密码、token 或私有凭证。

## 机器可读信息

- `latest.json`: http://10.1.111.12:18000/codex-turn-statusbar/latest.json
- `dmg`: http://10.1.111.12:18000/codex-turn-statusbar/codex-turn-statusbar-latest.dmg
- `dmg sha256`: `1180f619a6126f5c0f8162abc8302ff13466efaf167c588721e0174ffbe11bba`
- `tar.gz`: http://10.1.111.12:18000/codex-turn-statusbar/codex-turn-statusbar-latest.tar.gz
- `tar.gz sha256`: `e850ea20b531a67cac6bfdf10676456cedd0620816fbb23af544503fea64a56f`
- `zip`: http://10.1.111.12:18000/codex-turn-statusbar/codex-turn-statusbar-latest.zip
- `zip sha256`: `1e72fe431c4c5fe94028f7fe43509e2bc7a1011eeee162fa462fe9eed89e1de2`
- version: `0.2.0`
