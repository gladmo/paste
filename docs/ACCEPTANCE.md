# 验收清单（对照原版 Paste 4.4.0）

依据：原版 `Localizable.strings` 346 条文案 + `design/` 六张截图 + `PasteStack.storyboardc` 解码 + Core Data v6 模型。
标注：✅ 已实现 · ⚠️ 简化实现 · ❌ 有意裁剪（iCloud/付费墙/Firebase/AppIntents/Onboarding/What's New/iOS）

## 1. 主面板（对照 主界面.png / 主界面链接页.png）

| 项 | 规格（测量值） | 状态 |
|---|---|---|
| 面板尺寸 | 全屏宽 × 332pt 贴底，**直角无边**，真实 macOS vibrancy（HUD 材质，实测两份截图底色随桌面变化 #25–#2a / #3d–#61） | ✅ `overlay.rs` `set_effects` + `theme.ts` 半透明 tint |
| 工具栏 | 控件垂直中心 30pt；居中组合：搜索图标 + Tab 标签；卡片自 y=64pt 起；最右 22.5pt 处 `⋯` | ✅ |
| 搜索 | 收起态=16pt 放大镜图标（#9da1a2）；点击/输入展开输入框（300pt，#3d4245） | ✅ |
| Tab 栏 | 激活=#3d4245 圆角块（152.5×27.5pt，r≈5）+ 时钟图标 + 亮字 #e1e2e2；非激活=13pt 实心圆点 + #9a9c9e 文字；相邻 `+` 启动新固定板 | ✅ |
| 卡片 | 外框 248×248（4pt 边框槽），内容 240×240，间距 16pt（pitch 264），#141414 底，圆角 11pt；选中=4pt #0077fc 实线（无光晕） | ✅ |
| 卡片页脚 | 底部居中内容元信息 `%d characters` / `File` / `%d files`（取自原版 Localizable.stringsdict 复数键），#959595 11px；右下角类型字形 ~9.5pt | ✅ |
| Quick Paste | 按住修饰键显示 1-9 角标，数字键直接粘贴 | ✅（修饰键可配置） |
| 纯文本粘贴 | 按住修饰键 + 回车/数字 | ✅（默认 ⌥） |
| 多选 | ⌘点击/Select/Select All/Deselect All/批量删除确认（`alert.delete-selected-items.title`） | ✅ |
| 空态 | `general.history-empty` / `general.nothing-found` / `general.pinboard-empty` | ✅ |
| 翻页箭头 | 卡片区左右边缘悬停出现 | ✅ |
| 右键菜单 | Paste / Paste as Plain Text / Copy / Copy as Plain Text / Open Link / Reveal in Finder / Preview / Pin·Unpin / Select Pinboard▸ / Select / Select All / Delete | ✅ |

## 2. Pinboard（对照 Core Data SnippetList/Pinboard 实体）

| 项 | 状态 |
|---|---|
| 多板 + 内置 Pinned（id=1，红） | ✅ |
| 新建（名称 + 8 色板）/ 重命名（双击或菜单）/ 换色 / 删除（连带内容，确认框 `alert.delete-pinboard.*`） | ✅ |
| 固定到历史顶部（pinned 标志）+ 移入用户板（`general.action.select-pinboard`） | ✅ |
| ⌥⌘←/→ 切换板（全局快捷键，面板自动弹出） | ✅ |

## 3. 设置窗口（对照 设置页.png / 设置页热键.png）

| 项 | 规格 | 状态 |
|---|---|---|
| 窗口 | 641×590pt，200pt 侧栏，六 Pane | ✅ |
| General | 菜单栏图标开关（托盘显隐）/ 登录启动（autostart 同步）/ 音效 | ✅ |
| Keep History | 永久/1月/1周/1天 + 无限提示文案，插入与改设置时清理（保留固定与板） | ✅ |
| Paste Items | 到活动应用（AX 检测 + 跳系统设置）/ 到剪贴板 / 始终纯文本 | ✅ |
| Rules | 忽略应用（运行应用选择器）/ 瞬时内容（org.nspasteboard.TransientType 等）/ 机密内容（com.apple.is-password 等）/ 链接预览开关 | ✅ |
| Shortcuts | 6 项录制器（含两个纯修饰键）+ 重置确认 | ✅ |
| About | 图标 + 版本 + 帮助/隐私/条款链接 | ✅ |

## 4. 预览 / 新建（对照 `edit-item.*` / `create-item.*` 文案）

| 项 | 状态 |
|---|---|
| 富文本编辑 B/I/U/S + Save/Discard，textutil 重新生成 RTF | ✅ |
| 纯文本编辑 | ✅ |
| 图片左/右旋转（canvas 旋转 + 覆盖存储） | ✅ |
| 链接详情展示 | ✅ |
| 新建文本项（Create/Cancel/⌘↩） | ✅ |

## 5. Paste Stack（对照 storyboard 解码）

| 项 | 规格 | 状态 |
|---|---|---|
| 面板 | 264×438pt、毛玻璃、浮动、非激活 | ✅ |
| 行 | 44pt、类型图标 + 内容 + 时间 | ✅ |
| 方向切换 | sort-up/down 按钮，横竖互切（尺寸互换） | ✅ |
| 关闭按钮 + 空态占位 | storyboard close: / "Empty" | ✅ |
| 点击粘贴到前台应用 | CGEvent ⌘V | ✅ |

## 6. 托盘 / 暂停 / 音效（对照 托管小图标菜单页.png）

| 项 | 状态 |
|---|---|
| 菜单结构 Show/Pause▸/Erase/Manage/Settings/Help/Feedback/Quit | ✅（英文文案 ⚠️） |
| 暂停 1 小时/到明天/无限期 + Resume，暂停期间不采集 | ✅ |
| 捕获 Copy.aiff / 粘贴 Paste.aiff，受音效开关 | ✅ |

## 7. 数据与 i18n

| 项 | 状态 |
|---|---|
| 346 条 × 3 语全量资源（键=原版键名） | ✅ `src/i18n/` |
| 界面文案全部走 i18n（面板/设置/预览/新建/Stack） | ✅ |
| 旧库无损迁移（pinned/rtf/meta/pinboards 列） | ✅ |

## 8. 已知简化（⚠️）

1. 托盘菜单文案英文（原版随系统语言）
2. 颜色剪贴板不解码色值（占位色板 + 原始数据回写）
3. 多文件回写 = file-url + 路径文本
4. Pinboard 拖拽排序未做（按 display_order=插入时间）
5. 设置改小保留期时不弹「删除较旧项目」确认（直接清理）
6. Quick Paste 角标仅在卡片页脚显示数字（原版为悬停时显示）

## 9. 回归基线（不可破坏）

- ⌘⇧V 弹出/隐藏面板；非激活面板不抢焦点
- 点击面板外任意处（含同一应用的其它窗口）立即收起 —— 对齐原版 `windowDidResignKey:` 处理（300ms 宽限期防 show 时 key 窗口抖动）；Esc / 粘贴后同样收起；frontmost 轮询作为切应用时的后备
- Enter/双击粘贴到之前的应用（AX 或 pid 回退）
- 富文本粘贴保留格式（RTF 回写）
- 图片粘贴回 PNG
- 捕获去重（LAST_SEEN 哈希）与自身粘贴回环不重复入库
