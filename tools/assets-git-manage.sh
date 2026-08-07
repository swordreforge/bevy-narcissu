#!/usr/bin/env bash
# =============================================================================
# assets-git-manage.sh — 管理已跟踪资产在 git status 中的可见性
#
# 背景: examples/minimal/assets/ 全部资产(439MB)已提交进 git。
#       其中 image/ (1231 个 ETC1S KTX2) 是频繁重转的生成物,本地微调后
#       会在 `git status` 刷出上千行 modified。本脚本用
#       `git update-index --skip-worktree` 把这些文件标记为"本地忽略",
#       让 status 保持干净,同时保留 git 内的版本(新 clone 仍可拉取)。
#
# 用法:
#   ./tools/assets-git-manage.sh            # 显示帮助
#   ./tools/assets-git-manage.sh mark       # 标记 image/ 为 skip-worktree(过滤 status)
#   ./tools/assets-git-manage.sh unmark     # 解除标记(例如要 pull 资产更新前)
#   ./tools/assets-git-manage.sh status     # 查看当前哪些文件被标记
#   ./tools/assets-git-manage.sh pull       # 解除标记 → git pull → 重新标记
# =============================================================================
set -euo pipefail

# 仓库根目录(脚本位于 <root>/tools/ 下)
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# 要过滤的目录(相对仓库根)。可加更多目录,每个一行。
FILTER_DIRS=(
    "examples/minimal/assets/image"
)

# 把所有 filter 目录下的已跟踪文件打成一行(空格分隔)
collect_files() {
    local files=()
    for dir in "${FILTER_DIRS[@]}"; do
        while IFS= read -r f; do
            files+=("$f")
        done < <(git ls-files "$dir")
    done
    printf '%s\n' "${files[@]}"
}

cmd_mark() {
    local n=0
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        git update-index --skip-worktree "$f"
        n=$((n + 1))
    done < <(collect_files)
    echo "marked $n file(s) as skip-worktree"
}

cmd_unmark() {
    local n=0
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        git update-index --no-skip-worktree "$f"
        n=$((n + 1))
    done < <(collect_files)
    echo "unmarked $n file(s)"
}

cmd_status() {
    echo "skip-worktree 标记的文件(前缀 S):"
    git ls-files -v "${FILTER_DIRS[@]}" | grep '^S' || echo "(无)"
    echo
    echo "已跟踪但未标记的文件(前缀 H):"
    git ls-files -v "${FILTER_DIRS[@]}" | grep '^H' | head -5
    echo "(其余 H 文件省略)"
}

cmd_pull() {
    cmd_unmark
    git pull "$@"
    cmd_mark
}

usage() {
    # 只打印头部的帮助注释块(第一个 # ==== 到第二个 # ==== 之间)
    sed -n '/^# ===/,/^# ===/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

case "${1:-}" in
    mark)   cmd_mark ;;
    unmark) cmd_unmark ;;
    status) cmd_status ;;
    pull)   shift; cmd_pull "$@" ;;
    *)      usage ;;
esac
