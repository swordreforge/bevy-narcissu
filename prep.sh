#!/usr/bin/env bash
# 水仙10周年 — 预压缩 + 部署脚本
# 用法:
#   ./prep.sh              # 本地:为 dist/ 生成全部 .br/.gz 预压缩文件(幂等,可重复执行)
#   ./prep.sh deploy USER@HOST [/remote/path]   # 本地 prep 后 rsync 推送到服务器
#
# 说明:
#   - brotli -q 11 需要较长时间(wasm 35M 约 2-5 分钟),生成后重复执行会跳过已存在文件
#   - rsync 增量同步,已传文件秒过
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -d "$SCRIPT_DIR/examples/minimal/dist" ]; then
  DIST="$SCRIPT_DIR/examples/minimal/dist"
else
  DIST="$(cd "$SCRIPT_DIR/.." && pwd)/examples/minimal/dist"
fi
REMOTE_DIR="${3:-/var/www/suishen}"

# 需预压缩的扩展名(ogg/ktx2 本身已压缩,跳过)
EXT_PATTERNS=(
  "*.wasm" "*.js" "*.ron" "*.otf" "*.png" "*.jpg" "*.webp" "*.svg" "*.json" "*.list"
)

compress_br() {
  while IFS= read -r -d '' f; do
    if [ ! -f "$f.br" ]; then
      echo "[br] ${f#$DIST/}"
      brotli -q 11 -k -f "$f" || echo "  !! brotli 失败: $f"
    fi
  done
}

compress_gz() {
  while IFS= read -r -d '' f; do
    if [ ! -f "$f.gz" ]; then
      echo "[gz] ${f#$DIST/}"
      gzip -9 -k -f "$f" || echo "  !! gzip 失败: $f"
    fi
  done
}

echo "== 生成 .br 预压缩文件 =="
for pat in "${EXT_PATTERNS[@]}"; do
  find "$DIST" -type f -name "$pat" -not -name "*.br" -not -name "*.gz" -print0 | compress_br
done

echo "== 生成 .gz 兜底(仅对 >1MB 的大文件,避免小文件冗余)="
find "$DIST" -type f -size +1M \
  \( -name "*.wasm" -o -name "*.otf" -o -name "*.ron" \) \
  -not -name "*.br" -not -name "*.gz" -print0 | compress_gz

echo "== prep 完成 =="

if [ "${1:-}" = "deploy" ] && [ -n "${2:-}" ]; then
  echo "== rsync 推送到 ${2}:${REMOTE_DIR} =="
  rsync -avz --delete --exclude="*.orig.otf*" -e ssh "$DIST/" "$2:$REMOTE_DIR/"
  echo "== 部署完成 =="
fi
