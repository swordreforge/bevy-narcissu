#!/usr/bin/env python3
"""extract-doc.py — 提取老式二进制 .doc(OLE Word 97-2003)的正文文本。

用途:仓库原版资源中散落的 WPS/Word 笔记(如 artemis脚本/1+2合集/n2.doc
= 作者的翻译修订笔记)无法直接查看,本工具从 WordDocument 流的 FIB 头
(fcMin/fcMac)定位正文区,按 UTF-16LE 解码并清理控制字符。

注意:仅支持 .doc(复合文档),不支持 .docx(zip 容器)。

依赖:olefile(缺失时提示安装,pip install --break-system-packages olefile)

用法:
  python3 tools/extract-doc.py 笔记.doc            # 输出到 stdout
  python3 tools/extract-doc.py 笔记.doc -o out.txt  # 写入文件
"""

from __future__ import annotations

import argparse
import re
import struct
import sys
from pathlib import Path

try:
    import olefile
except ImportError:
    sys.exit(
        "error: 缺少 olefile 库。安装: pip install --break-system-packages olefile"
    )

# FIB base 关键偏移
_OFFSET_FLAGS = 0x0A  # fWhichTblStm=0x200 fComplex=0x008 fCompressed=0x004
_OFFSET_FCMIN = 0x18
_OFFSET_FCMAC = 0x1C

# 保留 \r(段落分隔)与 \t;剔除其余 C0 控制字符
_CLEAN_RE = re.compile(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]")


def extract_doc(path: Path) -> str:
    """从 .doc 提取正文文本;无法定位正文时抛 ValueError。"""
    if not olefile.isOleFile(str(path)):
        raise ValueError(f"{path}: 不是有效的 OLE 复合文档(.doc)")

    ole = olefile.OleFileIO(str(path))
    try:
        if not ole.exists("WordDocument"):
            raise ValueError(f"{path}: 缺少 WordDocument 流")
        wd = ole.openstream("WordDocument").read()
    finally:
        ole.close()

    if len(wd) < _OFFSET_FCMAC + 4:
        raise ValueError(f"{path}: WordDocument 流过短({len(wd)} 字节)")

    fc_min, fc_mac = struct.unpack("<II", wd[_OFFSET_FCMIN : _OFFSET_FCMAC + 4])
    if not (0 <= fc_min < fc_mac <= len(wd)):
        raise ValueError(f"{path}: 非法正文区 fcMin={fc_min} fcMac={fc_mac}")

    text = wd[fc_min:fc_mac].decode("utf-16-le", errors="replace")
    text = _CLEAN_RE.sub("", text)
    # Word 以 \r 分隔段落;Windows 行尾 \r\n 归一为 \n
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    # 折叠空行(去除段落之间的多余空行)
    text = re.sub(r"\n{3,}", "\n\n", text).strip()
    return text


def main() -> None:
    ap = argparse.ArgumentParser(description="提取 .doc(OLE Word)正文文本")
    ap.add_argument("file", type=Path, help=".doc 文件路径")
    ap.add_argument("-o", "--output", type=Path, help="输出文件(默认 stdout)")
    args = ap.parse_args()

    try:
        text = extract_doc(args.file)
    except (ValueError, OSError) as e:
        sys.exit(f"error: {e}")

    if args.output:
        args.output.write_text(text + "\n", encoding="utf-8")
        print(f"written: {args.output}", file=sys.stderr)
    else:
        print(text)


if __name__ == "__main__":
    main()
