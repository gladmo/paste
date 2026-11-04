#!/bin/bash
# 一次性生成开发用自签名代码签名证书（稳定身份，修复每次重编译后 TCC 权限失效）。
# 产物导入 login 钥匙串；导入后需在「钥匙串访问」中把证书设为「始终信任」（需要密码，无法全自动）。
set -euo pipefail

CERT_NAME="Paste-Dev"
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/paste-cert.XXXXXX")"  # 脚本一次性跑完，用完即删
KEYCHAIN="${HOME}/Library/Keychains/login.keychain-db"
trap 'rm -rf "$WORKDIR"' EXIT

# 已存在则直接退出
if security find-identity -v -p codesigning "$KEYCHAIN" 2>/dev/null | grep -q "\"$CERT_NAME\""; then
    echo "✔ 钥匙串中已存在代码签名证书 \"$CERT_NAME\"，无需重复创建。"
    exit 0
fi

echo "→ 生成自签名证书 ${CERT_NAME}（RSA 2048，10 年，EKU=codeSigning）..."
cat > "$WORKDIR/openssl.cnf" <<'EOF'
[req]
distinguished_name = dn
x509_extensions = v3
prompt = no
[dn]
CN = Paste-Dev
O = Paste Replica Dev
[v3]
basicConstraints = critical,CA:TRUE
keyUsage = critical,digitalSignature,keyCertSign
extendedKeyUsage = critical,codeSigning
EOF

openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 3650 \
    -keyout "$WORKDIR/key.pem" -out "$WORKDIR/cert.pem" \
    -config "$WORKDIR/openssl.cnf" 2>/dev/null

echo "→ 导入 login 钥匙串（p12 含私钥+证书，导入即为一个完整 identity）..."
# 注意：PKCS12 必须用 -legacy（SHA1/3DES），否则 macOS security 报 MAC verification failed
P12_PASS="paste-dev-import"
openssl pkcs12 -export -out "$WORKDIR/identity.p12" -legacy \
    -inkey "$WORKDIR/key.pem" -in "$WORKDIR/cert.pem" -name "$CERT_NAME" -passout "pass:$P12_PASS"
security import "$WORKDIR/identity.p12" -k "$KEYCHAIN" -P "$P12_PASS" -T /usr/bin/codesign -A >/dev/null

# 自签名根证书默认不受信任（CSSMERR_TP_NOT_TRUSTED），设置用户域信任（可能弹出确认框，请允许）
echo "→ 设置用户域代码签名信任（如弹窗请点允许）..."
security add-trusted-cert -r trustRoot -p codeSign -k "$KEYCHAIN" "$WORKDIR/cert.pem"

# 验证 identity 可被 codesign 使用
if ! security find-identity -v -p codesigning "$KEYCHAIN" | grep -q "\"$CERT_NAME\""; then
    echo "✘ 未找到可用（受信任）的 identity。若上方提示 NOT_TRUSTED，" >&2
    echo "  请打开「钥匙串访问」→ login → 双击证书 \"$CERT_NAME\" → 信任 → 始终信任。" >&2
    exit 1
fi

echo
echo "✔ 完成。之后重新编译（just dev / cargo build）即自动用 ${CERT_NAME} 证书签名，"
echo "   TCC（辅助功能权限）不再因重编译失效。"
echo "   首次使用请在 系统设置 → 隐私与安全性 → 辅助功能 重新授权一次，"
echo "   并删除旧的 ad-hoc 条目（显示为 paste / tauri 的重复项）。"
