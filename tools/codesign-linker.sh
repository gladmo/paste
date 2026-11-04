#!/bin/bash
# Cargo linker 包装脚本（macOS）：先调用真实链接器，再用稳定开发证书签名产物。
# 目的：让 `tauri dev` / `cargo build` 每次产出的二进制携带同一签名身份，
#       macOS TCC（辅助功能权限）不再因重编译而失效。
#
# 环境变量：
#   PASTE_SIGN_IDENTITY  签名身份（默认 Paste-Dev）；设为 "-" 则强制 ad-hoc。
# 签名身份不存在时打印警告并跳过（回退到默认 ad-hoc，不阻塞构建）。

set -euo pipefail

IDENTITY="${PASTE_SIGN_IDENTITY:-Paste-Dev}"

# 1. 执行真实链接（所有参数透传给 cc/clang）
REAL_LINKER="${CC:-cc}"
"$REAL_LINKER" "$@"

OUT=""
prev=""
for arg in "$@"; do
    case "$prev" in
        -o) OUT="$arg"; break ;;
    esac
    prev="$arg"
done
[ -n "$OUT" ] && [ -f "$OUT" ] || exit 0  # 没有可识别的输出文件（如仅编译探测）则不签名

if [ "$IDENTITY" = "-" ]; then exit 0; fi

if ! security find-identity -v -p codesigning 2>/dev/null | grep -q "\"$IDENTITY\""; then
    echo "⚠️  未找到代码签名证书 \"$IDENTITY\"，跳过签名（TCC 权限会随重编译失效）。" \
         "运行 just cert 创建。" >&2
    exit 0
fi

# 2. 签名（--force 覆盖 cargo/tauri 可能已有的 ad-hoc 签名）
if ! codesign --force --sign "$IDENTITY" "$OUT" 2>/dev/null; then
    echo "⚠️  codesign 失败（identity=${IDENTITY}），保留原签名继续。" >&2
fi
exit 0
