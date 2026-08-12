#!/usr/bin/env python3
"""gen-assets-manifest.py — 生成 SW 预取用的全量资源清单。

扫描 assets/ 目录下所有文件,输出 assets-manifest.json,供 Service Worker
的批量预取队列消费(SW 需要事先知道要 fetch 哪些文件、多大、以什么优先级)。

输出格式(顶层数组,按 path 字典序,稳定排序供位图索引使用):
    [
      {"path": "audio/voice/li/4syuji_001.ogg", "size": 29417, "priority": 0},
      ...
    ]

priority 取值 0-3,由文件大小(小文件优先,快速建立缓存命中率)与
目录权重(剧情推进最依赖的 voice/scripts 优先)综合决定:
  - voice/scripts/fonts: 小文件多、剧情高频依赖 -> 高优先级
  - image/ui/pa:         中
  - audio/bgm/audio/se:  较大、按需 -> 低
  - 剩余:                最低

用法:
  python3 tools/gen-assets-manifest.py                                # 输出到 examples/minimal/assets/assets-manifest.json
  python3 tools/gen-assets-manifest.py --assets PATH --output out.json
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path

# 目录前缀 -> 优先级权重(0-3)。顺序即覆盖顺序,先匹配先生效。
# 数值越大越优先。默认值 0。
PRIORITY_RULES: list[tuple[str, int]] = [
    ("audio/voice/", 3),
    ("scripts/", 3),
    ("fonts/", 2),
    ("audio/se/", 2),
    ("image/", 1),
    ("ui/", 1),
    ("pa/", 1),
    ("audio/bgm/", 1),
]


def priority_for(rel_path: str, size: int) -> int:
    # 目录权重
    pri = 0
    for prefix, weight in PRIORITY_RULES:
        if rel_path.startswith(prefix):
            pri = weight
            break
    # 小文件(<=64KB)升一级:海量小文件先落地,命中率提升最快
    if size <= 64 * 1024:
        pri = min(pri + 1, 3)
    return pri


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--assets",
        default="examples/minimal/assets",
        help="资产根目录(默认 examples/minimal/assets)",
    )
    parser.add_argument(
        "--output",
        default="examples/minimal/assets/assets-manifest.json",
        help="清单输出路径(默认 examples/minimal/assets/assets-manifest.json)",
    )
    args = parser.parse_args()

    root = Path(args.assets)
    if not root.is_dir():
        parser.error(f"资产目录不存在: {root}")

    entries: list[dict] = []
    for f in sorted(root.rglob("*")):
        if not f.is_file():
            continue
        rel = f.relative_to(root).as_posix()
        # 清单自身不进入清单(部署时 SW 会单独拉取清单文件)
        if rel == "assets-manifest.json":
            continue
        size = f.stat().st_size
        entries.append(
            {
                "path": rel,
                "size": size,
                "priority": priority_for(rel, size),
            }
        )

    # 稳定排序:path 字典序(位图按此顺序索引,SW 侧不再重排)
    entries.sort(key=lambda e: e["path"])

    manifest = {
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "assets_root": str(root),
        "count": len(entries),
        "total_bytes": sum(e["size"] for e in entries),
        "files": entries,
    }

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, ensure_ascii=False), encoding="utf-8")

    total_mb = manifest["total_bytes"] / 1024 / 1024
    print(
        f"已生成 {out} : {len(entries)} 个文件, "
        f"{total_mb:.1f} MB"
    )
    # 优先级分布摘要
    from collections import Counter

    dist = Counter(e["priority"] for e in entries)
    print("优先级分布:", dict(sorted(dist.items())))


if __name__ == "__main__":
    main()
