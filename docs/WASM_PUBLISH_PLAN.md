# WASM 发布计划 — Bevy VN Engine

> 状态：**已审计，决策已定（2026-08-08）**
> 目标：将 `bevy-vn-engine` 编译为 `wasm32-unknown-unknown` 并在浏览器运行《水仙 10 周年》。
> 本文档盘点现状差距、给出代码改动方案（feature 隔离 / 独立 impl / cfg 分支）、分阶段实施计划与风险清单。

---

## 0. 已定决策（审计结论）

| # | 决策项 | 结论 |
|---|---|---|
| D1 | 渲染后端 | **双后端 `webgpu` + `webgl2`**(Bevy 0.19 features 可共存;wgpu 29 运行时自动 fallback:WebGPU 可用则用之,否则(如 Firefox Linux)走 WebGL2。**冒烟已验证:Brave=BrowserWebGpu、Firefox=Gl**) |
| D2 | 存档存储 | **`localStorage`**(小 JSON,同步语义,5MB 配额足够) |
| D3 | 资产存储 | **`OPFS`(Origin Private File System)**(400MB 二进制小文件场景:写入快 3-4 倍、无单文件大小限制、文件系统语义匹配;**注意**:Bevy 自带 `web_asset_cache` 是 native-only 且写本地磁盘,wasm 端无任何现成缓存,需自研 `AssetReader` 包装层,见 §2.4) |
| D4 | 脚本加载 | **AssetServer 懒加载 + ScriptManifest 清单**(统一原生/wasm,顺带修 theme CWD bug;**wasm 上 `load_folder` 不可用**——Bevy 0.19 `HttpWasmAssetReader::read_directory` 返回空流,见 §2 手段 C) |
| D5 | 体积优化 | **后续再做**(本期直接静态托管 440M,AssetServer 天然按需加载) |
| D6 | 部署 | **GitHub Pages**(静态托管 + MIME 配置) |

---

## 1. 现状盘点（facts）

### 1.1 依赖与 feature 配置

| 项 | 现状 | wasm 影响 |
|---|---|---|
| Bevy | `0.19`，`default-features = false` | 需按 wasm 重选 feature |
| 渲染后端 | 未显式指定（默认 wgpu native） | wasm 需 `webgpu` 或 `webgl2` |
| `multi_threaded` | **显式开启** | wasm 无多线程（除非 threads feature），必须改 `single_threaded` |
| `x11` / `wayland` | 显式开启（Linux 窗口后端） | wasm 不需要，可保留（按 target 自动失效）或剔除 |
| `png` | 开启 | 无影响 |
| `vorbis`（lewton 纯 Rust 解码） | 开启 | ✅ wasm 可用 |
| `wgpu` | `29.0.4` | wasm 走 WebGPU/WebGL2 |
| `rodio` | `0.22.2` | wasm 需 bevy_audio 的 web 支持 |
| `bevy_basisu_loader` | `0.6.1` | ✅ 官方支持 wasm（内嵌 Emscripten 编译的 basisu transcoder，通过 wasm-bindgen/js-sys 调用），ETC1S/UASTC 均可转码 |

**关键结论**：纹理（KTX2）与音频（OGG vorbis）两条资产管线在 wasm 上原生可用，无阻塞性库问题。

### 1.2 平台硬依赖（std::fs）清单

wasm 下没有文件系统，以下 `std::fs` 使用点**全部编译可通过但运行必挂**（或直接编译失败），是首要改造对象：

| # | 位置 | 用途 | wasm 后果 | 改造方向 |
|---|---|---|---|---|
| 1 | `examples/minimal/src/main.rs:89-113` | `read_dir` + `read_to_string` 加载 83 个 `.vnscript.ron` | ❌ 直接 panic | 改走 `AssetServer`（推荐）或 `include_str!` |
| 2 | `crates/bevy-vn-save/src/lib.rs:109-151,170` | 存档 `saves/slot_N.json` 读写删 | ❌ 存档全挂 | 抽象 `SaveStorage` trait → FS impl + Web localStorage/IndexedDB impl |
| 3 | `crates/bevy-vn-ui/src/settings_data.rs:132-152` | 设置 `saves/settings.json` 读写 | ❌ 设置不保存 | 同上，共用存储抽象 |
| 4 | `crates/bevy-vn-ui/src/theme.rs:17` | `fs::read_to_string("assets/theme.ron")` | ❌ 且路径是**相对 CWD**，非 AssetServer，原生下也有隐患 | 改走 `AssetServer::load` |

> 注意 #4：`theme.ron` 的路径写死 `"assets/theme.ron"` 相对进程 CWD，绕过了 Bevy AssetServer。原生下依赖启动目录正确才碰巧能跑。**这本身是个已存在的 bug**，wasm 改造正好一并修掉。

### 1.3 视频插件

- `bevy-vn-video`：当前是 **stub**（`TODO: platform-specific video playback`，EOS 模拟直接完成），gstreamer/ffmpeg 均为 optional feature 且默认**不启用**。
- wasm 处理：只需在 example 里 `#[cfg(not(target_arch = "wasm32"))]` 条件注册 `VnVideoPlugin`（或加一个 no-op 空实现）。不影响编译，因为 gstreamer feature 默认关闭。

### 1.4 资产体积（最大痛点）

```
audio   363M   (7844 个 ogg)
image    40M   (1231 个 ktx2 + 103 png)
fonts    16M   (otf)
scripts  15M   (83 个 vnscript.ron)
pa       4.4M
ui       1.1M
─────────────────────
合计   ≈ 440M
```

- Bevy AssetServer 本身就是**懒加载**（用到才 fetch），浏览器下按需 HTTP 拉取，首屏不必全量加载。
- 但 440M 直接静态托管 = 加载慢 + 服务器带宽成本高；若追求体积，需启用已存在的 `bevy-vn-asset-packer`（BPAK v1 + zstd）——注意：**packer 目前只生产 .pak 文件，运行时没有任何 AssetReader 消费它**，要打通需写自定义 `AssetReader`（zstd 解码在 wasm 可用，但 BPAK 是流式 zstd 编码，需确认解码方式）。

### 1.5 其他

- 无任何 wasm 相关代码/配置（无 wasm-bindgen、无 trunk、无 webgpu feature、无构建脚本）。
- 输入：键盘 Space + 鼠标点击；wasm 桌面浏览器无差异，**touch 未处理**（次要，原版是鼠标游戏）。
- 存档/settings 体积极小（JSON），localStorage（~5MB 配额）绰绰有余。

---

## 2. 代码改动方案（三种手段搭配）

### 手段 A：Cargo feature / target 条件（`Cargo.toml`）

```toml
# examples/minimal/Cargo.toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
bevy = { version = "0.19", default-features = false, features = [
    "bevy_asset", "bevy_audio", "bevy_sprite", "bevy_ui", "bevy_text",
    "bevy_ui_render", "bevy_log", "bevy_state",
    "bevy_winit", "bevy_window", "bevy_render", "bevy_core_pipeline",
    "webgl2", "webgpu",   # 双后端(可共存;wgpu 29 运行时自动 fallback,见 Phase 0 冒烟结论)
    "png", "vorbis",
] }
```

- 原生段保留 `multi_threaded` + `x11`/`wayland`，wasm 段用 `single_threaded`（Bevy 默认即 single_threaded，不写即可）。
- **已定 D1**：wasm 段**同时启用 `webgl2` + `webgpu`**（Bevy 0.19 两者可共存，不是覆盖关系）。wgpu 29 在运行时自动选后端：`navigator.gpu` 可用 → WebGPU；否则 fallback `ContextWgpuCore`（WebGL2）。**必须在 App 里显式设 `WgpuSettings.backends = BROWSER_WEBGPU | GL`**（Bevy 默认只给 BROWSER_WEBGPU，否则 fallback 后无 GL 适配器，报 "Unable to find a GPU"），priority 保持默认 `Functionality`（bevy 自动取 adapter.limits()，避免 WebGL2 上 LimitsExceeded）。冒烟验证代码见 §5。
- 各 crate 的 `Cargo.toml` 同理按 target 调整（audio/render/ui/save 里不要有 `multi_threaded` 即可；`x11`/`wayland` 在 wasm target 下不会启用，可不动）。

### 手段 B：独立 impl（存储抽象，推荐）

存档与设置共用同一存储模式（JSON 字符串 ↔ 磁盘），抽象为 trait 最优雅：

```rust
// crates/bevy-vn-core/src/storage.rs（新，Phase 1 已实现）
pub trait AppStorage: Send + Sync {
    fn read(&self, key: &str) -> Result<Option<String>, String>; // Ok(None) = 不存在
    fn write(&self, key: &str, data: &str) -> Result<(), String>; // 自动建父目录
    fn remove(&self, key: &str) -> Result<(), String>;            // 不存在也 Ok
}
// 原生 impl: FsStorage（ZST，路径相对 CWD）          ← 现有 fs 代码已搬入
// wasm impl: WebStorage { }（Phase 3）             ← localStorage 包装（web-sys / gloo-storage）
// 注入方式: app.insert_resource(AppStorageResource(Arc<dyn AppStorage>))，VnSavePlugin::build 未显式注入时默认 Arc::new(FsStorage)
```

改造点（Phase 1 已完成）：
- `SaveManager::new()` / `refresh` / `save_with_meta` / `delete` → 持有 `Arc<dyn AppStorage>`（`save_dir` 改 `String`，`slot_path` 返回字符串路径）。
- `load_settings` / `save_settings` → 接受 `&dyn AppStorage` 参数；settings.rs 四个系统（`update_settings_overlay` / `setup_settings` / `handle_value_clicks` / `teardown_settings`）经 `Res<AppStorageResource>` 取用。
- **已定 D2**：wasm impl = `WebStorage`（localStorage 包装，同步 API 与现有 `fs` 语义一致，改动最小；用 `gloo-storage` 或直接 `web-sys`）。
- 好处：**引擎 crate 内零 `#[cfg]`**，平台差异收敛到一个文件一个 trait；以后加 Steam/云存档也走同一接口。

### 手段 C：cfg 分支（最小兜底，用于无法抽象的点）

1. **脚本加载**（#1）：**已定 D4，Phase 1+2 已实现** — 采用 **ScriptManifest 清单**方案：`assets/scripts/manifest.list`（每行一个文件名，`#` 注释），`ScriptManifestLoader`（`.list` 扩展）解析；`main.rs` 加载 manifest → 对每个文件名 `asset_server.load::<VnScriptAsset>("scripts/{f}")` → `Assets<VnScriptAsset>` 全就绪后按 `meta.name` 逐个 `engine.load_script`（`pack` 跳过）。原生/wasm 同一条代码路径。
   - ⚠️ **为何不用 `load_folder`**：Bevy 0.19 wasm 端 `HttpWasmAssetReader::read_directory`（`bevy_asset-0.19.0/src/io/wasm.rs:127-134`）**无条件返回空流**并记 error——wasm 上 `load_folder("scripts")` 会加载 0 个脚本。原生 OK，故原生/wasm 行为不一致，必须换成显式清单（文件名列表随 assets 一起发布）。
2. **theme.ron**（#4）：**Phase 1 已实现** — `VnTheme` 加 `#[derive(Asset, TypePath)]` + `VnThemeLoader`（同 `VnScriptLoader` 模式，解析 ron），UI 侧 `VnThemePlugin` 在 Startup `asset_server.load("theme.ron")`，Update 轮询 `Assets<VnTheme>` 就绪后 `insert_resource`（缺失/失败保持默认，与旧行为一致）。注册用 `app.register_asset_loader`（`init_asset_loader` 要求 `FromWorld`）。原生/wasm 统一，CWD bug 已修。
3. **视频插件**（#3）：example 里 `#[cfg(not(target_arch = "wasm32"))]` 注册即可，引擎侧不动。
4. **入口**：wasm 没有 `main` 的 `run()` 阻塞语义，Bevy 0.19 wasm 入口仍用 `App::run()`（内部适配），一般无需独立 main。若出现 wasm 特有问题再拆 `wasm_entry.rs`。

### 手段 D：OPFS 资产缓存层（D3，wasm 专属，Phase 3 引入）

> **选型依据（审计定案）**：400MB 二进制小文件场景下 OPFS 优于 IndexedDB：
>
> | 维度 | IndexedDB | OPFS |
> |---|---|---|
> | 设计目标 | 结构化数据对象存储 | 高性能二进制 I/O |
> | 写入性能 | ~850ms/50MB | ~90ms/100MB（快 3-4 倍） |
> | 单文件限制 | Firefox 单记录 ~2MB（超限需分块） | 无限制 |
> | API 复杂度 | 回调式事务模型 | 文件系统语义（目录/读写/删除） |
> | 查询能力 | 强（索引/查询） | 无（本场景不需要） |
>
> 浏览器支持：Chrome 86+ / Edge / Firefox 111+ / Safari 16.4+（均无需用户授权，`navigator.storage.getDirectory()` 直接可用，不像 File System Access API 需要 `showDirectoryPicker`）。

> **Bevy 官方现状（已核实 0.19.0 源码）**：
> - `bevy_asset::io::wasm::WebAssetReader` = 纯 `fetch`,**无缓存**。
> - `web_asset_cache` feature 是 **`cfg(all(not(target_arch = "wasm32"), ...))` = native-only**,写本地磁盘 `.web-asset-cache/`,wasm 端不生效。
> - **Bevy 0.19 代码库中不存在 `opfs` 模块**。
> - 结论：必须自研 `AssetReader` 包装层,OPFS 仅作为该包装层的缓存后端,与 Bevy 官方无依赖冲突。

方案：自定义 `AssetReader` 包装 `bevy::asset::io::web::WebAssetReader`，实现 3 层读取：

```
AssetServer ─→ OpfsAssetReader（自定义）
                ├─ 1. OPFS 命中（key = 路径 + 缓存版本）→ 直接返回 bytes
                ├─ 2. miss → 委托底层 WebAssetReader fetch
                └─ 3. fetch 成功后写入 OPFS（异步，不阻塞返回）
```

- **API 选型**：主线程用异步 `FileSystemDirectoryHandle.getFileHandle()/createWritable()`（web-sys `FileSystemFileHandle`/`FileSystemWritableFileStream`）；如后续把缓存读写放进 Web Worker,可用同步 `FileSystemSyncAccessHandle`（接近原生速度）。
- **现成库调研（2026-08 核实）**：
  - `opfs`（anchpop，0.2.0，MIT）— **已定采用**。通用 OPFS 封装：wasm 走 OPFS 浏览器 API、native 走 `tokio::fs`、内存文件系统供测试，统一异步 API，天然适配 `AssetReader` 包装。
  - `opfs-project`（utooland，0.2.10）— **不适用**。npm 项目管理专用（fuse-link 间接层 + content-addressable store + tgz 流式解压），与游戏资产缓存场景不匹配。
  - ✅ **源码核实（已 clone 审计）**：
    - `tokio` 声明于 `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`，**wasm 端不编译 tokio**，与 Bevy `wasm-bindgen-futures` 执行器无冲突。
    - API 覆盖 `AssetReader` 三接口：`read`（目录逐层 + `get_file_handle().read()`）、`is_directory`/`read_meta`（handle 获取失败即 NotFound）、`read_directory`（`entries()` → `Stream<(String, DirectoryEntry)>`）；另有 `read_range`/`size` 供元数据。
    - `app_specific_dir()`：wasm 返回 OPFS 根，native 返回 `~/.local/share`（目录自动创建）。
    - ⚠️ **Send/Sync 注意**：`DirectoryHandle` 包装 `web_sys::FileSystemDirectoryHandle`（`!Send + !Sync`，JS 对象），而 Bevy `AssetReader: Send + Sync + 'static`。**解法：`OpfsAssetReader` 不持有 handle 字段，每次调用现取 `app_specific_dir()`**（单次 JS 调用，开销可忽略；Bevy 自带 `WebAssetReader` 同为此纯数据设计）。
- **无分块逻辑**：OPFS 无单文件上限,免去 IndexedDB 在 Firefox 的 ~2MB 分块复杂度。
- 缓存失效策略：URL 路径带版本参数（`assets/v1/...`），发版时改路径前缀即全量失效。
- **加载进度**：`AssetReader` 层可暴露总字节进度（经 `AssetServer` 的加载事件或自建事件），Phase 3 做加载进度 UI 用。
- 注意：`WebAssetReader` 是 Bevy 内置（`bevy::asset::io::web`），包装器需在 wasm 段条件编译，原生段用 `FileAssetReader` 即可。

### 推荐组合

> **B（存储 trait）为主 + C（cfg 兜底 + AssetServer 化）+ D（IndexedDB 资产缓存，wasm 专属）+ A（target 条件 features）**
> 引擎 4 个 fs 点全部消除；`#[cfg]` 只出现在 example 层（视频插件、入口）与 wasm 专属的缓存 reader；引擎 crate 保持平台无关，符合现有 ARCHITECTURE.md 的"插件化、可配置、格式无关"设计原则。

---

## 3. 分阶段实施计划（每阶段可独立验证）

### Phase 0 — 工具链冒烟 ✅ 已完成（2026-08-08）
- [x] 安装 `wasm32-unknown-unknown` target（已预装）、`trunk`（0.21.14，经 cargo-binstall 预编译二进制）、`wasm-bindgen-cli`（0.2.127）、`wasm-pack`（0.15.0）。
- [x] 最小 wasm 测试（`/tmp/opencode/wasm-smoke`：空 App + Sprite + 1 张 PNG），`trunk build --release` 通过，产出 67.5MB wasm。
- [x] **双后端（D1）验证通过**：
  - Brave：`AdapterInfo { backend: BrowserWebGpu }` — WebGPU 路径。
  - Firefox(Linux)：`AdapterInfo { backend: Gl, driver_info: "WebGL 2.0" }` — 自动 fallback 到 WebGL2，渲染成功。
- **关键配置**（已实机验证，最终方案）：
  1. Cargo features 同时启用 `"webgl2", "webgpu"`（Bevy 0.19 两者共存，非覆盖）。
  2. App 中显式设 `WgpuSettings { backends: Some(Backends::BROWSER_WEBGPU | Backends::GL), ..default() }`。
  3. **不要**设 `priority: WebGL2`——`WgpuSettings::default()` 的 `limits` 字段按默认 priority 展开为 `Limits::default()`(compute workgroups=65535)，WebGL2 adapter 上限为 0 → `RequestDeviceError: LimitsExceeded`。保持默认 `Functionality`，bevy 自动取 `adapter.limits()`，双后端各自适配。
- **经验**：
  - `trunk` 源码编译在 nightly 1.99 上失败（`lightningcss` 61 个错误），改用 `cargo binstall` 预编译二进制成功。
  - wasm-bindgen CLI 0.2.127 vs Cargo.lock 0.2.126 的 patch 级差异无冲突。
  - **双后端 fallback 机制**：wgpu 29 `Instance::new` 运行时判断 `navigator.gpu` 是否存在，无则走 `ContextWgpuCore`(WebGL2)；但 Bevy 默认 `default_backends` 在 webgpu feature 下只含 `BROWSER_WEBGPU`，fallback 后无 GL 适配器 → 必须手动并集 `GL`。
  - WebGL2 优雅降级为预期行为：`Sparse buffer disabled`、`OIT not loaded`、`GPU preprocessing → CPU` 均为正常降级警告。
  - **构建资源**：双后端 release wasm 链接需大内存+磁盘；tmpfs `/tmp` 配额不足会导致 `rust-lld` SIGBUS(core dumped)——用 `CARGO_TARGET_DIR` 指向磁盘分区解决。
  - `Failed to deserialize meta for asset` 是缺 `.meta` 文件的无害警告，不影响加载。
  - **浏览器差异**：Firefox Linux 默认无 WebGPU（走 WebGL2 fallback，本方案已覆盖）；Brave/Chrome 用 WebGPU。
  - **⚠️ 成功判定教训**：不能只看 `AdapterInfo` 打印就判定成功——本冒烟曾误判（灰色屏幕 = 仅窗口背景，test.png 从未渲染）。**判定标准必须含资源渲染证据**：控制台 `image loaded OK`（或等价日志）+ 可见图像。资源 404 时 bevy 只报 `Failed to fetch path: {}`（空路径消息易误导），先确认 dist 里 assets 存在、URL 可 200。

### Phase 1 — 代码去平台化（1-2 天）
- [x] `bevy-vn-core/src/storage.rs` 新增 `AppStorage` trait + `FsStorage`（搬运现有 fs 逻辑）。
- [x] `bevy-vn-save`、`bevy-vn-ui/settings_data` 改用 trait。
- [x] `theme.rs` 改 AssetServer。
- [x] `main.rs` 脚本加载改 AssetServer（**D4 懒加载**）。
- **验证**：原生 `cargo run -p minimal --release` 全功能回归（存档、设置、主题、脚本）与改造前一致。

> **Phase 1 完成（2026-08-08）**：`cargo test --workspace` 81 个测试全绿；`cargo run -p bevy-vn-example --release` 实机验证 `Loaded 82 scripts via AssetServer`（83 − pack）与改造前一致。实现差异 vs 计划（详见 §2 手段 B/C）：① `AppStorage::read` 返回 `Result<Option<String>, String>`（区分"不存在"与"IO 错误"）；② 注入用 `Arc<dyn AppStorage>` 包成 `AppStorageResource`（Resource 要求具体类型，Arc 让 SaveManager 与 settings 共享同一实例）；③ `VnTheme` 同时 derive `Resource + Asset`，loader 用 `app.register_asset_loader`（`init_asset_loader` 要求 `FromWorld`）；④ 仓库中 theme.ron 实际不存在（旧 CWD 读取一直静默失败走默认），AssetServer 版本行为一致；⑤ 脚本 key 直接取 `meta.name`（83 个脚本全有 name），`ingest_scripts` 依赖 `LoadedFolder` 依赖就绪语义（全量成功才触发，现有脚本全部 parse 健康）。

### Phase 2 — wasm 编译通过 ✅ 已完成（2026-08-08）
- [x] 各 Cargo.toml 加 wasm target 条件段（**webgl2 + webgpu / D1**、single_threaded；仅 examples/minimal 需要，各 crate 无平台 feature）。
- [x] `trunk build` 出产物，修复编译错误（**wasm-bindgen 0.2.126 CLI 锁定**：Cargo.lock 锁定 0.2.126（js-sys 0.3.103 硬性 `=0.2.126`），trunk 下载机制被墙，`cargo install wasm-bindgen-cli --version 0.2.126 --root /tmp/opencode/wb126` 解决；emcc 用系统包 `source /etc/profile.d/emscripten.sh`（basisu_c_sys 需 emscripten 编译））。
- [x] **index.html `<link data-trunk rel="copy-dir" href="assets">`**：dist/ 完整包含 assets（478M = wasm 41M + assets 439M）。
- [x] example 层 `#[cfg(not(wasm))]` 屏蔽视频插件 + `#[cfg(wasm)]` 段设 `AssetPlugin.meta_check = AssetMetaCheck::Never` + `WgpuSettings { backends: Some(BROWSER_WEBGPU | GL) }`。
- [x] **AssetPlugin 设 `meta_check: AssetMetaCheck::Never`**(wasm 段):跳过 `.meta` 查找,避免每个资产一次 404 请求。
- [x] **脚本加载 wasm 缺陷修复（D4 关键发现）**：wasm 上 `load_folder` 返回空流（见 §2 手段 C）→ 定案 ScriptManifest 清单方案，core 新增 `ScriptManifest` + `ScriptManifestLoader`，main.rs 改 manifest 驱动。
- **验证**：`trunk build --release` 成功；浏览器实测标题画面出现、可进 RouteSelect、82 脚本加载、交互正常。
- **实现差异 vs 计划**：① **D4 方案变更**：`load_folder` → ScriptManifest 清单（wasm `read_directory` 空流，原生/wasm 统一为清单驱动，manifest.list 83 行随 assets 发布）；② wasm-bindgen CLI 版本必须与 Cargo.lock 完全一致（0.2.126），否则 wasm-bindgen 后处理产物不匹配；③ 原生回归时直接跑二进制会因 `current_exe()` 基路径解析到 `target/release/assets` 而全 404——Bevy 0.19 `FileAssetReader::get_base_path` 优先级为 `BEVY_ASSET_ROOT` → `CARGO_MANIFEST_DIR` → `current_exe()` 目录，用 `cargo run`（设 CARGO_MANIFEST_DIR）即正确；④ 已知 13 个语音文件在资源包中缺失（`aka_0409b`、`mom_a0{12,22,32,42,52,62}`、`npetcm36_010/011/012`、`npetcw2_146`、`0snyuka_0125/0128`，引用 6955 个中 0.2%），**预先存在的问题**（原生同样 404，用户资源包本身无此文件），引擎对缺失音频不阻塞剧情（`AudioPlayer` 直接 spawn 静默跳过），决定忽略；⑤ 构建产物 `dist/`（478M）加入 `.gitignore` 不入库，GitHub Pages 部署时单独处理（见 Phase 4）；⑥ **wasm 无声（autoplay policy）**：bevy_audio 0.19 的 `AudioOutput` 在 App 启动时初始化（`init_resource` 首次 update，无用户手势）→ rodio `MixerDeviceSink::open` 创建 cpal 流后立即 `stream.play()` → cpal 0.17 wasm `StreamTrait::play()` 调 `AudioContext.resume()`（`cpal-0.17.3/src/host/webaudio/mod.rs`）→ **浏览器 autoplay policy 拒绝（无手势）**，且 bevy_audio 源码无任何手势解锁处理（已 grep 确认），AudioContext 永久 suspended → 无声。**修复**：`index.html` monkey-patch `AudioContext` 构造函数捕获 cpal 创建的实例（wasm-bindgen 的 `new AudioContext()` 按全局作用域解析命中替换），首次 `pointerdown/keydown/touchend` 时对捕获的 ctx 调 `resume()`（Phaser/Unity 同款方案）。**CDP 验证（headless chromium + `--autoplay-policy=user-gesture-required` + 可信点击）**：patch 捕获 cpal ctx（`[audio-unlock] resumed 0/1`，headless 无真实手势故 ctx 非 suspended；真实浏览器 console 已证实现象为 suspended），unlock 逻辑在手势时正常触发。**待用户真实浏览器确认**（硬刷新听 BGM）。

### Phase 3 — 运行时验证 + OPFS 缓存（1-2 天）
- [ ] **D3**：实现 `OpfsAssetReader`（包装 `WebAssetReader`，主线程异步 OPFS API），wasm 段注册；首次加载后刷新页面验证命中缓存（Network 面板无重复请求）。
- [ ] 进游戏：BG/立绘/CG 纹理（basisu transcoder 在浏览器转码）✅/❌。
- [ ] BGM/语音/SE（rodio web + lewton ogg 解码）✅/❌。
- [ ] 存档/读档/删档（**localStorage / D2**）、设置保存 ✅/❌。
- [ ] 脚本跳转、自定义 tag（タイトル/brandlogo）✅/❌。
- [ ] 加载进度 UI（若需要）。
- **验证**：完整通关一条短故事（如 4novel / atogaki）。

### Phase 4 — 部署 GitHub Pages（半天，D6）
- [ ] `trunk build --release` 产物提交到 `gh-pages` 分支（或 `dist/` 目录 + Actions workflow）。
- [ ] 静态服务器 MIME 配置（`.wasm`、`.pkg`/`.data`、`.ktx2`、`.ogg`）。
- [ ] 仓库 Settings → Pages → 指定分支/目录。
- **验证**：`https://<user>.github.io/bevy-vn-engine/` 冷启动可玩，刷新不 404（SPA fallback 或纯静态无路由即可）。

### Phase 5 — 体积优化（后续再做，D5）
- [ ] 评估首屏实际拉取量（懒加载下 fonts 16M + theme + 首图首曲）。
- [ ] 可选：打通 `bevy-vn-asset-packer` → 自定义 `AssetReader`（BPAK + zstd）。
- [ ] 字体子集化（otf → woff2 子集）。
- **验证**：无缓存冷启动 ≤ 可接受时间，中段加载不卡死。

---

## 4. 风险清单（按严重度排序）

| # | 风险 | 等级 | 说明 / 缓解 |
|---|---|---|---|
| R1 | **资产体积 440M** | 🔴 高 | 带宽与加载时间成本。缓解：AssetServer 懒加载天然按需（D5 后续优化）；IndexedDB 缓存（D3）让二次访问零下载。 |
| R2 | **multi_threaded 在 wasm 不可用** | 🔴 高 | 不改必编译失败/运行 panic。Phase 2 必做，风险低但必须做对。 |
| R3 | **ETC1S→WebGPU 转码兼容性** | 🟠 中 | bevy_basisu_loader 0.6.1 声明支持 wasm，但具体浏览器（WebGPU 下 ASTC 支持）需实测。缓解：Phase 3 专项验证；WebGL2 fallback 路径已就绪（D1 双后端,无需回退切换）。 |
| R4 | **rodio web 音频** | 🟠 中 | bevy_audio 0.19 在 wasm 走 web audio；7844 个 ogg 逐个懒加载，需验证切换流畅性与内存。 |
| R5 | **存档/设置移植** | 🟠 中 | 全在 Phase 1 抽象范围内；**localStorage（D2）为同步 API**，可保持现有同步语义，改动最小。 |
| R6 | **theme.ron CWD bug** | 🟢 低 | 现存 bug，wasm 下必炸，Phase 1 顺手修。 |
| R7 | **字体 16M otf** | 🟢 低 | 首屏必拉，一次性；可考虑子集化（Phase 5 优化项）。 |
| R8 | **渲染后端浏览器覆盖** | 🟢 低 | **双后端已覆盖全浏览器**：Chrome/Brave/Edge/Safari 17+ 走 WebGPU；Firefox 全平台走 WebGL2 fallback（Phase 0 已实机验证 Firefox Linux）。无浏览器被排除。 |
| R9 | **视频 stub 无实现** | 🟢 低 | 当前本来就是模拟 EOS，wasm 禁用即可，行为与原生一致（都不真放视频）。 |
| R10 | **OPFS 自研缓存层** | 🟠 中 | Bevy 0.19 **无现成 OPFS 支持**(`web_asset_cache` 为 native-only 写本地磁盘;`WebAssetReader` 纯 fetch;代码库无 `opfs` 模块,已核实源码),自研 `AssetReader` 需覆盖 read/meta/directory 三接口 + 版本失效策略;主线程 OPFS 为异步 API(Web Worker 内才有同步句柄)。底层库 `opfs`(anchpop)已 clone 源码核实:wasm 端无 tokio、API 全覆盖,唯一注意点是 JS handle `!Send+!Sync`(解法:结构体不持有 handle,调用时现取)。缓解：Phase 3 独立验证命中/失效路径;浏览器需 Chrome 86+/FF 111+/Safari 16.4+。 |
| R11 | **GitHub Pages 单包体积** | 🟢 低 | Pages 单文件 100MB 上限,trunk 产物（wasm ~50-100MB debug 或 release 更小）+ 440M assets 需确认：assets 若 push 进仓库则仓库膨胀（当前 assets 已在 git 中 439MB）；若 assets 走 release 附件/独立 CDN 则需改部署结构。Phase 4 评估。 |

---

## 5. 待决策问题

> **已全部定案（见 §0）**。实施期间若出现新分歧点（如 R3 转码失败需回退 webgl2、R11 部署结构变化），回到此处更新。

## 6. 审计要点（reviewer 请重点核对）

- [ ] 手段 B 的 `AppStorage` trait 是否覆盖 save + settings 两个使用方，接口是否够用（读/写/删，同步语义）。
- [ ] Phase 1 完成后的原生回归清单是否完整（存档 CRUD、设置读写、主题加载、脚本加载、CG/立绘/音频）。
- [ ] Phase 2 的 wasm feature 组合与 Bevy 0.19 官方 wasm 示例是否一致（尤其 single_threaded 缺省、webgpu 覆盖 webgl2 的语义）。
- [ ] R3/R4 是否安排了独立验证步骤，而非混在整体联调里。
- [ ] **D3** OpfsAssetReader 的 read/meta/directory 三接口实现与版本失效策略（Phase 3 独立验证命中/失效）；浏览器兼容基线 Chrome 86+/FF 111+/Safari 16.4+。
- [ ] **R11** GitHub Pages 的 440M assets 部署结构（git 仓库内 vs 外部 CDN/release）。
