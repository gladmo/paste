# Paste (Tauri 复刻) — 常用任务
# 依赖: macOS + Rust + pnpm 10 (corepack enable) + just
#   brew install just   # 如未安装

default:
    @just --list

# 安装 JS 依赖
install:
    pnpm install

# 开发模式（热重载, ⇧⌘V 呼出面板）
dev: install
    pnpm tauri dev

# 发布构建：产出可直接使用的 .app 和 .dmg（当前 Mac 架构）
release: install
    #!/bin/bash
    set -euo pipefail
    pnpm tauri build
    echo ""
    echo "✔ 构建产物:"
    ls -d src-tauri/target/release/bundle/macos/*.app 2>/dev/null || true
    ls src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null || true

# 发布构建并在 Finder 中打开 .app 所在目录
release-open: release
    open src-tauri/target/release/bundle/macos

# 通用二进制（Apple Silicon + Intel）发布构建
# 首次使用前: rustup target add aarch64-apple-darwin x86_64-apple-darwin
release-universal: install
    #!/bin/bash
    set -euo pipefail
    pnpm tauri build --target universal-apple-darwin
    echo ""
    echo "✔ 构建产物:"
    ls -d src-tauri/target/universal-apple-darwin/release/bundle/macos/*.app 2>/dev/null || true
    ls src-tauri/target/universal-apple-darwin/release/bundle/dmg/*.dmg 2>/dev/null || true

# 仅做前端类型检查 + 构建（不打包）
check: install
    pnpm build

# 清理构建产物
clean:
    @rm -rf dist src-tauri/target
