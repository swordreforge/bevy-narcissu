#!/usr/bin/env python3
"""asset-audit.py — 静态比对脚本引用的资源与磁盘上的实际文件。

扫描 examples/minimal/assets/scripts/*.vnscript.ron,按引擎的真实路径映射规则
(见 crates/bevy-vn-render/src/lib.rs 的 AssetPathProvider 与
 crates/bevy-vn-audio/src/channel.rs 的 audio_channel_impl! 宏)解析每个资源引用,
与磁盘文件比对,输出缺失清单。每条缺失项附带:
  - 引用位置(脚本文件、行号、最近 label)
  - 上下文台词(引用点前后的 dialogue 文本摘录),方便对照原文补资源
  - 相似资源提示(优先同 token 前缀 / 共享 token 的近似文件名),供开发者参考
    是否已有可用素材或应修正脚本引用名。

引擎映射规则(零规范化,名字原样作为文件名 stem):
  set_bg / unlock?       image          -> image/bg/{image}.basisu.ktx2
  sprite                 image          -> image/anime/{image}.basisu.ktx2
  show_cg / unlock_cg    image          -> image/ev/{image}.basisu.ktx2
  show_fg / show_face    char_id+expr   -> image/obj/{char_id}/{expression}.basisu.ktx2
  play_bgm / unlock_bgm  id             -> audio/bgm/{id}.opus
  play_se                file           -> audio/se/{file}.opus
  play_voice / dialogue  voice(file)    -> audio/voice/{file}.opus
音频 file 字段可含子目录(如 "extra/n001" / "li/4syuji_009")。

与引擎行为保持一致:跳过 pack.vnscript.ron(引擎运行时也跳过,见
examples/minimal/src/main.rs);screen_effect 的 "black"/"white"/"1"/"2" 是纯色,
不是文件引用,不参与比对。

用法:
  python3 tools/asset-audit.py                      # 默认扫描 examples/minimal/assets
  python3 tools/asset-audit.py --assets PATH        # 指定资产根目录
  python3 tools/asset-audit.py --output out.json    # 指定导出路径(默认 ./asset-audit-report.json)
  python3 tools/asset-audit.py --min-score 0.5      # 相似提示最低分数
  python3 tools/asset-audit.py --top-k 3            # 每条缺失最多提示的相似候选数
输出:JSON(缺失清单 + 相似提示 + 统计),stderr 打印摘要。
"""

from __future__ import annotations

import argparse
import difflib
import json
import re
import sys
import unicodedata
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------- 常量

SKIPPED_SCRIPTS = {"pack.vnscript.ron"}  # 引擎运行时显式跳过(宣传脚本,资源几乎全缺)

# 引擎内置哨兵值:名字指向"无资源"而非真实文件,不参与缺失比对。
# "nobaby" = artemis 原版引擎的"清除立绘图层"哨兵(原版脚本中紧随"立绘代码-消失"
# 注释、path=":fg/");bevy-vn-render/src/sprite.rs 已特判跳过加载。
SENTINEL_NAMES: dict[str, str] = {
    "nobaby": "清除立绘图层哨兵(artemis 原版,非真实文件)",
}

# 脚本指令 -> (资源类型, 需要的字段)。字段值取自 args。
REFERENCE_CMDS: dict[str, tuple[str, tuple[str, ...]]] = {
    "set_bg":       ("bg",    ("image",)),
    "sprite":       ("sprite", ("image",)),
    "show_cg":      ("cg",    ("image",)),
    "unlock_cg":    ("cg",    ("image",)),
    "show_fg":      ("fg",    ("char_id", "expression")),
    "show_face":    ("fg",    ("char_id", "expression")),
    "play_bgm":     ("bgm",   ("id",)),
    "unlock_bgm":   ("bgm",   ("id",)),
    "play_se":      ("se",    ("file",)),
    "play_voice":   ("voice", ("file",)),
    "dialogue":     ("voice", ("voice",)),  # 仅 voice: Some("...") 时算引用
}

# 资源类型 -> (期望目录(相对 assets), 扩展名, 名称是否可含子目录)
TYPE_DIRS: dict[str, tuple[str, str, bool]] = {
    "bg":     ("image/bg",     ".basisu.ktx2", False),
    "sprite": ("image/anime",  ".basisu.ktx2", False),
    "cg":     ("image/ev",     ".basisu.ktx2", False),
    "fg":     ("image/obj",    ".basisu.ktx2", False),
    "bgm":    ("audio/bgm",    ".opus",        False),
    "se":     ("audio/se",     ".opus",        True),
    "voice":  ("audio/voice",  ".opus",        True),
}

# 相似候选的搜索范围:该类型的缺失名会到这些目录树里找近似文件。
# 故意放宽(如 bg 缺失也搜 image/anime),因为水仙脚本存在引用名实际躺在
# 其它 image 子目录的情况(如 set_bg 引用 akar_05,文件在 image/anime/)。
SIMILAR_ROOTS: dict[str, list[str]] = {
    "bg":     ["image"],
    "sprite": ["image"],
    "cg":     ["image"],
    "fg":     ["image"],
    "bgm":    ["audio"],
    "se":     ["audio"],
    "voice":  ["audio"],
}

CONTEXT_LOOKBACK = 10      # 台词上下文:向前/向后最多扫多少条指令
CONTEXT_LINES = 2          # 前后各取几条台词
TEXT_DISPLAY_MAX = 80      # 台词摘录截断长度
DEFAULT_MIN_SCORE = 0.45   # 相似提示最低综合分
DEFAULT_TOP_K = 5          # 每条缺失最多几个相似候选

# ---------------------------------------------------------------- RON 解析

_FIELD_RE = re.compile(r"^\s*([A-Za-z_]\w*):\s*(.*?),\s*$")
_STRING_RE = re.compile(r'(?:Some\()?"((?:[^"\\]|\\.)*)"')


def _unescape_ron(s: str) -> str:
    """把 RON 字符串转义还原为可读文本(处理 \\u{XXXX}、\\n、\\"、\\\\)。"""

    def u_repl(m: re.Match[str]) -> str:
        return chr(int(m.group(1), 16))

    s = re.sub(r"\\u\{([0-9a-fA-F]+)\}", u_repl, s)
    s = s.replace("\\n", "\n").replace('\\"', '"').replace("\\\\", "\\")
    return s


def _clip(text: str, limit: int = TEXT_DISPLAY_MAX) -> str:
    text = text.strip()
    if len(text) > limit:
        return text[: limit - 1] + "…"
    return text


class Instruction:
    __slots__ = ("line", "cmd", "fields", "label")

    def __init__(self, line: int, cmd: str, fields: dict[str, str | None], label: str | None):
        self.line = line
        self.cmd = cmd
        self.fields = fields
        self.label = label


def parse_script(path: Path) -> list[Instruction]:
    """把 .vnscript.ron 解析为指令列表(仅 instructions: [...] 段)。"""
    text = path.read_text(encoding="utf-8")
    instrs: list[Instruction] = []
    current_label: str | None = None
    block: list[tuple[int, str]] | None = None
    depth = 0
    in_instructions = False

    for line_no, raw in enumerate(text.splitlines(), 1):
        if not in_instructions:
            if "instructions: [" in raw:
                in_instructions = True
            continue
        if block is None:
            if "(" not in raw:
                continue
            block = []
            depth = 0
        block.append((line_no, raw))
        depth += raw.count("(") - raw.count(")")
        if depth <= 0:
            current_label = _finish_block(block, instrs, current_label)
            block = None
    return instrs


def _finish_block(
    block: list[tuple[int, str]], instrs: list[Instruction], label: str | None
) -> str | None:
    fields: dict[str, str | None] = {}
    cmd: str | None = None
    for line_no, raw in block:
        m = _FIELD_RE.match(raw)
        if not m:
            continue
        key, val = m.group(1), m.group(2)
        if key == "cmd":
            cmd = val
            continue
        if key in ("name",):
            # label / save_point 的 name 也是字符串,统一走字符串提取
            sm = _STRING_RE.search(val)
            fields[key] = _unescape_ron(sm.group(1)) if sm else None
            continue
        sm = _STRING_RE.search(val)
        if sm:
            fields[key] = _unescape_ron(sm.group(1))
        elif val == "None":
            fields[key] = None
        # 其它(数字、跨行复合值)不关心
    if cmd is not None:
        line = block[0][0]
        if cmd == "label" and fields.get("name"):
            label = fields["name"]
        instrs.append(Instruction(line, cmd, fields, label))
    return label


# ---------------------------------------------------------------- 引用提取

def iter_references(instrs: list[Instruction], script_name: str):
    """产出 (type, name, ref) 。ref 含位置与上下文台词。"""
    # 台词上下文:为每个引用点预计算前后台词。
    n = len(instrs)
    for i, ins in enumerate(instrs):
        spec = REFERENCE_CMDS.get(ins.cmd)
        if spec is None:
            continue
        rtype, fields_needed = spec
        if ins.cmd == "dialogue":
            if not ins.fields.get("voice"):
                continue
            name = ins.fields["voice"]
        else:
            vals = [ins.fields.get(f) for f in fields_needed]
            if any(v is None for v in vals):
                continue
            name = "/".join(vals)  # type: ignore[arg-type]
        before = _nearby_dialogue(instrs, i, -1)
        after = _nearby_dialogue(instrs, i, +1)
        yield rtype, name, {
            "script": script_name,
            "line": ins.line,
            "label": ins.label,
            "context_before": before,
            "context_after": after,
        }


def _nearby_dialogue(instrs: list[Instruction], idx: int, direction: int) -> list[str]:
    texts: list[str] = []
    cursor = idx + direction
    scanned = 0
    while 0 <= cursor < len(instrs) and len(texts) < CONTEXT_LINES and scanned < CONTEXT_LOOKBACK:
        ins = instrs[cursor]
        if ins.cmd == "dialogue":
            t = ins.fields.get("text")
            if t:
                texts.append(_clip(t))
        cursor += direction
        scanned += 1
    return texts if direction > 0 else list(reversed(texts))


# ---------------------------------------------------------------- 磁盘索引

class DiskIndex:
    """预构建各目录的文件 stem 集合与全树候选(用于相似提示)。"""

    def __init__(self, assets_root: Path):
        self.root = assets_root
        self.dir_files: dict[str, set[str]] = {}   # 类型 -> 相对 stem 集合(可含子目录)
        self.dir_exists: dict[str, bool] = {}
        self.tree_files: dict[str, list[tuple[str, str]]] = {}  # 根 -> [(stem, relpath)]
        for rtype, (rel_dir, ext, nested) in TYPE_DIRS.items():
            d = assets_root / rel_dir
            exists = d.is_dir()
            self.dir_exists[rtype] = exists
            names: set[str] = set()
            if exists:
                for p in d.rglob(f"*{ext}") if nested else d.glob(f"*{ext}"):
                    if p.is_file():
                        names.add(p.relative_to(d).as_posix()[: -len(ext)])
            self.dir_files[rtype] = names
        for root in sorted({r for rs in SIMILAR_ROOTS.values() for r in rs}):
            entries: list[tuple[str, str]] = []
            base = assets_root / root
            if base.is_dir():
                for p in base.rglob("*"):
                    if p.is_file():
                        entries.append((p.stem, p.relative_to(assets_root).as_posix()))
            self.tree_files[root] = entries

    def missing_in(self, rtype: str, name: str) -> bool:
        return name not in self.dir_files.get(rtype, set())


# ---------------------------------------------------------------- 相似提示

def _tokenize(name: str) -> list[str]:
    return [t.lower() for t in re.findall(r"[a-z]+|\d+", name, re.I)]


def _similarity(name: str, cand_name: str) -> tuple[float, str]:
    """返回 (综合分, 匹配类型)。优先同 token 前缀,其次共享 token,最后字符相似。"""
    if name == cand_name:
        return 1.0, "same-name"
    a, b = _tokenize(name), _tokenize(cand_name)
    if a and b:
        prefix = sum(1 for x, y in zip(a, b) if x == y) / max(len(a), len(b))
        overlap = len(set(a) & set(b)) / max(len(a), len(b))
    else:
        prefix = overlap = 0.0
    chars = difflib.SequenceMatcher(None, name, cand_name).ratio()
    score = 0.5 * prefix + 0.3 * overlap + 0.2 * chars
    if prefix >= 0.4:
        kind = "prefix"
    elif overlap >= 0.4:
        kind = "token-overlap"
    else:
        kind = "edit"
    return score, kind


def find_similar(index: DiskIndex, rtype: str, name: str, top_k: int, min_score: float):
    name_toks = _tokenize(name)
    candidates: list[tuple[float, str, str]] = []
    for root in SIMILAR_ROOTS.get(rtype, []):
        for cand_stem, rel_path in index.tree_files.get(root, []):
            # 粗筛:同名(跨目录)、共享首 token、或字符相似度 >= 0.4
            if cand_stem == name:
                score, kind = 1.0, "same-name-different-dir"
                candidates.append((score, kind, rel_path))
                continue
            if not (name_toks and _tokenize(cand_stem) and name_toks[0] == _tokenize(cand_stem)[0]):
                if difflib.SequenceMatcher(None, name, cand_stem).ratio() < 0.4:
                    continue
            score, kind = _similarity(name, cand_stem)
            if score >= min_score:
                candidates.append((score, kind, rel_path))
    candidates.sort(key=lambda t: (-t[0], t[2]))
    return [
        {
            "path": rel,
            "name": re.sub(r"\.(?:basisu\.)?ktx2$|\.opus$", "", Path(rel).name),
            "score": round(score, 3),
            "match": kind,
        }
        for score, kind, rel in candidates[:top_k]
    ]


# ---------------------------------------------------------------- 主流程

def audit(assets_root: Path, output: Path, min_score: float, top_k: int) -> dict:
    scripts_dir = assets_root / "scripts"
    if not scripts_dir.is_dir():
        sys.exit(f"error: no scripts dir at {scripts_dir}")

    index = DiskIndex(assets_root)

    missing: dict[str, dict] = {}   # (type, name) -> 聚合条目
    counters: Counter[str] = Counter()
    ref_counts: Counter[str] = Counter()
    sentinel_hits: Counter[str] = Counter()  # 哨兵名 -> 引用次数

    for script_path in sorted(scripts_dir.glob("*.vnscript.ron")):
        if script_path.name in SKIPPED_SCRIPTS:
            continue
        instrs = parse_script(script_path)
        for rtype, name, ref in iter_references(instrs, script_path.name):
            ref_counts[rtype] += 1
            if name in SENTINEL_NAMES:
                sentinel_hits[name] += 1
                continue
            if not index.missing_in(rtype, name):
                continue
            counters[rtype] += 1
            key = (rtype, name)
            entry = missing.setdefault(
                key,
                {
                    "type": rtype,
                    "name": name,
                    "expected_path": _expected_path(rtype, name),
                    "reference_count": 0,
                    "references": [],
                    "similar": [],
                },
            )
            entry["reference_count"] += 1
            entry["references"].append(ref)

    # 相似提示(仅对缺失项)
    for key, entry in missing.items():
        rtype, name = key
        entry["similar"] = find_similar(index, rtype, name, top_k, min_score)

    missing_list = sorted(
        missing.values(),
        key=lambda e: (-e["reference_count"], e["type"], e["name"]),
    )

    missing_dirs = [
        {"dir": d, "type": t}
        for t, d in (("cg", "image/ev"), ("fg", "image/obj"))
        if not (assets_root / d).is_dir()
    ]

    by_type = {}
    for t in TYPE_DIRS:
        by_type[t] = {
            "references": ref_counts[t],
            "missing_names": counters[t],
            "dir_exists": index.dir_exists[t],
        }

    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "assets_root": str(assets_root),
        "skipped_scripts": sorted(SKIPPED_SCRIPTS),
        "sentinel_skipped": {
            name: {"references": n, "note": SENTINEL_NAMES[name]}
            for name, n in sorted(sentinel_hits.items())
        },
        "summary": {
            "scripts_scanned": len(
                [p for p in scripts_dir.glob("*.vnscript.ron") if p.name not in SKIPPED_SCRIPTS]
            ),
            "references_total": sum(ref_counts.values()),
            "missing_total": sum(counters.values()),
            "by_type": by_type,
        },
        "missing_dirs": missing_dirs,
        "missing": missing_list,
    }

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")

    print(f"scanned: {report['summary']['scripts_scanned']} scripts", file=sys.stderr)
    print(
        f"references: {report['summary']['references_total']}  "
        f"missing: {report['summary']['missing_total']}  "
        f"sentinel-skipped: {sum(sentinel_hits.values())}",
        file=sys.stderr,
    )
    for t in TYPE_DIRS:
        s = by_type[t]
        print(
            f"  {t:7s} refs={s['references']:6d}  missing={s['missing_names']:5d}  "
            f"dir={'ok' if s['dir_exists'] else 'MISSING'}",
            file=sys.stderr,
        )
    print(f"report written: {output}", file=sys.stderr)
    return report


def _expected_path(rtype: str, name: str) -> str:
    rel_dir, ext, _ = TYPE_DIRS[rtype]
    return f"{rel_dir}/{name}{ext}"


def main() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    default_assets = repo_root / "examples" / "minimal" / "assets"

    ap = argparse.ArgumentParser(description="比对 VN 脚本引用的资源与磁盘实际文件,输出缺失清单 JSON。")
    ap.add_argument("--assets", type=Path, default=default_assets, help="资产根目录(默认 examples/minimal/assets)")
    ap.add_argument("--output", type=Path, default=repo_root / "asset-audit-report.json", help="JSON 导出路径")
    ap.add_argument("--min-score", type=float, default=DEFAULT_MIN_SCORE, help="相似提示最低分数")
    ap.add_argument("--top-k", type=int, default=DEFAULT_TOP_K, help="每条缺失最多相似候选数")
    args = ap.parse_args()

    audit(args.assets, args.output, args.min_score, args.top_k)


if __name__ == "__main__":
    main()
