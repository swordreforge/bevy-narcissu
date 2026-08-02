# Bevy VN Engine — Architecture Design Document

> 基于 bevy-vn-test (Eustia port) 的不足分析，使用 Bevy 0.19 的 `bsn!{}` 宏重新设计的通用视觉小说引擎。

---

## 目录

1. [项目目标](#1-项目目标)
2. [Crate 结构与 Workspace](#2-crate-结构与-workspace)
3. [核心引擎架构](#3-核心引擎架构)
4. [脚本系统设计](#4-脚本系统设计)
5. [插件通信架构](#5-插件通信架构)
6. [UI 配置与主题系统](#6-ui-配置与主题系统)
7. [资产管线](#7-资产管线)
8. [关键解耦设计](#8-关键解耦设计)
9. [实现阶段](#9-实现阶段)

---

## 1. 项目目标

### 定位

一个**可发布的通用 Bevy VN 引擎 crate**。任何视觉小说项目通过 `Cargo.toml` 添加依赖即可使用，通过 trait 实现和配置文件完成游戏定制。

### 核心设计原则

| 原则 | 说明 |
|------|------|
| **插件化** | 每个子系统是独立的 Bevy `Plugin`，通过明确的消息接口通信 |
| **可配置** | UI 默认值 + RON 主题覆盖 + `bsn!{}` 组件覆盖三层模型 |
| **格式无关** | 脚本 IR (`ScriptCmd`) 通用化；特定格式转换器作为独立工具 |
| **零样板** | 利用 `bsn!{}` / `SceneComponent` / `#[require]` / `on()` 消除 Bundle/System 样板 |
| **资产热加载** | 脚本通过 `AssetServer` 加载，修改即刷新 |

### Bevy 0.19 关键特性使用

| 特性 | 用途 |
|------|------|
| `bsn!{}` / `bsn_list!` | UI 组件组合、场景构建，替代所有 `spawn(Bundle)` 样板 |
| `SceneComponent` | Marker 组件保证其子实体在 spawn 时完整存在（`DialogueLine`, `CharacterPortrait`, `Choice` 等） |
| `#[require(...)]` | 组件契约——插入 A 时自动插入 B/C |
| `on(|event: On<...>\| ...)` | 内联观察者替代分散的系统（按钮点击等 UI 交互） |
| `FromTemplate` | 资产路径字符串 → `Handle<T>` 自动解析 |
| `EntityEvent` | 实体级事件（对话行完成、角色淡入完成等） |
| `#[derive(Event)]` + `add_event` | 跨插件通信（替代旧引擎直接资源写入） |
| `AssetServer` 脚本加载 | `.vnscript.ron` 运行时加载 + 热重载 |

---

## 2. Crate 结构与 Workspace

```
bevy-vn-engine/
├── Cargo.toml                    # workspace root
├── docs/
│   └── ARCHITECTURE.md           # 本文档
├── crates/
│   ├── bevy-vn-core/             # 核心引擎 crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # VnCorePlugin, 公开 API
│   │       ├── script/           # 脚本系统
│   │       │   ├── mod.rs
│   │       │   ├── cmd.rs        # ScriptCmd 枚举 (版本化 + #[serde(other)])
│   │       │   ├── engine.rs      # ScriptEngine 解释器
│   │       │   ├── asset.rs      # VnScript Asset 类型 (AssetServer 加载)
│   │       │   └── expression.rs # 表达式求值器
│   │       ├── state/            # 状态机 (AppState)
│   │       │   ├── mod.rs
│   │       │   └── transition.rs # 状态过渡 (ScreenTransition 替代)
│   │       ├── messages.rs       # 引擎级 Message 定义 (跨插件通信)
│   │       ├── theme.rs          # 主题系统 (VnTheme 资源)
│   │       └── engine_config.rs  # VnEngineConfig: 路径、分辨率等
│   │
│   ├── bevy-vn-render/           # 渲染插件
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # VnRenderPlugin
│   │       ├── bg.rs             # 双缓冲背景系统
│   │       ├── fg.rs             # 立绘槽位池（可配置槽位数和布局）
│   │       ├── cg.rs             # CG/事件图显示
│   │       ├── overlay.rs        # 屏幕覆盖层（渐变/闪光/震动）
│   │       ├── sprite.rs         # Sprite 覆盖层（通用，替代 DrawSprite/DrawSpriteEx）
│   │       └── messages.rs       # 渲染消息类型 (SetBg, ShowFg, ...)
│   │
│   ├── bevy-vn-audio/            # 音频插件
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # VnAudioPlugin
│   │       ├── bgm.rs            # BGM 管理 (A/B 段拼接可选)
│   │       ├── se.rs             # SE 管理 (OneShot + Loop)
│   │       ├── voice.rs          # 语音管理
│   │       └── messages.rs       # 音频消息类型
│   │
│   ├── bevy-vn-ui/               # UI 插件
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # VnUiPlugin
│   │       ├── dialogue.rs       # 对话框 (带主题化)
│   │       ├── choice.rs         # 选项分支 UI
│   │       ├── backlog.rs        # 历史记录
│   │       ├── save_load_ui.rs   # 存档/读档 UI
│   │       ├── settings_ui.rs    # 设置 UI
│   │       ├── title.rs          # 标题界面
│   │       ├── gallery.rs        # CG/音乐鉴赏
│   │       ├── screen.rs         # 通用的 Screen 抽象 (OnEnter/Update/OnExit 模式)
│   │       ├── theme.rs          # 主题加载和 VnTheme → 组件应用
│   │       └── components.rs     # UI 标记组件
│   │
│   ├── bevy-vn-save/             # 存档系统
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # VnSavePlugin
│   │       ├── manager.rs        # SaveManager (JSON 序列化)
│   │       ├── snapshot.rs       # SaveData 快照 (通过 trait 收集状态)
│   │       └── migration.rs      # 存档版本迁移
│   │
│   └── bevy-vn-video/            # 视频播放插件
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs            # VnVideoPlugin
│           ├── desktop.rs        # GStreamer 后端
│           └── android.rs        # FFmpeg 后端
│
├── tools/
│   ├── bevy-vn-asset-packer/     # 通用 PAK 打包工具 (从 asset_packer 提取)
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── artemis-converter/        # Artemis ASB/IET → .vnscript.ron 转换器
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs           # CLI 入口
│           ├── asb.rs            # ASB 二进制解析
│           ├── iet.rs            # IET 文本解析
│           ├── mapper.rs         # ASB 标签 → ScriptCmd 映射
│           └── lua_config.rs     # Lua 配置提取
│
├── examples/
│   └── minimal/                  # 最小可行示例
│       ├── Cargo.toml
│       ├── assets/
│       │   ├── theme.ron         # 默认主题
│       │   ├── scripts/          # .vnscript.ron 脚本
│       │   └── images/           # 示例图片
│       └── src/
│           └── main.rs
│
└── themes/
    └── default.ron               # 内置默认主题
```

### Crate 依赖关系

```
bevy-vn-core ───────────────────────────── 核心 (无 UI/Render 依赖)
  ├── 定义 ScriptCmd, ScriptEngine, AppState
  ├── 定义所有 Event 类型
  ├── 定义 trait (VnAssetProvider, SaveStateProvider, GameConfig)
  └── 定义 VnTheme 结构

bevy-vn-render ───→ bevy-vn-core  消费渲染 Event, 写入渲染状态资源
bevy-vn-audio ────→ bevy-vn-core  消费音频 Event, 写入音频状态资源
bevy-vn-ui ───────→ bevy-vn-core  消费状态资源, 发送 UI 事件
bevy-vn-save ─────→ bevy-vn-core  通过 trait 收集/恢复所有插件状态
bevy-vn-video ────→ bevy-vn-core  消费视频 Event, 发送 AdvanceEvent

artemis-converter ─→ bevy-vn-core  (将 ASB/IET → ScriptCmd, 单向上游依赖)
bevy-vn-asset-packer  独立         (通用工具, 无依赖)
```

**关键改进**: 所有插件**只依赖 `bevy-vn-core`**，互相之间零直接依赖。通信通过 Event 和 trait。

---

## 3. 核心引擎架构

### 3.1 `bevy-vn-core` — 公开 API

```rust
// crates/bevy-vn-core/src/lib.rs

use bevy::prelude::*;

/// 核心引擎插件。注册所有 Event 类型、ScriptEngine、AppState。
/// 其他子插件 (render/audio/ui/save/video) 依赖此插件。
pub struct VnCorePlugin {
    /// 引擎全局配置
    pub config: VnEngineConfig,
}

#[derive(Resource)]
pub struct VnEngineConfig {
    /// 窗口逻辑分辨率
    pub resolution: (f32, f32),  // 默认 1280×720
    /// 脚本加载目录 (相对于 assets/)
    pub script_dir: String,      // 默认 "scripts"
    /// 存档目录
    pub save_dir: String,        // 默认 "saves"
    /// 默认字体路径
    pub default_font: String,
    /// 文本显示速度 (字符/秒) — 全局默认值
    /// 可通过 VnTheme.dialogue.text_speed 按主题覆盖
    pub text_speed: f64,         // 默认 50.0
    /// 自动模式延迟 (秒)
    pub auto_delay: f64,         // 默认 2.0
}
```

### 3.2 状态机 (`state/`)

```rust
// crates/bevy-vn-core/src/state/mod.rs

/// 引擎顶层状态。游戏可扩展。
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum VnAppState {
    #[default]
    Boot,
    Splash,
    Title,
    /// 游戏主循环。所有脚本执行在此状态下。
    Gameplay,
    /// 通用菜单容器。具体菜单通过 VnMenuState 子状态区分。
    Menu,
}

/// 菜单子状态 (在 VnAppState::Menu 内)
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[source(VnAppState = VnAppState::Menu)]
pub enum VnMenuState {
    #[default]
    Main,
    SaveLoad,
    Settings,
    Gallery,
    Backlog,
    RouteSelect,
    AfterStory,
}
```

**改进**: 旧引擎中 SaveLoad/Gallery/Settings/Backlog/RouteEnd/AfterStory 都是 `AppState` 的同级变体，导致每个菜单都需要在顶层状态枚举中占位。此处使用 `SubStates`，菜单内部切换不污染顶层状态。

### 3.3 状态过渡 (`state/transition.rs`)

```rust
/// 统一的状态过渡资源 —— 替代旧引擎中多个插件直接写 ScreenTransition 的模式。
#[derive(Resource, Default)]
pub struct VnTransition {
    pub phase: TransitionPhase,
    pub pending: Option<VnAppState>,
    /// 过渡动画时长 (秒)
    pub duration: f32,
}

/// 过渡请求 —— 任何插件通过 Event 请求过渡，而非直接写资源。
#[derive(Event)]
pub struct TransitionRequest {
    pub target: VnAppState,
    pub duration: Option<f32>,
}

/// 过渡完成通知
#[derive(Event)]
pub struct TransitionComplete {
    pub target: VnAppState,
}
```

**改进**: 旧引擎中 Splash/Title/Menu/Routing 等 5+ 插件直接写 `ScreenTransition.pending_state`。现在通过 `TransitionRequest` Event 解耦——所有插件只发送请求，`transition.rs` 统一执行过渡动画。

---

## 4. 脚本系统设计

### 4.1 ScriptCmd IR — 版本化 + 扩展友好

```rust
// crates/bevy-vn-core/src/script/cmd.rs

/// 脚本版本号，嵌入在 .vnscript.ron 的元数据中。
/// 引擎根据版本做兼容处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptVersion { V1 }

/// 一个完整的脚本文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnScript {
    pub version: ScriptVersion,
    /// 脚本元数据 (名称、作者等)
    pub meta: ScriptMeta,
    /// 指令序列 + 标签
    pub instructions: Vec<ScriptCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScriptMeta {
    pub name: Option<String>,
    /// 下一个脚本名 (替代旧引擎的 +10 数字递增)
    pub next_script: Option<String>,
}

/// 通用 VN 指令集。
///
/// 设计约束：
/// - 所有游戏特定数据通过 string key 引用，不硬编码 ID 映射
/// - #[serde(other)] 确保未知变体不会导致解析失败
/// - 新增变体追加到末尾，不影响已有脚本
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "args")]
#[serde(rename_all = "snake_case")]
pub enum ScriptCmd {
```

> **注意**: `#[serde(tag = "cmd")]` 使用了 internally-tagged 枚举格式，与旧引擎的扁平 RON 格式不同：
> ```ron
> // 旧格式: Dialogue(speaker: None, text: "Hello")
> // 新格式: { cmd: "dialogue", args: { speaker: null, text: "Hello" } }
> ```
> 这意味着 `artemis-converter` 的输出格式需要更新，旧 `.bscript.ron` 文件不直接兼容。
> 如果希望保持扁平格式以兼容旧脚本，可以去掉 `tag = "cmd"` 改用 `#[serde(untagged)]`，
> 但会丢失 `#[serde(other)]` 的未知变体回退能力。
    // —— 控制流 ——
    Label { name: String },
    Jump { label: String },
    Call { label: String },
    CallScript { script: String, label: Option<String> },
    Return,
    /// 条件分支 (表达式: "flag_key op value")
    Condition {
        expression: String,
        goto_true: String,
        goto_false: Option<String>,
    },
    Halt,

    // —— 对话 ——
    /// 对话行
    Dialogue {
        speaker: Option<String>,
        text: String,
        voice: Option<String>,
    },
    /// 清空对话框
    ClearDialogue,

    // —— 选项 ——
    ChoiceBegin,
    ChoiceOption {
        text: String,
        goto: String,
        /// 可选的好感度变化: [(角色key, 变化值), ...]
        affection: Vec<(String, i32)>,
    },
    ChoiceEnd { convergence: String },

    // —— 渲染 ——
    SetBg { image: String, transition: Option<Transition> },
    ShowFg {
        char_id: String,
        expression: String,
        position: FgPosition,
        transition: Option<Transition>,
    },
    HideFg { char_id: String, transition: Option<Transition> },
    ShowFace { char_id: String, expression: String },
    HideFace { char_id: String },
    ShowCg { image: String, transition: Option<Transition> },
    HideCg { transition: Option<Transition> },
    ScrollBg { speed_x: f32, speed_y: f32, time_ms: u64 },
    /// 通用精灵覆盖层 (替代 DrawSprite/DrawSpriteEx)
    Sprite {
        id: String,
        image: String,
        x: f32,
        y: f32,
        anchor_x: Option<f32>,
        anchor_y: Option<f32>,
        z: Option<i32>,
    },
    SpriteFade { id: String, opacity: f32, duration_ms: u64 },
    SpriteMove { id: String, x: f32, y: f32, duration_ms: u64 },
    SpriteRemove { id: String },

    // —— 音频 ——
    PlayBgm { id: String, volume: Option<f32>, fade_ms: Option<u64> },
    StopBgm { fade_ms: Option<u64> },
    PlaySe { file: String, channel: Option<usize>, volume: Option<f32> },
    StopSe { channel: Option<usize> },
    PlayVoice { file: String, volume: Option<f32> },
    /// 设置音量 (0.0-1.0)
    SetVolume { bgm: Option<f32>, se: Option<f32>, voice: Option<f32> },

    // —— 效果 ——
    Wait { time_ms: u64 },
    /// 全屏覆盖 (Flash/Fade/ScreenOverlay 统一)
    ScreenEffect {
        kind: ScreenEffectKind,
        color: Option<String>,  // "White", "Black", "#RRGGBB"
        duration_ms: u64,
    },
    Shake { intensity: f32, duration_frames: u32 },
    /// 视口/背景滚动
    ScrollView { x: f32, y: f32, duration_ms: u64 },

    // —— 状态管理 ——
    SetFlag { key: String, value: i32 },
    SetGlobalFlag { flag_id: u32, value: i32 },
    /// 条件分支 (基于标记值)
    IfFlag { flag_key: String, op: ConditionOp, value: String, goto: String },
    /// 解锁收集品
    UnlockCg { image: String },
    UnlockBgm { id: String },
    UnlockScene { scene_id: String },

    // —— 元指令 ——
    /// 存档点标记
    SavePoint { id: String },
    /// 设置下一个脚本 (替代旧引擎的 +10 数字递增)
    SetNextScript { script: String },
    /// 路线标记 (用于路线完成检测)
    RouteFlag { route_key: String },
    /// 游戏模式切换
    SetMode { mode: String },  // "novel", "auto", etc.

    // —— 视频 ——
    PlayMovie { file: String, blocking: bool },
    StopMovie,
    SpriteVideo { id: String, file: String, x: f32, y: f32 },
    StopSpriteVideo { id: String },

    // —— 游戏特定扩展点 ——
    /// 用户自定义指令 (游戏通过 trait 注册处理器)
    Custom { tag: String, data: HashMap<String, String> },

    // —— 兼容 ——
    /// 未知/未实现指令 (跳过，记录警告)
    #[serde(other)]
    Unknown,
}
```

**关键改进 vs 旧引擎**:

| 旧引擎问题 | 新设计 |
|-----------|--------|
| 78 个扁平变体，每一个都在 runner 中硬编码 | 按功能域分组，通过 `Custom` 支持扩展 |
| `#[serde(other)]` 缺失，未知变体 panic | `Unknown` 变体 + `#[serde(other)]` 安全跳过 |
| `AffectionChange(id, delta)` — 好感度内嵌 | `ChoiceOption.affection` — 好感度是选项的属性 |
| `DrawSpriteEx` 用 `current_dir()/assets` 路径 | `Sprite` 用逻辑路径，由 `sprite_path_provider` 解析 |
| `FadeScene`/`Flash`/`ScreenOverlay` 三个变体 | `ScreenEffect { kind, color, duration_ms }` 一个变体 |
| `BgmVol` 硬编码 "MIN"/"LOW"/"NORM" | `SetVolume { bgm: f32 }` — 0.0-1.0 浮点数 |
| `ShakeScreen` + `ShakeSprite` 分开 | `Shake` — 通用震动，目标通过组件标记 |
| 脚本命名 `aiy00010` +10 递增硬编码 | `meta.next_script` 显式指定下一脚本 |

### 4.2 ScriptEngine — 纯解释器

```rust
// crates/bevy-vn-core/src/script/engine.rs

/// 脚本引擎 —— 与 Bevy ECS 无关的纯数据解释器。
/// 不持有任何 Bevy 资源引用，完全可测试。
#[derive(Resource)]
pub struct ScriptEngine {
    /// 已加载的脚本: name → VnScript
    pub scripts: HashMap<String, VnScript>,
    /// 当前脚本名
    pub current_script: String,
    /// 当前指令指针
    pub current_line: usize,
    /// 调用栈: (script_name, return_line)
    pub call_stack: Vec<(String, usize)>,
    /// 命名标记 (SetFlag/ChoiceOption.affection)
    pub flags: HashMap<String, i32>,
    /// 全局标记 (SetGlobalFlag/RouteFlag)
    pub global_flags: HashMap<u32, i32>,
    /// 当前路线
    pub current_route: Option<String>,
    /// 脚本是否执行完毕
    pub finished: bool,
}

impl ScriptEngine {
    /// 获取当前指令
    pub fn current(&self) -> Option<&ScriptCmd> { ... }

    /// 推进到下一条指令
    pub fn advance(&mut self) -> Option<&ScriptCmd> { ... }

    /// 跳转到标签 (在当前脚本内)
    pub fn jump_to_label(&mut self, label: &str) -> Result<(), ScriptError> { ... }

    /// 压栈 + 跳转
    pub fn call_label(&mut self, label: &str) -> Result<(), ScriptError> { ... }

    /// 切换到另一个脚本
    pub fn call_script(&mut self, script: &str, label: Option<&str>) -> Result<(), ScriptError> { ... }

    /// 弹栈返回
    pub fn return_from_call(&mut self) -> Result<(), ScriptError> { ... }

    /// 还有更多指令吗？
    pub fn has_more(&self) -> bool { ... }

    /// 下一个脚本 (按 meta.next_script → 数字扫描 → None)
    pub fn next_script(&self) -> Option<&str> { ... }
}
```

### 4.3 脚本 Asset 加载

```rust
// crates/bevy-vn-core/src/script/asset.rs

/// VnScript 作为 Bevy Asset，支持 AssetServer 加载和热重载。
#[derive(Asset, TypePath)]
pub struct VnScriptAsset {
    pub script: VnScript,
}

/// AssetLoader 实现
pub struct VnScriptLoader;

impl AssetLoader for VnScriptLoader {
    type Asset = VnScriptAsset;
    type Settings = ();
    type Error = ScriptLoadError;

    fn load<'a>(
        &'a self,
        reader: &'a mut dyn Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let script: VnScript = ron::de::from_bytes(&bytes)?;
            Ok(VnScriptAsset { script })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["vnscript.ron"]
    }
}
```

**改进**: 旧引擎通过 `build.rs` + `include_str!` 在编译期嵌入脚本。新引擎通过 `AssetServer::load("scripts/main.vnscript.ron")` 运行时加载，支持热重载。`build.rs` 不再需要扫描脚本目录。

### 4.4 表达式求值器

```rust
// crates/bevy-vn-core/src/script/expression.rs

/// 条件表达式求值器 —— 替代旧引擎中硬编码的 "t.tmp" 解析。
///
/// 支持的表达式格式:
///   "flag_name op value"     → flag_name >= 3, flag_name != 0
///   "flag_name"              → flag_name != 0 (隐式 != 0)
///
/// 支持的操作符: ==, !=, >=, <=, >, <
pub fn evaluate_condition(
    expression: &str,
    flags: &HashMap<String, i32>,
) -> Result<bool, ExpressionError> { ... }

/// 工作变量表达式求值: "t.tmp + 3" → 读取 tmp flag + 计算
pub fn evaluate_work_expression(
    expression: &str,
    flags: &HashMap<String, i32>,
) -> Result<i32, ExpressionError> { ... }
```

**改进**: 旧引擎的表达式解析器硬编码了 `t.tmp` 魔术变量名。新引擎接受任意 flag key，消除游戏特定的命名假设。

---

## 5. 插件通信架构

### 5.1 通信协议分层

```
┌─────────────────────────────────────────────────────┐
│                    ScriptRunner                      │
│  (唯一驱动源: 消费 AdvanceEvent → 执行 ScriptCmd)      │
│                                                     │
│  对所有子系统: 通过 Event 发送命令                     │
│  对状态资源:   通过 StateWriteEvent 写入               │
└───────┬──────────┬──────────┬──────────┬────────────┘
        │Events    │Events    │Events    │Events
        ▼          ▼          ▼          ▼
   ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐
   │ Render │ │ Audio  │ │ Video  │ │ Dialogue  │
   │ Plugin │ │ Plugin │ │ Plugin │ │ Plugin    │
   └────────┘ └────────┘ └────────┘ └──────────┘
        │          │          │          │
        │ 渲染完成 │ 播放完成 │ EOS      │ 文本显示完成
        ▼          ▼          ▼          ▼
   ┌──────────────────────────────────────────────────┐
   │              AdvanceEvent (恢复脚本执行)            │
   └──────────────────────────────────────────────────┘

   ALL 跨插件状态共享: 通过 trait 而非直接资源访问
   ALL 脚本状态变更:   通过 StateWriteEvent
```

### 5.2 Event 定义 (集中在 `bevy-vn-core`)

> **注意**: Bevy 0.18 使用 `Message`/`MessageWriter`/`add_message`，Bevy 0.19 恢复为 `Event`/`EventWriter`/`add_event`。
> 以下全部使用 0.19 的 `#[derive(Event)]`。

```rust
// crates/bevy-vn-core/src/messages.rs

use bevy::prelude::*;

// ── 渲染事件 ──
#[derive(Event)] pub struct SetBgEvent { ... }
#[derive(Event)] pub struct ShowFgEvent { ... }
#[derive(Event)] pub struct HideFgEvent { ... }
#[derive(Event)] pub struct ShowFaceEvent { ... }
#[derive(Event)] pub struct HideFaceEvent { ... }
#[derive(Event)] pub struct ShowCgEvent { ... }
#[derive(Event)] pub struct HideCgEvent { ... }
#[derive(Event)] pub struct ScrollBgEvent { ... }
#[derive(Event)] pub struct SpriteEvent { ... }        // 统一精灵
#[derive(Event)] pub struct SpriteEffectEvent { ... }  // 统一特效
#[derive(Event)] pub struct ScreenEffectEvent { ... }  // 统一覆盖层

// ── 音频事件 ──
#[derive(Event)] pub struct PlayBgmEvent { ... }
#[derive(Event)] pub struct StopBgmEvent { ... }
#[derive(Event)] pub struct PlaySeEvent { ... }
#[derive(Event)] pub struct StopSeEvent { ... }
#[derive(Event)] pub struct PlayVoiceEvent { ... }
#[derive(Event)] pub struct SetVolumeEvent { ... }

// ── 引擎控制 ──
#[derive(Event)] pub struct AdvanceEvent { pub source: AdvanceSource }
#[derive(EntityEvent)] pub struct ScriptCommandComplete { pub cmd: String }

// ── 状态写入 (替代直接资源写入) ──
#[derive(Event)] pub struct UnlockCgEvent { pub image: String }
#[derive(Event)] pub struct UnlockBgmEvent { pub id: String }
#[derive(Event)] pub struct AffectionChangeEvent { pub char_key: String, pub delta: i32 }
#[derive(Event)] pub struct BacklogPushEvent { pub entry: BacklogEntry }
#[derive(Event)] pub struct SavePointEvent { pub id: String }
#[derive(Event)] pub struct VolumeChangeEvent { pub bgm: Option<f32>, pub se: Option<f32>, pub voice: Option<f32> }

// ── UI 状态 ──
#[derive(Event)] pub struct DialogueStateEvent { pub speaker: Option<String>, pub text: String }
#[derive(Event)] pub struct ChoiceStateEvent { pub options: Vec<ChoiceOption> }
#[derive(Event)] pub struct ChoiceSelectedEvent { pub index: usize }
```

**关键改进**: 旧引擎中 ScriptRunner 直接写入 `DialogueState`、`AffectionMap`、`UnlockState`、`Backlog`、`ChoiceState`、`Settings`、`GameRestrictions` 等 10+ 资源。新引擎**所有状态变更都通过 Event**，每个资源的写入者只有自己。

### 5.3 状态所有权模型

```rust
/// 状态写入者契约 —— 替代旧引擎中多个插件共享可变资源。
///
/// 每个状态资源有且仅有一个写入者。
/// ScriptRunner 通过 Event 请求状态变更，资源所有者消费 Event。

// ── 所有权表 ──
// DialogueState     → VnUiPlugin 拥有，消费 DialogueStateEvent
// ChoiceState       → VnUiPlugin 拥有，消费 ChoiceStateEvent
// AffectionMap      → 游戏插件拥有，消费 AffectionChangeEvent
// UnlockState       → 游戏插件拥有，消费 UnlockCg/Bgm/SceneEvent
// Backlog           → VnUiPlugin 拥有，消费 BacklogPushEvent
// Settings          → VnUiPlugin 拥有，消费 VolumeChangeEvent
// SaveManager       → VnSavePlugin 拥有，消费 SavePointEvent
```

### 5.4 存档系统 (`SaveStateProvider` trait)

```rust
// crates/bevy-vn-core/src/save.rs

/// 每个需要存档的子系统实现此 trait。
/// VnSavePlugin 在存档时遍历所有注册的 provider 收集状态。
pub trait SaveStateProvider: Send + Sync {
    /// 收集当前状态用于存档
    fn collect_save_data(&self, world: &World) -> serde_json::Value;

    /// 从存档数据恢复状态
    fn restore_save_data(&self, world: &mut World, data: &serde_json::Value) -> Result<(), String>;
}

/// 存档时收集的数据
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub timestamp: u64,
    pub screenshot: Option<Vec<u8>>,
    /// 每个系统自己的数据: key → JSON
    pub subsystems: HashMap<String, serde_json::Value>,
}
```

**关键改进**: 旧引擎的 `SaveData` 结构体硬编码了所有子系统字段 (`bg_file`, `cg_file`, `fg_sprites`, `dialogue_*`, `bgm_id`, ...)，加字段需要改结构体。新引擎使用 trait + JSON blob，各子系统自行定义自己的持久化格式，`SaveData` 不需要知道细节。

### 5.5 Scene Restore 解耦

旧引擎的 SaveLoad 直接操作渲染组件实体(`BgState`, `CgState`, `SpriteManager`)。新引擎中**存档恢复通过重放 Event**：

```rust
// VnSavePlugin 在加载存档时：
// 1. 恢复 ScriptEngine 状态 (current_script, current_line, call_stack, flags)
// 2. 清空所有渲染实体
// 3. 将脚本从开头快进到当前行，只执行渲染/音频 Event (跳过 Wait/Dialogue)
//    (这保证了渲染状态与脚本状态一致，不需要 SaveLoad 知道渲染内部细节)
```

这要求 `ScriptEngine` 支持 "dry-run" 模式：只产生渲染/音频 Event，不等待、不显示文本。

---

## 6. UI 配置与主题系统

### 6.1 三层覆盖模型

```
Layer 1: 引擎内置默认值 (VnTheme::default())
     │
     ▼  覆盖
Layer 2: RON 主题文件 (assets/theme.ron)
     │
     ▼  覆盖
Layer 3: bsn!{} 组件直接指定 (代码级单点覆盖)
```

### 6.2 VnTheme 结构

```rust
// crates/bevy-vn-core/src/theme.rs

/// 引擎全局主题配置。
/// 通过 RON 文件加载，运行时可通过资源修改。
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct VnTheme {
    /// 对话框主题
    pub dialogue: DialogueTheme,
    /// 选项主题
    pub choice: ChoiceTheme,
    /// 标题界面主题
    pub title: TitleTheme,
    /// 设置界面主题
    pub settings: SettingsTheme,
    /// 存档界面主题
    pub save_load: SaveLoadTheme,
    /// 历史记录主题
    pub backlog: BacklogTheme,
    /// 画廊主题
    pub gallery: GalleryTheme,
    /// 过渡动画参数
    pub transitions: TransitionTheme,
    /// 全局字体
    pub fonts: FontTheme,
    /// 全局颜色
    pub colors: ColorTheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTheme {
    /// 对话框相对于屏幕底部的位置
    pub margin_bottom: f32,           // 默认 20.0
    pub height: f32,                  // 默认 180.0
    /// 背景颜色 RGBA
    pub background_color: [f32; 4],   // 默认 [0.0, 0.0, 0.0, 0.7]
    /// 文字颜色
    pub text_color: [f32; 4],         // 默认 [1.0, 1.0, 1.0, 1.0]
    /// 说话人名字颜色
    pub speaker_color: [f32; 4],      // 默认 [0.8, 0.8, 1.0, 1.0]
    /// 说话人名字框宽度
    pub speaker_box_width: f32,       // 默认 200.0
    /// 字体大小 (像素, 使用时通过 px() 转为 FontSize)
    pub font_size: f32,               // 默认 28.0
    /// 说话人名字字体大小 (像素)
    pub speaker_font_size: f32,       // 默认 22.0
    /// 文字内边距
    pub padding: [f32; 4],            // 默认 [20, 20, 20, 20]
    /// 文本逐字显示速度 (覆盖 VnEngineConfig.text_speed)
    pub text_speed: f64,              // 默认 50.0
    /// 窗口设计: "default" | "transparent" | "minimal"
    pub design: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionTheme {
    /// 场景切换渐变时长
    pub fade_duration: f32,           // 默认 1.0
    /// 立绘淡入淡出时长
    pub fg_fade_duration: f32,        // 默认 0.5
    /// CG 切换时长
    pub cg_fade_duration: f32,        // 默认 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorTheme {
    /// 菜单背景
    pub menu_bg: [f32; 4],
    /// 菜单文字
    pub menu_text: [f32; 4],
    /// 按钮正常
    pub button_normal: [f32; 4],
    /// 按钮悬停
    pub button_hover: [f32; 4],
    /// 按钮按下
    pub button_press: [f32; 4],
    /// 选项高亮
    pub choice_highlight: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontTheme {
    /// 默认 UI 字体
    pub default: String,             // "fonts/default.ttf"
    /// 对话字体
    pub dialogue: Option<String>,    // None = 使用默认
    /// 标题字体
    pub title: Option<String>,
    /// 等宽字体 (设置界面)
    pub mono: Option<String>,
}
```

### 6.3 主题加载

```rust
// crates/bevy-vn-ui/src/theme.rs

/// 主题加载插件。在 Startup 阶段从 assets/theme.ron 加载主题。
/// 如果文件不存在，使用 VnTheme::default()。
pub struct VnThemePlugin;

impl Plugin for VnThemePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_theme);
    }
}

fn load_theme(mut commands: Commands, asset_server: Res<AssetServer>) {
    // 尝试加载 theme.ron
    // 注意: 这里用 include_str 作为 fallback，因为 RON 文件目前没有 AssetLoader
    // 未来可以注册 RON asset loader 实现热加载
    let theme = match std::fs::read_to_string("assets/theme.ron") {
        Ok(content) => ron::from_str::<VnTheme>(&content)
            .inspect_err(|e| warn!("Failed to parse theme.ron: {e}, using default"))
            .unwrap_or_default(),
        Err(_) => {
            info!("No theme.ron found, using default theme");
            VnTheme::default()
        }
    };
    commands.insert_resource(theme);
}
```

### 6.4 主题应用 (bsn!{} 覆盖)

```rust
// crates/bevy-vn-ui/src/dialogue.rs

/// 创建对话 UI 的 Scene 函数。读取 VnTheme 资源并应用。
fn dialogue_scene(theme: Res<VnTheme>) -> impl Scene {
    let dt = &theme.dialogue;
    let c = &theme.colors;

    bsn! {
        // 对话框根节点
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(dt.margin_bottom),
            left: Val::Px(0.0),
            width: percent(100),
            height: Val::Px(dt.height),
            padding: UiRect {
                left: Val::Px(dt.padding[0]),
                right: Val::Px(dt.padding[1]),
                top: Val::Px(dt.padding[2]),
                bottom: Val::Px(dt.padding[3]),
            },
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
        }
        BackgroundColor(Color::rgba(
            dt.background_color[0],
            dt.background_color[1],
            dt.background_color[2],
            dt.background_color[3],
        ))
        Name::new("DialogueRoot")

        Children [
            // 说话人名字
            (
                Text::new("")
                TextFont {
                    font_size: px(dt.speaker_font_size),
                    ..default()
                }
                TextColor(Color::rgba(
                    dt.speaker_color[0],
                    dt.speaker_color[1],
                    dt.speaker_color[2],
                    dt.speaker_color[3],
                ))
                Name::new("SpeakerName")
                Node {
                    width: Val::Px(dt.speaker_box_width),
                    height: Val::Px(dt.speaker_font_size + 4.0),
                }
            ),
            // 对话文本
            (
                Text::new("")
                TextFont {
                    font_size: px(dt.font_size),
                    ..default()
                }
                TextColor(Color::rgba(
                    dt.text_color[0],
                    dt.text_color[1],
                    dt.text_color[2],
                    dt.text_color[3],
                ))
                Name::new("DialogueText")
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                }
            ),
        ]
    }
}

/// 如果需要在 spawn 后覆盖特定属性，使用 apply_scene:
/// commands.spawn_scene(dialogue_scene(theme))
///         .apply_scene(bsn!{ BackgroundColor(MY_CUSTOM_COLOR) })
```

### 6.5 示例主题文件

```ron
// themes/default.ron
VnTheme(
    dialogue: DialogueTheme(
        margin_bottom: 20.0,
        height: 180.0,
        background_color: (0.0, 0.0, 0.0, 0.7),
        text_color: (1.0, 1.0, 1.0, 1.0),
        speaker_color: (0.8, 0.8, 1.0, 1.0),
        speaker_box_width: 200.0,
        font_size: 28.0,
        speaker_font_size: 22.0,
        padding: (20.0, 20.0, 20.0, 20.0),
        text_speed: 50.0,
        design: "default",
    ),
    choice: ChoiceTheme(
        max_visible: 6,
        item_height: 48.0,
        font_size: 24.0,
        padding: (12.0, 24.0, 12.0, 24.0),
    ),
    transitions: TransitionTheme(
        fade_duration: 1.0,
        fg_fade_duration: 0.5,
        cg_fade_duration: 1.0,
    ),
    fonts: FontTheme(
        default: "fonts/sourcehansans-medium.otf",
        dialogue: None,
        title: None,
        mono: None,
    ),
    colors: ColorTheme(
        menu_bg: (0.05, 0.05, 0.1, 0.95),
        menu_text: (0.9, 0.9, 0.95, 1.0),
        button_normal: (0.15, 0.15, 0.25, 1.0),
        button_hover: (0.25, 0.25, 0.40, 1.0),
        button_press: (0.35, 0.35, 0.50, 1.0),
        choice_highlight: (0.3, 0.5, 0.8, 1.0),
    ),
    // ... 其他主题
)
```

---

## 7. 资产管线

### 7.1 Bevy AssetServer 统一加载

```rust
// 资源加载对比

// 旧引擎:
// 编译期: build.rs → include_str! → 嵌入二进制
// 运行时: asset_pak.rs → mmap + zstd 解压 OR 文件系统回退
// 视频: current_dir()/assets/movie/ ← 绕过 AssetServer

// 新引擎:
// 编译期: 无 (build.rs 只做平台相关配置)
// 运行时: AssetServer::load("<path>") → 自动走 PAK reader 或文件系统
// 视频: AssetServer::load("movie/op.ogv") (通过自定义 VideoLoader)
```

### 7.2 资产路径提供者

```rust
// crates/bevy-vn-core/src/assets.rs

/// 资产路径映射 trait。游戏实现此 trait 来定义自己的资产查找逻辑。
pub trait VnAssetProvider: Send + Sync {
    /// 根据逻辑 ID 解析立绘路径
    fn fg_path(&self, char_id: &str, expression: &str) -> String;
    /// 根据逻辑 ID 解析背景路径
    fn bg_path(&self, image: &str) -> String;
    /// 根据逻辑 ID 解析 CG 路径
    fn cg_path(&self, image: &str) -> String;
    /// 根据逻辑 ID 解析 BGM 路径 (包含 A/B 段信息)
    fn bgm_path(&self, id: &str) -> BgmPathInfo;
    /// 根据逻辑 ID 解析 SE 路径
    fn se_path(&self, file: &str) -> String;
    /// 根据逻辑 ID 解析语音路径
    fn voice_path(&self, file: &str) -> String;
    /// 精灵覆盖层路径
    fn sprite_path(&self, id: &str) -> Option<String>;
}

/// 默认实现：简单的前缀拼接
pub struct DefaultAssetProvider {
    pub fg_dir: String,       // "image/obj"
    pub bg_dir: String,       // "image/bg"
    pub cg_dir: String,       // "image/ev"
    pub bgm_dir: String,      // "audio/bgm"
    pub se_dir: String,       // "audio/se"
    pub voice_dir: String,    // "audio/voice"
}

// 使用示例：
// app.insert_resource(DefaultAssetProvider { ... });
// render_plugin 通过 Res<VnAssetProvider> 获取路径
```

**关键改进**: 旧引擎中的路径映射分散在 7+ 个地方（`build.rs` 命名约定、`audio.rs` BGM 拼接、`rendering.rs` `char_dir`/`resolve_fg_path`、`script_runner.rs` `current_dir()` 路径、`mapper.rs` `aiy{:05}` 等）。新引擎通过单一 trait 集中管理，游戏开发者只需实现一个 trait 即可定制资产组织方式。

### 7.3 PAK Reader (从 asset_pak.rs 提取)

保持与旧引擎兼容的 BPAK 格式。核心改动：将硬编码的 bundle 名称列表参数化。

```rust
// crates/bevy-vn-core/src/pak.rs (或独立为 bevy_pak crate)

pub struct PakAssetReader {
    sources: Vec<PakSource>,
}

impl PakAssetReader {
    /// bundle_names: 按优先级排序的包名列表
    pub fn new(pak_dir: &Path, bundle_names: &[&str]) -> Self { ... }
}
```

---

## 8. 关键解耦设计

### 8.1 ScriptRunner 拆分

旧引擎的 `process_advance` (1878 行, 35+ SystemParam) → 拆分为：

```
ScriptRunnerPlugin
├── advance_system        (消费 AdvanceEvent → engine.advance() → dispatch)
│   ├── dispatch_dialogue   → DialogueStateEvent
│   ├── dispatch_control    → engine.jump/call/return
│   ├── dispatch_audio      → PlayBgm/PlaySe/... Events
│   ├── dispatch_render     → SetBg/ShowFg/... Events
│   ├── dispatch_effect     → ScreenEffect/Shake/... Events
│   ├── dispatch_state      → UnlockCg/BacklogPush/SavePoint/... Events
│   ├── dispatch_video      → PlayMovie/SpriteVideo/... Events
│   └── dispatch_custom     → CustomEvent (游戏注册)
├── auto_skip_system     (处理 Auto/Skip 模式定时器)
├── text_reveal_system   (逐字显示定时器)
└── persistence_system   (进入 Title 时持久化 UnlockState/Settings)
```

每个 `dispatch_*` 是独立函数，可测试。

### 8.2 Skip 模式去重

旧引擎中 skip 模式和 normal 模式是两个 500 行的重复 `match`。新引擎通过参数化单一循环消除重复：

```rust
struct AdvanceCtx {
    skip: bool,           // 是否跳过对话/等待
    suppress_se: bool,    // 是否压制 SE
}

fn dispatch_cmd(cmd: &ScriptCmd, ctx: &AdvanceCtx, world: &mut World) -> AdvanceResult {
    match cmd {
        ScriptCmd::Dialogue { .. } if ctx.skip => AdvanceResult::Continue,
        ScriptCmd::Wait { .. } if ctx.skip => AdvanceResult::Continue,
        ScriptCmd::PlaySe { .. } if ctx.suppress_se => AdvanceResult::Continue,
        // ... 其他全部复用同一份 dispatch 逻辑
        _ => dispatch_impl(cmd, world),
    }
}

enum AdvanceResult {
    Continue,   // 继续执行下一条
    Block,      // 等待外部事件 (文本显示完/音频播放完/视频EOS)
    Finished,   // 脚本结束
}
```

### 8.3 Screen 抽象 (消除 UI 样板)

旧引擎中每个界面 (Title/Menu/Settings/Gallery/SaveLoad/Backlog/RouteEnd/AfterStory) 都重复 `OnEnter` spawn → `Update` interaction → `OnExit` despawn 模式。

```rust
// crates/bevy-vn-ui/src/screen.rs

/// 通用 Screen trait。任何界面实现此 trait 即可获得标准的进入/更新/退出生命周期。
pub trait VnScreen: Send + Sync + 'static {
    /// 进入此界面时调用的 scene 函数
    fn enter(world: &mut World) -> impl Scene;
    /// 每帧更新
    fn update(world: &mut World);
    /// 退出时清理 (默认 despawn 所有带 ScreenMarker 组件的实体)
    fn exit(world: &mut World);

    /// 将此 Screen 注册为 Bevy Plugin
    fn plugin<S: States>(self, state: S) -> ScreenPlugin<Self, S>
    where Self: Sized { ... }
}

// 使用示例:
pub struct TitleScreen;
impl VnScreen for TitleScreen {
    fn enter(world: &mut World) -> impl Scene {
        let theme = world.resource::<VnTheme>();
        bsn! { /* title UI using theme.title */ }
    }
    fn update(world: &mut World) { /* handle button clicks */ }
    fn exit(world: &mut World) { /* cleanup */ }
}

app.add_plugins(TitleScreen.plugin(VnAppState::Title));
```

---

## 9. 实现阶段

### Phase 1: 核心引擎 (`bevy-vn-core`) — 2-3 周

- [ ] `VnCorePlugin` 结构: `VnEngineConfig`, `AppState`, 所有 Event 注册
- [ ] `VnScript` Asset 定义 + `VnScriptLoader`
- [ ] `ScriptCmd` 枚举 (版本化, `#[serde(other)]`)
- [ ] `ScriptEngine` 解释器 (纯数据, 可测试)
- [ ] 表达式求值器
- [ ] `VnTheme` 结构 + `Default` 实现
- [ ] `SaveStateProvider` trait
- [ ] `VnAssetProvider` trait
- [ ] 单元测试覆盖 `ScriptEngine` 核心路径

### Phase 2: 渲染插件 (`bevy-vn-render`) — 2 周

- [ ] `VnRenderPlugin`: 双缓冲 BG、可配置槽位 FG、CG 层
- [ ] 消费所有渲染 Event
- [ ] 屏幕覆盖层系统 (合并 Flash/FadeScene/ScreenOverlay → `ScreenEffect`)
- [ ] 通用 Sprite 覆盖层
- [ ] 震动效果
- [ ] 通过 `VnAssetProvider` 解析资产路径

### Phase 3: 音频插件 (`bevy-vn-audio`) — 1 周

- [ ] `VnAudioPlugin`: BGM/SE/Voice 管理
- [ ] BGM A/B 段拼接 (可选, 通过 `VnAssetProvider` 配置)
- [ ] 消费所有音频 Event
- [ ] `SetVolume` 统一音量控制

### Phase 4: UI 插件 (`bevy-vn-ui`) — 3 周

- [ ] `VnScreen` trait + 通用生命周期
- [ ] `VnThemePlugin`: 主题加载 + 应用
- [ ] 对话框 (逐字显示, 说话人名字, 主题化)
- [ ] 选项系统 (通过 `ChoiceStateEvent` 触发, `ChoiceSelectedEvent` 响应)
- [ ] 历史记录 Backlog
- [ ] 标题界面
- [ ] 设置界面
- [ ] 存档/读档 UI
- [ ] CG 画廊
- [ ] 路线选择界面

### Phase 5: 存档系统 (`bevy-vn-save`) — 1 周

- [ ] `VnSavePlugin`: 通过 `SaveStateProvider` trait 收集/恢复状态
- [ ] JSON 序列化
- [ ] 存档缩略图
- [ ] 快进恢复 (dry-run ScriptEngine 到当前行)

### Phase 6: 视频插件 (`bevy-vn-video`) — 1 周

- [ ] `VnVideoPlugin`: GStreamer (桌面) / FFmpeg (Android)
- [ ] 通过 `AssetServer` 加载视频资源
- [ ] EOS → `AdvanceEvent` 恢复

### Phase 7: 工具链 — 2 周

- [ ] `bevy-vn-asset-packer`: 从旧 `tools/asset_packer` 提取，参数化 bundle 名
- [ ] `artemis-converter`: 从旧 `tools/artemis-export` 提取，输出新版 `VnScript`
- [ ] 示例项目 (`examples/minimal/`)

### Phase 8: 旧项目迁移验证 — 1 周

- [ ] Eustia 项目的 `routes.ron` → 新配置格式
- [ ] Artemis 脚本 → `.vnscript.ron` 转换测试
- [ ] 集成测试: 完整游戏流程

---

## 附录 A: 与旧引擎的对比总结

| 维度 | 旧引擎 (bevy-vn-test) | 新引擎 (bevy-vn-engine) |
|------|----------------------|------------------------|
| Bevy 版本 | 0.18 (Message API) | 0.19 (bsn!, SceneComponent, EntityEvent) |
| 实体创建 | `spawn(Bundle)` + 手写 system | `bsn!{}` 场景组合 + `on()` 内联观察者 |
| 脚本加载 | build.rs include_str! 编译期嵌入 | AssetServer 运行时加载 + 热重载 |
| ScriptCmd | 78 扁平变体, 无版本化 | 分组变体 + `#[serde(other)]` + ScriptVersion |
| 插件通信 | 直接读写其他插件的资源 | Message/Event + trait |
| 存档/渲染耦合 | SaveLoad 直接操作 BgState/CgState/SpriteManager | 存档通过 dry-run 重放 Event 恢复渲染状态 |
| UI 样式 | 全部硬编码 | VnTheme RON 配置 + bsn!{} 覆盖 |
| 资产路径 | 分散在 7+ 个文件中硬编码 | VnAssetProvider trait 集中管理 |
| 状态机 | 15+ 同级 AppState 变体 | 顶层 AppState + SubStates 子状态 |
| 重复代码 | skip/normal 两个 500 行 match | 参数化单一 dispatch 循环 |
| 脚本命名 | aiy00010 +10 递增硬编码 | meta.next_script 显式指定 |
| 好感度系统 | HEROINE_WORK_MAP 硬编码 + 与 routes.ron 不一致 | ChoiceOption.affection + 游戏插件自定义处理 |
| 音量控制 | "MIN"/"LOW"/"NORM" 字符串 | 0.0-1.0 f32 |
| 游戏特定数据 | VIEW_TABLE, 魔法数字遍布 | 全部移入游戏插件/配置文件 |
| 构建依赖 | Build script 扫描 4 个资产目录 | 无编译期资产扫描 |
| Crate 结构 | 单体 workspace (cdylib + 2 tools) | 6 个独立 crate + 2 tools |

## 附录 B: 新引擎引入方式 (用户视角)

```toml
# 用户项目的 Cargo.toml
[dependencies]
bevy = "0.19"
bevy-vn-core = { git = "..." }
bevy-vn-render = { git = "..." }
bevy-vn-audio = { git = "..." }
bevy-vn-ui = { git = "..." }
bevy-vn-save = { git = "..." }
# bevy-vn-video 是可选的
```

```rust
// 用户项目的 main.rs
use bevy::prelude::*;
use bevy_vn_core::prelude::*;
use bevy_vn_render::VnRenderPlugin;
use bevy_vn_audio::VnAudioPlugin;
use bevy_vn_ui::VnUiPlugin;
use bevy_vn_save::VnSavePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VnCorePlugin {
            config: VnEngineConfig {
                resolution: (1280.0, 720.0),
                default_font: "fonts/NotoSansSC-Regular.otf".into(),
                ..default()
            }
        })
        .add_plugins(VnRenderPlugin)
        .add_plugins(VnAudioPlugin)
        .add_plugins(VnUiPlugin)
        .add_plugins(VnSavePlugin)
        .add_plugins(MyGamePlugin)  // 用户自己的游戏逻辑
        .run();
}
```
