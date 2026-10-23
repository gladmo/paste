# AGENTS.md — 在本仓库工作的 AI 代理须知

Paste（macOS 剪贴板管理器 pasteapp.io v4.4.0）的 Tauri 2 复刻。**UI 对齐优先，功能仅面向 macOS**。当前主面板视觉基准是 `paste-ui.html` 卡片架设计（彩色类型头 + 应用徽标 + 元信息页脚）；设置/预览/Stack 等窗口仍以原版截图为基准。

## 环境（先读，能省一小时）

- **开发机是 Linux**，只能交叉 `cargo check`（`aarch64-apple-darwin`），**不能链接/运行 macOS 应用**。链接与运行验证由用户在 Mac 上执行（`just dev` / `just release`）。
- 每条 bash 命令都是独立 shell：**/tmp 写入会丢**，临时文件放 workspace 内；长命令写脚本文件再执行。
- `cargo`/`rustup`/`pnpm` 环境变量必须用 workspace 内的工具链（见 `../tools/check-darwin.sh` 开头的 exports）；`RUSTUP_HOME`/`CARGO_HOME` 系统默认只读。
- pnpm 安装需 `--store-dir /root/workspace/paste/.pnpm-store`。
- `cargo check` 原生 Linux 目标会因 glib 过旧失败 —— **不要**用 `cargo check`（native），一律用 `../tools/check-darwin.sh`。

## 仓库布局（workspace 根 = /root/workspace/paste）

```
paste/            本项目（git 仓库根，所有改动在此提交）
Paste.zip         原版应用包（本地化 strings/stringsdict、Assets.car、storyboard、二进制 strings —— 调研原版行为的依据）
paste-ui.html     主面板当前设计基准（用户提供的 HTML 原型）
design/           原版截图（主界面/设置/预览/托盘等 6 张，2x Retina，pt = px/2）
tools/            check-darwin.sh（交叉检查）、gen_i18n.py（从 Paste.zip 重生成 i18n）、extract_assets.py
```

## 验证（提交前必做，全部通过才 commit）

```bash
cd /root/workspace/paste/paste
./node_modules/.bin/tsc --noEmit          # 前端类型
./node_modules/.bin/vite build            # 前端构建
/root/workspace/paste/tools/check-darwin.sh   # Rust 交叉检查（零错误零警告）
```

Rust 有行为改动时，提醒用户在 Mac 上跑 `just dev` 复验；UI 改动请用户回传截图做像素复核。

## Git 约定

- 提交用内联作者：`git -c user.name=paste-replica -c user.email=dev@local commit`
- Message 风格：`type(scope): 中文或英文摘要`，正文写"为什么"与测量依据。
- 不要 push；不要改 `paste/` 以外的目录。

## 代码约定

- **Tailwind v4 兼容性**：目标含 macOS 12 WebKit，**禁用 oklch/color-mix 依赖**——不写 `bg-white/5`、`text-white/80` 这类透明度修饰符，统一用任意值 `bg-[rgba(255,255,255,0.05)]`；颜色只用 hex/rgba。`src/index.css` 的 `@theme` 已把常用色阶覆盖为 hex。
- **Tailwind 类必须字面量**（JIT 编译期扫描）：禁止 `` ring-[${color}] `` 之类的模板拼接。
- **i18n**：文案一律走 `src/i18n/`。原版有的键用原键名（en/zh-Hans/zh-Hant 三份都要加）；复刻自造的键放 `replica.*`（index.ts 的 REPLICA 表，三个 locale 同步添加——**重复键会 TS1117 报错**）。复数格式沿用原版 stringsdict：`%@ characters` / `File` / `%@ files` / `%@项` 等。
- 几何/颜色常量集中在 `src/ui/theme.ts`；改尺寸先想清楚面板总高 = 12 + 28(工具栏) + 12 + 8 + 280(卡片) + 8 + 20 = **368pt**，与 `overlay.rs` 的 `PANEL_HEIGHT` 必须一致。
- 多窗口共用一个 `index.html`，用 hash 路由分发（`#/settings`、`#/preview?id=N`、`#/create`、`#/stack`，默认 overlay）。

## 架构速览

```
前端 src/（React 19 + zustand）
  overlay/    主面板（Overlay=骨架、SearchBar 两态、TabBar、ClipCard、Menu）
  settings|preview|create|stack/   各窗口
  stores/     panel（视图/搜索/选中）、settings（语言覆盖等）
后端 src-tauri/src/
  overlay.rs  贴底 NSPanel：非激活面板 + 真实 vibrancy(HudWindow) + 显示后 300ms
              resign-key 宽限；windowDidResignKey → 点击外部收起（对齐原版）
  clipboard.rs 定时轮询 pasteboard，类型识别/规则门控/音效/保留期
  db.rs       SQLite：clips + pinboards + FTS5(trigram)；内置 Pinned 板 id=1
  paste.rs    AX 权限；CGEvent ⌘V 定向投递到记录的 target pid
  stack.rs    Paste Stack NSPanel（storyboard 原尺寸）
  tray.rs     托盘菜单（无 Help/Feedback——用户已要求移除）
  settings.rs tauri-plugin-store 持久化；含 language 字段（""=跟随系统）
```

关键不变量：面板是**非激活** NSPanel（不抢前 app 焦点，⌘V 直接投向目标 pid）；`macOSPrivateApi: true` 是 vibrancy 的前提；语言设置变更靠 `settings:updated` 事件广播到所有窗口。

## 调研原版行为的方法

原版 app 就在 `Paste.zip`：`unzip -p Paste.zip 'Paste.app/Contents/MacOS/Paste' | strings | grep <selector>` 查行为线索（如 `windowDidResignKey:`）；`Resources/*.lproj/Localizable.strings|stringsdict` 查文案与复数；`Assets.car`、`PasteStack.storyboardc` 用 REA 工具解码；`design/*.png` 用 PIL 像素测量（2x，logical pt = px/2）。做 UI 对齐前先测量、后写码，数值写进 theme.ts 注释。

## 明确不做（已与用户约定）

iCloud 同步、订阅/付费墙、Firebase、AppIntents、Onboarding、What's New、iOS 端。已知简化记录在 `docs/ACCEPTANCE.md` §8。

## 已知坑

- `tauri.conf.json` 的 `beforeDevCommand` 先 `pnpm build` 再起 vite —— 不要"优化"掉（全新 clone 没有 dist 会报 frontendDist 错误）。
- overlay 显示后 300ms 内的 resignKey 是 macOS key 窗口抖动，不是用户点击（见 overlay.rs `RESIGN_GRACE`），处理焦点逻辑时别绕过它。
- 富文本卡片渲染前必须 DOMPurify（剪贴板 HTML 不可信）。
- 图片/文件资源走 asset protocol（`convertFileSrc`），scope 在 `tauri.conf.json`。
