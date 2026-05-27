# macOS Development Notes

这份笔记记录 Codex Turn Status Bar 这次开发 macOS 菜单栏软件和打包 `.dmg` 时确认下来的经验、踩坑和建议做法。

## 菜单栏应用

### 使用 Accessory/LSUIElement

这个应用只需要出现在菜单栏，不需要 Dock 图标和主窗口：

- 运行时设置 `ActivationPolicy::Accessory`。
- `.app/Contents/Info.plist` 设置 `LSUIElement = true`。
- Tauri 配置里可以不创建窗口，直接用 tray API 创建菜单栏图标。

如果缺少 `LSUIElement`，用户会看到一个多余 Dock 图标；如果缺少运行时的 accessory policy，某些 macOS 行为会更像普通前台应用。

### 不要在轮询里反复重建菜单

macOS 菜单栏的菜单正在展开时，如果程序调用 `set_menu(Some(menu))` 重新替换菜单，系统会关闭当前菜单。我们一开始每秒刷新状态时都会重建菜单，结果用户点击图标后菜单只闪现一瞬间。

正确做法：

- 每秒可以刷新图标和 tooltip。
- 只有菜单内容真的变化时才重建菜单。
- 用一个轻量 `MenuContentKey` 记录菜单相关状态：`state`、`title`、`detail`、`cwd`、`updated_at`、`can_mark_handled`。
- `Refresh` 这类显式操作可以强制重建菜单。

### 彩色菜单栏图标不要使用 template 模式

macOS template icon 会被系统按菜单栏风格重绘，彩色图标可能变成黑色或单色。这个项目的 NeedsAttention 必须是绿色提醒，所以每次设置图标后都要确保：

```rust
tray.set_icon_as_template(false)
```

如果需要系统自动适配深浅色主题，template 图标是好选择；但这里需要强提醒色，不能启用 template。

### 图标尺寸要按视觉面积调，不是只看画布大小

菜单栏图标虽然用的是 32x32 RGBA 画布，但真正的视觉尺寸取决于图形占用范围、线宽、透明度和周围图标风格。

这次比较稳的方向：

- Idle：白色圆环加中心点，外径约 19px，线宽约 2.35px。
- NeedsAttention：绿色线框消息气泡，主体尽量占到 20px 以上，再加右上角圆点。
- Warning：黄色线框三角，尺寸和 Idle/NeedsAttention 保持接近。
- 三种状态都保持线描风格，避免某个状态突然变成大色块。

实际调图标时一定要在真实菜单栏里看截图。单独看 32x32 PNG 很容易误判。

## Codex 状态来源

### notify 是稳定兜底

Codex 的 `notify` 可以在一轮完成后调用本项目的 `codex-turn-notify`，写入：

```text
~/.codex/codex-turn-status.json
~/.codex/codex-turn-status-event.json
```

这个状态文件适合作为跨平台兜底。Windows 目前主要依赖这条路径。

### macOS IPC 不是完整初始快照

Codex Desktop 的本地 IPC 可以收到一些 unread/read 广播，例如：

- `thread-read-state-changed`
- `thread-stream-state-changed`

但它更像事件流，不一定在我们的菜单栏 app 启动时给出当前所有未读会话的完整快照。因此不能因为 IPC 已连接且当前内存未读计数为 0，就压掉 `notify` 的 NeedsAttention。

当前策略：

- IPC 未读计数大于 0 时，显示 NeedsAttention。
- `notify` 已经写入 NeedsAttention 时，继续保留兜底提醒。
- 收到匹配 thread 的 read-state false 事件时，再自动清掉 notify 兜底状态。

这避免了“已经有未处理内容，但启动时没收到历史 unread 事件，所以状态栏是 Idle”的问题。

### 不要用“Codex 前台活跃”代表已处理

用户可能一直停留在 Codex 窗口，但还没有处理新完成的轮次。仅凭 Codex 成为前台应用就自动清除提醒，会漏掉真实待处理任务。

更可靠的清除条件是：

- Codex 明确发出 read-state false。
- 用户手动点击 `Mark Handled`。

### 过滤内部授权事件

Codex Desktop 有时会因为内部授权/风险判断流程触发类似 turn-ended 的 notify。典型 payload 的 `last-assistant-message` 是 JSON，包含：

- `risk_level`
- `user_authorization`
- `outcome`
- `rationale`

这类事件不应该让状态栏变绿，否则会出现“状态栏提醒，但 Codex 没有完成轮次”的误报。

## 打包和安装

### universal macOS 构建

macOS 同时支持 Apple Silicon 和 Intel，需要分别构建两个目标，然后用 `lipo` 合并：

```sh
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin

cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create arm64-binary x86_64-binary -output universal-binary
```

注意：不仅 `.app` 里的主程序要 universal，随包分发的 `codex-turn-notify` 也要 universal。

验证命令：

```sh
file CodexTurnStatusBar.app/Contents/MacOS/CodexTurnStatusBar
file codex-turn-notify
```

应该同时看到 `x86_64` 和 `arm64`。

### .app Bundle 的关键文件

最小可用 `.app` 结构：

```text
CodexTurnStatusBar.app/
  Contents/
    Info.plist
    MacOS/CodexTurnStatusBar
    Resources/icon.png
```

`Info.plist` 关键项：

- `CFBundleExecutable`
- `CFBundleIdentifier`
- `CFBundleName`
- `CFBundlePackageType = APPL`
- `CFBundleShortVersionString`
- `CFBundleVersion`
- `LSUIElement = true`

### dmg 不是把 zip 目录直接封进去

第一次生成 dmg 时，直接把完整 package 目录作为 `srcfolder`，用户打开后只看到一个普通文件夹，没有熟悉的拖拽安装体验。

更像标准 macOS 软件的 dmg 根目录应该是：

```text
CodexTurnStatusBar.app
Applications -> /Applications
```

用户看到 `CodexTurnStatusBar.app` 和 `Applications` 后，就能按习惯拖拽安装。为了让用户更直观，构建脚本会生成背景图，写上“拖动到 Applications 完成安装”，并用 Finder AppleScript 设置图标位置、背景图、窗口大小和隐藏 toolbar/statusbar。这些 Finder 视觉设置最终会写入 dmg 里的 `.DS_Store`。

背景图不要放在 dmg 根目录的 `.background` 文件夹里，因为用户如果开启了 Finder 显示隐藏文件，会直接看到 `.background`。当前做法是把背景图放在 `CodexTurnStatusBar.app/Contents/Resources/dmg-background.png`，Finder 背景引用这个 bundle 内部文件。

背景图生成走 Objective-C/AppKit 小工具，而不是 Swift 脚本。这次本机 Swift 遇到 SDK/编译器版本不匹配和模块缓存写入问题；Objective-C 只需要 `clang -fobjc-arc -framework AppKit`，更适合这种构建期小工具。

### App 安装和 notify 配置是两件事

把 `.app` 拖进 `/Applications` 只安装菜单栏程序。为了避免额外脚本步骤，当前做法是把 `codex-turn-notify` 嵌入 `.app/Contents/Resources`，App 启动时自动同步到用户的 `~/.codex/bin/codex-turn-notify` 并更新 `~/.codex/config.toml`。

启动时自动配置会：

- 复制 `.app/Contents/Resources/codex-turn-notify` 到 `~/.codex/bin/codex-turn-notify`。
- 备份现有 `~/.codex/config.toml`。
- 写入或替换 `notify = [".../codex-turn-notify"]`。
- 用户仍需要重启 Codex Desktop 或新开 Codex 会话，让 Codex 重新加载配置。

如果后续要做“真正一键安装且立刻生效”，需要研究 Codex 是否能热重载 notify 配置；否则重启 Codex 这一步仍然无法完全消除。

### hdiutil 可能需要更高权限

在受限运行环境里，`hdiutil create` 可能失败并报：

```text
hdiutil: create failed - 设备未配置
```

这不是脚本逻辑错误，而是沙箱/设备访问限制。放到正常终端里跑，或在当前环境里提权运行即可。

## 签名、公证和 Gatekeeper

当前包没有 Apple Developer ID 签名和 notarization。用户首次打开时可能遇到：

```text
无法打开，因为无法验证开发者
```

临时解决：

- Finder 里右键 `.app`，选择“打开”。
- 或在系统设置的隐私与安全里允许打开。

正式分发建议：

- 使用 Developer ID Application 证书签名 `.app`。
- notarize `.dmg`。
- staple notarization ticket。
- 如果重新引入 `.command` 或 `.pkg` 安装脚本，也要确认 Gatekeeper 提示和用户体验。

## 验证清单

每次发布前至少跑：

```sh
cargo test --workspace
zsh -n scripts/package-tauri-macos-universal.sh
./scripts/package-tauri-macos-universal.sh
file dist-cross/CodexTurnStatusBar-0.2.0-macos-universal/CodexTurnStatusBar.app/Contents/MacOS/CodexTurnStatusBar
```

手工验证：

- 状态栏图标尺寸和旁边系统/第三方图标接近。
- Idle、NeedsAttention、Warning 三种图标风格一致。
- 点击菜单栏图标后菜单不会被轮询刷新关掉。
- NeedsAttention 在 Codex 已经前台时仍然能显示。
- 匹配 thread 被读后，提醒能自动清除。
- dmg 打开后能看到 App、Applications 快捷入口、提示文案和箭头背景。
- 拖拽 App 到 Applications 后能启动。
- App 首次启动后，`~/.codex/config.toml` 正确更新。

## 后续改进

- 做签名和 notarization，减少 Gatekeeper 阻碍。
- 为 dmg 增加背景图和图标位置，进一步接近正式发行体验。
- Windows 侧补 MSI/WiX 构建，并明确 WebView2、开机启动和 notify 配置策略。
- 如果 Codex 后续提供正式 unread snapshot API，替换当前 IPC 事件流加 notify 兜底的策略。
