## ⚠️ 重要声明 / Legal Disclaimer

**本仓库仅供学习、研究与技术交流之用。**

1. **版权归属**：本仓库所涉及的游戏《Narcissu 10th Anniversary Anthology Project》及其相关素材（包括但不限于文本、图像、音乐、音效等）的版权归原作者 **stage-nana** 及发行商 **Sekai Project** 所有[reference:0]。本仓库不拥有任何版权，亦不以此牟利[reference:1]。

2. **非商业用途**：本仓库内容**严禁用于任何商业用途**。请勿将本仓库内容用于任何非法或未授权的活动[reference:3]。

3. **用户责任**：使用本仓库内容所产生的任何后果（包括但不限于法律纠纷等）**由使用者自行承担**，与本仓库作者无关。

4. **支持正版**：如果您喜欢这部作品，请**购买正版**以支持创作者：
   - [Narcissu 10th Anniversary Anthology Project on Steam](https://store.steampowered.com/app/426690/Narcissu_10th_Anniversary_Anthology_Project/?l=schinese)

5. **侵权处理**：如版权方认为本仓库内容侵犯了您的合法权益，请通过 Issue 或邮件联系，我们将在第一时间处理[reference:5]。
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
