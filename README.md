# Paste（Tauri 复刻版）

用 Tauri 2 + React 19 对 macOS 剪贴板管理工具 [Paste](https://pasteapp.io)（v4.4.0）进行的 1:1 复刻，UI 对齐优先。原版界面文案（en / zh-Hans / zh-Hant 共 346 条）与音效、图标均提取自原版应用包；面板几何参数来自原版截图的像素级测量与 `PasteStack.storyboardc` 的解码结果。

## 构建（仅 macOS）

```bash
pnpm install
just dev           # 开发（等价 pnpm tauri dev，自动装依赖）
just release       # 产出 .app / .dmg 并打印路径
just release-open  # 构建后在 Finder 中打开 .app
just release-universal  # 通用二进制（Apple Silicon + Intel）

# 或直接使用 pnpm：
pnpm tauri dev     # 开发
pnpm tauri build   # 产出 .app / .dmg
```

> 首次运行需要授予「辅助功能」权限（直接粘贴到其他应用需要；未授权时自动回退到 post_to_pid）。

Rust 后端已在 Linux 上用 `aarch64-apple-darwin` 目标 + zig 交叉 C 编译器完成全量 `cargo check`（零错误零警告）；最终链接与运行仍需在 Mac 上执行 `pnpm tauri dev/build`。

## 功能

| 区域 | 能力 |
|---|---|
| 主面板（⌘⇧V） | 历史记录 + 固定（Pinned）+ 多 Pinboard（彩色标签、新建/重命名/换色/删除）、搜索（FTS5 trigram）、方向键/⌘数字/回车/⌥纯文本粘贴、多选与批量删除、右键菜单全量（粘贴/纯文本粘贴/复制/固定/选板/预览/打开链接/Finder 显示/删除）、清空历史（保留固定）、边缘翻页箭头 |
| 内容类型 | 文本、富文本（RTF/HTML，粘贴还原格式）、链接（自动抓取标题）、图片、文件（Finder 复制，含数量/大小）、颜色 |
| Paste Stack（⌃⌘V） | 264×438 毛玻璃侧板（storyboard 原尺寸）、44pt 行、横竖方向切换、点击直接粘贴到前台应用 |
| 设置 | 通用（菜单栏图标/登录启动/音效）、保留历史（永久/1月/1周/1天）、粘贴项目（到活动应用-AX / 到剪贴板 / 始终纯文本）、规则（忽略应用/瞬时内容/机密内容/链接预览）、快捷键（录制器 + 重置）、关于 |
| 暂停 | 托盘菜单暂停 1 小时/到明天/无限期，托盘恢复 |
| 音效 | 原版 Copy.aiff / Paste.aiff（可关） |
| 托盘 | Show / Pause ▸ / Erase History… / Manage Pinboards… / Settings… / Help / Feedback / Quit |
| 预览窗口 | 富文本 B/I/U/S 编辑（textutil 重新生成 RTF）、纯文本编辑、图片左/右旋转 |
| 新建 | 新建文本项窗口 |
| i18n | en / zh-Hans / zh-Hant 跟随系统（原版文案） |

默认快捷键：激活 ⇧⌘V、Paste Stack ⌃⌘V、Quick Paste `修饰键`+1-9（默认 ⌘）、纯文本修饰键（默认 ⌥）、上/下一固定板 ⌥⌘←/→ —— 全部可在设置中重录。

## 架构

```
src/
  i18n/        三语资源（346 键 × 3，由 tools/gen_i18n.py 从原版生成）
  ui/theme.ts  截图测量出的几何/颜色常量（面板 332pt、卡片 242×248、#0077fc 选中环等）
  overlay/     主面板：SearchBar / TabBar / ClipCard / Menu / Overlay
  settings/    设置窗口（六 Pane + 快捷键录制器）
  preview/     预览窗口（文本/富文本/图片）
  create/      新建文本项窗口
  stack/       Paste Stack 面板
  stores/      zustand（panel / settings）
src-tauri/src/
  clipboard.rs 轮询捕获（富文本/文本/链接/图片/文件/颜色）+ 规则门控 + 音效 + 保留期
  db.rs        SQLite（clips + pinboards + FTS5）与视图查询
  overlay.rs   主 NSPanel（贴底全宽、非激活）
  stack.rs     Paste Stack NSPanel
  paste.rs     AX 权限 + CGEvent ⌘V 直接粘贴
  settings.rs  tauri-plugin-store 持久化 + 设置窗口 + 运行应用列表
  pause.rs     暂停状态 + 音效播放（afplay）
  tray.rs      托盘菜单（暂停子菜单、随状态重建）
```

多窗口 = 单 `index.html` + hash 路由（`#/settings`、`#/preview?id=`、`#/create`、`#/stack`）。

## 与原版的已知差异（有意裁剪）

- iCloud 同步、订阅/付费墙、Firebase、AppIntents、Onboarding、What's New、iOS 专属功能 —— 不做
- 托盘/菜单文案暂为英文（原版跟随系统语言）
- 颜色剪贴板捕获时未解码具体色值（预览用占位色板，粘贴还原原始数据）
- 多文件粘贴回写为 file-url + 路径文本（非多 NSPasteboardItem）

## 工具脚本（仓库根 `tools/`）

- `gen_i18n.py` — 从 `Paste.zip` 重新生成三语资源
- `extract_assets.py` — 提取音效与 AppIcon.icns
- `check-darwin.sh` — 在 Linux 上交叉类型检查 macOS 后端（zig + darwin std）

验收清单见 [docs/ACCEPTANCE.md](docs/ACCEPTANCE.md)。
