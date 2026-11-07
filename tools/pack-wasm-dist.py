#!/usr/bin/env python3
"""Pack a release wasm build into examples/minimal/dist.

Pipeline (docs/WASM_PUBLISH_PLAN.md §2.5):

    cargo build --release --target wasm32-unknown-unknown -p bevy-vn-example
    python3 tools/pack-wasm-dist.py            # this script
    ./prep.sh                                  # optional: --prep

What it does:

1. `wasm-bindgen --target web --no-typescript` into a temp dir.
2. `wasm-opt -O4` with the feature flags the Bevy/wgpu wasm output needs.
3. Content-hash the optimized wasm and rename the JS/wasm pair to
   `bevy-vn-example-<hash>.js` / `bevy-vn-example-<hash>_bg.wasm`.
4. Copy `assets/`, `index.html`, `sw.js`, `cache-panel.js` and the
   wasm-bindgen `snippets/` into `dist/`.
5. Patch the hashed filenames in `index.html` and `sw.js`, recompute every
   `integrity="sha384-..."` from the files actually written, and give the
   service worker a content-derived `CORE_CACHE` name so a new build cannot
   serve an old cached index.html.
6. Optionally run `prep.sh` to regenerate `.br`/`.gz`.

The source `examples/minimal/index.html` keeps its old hashes as placeholders;
only the copy under `dist/` is rewritten.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_WASM = ROOT / "target/wasm32-unknown-unknown/release/bevy-vn-example.wasm"
DEFAULT_SRC = ROOT / "examples/minimal"
DEFAULT_OUT = DEFAULT_SRC / "dist"

HASH_RE = re.compile(r"bevy-vn-example-[0-9a-f]{16}")
LINK_RE = re.compile(r"<link\b[^>]*>")


def sri(path: Path) -> str:
    digest = hashlib.sha384(path.read_bytes()).digest()
    return "sha384-" + base64.b64encode(digest).decode()


def run(cmd: list[str]) -> None:
    print("+", " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True)


def patch_integrity(html: str, out_dir: Path) -> str:
    """Recompute SRI for every <link> whose href points at a file in dist."""

    def repl(match: re.Match[str]) -> str:
        tag = match.group(0)
        href_m = re.search(r'href="([^"]+)"', tag)
        if not href_m or "integrity=" not in tag:
            return tag
        href = href_m.group(1)
        if not href.startswith("./"):
            return tag
        target = out_dir / href[2:]
        if not target.is_file():
            return tag
        return re.sub(r'integrity="[^"]*"', f'integrity="{sri(target)}"', tag)

    return LINK_RE.sub(repl, html)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--wasm", type=Path, default=DEFAULT_WASM)
    ap.add_argument("--src", type=Path, default=DEFAULT_SRC)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--crate-name", default="bevy-vn-example")
    ap.add_argument("--wasm-bindgen", default=str(Path.home() / ".cargo/bin/wasm-bindgen"))
    ap.add_argument("--wasm-opt", default="/usr/bin/wasm-opt")
    ap.add_argument("--skip-assets", action="store_true", help="reuse dist/assets")
    ap.add_argument("--no-opt", action="store_true", help="skip wasm-opt (debug)")
    ap.add_argument("--prep", action="store_true", help="run prep.sh afterwards")
    args = ap.parse_args()

    if not args.wasm.is_file():
        print(f"wasm not found: {args.wasm}", file=sys.stderr)
        print("build it first: cargo build --release --target wasm32-unknown-unknown -p bevy-vn-example", file=sys.stderr)
        return 2
    for tool in (args.wasm_bindgen, args.wasm_opt):
        if not shutil.which(tool) and not Path(tool).is_file():
            print(f"tool not found: {tool}", file=sys.stderr)
            return 2

    out: Path = args.out
    out.mkdir(parents=True, exist_ok=True)
    tmp_parent = Path("/tmp/opencode")
    tmp_parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="wbgen-", dir=tmp_parent) as td:
        td = Path(td)

        run([
            args.wasm_bindgen,
            str(args.wasm),
            "--target", "web",
            "--out-dir", str(td),
            "--no-typescript",
        ])

        bindgen_js = td / f"{args.crate_name}.js"
        bindgen_wasm = td / f"{args.crate_name}_bg.wasm"
        if not bindgen_js.is_file() or not bindgen_wasm.is_file():
            print(f"wasm-bindgen output missing in {td}", file=sys.stderr)
            return 2

        if args.no_opt:
            packed_wasm = bindgen_wasm
        else:
            packed_wasm = td / "opt.wasm"
            run([
                args.wasm_opt,
                str(bindgen_wasm),
                "-O4",
                "-o", str(packed_wasm),
                "--enable-bulk-memory",
                "--enable-mutable-globals",
                "--enable-nontrapping-float-to-int",
                "--enable-sign-ext",
                "--enable-reference-types",
            ])

        digest = hashlib.sha256(packed_wasm.read_bytes()).hexdigest()
        short = digest[:16]
        js_name = f"bevy-vn-example-{short}.js"
        wasm_name = f"bevy-vn-example-{short}_bg.wasm"

        # Static shell files (source index.html is treated as a template).
        shutil.copy2(args.src / "index.html", out / "index.html")
        shutil.copy2(args.src / "sw.js", out / "sw.js")
        shutil.copy2(args.src / "cache-panel.js", out / "cache-panel.js")
        for optional in ("manifest.webmanifest", "icon-192.png", "icon-512.png", "favicon.ico"):
            src = args.src / optional
            if src.is_file():
                shutil.copy2(src, out / optional)
            else:
                print(f"warning: {src} missing (PWA install will be incomplete)")

        licenses_src = args.src / "licenses"
        if licenses_src.is_dir():
            shutil.copytree(licenses_src, out / "licenses", dirs_exist_ok=True)

        if not args.skip_assets:
            assets_dst = out / "assets"
            if assets_dst.exists():
                shutil.rmtree(assets_dst)
            print(f"+ copy {args.src / 'assets'} -> {assets_dst}", flush=True)
            shutil.copytree(args.src / "assets", assets_dst)

        # wasm-bindgen snippets (basisu inline JS glue).
        snippets_src = td / "snippets"
        if snippets_src.is_dir():
            snippets_dst = out / "snippets"
            if snippets_dst.exists():
                shutil.rmtree(snippets_dst)
            shutil.copytree(snippets_src, snippets_dst)
        snippet_rel = None
        inline = sorted(snippets_src.glob("*/inline0.js")) if snippets_src.is_dir() else []
        if inline:
            snippet_rel = inline[0].relative_to(td).as_posix()

        shutil.copy2(bindgen_js, out / js_name)
        shutil.copy2(packed_wasm, out / wasm_name)

        # index.html: rename assets and replace all SRI values.
        html = (out / "index.html").read_text(encoding="utf-8")
        html = HASH_RE.sub(f"bevy-vn-example-{short}", html)
        if snippet_rel:
            html = re.sub(r"snippets/[^\"']+/inline0\.js", snippet_rel, html)
        html = patch_integrity(html, out)
        (out / "index.html").write_text(html, encoding="utf-8")

        # sw.js: the core JS and the separately prefetched wasm URL.
        sw = (out / "sw.js").read_text(encoding="utf-8")
        sw = HASH_RE.sub(f"bevy-vn-example-{short}", sw)
        # Derive the cache name from everything that determines the shell, not
        # just the wasm: an index.html-only change (e.g. the license notice)
        # must also invalidate the cached index.
        shell_id = hashlib.sha256(
            (out / "index.html").read_bytes()
            + (out / js_name).read_bytes()
            + packed_wasm.read_bytes()
        ).hexdigest()[:12]
        sw = re.sub(
            r"const CORE_CACHE = '[^']+';",
            f"const CORE_CACHE = 'core-{shell_id}';",
            sw,
        )
        (out / "sw.js").write_text(sw, encoding="utf-8")

        print()
        print(f"dist:   {out}")
        print(f"js:     {js_name}  ({ (out / js_name).stat().st_size / 1024 / 1024:.1f} MiB)")
        print(f"wasm:   {wasm_name}  ({ (out / wasm_name).stat().st_size / 1024 / 1024:.1f} MiB)")
        if snippet_rel:
            print(f"inline: {snippet_rel}")

    if args.prep:
        # prep.sh skips existing .br/.gz, so drop stale top-level ones first.
        for f in out.glob("*.br"):
            f.unlink()
        for f in out.glob("*.gz"):
            f.unlink()
        run([str(ROOT / "prep.sh")])

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
