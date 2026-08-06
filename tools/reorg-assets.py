#!/usr/bin/env python3
"""重组《水仙10周年》游戏资源到 bevy-vn-engine 的 assets 目录结构。

引用来源: /tmp/opencode/vn_all/*.vnscript.ron (已转换的脚本, 排除 pack)
原始资源: /home/swordreforge/下载/水仙10周年版本_1.2.0/assets (游戏数据)
输出目标: examples/minimal/assets/ (引擎路径约定)

引擎路径约定:
  SetBg  image          -> image/bg/{}.png        (AssetPathProvider.bg)
  Sprite image          -> image/anime/{}.png     (AssetPathProvider.sprite)
  ShowCg image          -> image/ev/{}.png        (AssetPathProvider.cg, 数据缺失)
  PlayBgm id            -> audio/bgm/{}.ogg       (bgm.rs, id 已去 bgm 前缀)
  PlaySe  file          -> audio/se/{}.ogg        (se.rs)
  Dialogue voice        -> audio/voice/{}.ogg     (voice.rs, file 形如 li/n002)

缺失资源容错跳过, 输出缺失清单。
"""
import re
import shutil
import sys
from pathlib import Path

GAME = Path("/home/swordreforge/下载/水仙10周年版本_1.2.0/assets")
RON_DIR = Path("/tmp/opencode/vn_all")
OUT = Path(__file__).resolve().parent.parent / "examples" / "minimal" / "assets"
SKIP = {"pack.vnscript.ron"}  # 宣传脚本: cg_ts*/song*/z9* 资源全缺失

missing: dict[str, list[str]] = {}
copied: dict[str, list[str]] = {}


def build_index(directory: Path) -> dict[str, Path]:
    """小写文件名 -> 实际路径索引 (处理大小写不一致, 如 0SNsumi vs 0snsumi)."""
    idx: dict[str, Path] = {}
    if not directory.exists():
        return idx
    for p in directory.rglob("*"):
        if p.is_file():
            idx[p.name.lower()] = p
    return idx


def copy_one(cat: str, src: Path | None, dst: Path):
    dst.parent.mkdir(parents=True, exist_ok=True)
    if src is None or not src.exists():
        missing.setdefault(cat, []).append(dst.name)
        return
    if not dst.exists():
        shutil.copy2(src, dst)
    copied.setdefault(cat, []).append(dst.name)


def collect_refs() -> tuple[set[str], set[str], set[str], set[str], set[str]]:
    """从 .ron 提取各类型引用: bg, anime(fg), bgm, se, vo."""
    bg, anime, bgm, se, vo = set(), set(), set(), set(), set()
    for ron in sorted(RON_DIR.glob("*.ron")):
        if ron.name in SKIP:
            continue
        text = ron.read_text(encoding="utf-8")
        for m in re.finditer(r"cmd: set_bg,.*?args: \(\s*image: \"([^\"]+)\"", text, re.S):
            bg.add(m.group(1))
        for m in re.finditer(r"cmd: sprite,.*?args: \(\s*id: \"[^\"]+\",\s*image: \"([^\"]+)\"", text, re.S):
            anime.add(m.group(1))
        for m in re.finditer(r"cmd: play_bgm,.*?args: \(\s*id: \"([^\"]+)\"", text, re.S):
            bgm.add(m.group(1))
        for m in re.finditer(r"cmd: play_se,.*?args: \(\s*file: \"([^\"]+)\"", text, re.S):
            se.add(m.group(1))
        for m in re.finditer(r"voice: Some\(\"([^\"]+)\"\)", text):
            vo.add(m.group(1))
    return bg, anime, bgm, se, vo


def main():
    print(f"输出目录: {OUT}")
    bg, anime, bgm, se, vo = collect_refs()
    print(f"引用统计: bg={len(bg)} anime={len(anime)} bgm={len(bgm)} se={len(se)} vo={len(vo)}")

    # ── 1. BG: image/hd/bg/ -> image/bg/
    bg_idx = build_index(GAME / "image" / "hd" / "bg")
    for name in sorted(bg):
        src = bg_idx.get((name + ".png").lower())
        copy_one("bg", src, OUT / "image" / "bg" / f"{name}.png")

    # ── 2. 立绘 (Sprite): image/hd/fg/ -> image/anime/
    fg_idx = build_index(GAME / "image" / "hd" / "fg")
    for name in sorted(anime):
        src = fg_idx.get((name + ".png").lower())
        copy_one("anime", src, OUT / "image" / "anime" / f"{name}.png")

    # ── 3. BGM: other-list.txt 映射 bgm{id} -> 文件名, 源 sound/bgm/other/
    other_list = {}
    if (GAME / "sound" / "bgm" / "other-list.txt").exists():
        for line in (GAME / "sound" / "bgm" / "other-list.txt").read_text(encoding="utf-8").splitlines():
            m = re.match(r"bgm(\d+)\s*->\s*(\S+)", line)
            if m:
                other_list[m.group(1)] = m.group(2)
    bgm_idx = build_index(GAME / "sound" / "bgm" / "other")
    for bgm_id in sorted(bgm, key=lambda x: int(x)):
        fname = other_list.get(bgm_id)
        if not fname:
            missing.setdefault("bgm", []).append(f"{bgm_id} (no other-list entry)")
            continue
        src = bgm_idx.get((fname + ".ogg").lower())
        copy_one("bgm", src, OUT / "audio" / "bgm" / f"{bgm_id}.ogg")

    # ── 4. SE: sound/se/ha/ (extra/ 子目录) -> audio/se/; voice/ 前缀源在语音目录;
    #         裸名先查 SE 目录, 未命中回退语音目录 (游戏通过 SE 通道播放部分语音);
    #         引用带 .wav 扩展名时按 .ogg 实际文件解析 (n4taore.wav -> n4taore.ogg)
    se_root = build_index(GAME / "sound" / "se" / "ha")
    vo_root = build_index(GAME / "sound" / "vo" / "ha")
    for name in sorted(se):
        lower = name.lower()
        if lower.startswith("extra/"):
            src = se_root.get((name.split("/", 1)[1] + ".ogg").lower())
            copy_one("se", src, OUT / "audio" / "se" / f"{name}.ogg")
        elif lower.startswith("voice/"):
            base = name.split("/", 1)[1]
            src = vo_root.get((base + ".ogg").lower())
            copy_one("se", src, OUT / "audio" / "se" / f"{name}.ogg")
        else:
            base = name[:-4] if lower.endswith((".wav", ".ogg")) else name
            src = se_root.get((base + ".ogg").lower())
            if src is None:
                src = vo_root.get((base + ".ogg").lower())
            copy_one("se", src, OUT / "audio" / "se" / f"{name}.ogg")

    # ── 5. VO: sound/vo/ha/li/{base}.ogg -> audio/voice/{file}.ogg (file 形如 li/n002)
    for name in sorted(vo):
        base = name.rsplit("/", 1)[-1]
        src = vo_root.get((base + ".ogg").lower())
        copy_one("vo", src, OUT / "audio" / "voice" / f"{name}.ogg")

    # ── 6. 字体 (引擎 UI 后续使用)
    font_src = GAME / "font" / "font-2.otf"
    if font_src.exists():
        (OUT / "fonts").mkdir(parents=True, exist_ok=True)
        shutil.copy2(font_src, OUT / "fonts" / "font-2.otf")

    # ── 汇总
    print("\n=== 复制统计 ===")
    for cat in ("bg", "anime", "bgm", "se", "vo"):
        print(f"  {cat:8s}: 引用={len({'bg': bg, 'anime': anime, 'bgm': bgm, 'se': se, 'vo': vo}[cat])} "
              f"复制={len(copied.get(cat, []))} 缺失={len(missing.get(cat, []))}")
    print("\n=== 缺失清单 ===")
    for cat in ("bg", "anime", "bgm", "se", "vo"):
        miss = missing.get(cat, [])
        if miss:
            print(f"  [{cat}] {len(miss)} 个:")
            for x in miss:
                print(f"    - {x}")
    if not any(missing.values()):
        print("  (无缺失)")

    total_copied = sum(len(v) for v in copied.values())
    total_missing = sum(len(v) for v in missing.values())
    print(f"\n总计: 复制 {total_copied} 个, 缺失 {total_missing} 个")
    return 0 if total_missing == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
