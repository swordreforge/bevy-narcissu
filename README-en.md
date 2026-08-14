# bevy-narcissu
[中文版](./README.md) | [English](./README-en.md)

A general-purpose visual novel engine built on **Bevy 0.19**, supporting script‑driven AVG games. Currently powering the *Narcissu 10th Anniversary* porting project.

## ⚠️ Important Notice / Legal Disclaimer

**This repository is for learning, research, and technical exchange only.**

1. **Copyright Ownership**: The game *Narcissu 10th Anniversary Anthology Project* and all related assets (including but not limited to text, images, music, sound effects, etc.) involved in this repository are the property of the original author **stage‑nana** and publisher **Sekai Project**[reference:0]. This repository does not claim any copyright nor does it profit from these materials[reference:1].

2. **Non‑Commercial Use**: The contents of this repository are **strictly prohibited from any commercial use**. Do not use them for any illegal or unauthorised activities[reference:3].

3. **User Responsibility**: Any consequences arising from the use of this repository’s contents (including but not limited to legal disputes) are **the sole responsibility of the user** and are not related to the repository author.

4. **Support the Official Release**: If you enjoy this work, please **purchase the official version** to support the creators:  
   - [Narcissu 10th Anniversary Anthology Project on Steam](https://store.steampowered.com/app/426690/Narcissu_10th_Anniversary_Anthology_Project/?l=schinese)

5. **Copyright Infringement**: If the copyright holder believes that any content in this repository infringes upon your legal rights, please contact us via Issue or email, and we will handle it promptly[reference:5].

---

## Quick Start

```bash
cargo run --release --bin bevy-vn-example
```

### WASM Build

```bash
source /etc/profile.d/emscripten.sh   # Required: basisu_c_sys depends on emcc to compile C code
cargo build --target wasm32-unknown-unknown --release --package bevy-vn-example \
  --target-dir /path/to/wasm-target
```

For the full release pipeline (wasm‑bindgen → wasm‑opt → SRI update → pre‑compression), see [`docs/WASM_PUBLISH_PLAN.md` §2.5](docs/WASM_PUBLISH_PLAN.md). Note that the WASM release build logs are downgraded to ERROR only, and `.cargo/config.toml` uses `--remap-path-prefix` to hide local build paths (D7/D8).

---

## Workspace Structure

| Path | Description |
|------|-------------|
| `crates/bevy-vn-core` | Core engine: state machine, asset loading, script system |
| `crates/bevy-vn-render` | Rendering: BG / CG / sprites / foreground layers |
| `crates/bevy-vn-audio` | Audio playback |
| `crates/bevy-vn-ui` | UI: dialogue boxes, menus, gallery, in‑game menu |
| `crates/bevy-vn-save` | Save system |
| `crates/bevy-vn-video` | Video playback |
| `tools/bevy-vn-asset-packer` | Asset generation/packaging tool |
| `tools/artemis-converter` | Script conversion tool |
| `examples/minimal` | Example game (Narcissu 10th Anniversary) |
| `docs/` | Architecture and design documents |

---

## Assets

All assets required to run the game are already committed to Git (~291 MB, about 9000+ files). You can clone and run directly.

### Generated Assets and Git Status Filtering

The 1231 textures under `examples/minimal/assets/image/` are **ETC1S KTX2 files regenerated from PNG sources**. Locally regenerating or tweaking these files will cause `git status` to show thousands of modified lines.

A management script [`tools/assets-git-manage.sh`](tools/assets-git-manage.sh) is provided to mark these files as "locally ignored" using `git update-index --skip-worktree`:

```bash
# Mark image/ as skip-worktree (filter out git status noise)
./tools/assets-git-manage.sh mark

# Show which files are currently marked
./tools/assets-git-manage.sh status

# Unmark (e.g., before pulling asset updates)
./tools/assets-git-manage.sh unmark

# Unmark → git pull → remark (one‑stop)
./tools/assets-git-manage.sh pull
```

> Note: `--skip-worktree` is a **local repository state** and does not propagate with commits. Everyone who clones this repository needs to run `./tools/assets-git-manage.sh mark` once for it to take effect.

### Asset Directory Breakdown

| Directory | Size | Description |
|-----------|------|-------------|
| `audio/` | 200+ MB | OPUS audio |
| `image/` | 40 MB | ETC1S KTX2 textures (regenerated artifacts, skip‑worktree) |
| `fonts/` | 16 MB | Fonts |
| `scripts/` | 6 MB | Game scripts |
| `pa/` | 4.4 MB | Character sprites |
| `ui/` | 1.1 MB | UI assets |

### Preview Code (Git Sparse Checkout)

```
If you only want to browse the code without downloading hundreds of megabytes of images/models, clone like this:
# 1. Clone the repository, but do not check out any files yet
git clone --filter=blob:none --no-checkout https://github.com/swordreforge/bevy-vn-engine
cd bevy-vn-engine/

# 2. Enable sparse-checkout feature (recommended cone mode) [reference:1]
git sparse-checkout init --cone

# 3. Set the directories you want to check out (i.e., src, crates, tools)
git sparse-checkout set src crates tools

# 4. Finally, check out files from the remote repository
git checkout main
git sparse-checkout set src crates tools   # only fetch code folders
```

---

## Documentation

- [Architecture Design](docs/ARCHITECTURE.md)
- [Bevy 0.19 API Reference](docs/BEVY_0_19_API_REFERENCE.md)
- [WASM Publishing Plan](docs/WASM_PUBLISH_PLAN.md) (includes build pipeline, log downgrade, path hiding, SRI maintenance)
