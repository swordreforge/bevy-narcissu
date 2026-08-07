# Bevy VN Engine

基于 Bevy 0.19 的通用视觉小说引擎,支持脚本驱动的 AVG 游戏。当前承载《水仙 10 周年》移植项目。

## 快速开始

```bash
cargo run -p minimal --release
```

## 工作区结构

| 路径 | 说明 |
|---|---|
| `crates/bevy-vn-core` | 核心引擎:状态机、资产加载、脚本系统 |
| `crates/bevy-vn-render` | 渲染:BG / CG / 立绘 / 前景图层 |
| `crates/bevy-vn-audio` | 音频播放 |
| `crates/bevy-vn-ui` | 界面:对话框、菜单、图鉴、游戏内菜单 |
| `crates/bevy-vn-save` | 存档系统 |
| `crates/bevy-vn-video` | 视频播放 |
| `tools/bevy-vn-asset-packer` | 资产生成/打包工具 |
| `tools/artemis-converter` | 脚本转换工具 |
| `examples/minimal` | 示例游戏(水仙 10 周年) |
| `docs/` | 架构与设计文档 |

## 资产

游戏运行所需全部资产已提交进 git(约 439MB,9263 个文件),clone 后即可直接运行。

### 资产生成物与 git status 过滤

`examples/minimal/assets/image/` 下的 1231 张纹理是**从 PNG 重转的 ETC1S KTX2 生成物**(原始 PNG 备份见 `/home/swordreforge/下载/水仙10周年版本_1.2.0.zip`)。本地重转/微调这些文件会让 `git status` 刷出上千行 modified。

为此提供了管理脚本 [`tools/assets-git-manage.sh`](tools/assets-git-manage.sh),用 `git update-index --skip-worktree` 把这些文件标记为"本地忽略":

```bash
# 标记 image/ 为 skip-worktree(过滤 git status 噪音)
./tools/assets-git-manage.sh mark

# 查看当前哪些文件被标记
./tools/assets-git-manage.sh status

# 解除标记(例如要 pull 资产更新前)
./tools/assets-git-manage.sh unmark

# 解除标记 → git pull → 重新标记(一条龙)
./tools/assets-git-manage.sh pull
```

> 注意:`--skip-worktree` 是**本地仓库状态**,不随 commit 传播。每个 clone 此仓库的人需要自己运行一次 `./tools/assets-git-manage.sh mark` 才会生效。

### 资产目录构成

| 目录 | 大小 | 说明 |
|---|---|---|
| `audio/` | 363M | OGG 音频 |
| `image/` | 40M | ETC1S KTX2 纹理(重转生成物,skip-worktree) |
| `fonts/` | 16M | 字体 |
| `scripts/` | 15M | 游戏脚本 |
| `pa/` | 4.4M | 立绘 |
| `ui/` | 1.1M | UI 素材 |

## 文档

- [架构设计](docs/ARCHITECTURE.md)
- [Bevy 0.19 API 参考](docs/BEVY_0_19_API_REFERENCE.md)
