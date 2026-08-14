# Bevy 0.19 API 参考文档

> 基于本地缓存的 Bevy 0.19.0 源代码验证（`~/.cargo/registry/src/rsproxy.cn-e3de039b2554c837/`）
> 发布日期: 2026-06-19 | 目标: bevy-vn-engine 项目开发对接

---

## 版本确认

| 项目 | 版本 |
|------|------|
| bevy (meta-crate) | 0.19.0 |
| bevy_ecs | 0.19.0 |
| bevy_scene | 0.19.0 (新 BSN 场景系统) |
| bevy_scene_macros | 0.19.0 (`bsn!{}` 宏实现) |
| bevy_ui | 0.19.0 |
| bevy_text | 0.19.0 |
| bevy_asset | 0.19.0 |
| bevy_audio | 0.19.0 |

**Cargo.toml 依赖写法**:
```toml
[dependencies]
bevy = "0.19"
```

---

## 1. `bsn!{}` / `bsn_list!` — Bevy Scene Notation

### 1.1 基本语法

```rust
use bevy::prelude::*;

// 单根实体的场景
fn my_scene() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
        }
        BackgroundColor(Color::srgb(0.1, 0.1, 0.15))
        Children [
            (Text("Hello") TextFont { font_size: px(28.0) }),
            (Text("World") TextFont { font_size: px(28.0) }),
        ]
    }
}

// 多根实体的场景列表
fn my_scene_list() -> impl SceneList {
    bsn_list![
        Camera2d,
        my_scene(),
    ]
}
```

### 1.2 入口前缀速查

| 前缀 / 语法 | 含义 | 示例 |
|------------|------|------|
| `Type { fields }` | 插入/修补组件 (FromTemplate) | `Node { width: px(100) }` |
| `~Type { fields }` | 原始 Template 修补 | `~MyTemplate { x: 1 }` |
| `#Name` | 命名实体引用 | `#DialogueRoot` |
| `:` | 缓存场景 (必须是第一个条目) | `:"scene.asset"` — 暂无 loader |
| `{expr} / @Type` | 未缓存场景包含 | `@MyWidget { @prop: val }` |
| `Type [ ... ]` | 关系场景列表 (Children) | `Children [ ... ]` |
| `on(\|evt: On<E>\| ...)` | 内联观察者 | `on(\|_e: On\<Pointer\<Press\>\>\| ...)` |
| `"string"` (字段值) | 资产路径 → HandleTemplate | `image: "player.png"` |

### 1.3 实体分隔规则（关键）

```rust
// Children / bsn_list! 中:
// 逗号 = 分隔实体, 空白 = 同实体内多个组件
Children [
    A B C,        // 实体1: 组件 A, B, C
    D,            // 实体2: 组件 D
    (E F),        // 实体3: 组件 E, F (括号分组)
]
```

### 1.4 资产在 bsn! 中的使用

```rust
bsn! {
    // 方式1: 字符串自动解析为 Handle
    Sprite { image: "player.png" }

    // 方式2: asset_value() 函数 (需要 AssetServer loaded)
    Mesh3d(asset_value(Cuboid::new(1.0, 1.0, 1.0)))

    // 方式3: template_value() 设置非 FromTemplate 值
    template_value(Transform::from_xyz(4.0, 8.0, 4.0))

    // 方式4: 字体 via FontSourceTemplate
    TextFont {
        font: FontSourceTemplate::Handle("fonts/my-font.ttf"),
        font_size: px(28.0),
    }
}
```

### 1.5 SceneComponent — 标记组件 + 场景绑定

```rust
#[derive(SceneComponent, Default, Clone)]
struct DialogueBox {
    speaker_name: String,
}

impl DialogueBox {
    fn scene(props: Self) -> impl Scene {
        bsn! {
            Node { /* 对话框布局 */ }
            Children [
                (Text(props.speaker_name) Name::new("Speaker")),
                (Text("") Name::new("DialogueText")),
            ]
        }
    }
}

// 使用:
world.spawn_scene(bsn! { @DialogueBox { @speaker_name: "菲奥奈".into() } });
// World::spawn(DialogueBox::default()) ← debug 模式会报错
```

### 1.6 Spawn / Apply API

```rust
// World
world.spawn_scene(my_scene())?;           // 立即 → Result<EntityWorldMut, SpawnSceneError>
world.queue_spawn_scene(my_scene());       // 排队等待资产加载
world.spawn_scene_list(bsn_list![...])?;   // 多根实体
world.queue_spawn_scene_list(bsn_list![...]);

// Commands
commands.spawn_scene(my_scene());          // 返回 EntityCommands
commands.queue_spawn_scene(my_scene());
commands.spawn_scene_list(bsn_list![...]);
commands.queue_spawn_scene_list(bsn_list![...]);

// 应用到已有实体 (修补)
entity.apply_scene(bsn! { BackgroundColor(RED) })?;
entity.queue_spawn_related_scenes::<Children>(bsn_list![...]);

// 系统函数适配器 (推荐)
fn my_scene() -> impl SceneList { bsn_list![Camera2d, ui()] }

App::new()
    .add_systems(Startup, my_scene.spawn())  // ← .spawn() 适配器
    .run();
```

### 1.7 `#Name` 实体引用

```rust
bsn! {
    #Parent
    Children [
        #Child1 SomeComponent,
        (#Child2 ComponentB Reference(#Parent)),  // 前向引用
    ]
}
// 同一 bsn! 调用内所有 #Name 可见
// bsn_list! 内所有根实体共享同一作用域
// 嵌套的 bsn! (场景函数) 各自独立作用域
```

### 1.8 限制与注意事项

- **无 `.bsn` 文件 loader** — 0.19 只支持代码内 `bsn!{}`，文件格式在后续版本
- **缓存场景** (`:"path"`) 必须是第一个条目且仅能有一个
- SceneComponent 必须通过 scene API spawn，`world.spawn()` 会报错
- 元组最多 12 个 scene 元素 (通过 `auto_nest_tuple!` 自动嵌套)

---

## 2. 组件系统 (ECS)

### 2.1 `#[require(...)]` — 必需组件

```rust
#[derive(Component)]
#[require(B)]                           // B: Default 自动插入
#[require(B, C)]                        // 多个
#[require(
    B(1),                               // 元组构造
    C { x: 1 },                         // 命名字段
    D::One,                             // 枚举变体
    E::new(1)                           // 关联函数
)]
#[require(C = init_c())]                // 任意表达式
struct A;

// 优先级: 直接 require > 继承的 require (DFS 先到先得)
// 运行时注册 (必须在首次插入前):
world.register_required_components::<A, B>();
world.register_required_components_with::<A, B>(|| B(42));
```

### 2.2 组件生命周期勾子

```rust
// 方式1: derive 属性
#[derive(Component)]
#[component(on_add = my_on_add)]       // 插入新组件时
#[component(on_insert = my_on_insert)] // 任何插入时
#[component(on_discard = my_on_discard)] // 替换/移除前 (0.18 叫 on_replace)
#[component(on_remove = my_on_remove)] // 移除后
#[component(on_despawn = my_on_despawn)] // 实体销毁时
struct MyComp;

// 方式2: impl Component
impl Component for MyComp {
    fn on_add() -> Option<ComponentHook> { Some(my_on_add) }
}

// 勾子签名:
type ComponentHook = for<'w> fn(DeferredWorld<'w>, HookContext);
// HookContext { entity, component_id, caller, relationship_hook_mode }

// 运行时注册 (组件未使用时):
world.register_component_hooks::<MyComp>()
    .on_add(|mut world, HookContext { entity, .. }| {
        world.resource_mut::<MyRes>().insert(entity);
        world.write_message(MyMessage);     // 0.19 使用 Message
    })
    .on_remove(|mut world, HookContext { entity, .. }| {
        world.commands().entity(entity).despawn();
    });
```

### 2.3 Resource 现在是 Component

```rust
// 0.19: Resource 是 Component 的子 trait
pub trait Resource: Component {}

#[derive(Resource)]  // 同时实现了 Component + Resource
struct MySettings { volume: f32 }

// 影响:
// 1. 不能再同时 derive Component + Resource
// 2. 宽泛查询 (Query<EntityMut>, Query<Option<&T>>) 会看到资源实体
//    → 添加 Without<IsResource> 过滤
// 3. ResMut 现在要求 R: Resource<Mutability = Mutable>
// 4. rename: init_non_send_resource → init_non_send
//             non_send_resource → non_send
//             register_resource → register_component
```

---

## 3. 事件与观察者

### 3.1 Event 和 EntityEvent

```rust
// 全局事件
#[derive(Event)]
struct GameOver { score: u32 }

// 实体事件 (目标特定实体)
#[derive(EntityEvent)]
struct DialogueComplete { entity: Entity }

// 自定义 Trigger
#[derive(Event)]
#[event(trigger = MyTrigger)]
struct CustomEvent;
```

### 3.2 观察者注册

```rust
// 全局观察者
app.add_observer(|event: On<GameOver>| { /* ... */ });

// 实体观察者 (实体销毁时自动移除)
commands.spawn(MyComp)
    .observe(|event: On<DialogueComplete>, mut q: Query<&mut MyComp>| {
        // 只对绑定的实体触发
    });

// 观察者复用
let mut observer = Observer::new(my_handler);
observer.watch_entity(e1);
observer.watch_entity(e2);
world.spawn(observer);

// 带运行条件的观察者
app.add_observer(my_handler.run_if(|enabled: Res<MyFlag>| enabled.0));

// 组件生命周期观察者 (替代 Added<T> / RemovedComponents<T>)
app.add_observer(|add: On<Add, MyComp>, q: Query<&MyComp>| { /* ... */ });
app.add_observer(|remove: On<Remove, MyComp>| { /* ... */ });
```

### 3.3 触发事件

```rust
// 全局
commands.trigger(GameOver { score: 100 });
world.trigger(ExplodeMines { pos: Vec2::ZERO, radius: 10.0 });

// 实体
commands.entity(e).trigger(DialogueComplete { entity: e });
// 或通过闭包
entity_mut.trigger(|entity| DialogueComplete { entity });

// 在 bsn! 中内联观察者
bsn! {
    Button
    on(|_press: On<Pointer<Press>>| info!("clicked"))
}
```

### 3.4 EntityComponentsTrigger 变化 (0.18→0.19)

```rust
// 0.18
let EntityComponentsTrigger { components } = e.trigger();
// 0.19 — 新字段
let EntityComponentsTrigger { components, .. } = e.trigger();
// 新增: old_archetype: Option<&Archetype>, new_archetype: Option<&Archetype>
```

---

## 4. UI 系统

### 4.1 Node 组件

```rust
// 0.19 Node 字段 (直接字段, 无 Style 结构体)
#[derive(Component)]
pub struct Node {
    pub display: Display,
    pub overflow: Overflow,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
    pub justify_content: JustifyContent,
    pub gap: UiRect,
    pub row_gap: Val,
    pub column_gap: Val,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Val,
    pub inset: UiRect,
    pub width: Val,
    pub height: Val,
    pub min_width: Val,
    pub min_height: Val,
    pub max_width: Val,
    pub max_height: Val,
    pub aspect_ratio: Option<f32>,
    pub position_type: PositionType,
    pub margin: UiRect,
    pub padding: UiRect,
    pub border: UiRect,
    // 0.19 新增 ↓
    pub direction: InlineDirection,     // Ltr | Rtl (默认 Ltr)
}

// Val 新增方法
Val::try_add(other) -> Result<Val, ValArithmeticError>
Val::try_sub(other) -> Result<Val, ValArithmeticError>

// Val::resolve 签名变化
// 0.18: resolve(self, parent_size, viewport_size) → f32
// 0.19: resolve(self, rem, parent_size, viewport_size) → f32
```

### 4.2 UI 相关组件

```rust
// 不变
BackgroundColor(pub Color)
BorderColor { pub top, right, bottom, left: Color }   // 已是多字段
BorderRadius { pub top_left, top_right, bottom_left, bottom_right: Val }
UiRect { pub left, right, top, bottom: Val }
ZIndex(pub u16)
GlobalZIndex(pub u16)
Interaction { Pressed, Hovered, None }

// 0.19 新增
OuterColor(pub Color)                    // 外描边颜色
ComputedStackIndex(pub u32)             // 替代 ComputedNode.stack_index
InlineDirection { Ltr, Rtl }            // 文本方向

// 交互组件
Pressed, Hovered, InteractionDisabled   // interaction_states 模块
Selectable, Selected                    // 0.19 新增 (选择/选中)
Checkable, Checked                      // 复选框
```

### 4.3 UI 快捷函数

```rust
// 大小
px(100.0)              // Val::Px(100.)
percent(50.0)          // Val::Percent(50.)

// UiRect 构建
px(12).all()           // UiRect::all(Val::Px(12.))
px(12).left()          // UiRect::left(Val::Px(12.))
px(4).top()
```

### 4.4 ImageNode (图片显示)

```rust
// 0.19: 新增 visual_box 字段
#[derive(Component, FromTemplate)]
#[require(Node, ImageNodeSize)]
pub struct ImageNode {
    pub image: Handle<Image>,
    pub visual_box: VisualBox,       // 0.19 新增
}

#[derive(Component)]
#[require(ContentSize)]
pub struct ImageNodeSize {
    pub size: Option<Vec2>,
    pub visual_box: VisualBox,       // 0.19 新增
}
```

---

## 5. 文本系统

### 5.1 Text 组件 (UI)

```rust
// 0.19: 新增 LetterSpacing 必需组件
#[derive(Component)]
#[require(
    Node, TextLayout, TextFont, TextColor,
    LineHeight, LetterSpacing,        // ← 0.19 新增 LetterSpacing
    TextNodeFlags, ContentSize,
    FontHinting::Enabled              // 0.19: 默认 Enabled (0.18: Disabled)
)]
pub struct Text(pub String);

// TextSpan (富文本子段)
#[derive(Component)]
#[require(TextFont, TextColor, LineHeight, LetterSpacing)]
pub struct TextSpan;

// TextLayout
#[derive(Component)]
#[require(ComputedTextBlock, TextLayoutInfo)]
pub struct TextLayout {
    pub justify: Justify,
    pub linebreak: LineBreak,
}

// TextLayout 构造方法 rename:
// 0.18                         →  0.19
TextLayout::new_with_justify(j) → TextLayout::justify(j)
TextLayout::new_with_linebreak(l) → TextLayout::linebreak(l)
TextLayout::new_with_no_wrap()   → TextLayout::no_wrap()
```

### 5.2 TextFont — 字体配置 (0.19 大改)

```rust
#[derive(Component, Clone)]
pub struct TextFont {
    // 0.18: font: Handle<Font>, font_size: f32
    // 0.19:
    pub font: FontSource,              // 字体来源 (枚举)
    pub font_size: FontSize,           // 大小 (枚举, 支持多单位)
    pub font_smoothing: FontSmoothing, // 0.19 新增: 抗锯齿
    pub width: FontWidth,              // 0.19 新增
    pub style: FontStyle,              // 0.19 新增: Normal | Italic
    pub font_variations: FontVariations, // 0.19 新增: 可变字体
}

// FontSource 枚举
pub enum FontSource {
    Handle(Handle<Font>),
    Family(FontFamily),
    Serif, SansSerif, Cursive, Fantasy, Monospace,
    SystemUi, UiSerif, UiSansSerif, UiMonospace,
    UiRounded, Emoji, Math, FangSong,
}

// FontSize 枚举
pub enum FontSize {
    Px(f32),        // 像素
    Rem(f32),       // 相对于根字体大小
    Vw(f32),        // 视口宽度百分比
    Vh(f32),        // 视口高度百分比
    VMin(f32),      // 视口较小边百分比
    VMax(f32),      // 视口较大边百分比
}

// FontSmoothing
pub enum FontSmoothing {
    None,
    Antialiased,
    SubpixelAntiAliased,
}

// 0.19 新组件
#[derive(Component, Default, Clone)]
pub struct LetterSpacing(pub f32);   // 字间距

// 文本装饰组件 (不变)
#[derive(Component)] pub struct TextColor(pub Color);
#[derive(Component)] pub struct TextBackgroundColor(pub Color);
#[derive(Component)] pub struct Underline;
#[derive(Component)] pub struct Strikethrough;
#[derive(Component)] pub struct TextShadow;
#[derive(Component)] pub struct LineHeight(pub f32);

// 使用示例
bsn! {
    Text("Hello World")
    TextFont {
        font: FontSource::Handle(asset_server.load("fonts/my-font.ttf")),
        // 或: font: FontSourceTemplate::Handle("fonts/my-font.ttf")  ← bsn! 内
        font_size: px(28.0),
        font_smoothing: FontSmoothing::Antialiased,
    }
    TextColor(Color::WHITE)
    LetterSpacing(1.5)
    LineHeight(1.2)
}
```

### 5.3 Font 加载变化

```rust
// 0.18
let font = Font::try_from_bytes(bytes)?;

// 0.19 — 不可失败, 无 family name 参数
let font = Font::from_bytes(bytes);
// Font 新增 alias: String 字段
```

### 5.4 Text2d (2D 世界空间文本)

```rust
// 0.19: 同样新增 LetterSpacing 必需
#[derive(Component)]
#[require(Transform, TextLayout, TextFont, TextColor, LineHeight, LetterSpacing)]
pub struct Text2d(pub String);
```

### 5.5 辅助资源 (bevy_text)

```rust
#[derive(Resource)] pub struct FontCx;    // 字体上下文 (替代旧 CosmicFontSystem)
#[derive(Resource)] pub struct LayoutCx;  // 排版上下文
#[derive(Resource)] pub struct ScaleCx;   // 缩放上下文
#[derive(Resource)] pub struct RemSize(pub f32); // 根字体大小 (用于 Rem 单位)
```

---

## 6. 资产系统

### 6.1 AssetServer

```rust
// 基础加载
asset_server.load::<Font>("fonts/my-font.ttf")
asset_server.load::<Image>("images/bg.png")
asset_server.load::<OpusAudio>("audio/bgm.opus") // bevy-opus-audio crate

// LoadBuilder — 批量加载
asset_server.load_builder()
    .load::<Image>("images/bg1.png")
    .load::<Image>("images/bg2.png")
    .load_untyped("scripts/main.vnscript.ron")
    .build()

// 加载文件夹
asset_server.load_folder("images/ev/")

// AssetPath 变化
let path = AssetPath::from("images/bg.png");
path.resolve(&asset_server);        // 获取最终路径
path.get_full_extension();          // 获取完整扩展名 (含多段)
```

### 6.2 自定义 Asset / AssetLoader

```rust
#[derive(Asset, TypePath)]
pub struct VnScriptAsset {
    pub script: VnScript,
}

pub struct VnScriptLoader;

impl AssetLoader for VnScriptLoader {
    type Asset = VnScriptAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let script: VnScript = ron::de::from_bytes(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(VnScriptAsset { script })
    }

    fn extensions(&self) -> &[&str] {
        &["vnscript.ron"]
    }
}

// 注册
app.init_asset::<VnScriptAsset>()
   .register_asset_loader(VnScriptLoader);
```

### 6.3 Handle 变化

```rust
// Handle 现在 derive FromTemplate (用于 bsn! 模板)
// 0.19 新增:
Handle::weak_from(strong_handle)
```

---

## 7. 音频系统

### 7.1 AudioPlayer

```rust
// 0.19: AudioPlayer 现在 derive FromTemplate
#[derive(Component, FromTemplate)]
#[require(PlaybackSettings)]
pub struct AudioPlayer<Source = AudioSource>(pub Handle<Source>);
// 引擎实际用 bevy-opus-audio 的 OpusAudio 替代默认 AudioSource：
//   AudioPlayer::<OpusAudio>(handle)

// AudioPlugin 不变
app.add_plugins(AudioPlugin::default());
```

### 7.2 音频播放

```rust
// 背景音乐 (Loop)
commands.spawn((
    AudioPlayer::<OpusAudio>(asset_server.load::<OpusAudio>("audio/bgm/title.opus")),
    PlaybackSettings {
        mode: PlaybackMode::Loop,
        volume: Volume::new(0.8),
        ..default()
    },
));

// 音效 (OneShot — PlaybackSettings::DESPAWN)
commands.spawn((
    AudioPlayer::<OpusAudio>(asset_server.load::<OpusAudio>("audio/se/click.opus")),
    PlaybackSettings::DESPAWN,
));

// 音量控制
use bevy::audio::Volume;
Volume::new(0.5);                    // 0.0 - 1.0
Volume::ZERO;
```

### 7.3 依赖变化

```rust
// bevy_audio 重新导出 rodio 类型
use bevy::audio::{ChannelCount, SampleRate};

// 0.19: Decodable trait 移除 DecoderItem 关联类型
// (Decoder 现在直接迭代 rodio::Sample = f32)
```

---

## 8. 窗口系统

### 8.1 Window 组件

```rust
// 0.19 新增字段
pub struct Window {
    // ... 原有字段 ...
    pub borderless_game: bool,  // 0.19 新增, 默认 true
}

// 0.19 新组件
#[derive(Component)]
#[relationship(relationship_target = HasWindows)]
pub struct OnMonitor(pub Entity);  // 窗口绑定到显示器

// ExitSystems SystemSet (退出系统现在在 Last schedule)
```

### 8.2 窗口插件

```rust
// DefaultPlugins 中包含
app.add_plugins(DefaultPlugins.set(WindowPlugin {
    primary_window: Some(Window {
        title: "Bevy VN Engine".into(),
        resolution: (1280.0, 720.0).into(),
        present_mode: PresentMode::AutoVsync,
        ..default()
    }),
    ..default()
}));
```

---

## 9. 从 0.18 到 0.19 的迁移速查 (VN 引擎相关)

### 立即需要的修改

| 0.18 写法 | 0.19 写法 |
|-----------|-----------|
| `bevy = "0.18"` | `bevy = "0.19"` |
| `Event` / `EventWriter` / `add_event` | `Event` / `EventWriter` / `add_event` (不变!) |
| `Message` / `MessageWriter` / `add_message` | 0.18 专属; 0.19 恢复为 Event |
| `commands.spawn(Bundle)` | `commands.spawn_scene(bsn!{...})` (推荐) |
| `app.add_systems(Startup, setup)` | `app.add_systems(Startup, scene.spawn())` |
| `TextFont { font: handle, font_size: 28.0 }` | `TextFont { font: FontSource::Handle(handle), font_size: px(28.0) }` |
| `Font::try_from_bytes(bytes)?` | `Font::from_bytes(bytes)` |
| `ComputedNode { stack_index, .. }` | `ComputedNode { .. }` + separate `ComputedStackIndex` |
| `TextLayout::new_with_justify(j)` | `TextLayout::justify(j)` |
| `#[component(on_replace = ...)]` | `#[component(on_discard = ...)]` |
| `init_non_send_resource::<T>()` | `init_non_send::<T>()` |
| `register_resource::<T>()` | `register_component::<T>()` |
| `entities_allocator` | `entity_allocator` |
| `bevy_scene::*` (旧反射场景) | `bevy_world_serialization::*` |
| `bevy::scene::bsn!` | `bevy::prelude::bsn!` |
| `spawn(TransformBundle::default())` | `Transform::default()` 直接作为组件 |
| `Camera2dBundle::default()` | `Camera2d` 直接作为组件 |

### 不需要改的

| 项目 | 说明 |
|------|------|
| `Query`, `Res`, `ResMut`, `Commands` | API 不变 |
| `App`, `Plugin`, `States`, `OnEnter/OnExit` | API 不变 |
| `Handle<T>`, `AssetServer::load` | API 不变 |
| `BackgroundColor`, `BorderColor`, `ZIndex` | 不变 |
| `Interaction { Pressed, Hovered, None }` | 不变 |
| `Node` 字段 (除新增 direction) | 布局字段不变 |
| `Window`, `ClearColor` | 字段几乎不变 |
| `bevy_framepace` | 外部 crate, API 不变 |
| `bevy_android` | 外部 crate, API 不变 |

### 可选改进 (不阻塞迁移)

| 旧模式 | 新模式 |
|--------|--------|
| Bundle 函数传 AssetServer | `bsn!{}` + 字符串资产路径 |
| `spawn((A, B, C))` | `bsn! { A B C }` |
| `commands.entity(e).observe(f)` | 同上 + `on(\|...\| ...)` in bsn! |
| `Added<T>` 查询 | `On<Add, T>` 观察者 |
| `children![...]` 宏 | `Children [ ... ]` in bsn! |
| `commands.trigger_targets(event, entity)` | `commands.entity(entity).trigger(event)` |

---

## 10. 关键源文件路径 (本地缓存)

```
基础路径: ~/.cargo/registry/src/rsproxy.cn-e3de039b2554c837/

bevy-0.19.0/                       元 crate (pub use 重导出)
bevy_scene-0.19.0/src/lib.rs       场景系统 (Scene, SceneComponent, bsn! 文档)
bevy_scene-0.19.0/src/scene.rs     场景 trait (Scene, TemplatePatch, OnTemplate)
bevy_scene-0.19.0/src/spawn.rs     生成 API (WorldSceneExt, CommandsSceneExt)
bevy_scene-0.19.0/src/scene_patch.rs 场景资产 (ScenePatch, ScenePatchInstance)
bevy_scene_macros-0.19.0/src/bsn/parse.rs   bsn! 语法解析 (前缀、字段、值)
bevy_scene_macros-0.19.0/src/bsn/codegen.rs bsn! 代码生成
bevy_ecs-0.19.0/src/component/mod.rs     Component trait, #[require], #[component]
bevy_ecs-0.19.0/src/lifecycle.rs         ComponentHooks, HookContext
bevy_ecs-0.19.0/src/observer/            Observer, ObserverState, On 系统参数
bevy_ecs-0.19.0/src/event/mod.rs         Event, EntityEvent trait
bevy_ecs-0.19.0/src/resource.rs          Resource: Component (pub trait Resource: Component {})
bevy_ui-0.19.0/src/ui_node.rs            Node, Style, BackgroundColor, BorderColor
bevy_text-0.19.0/src/text.rs             Text, TextFont, TextSpan, TextLayout
bevy_ui-0.19.0/src/widget/text.rs        UI Text widget (require 列表)
bevy_asset-0.19.0/src/server/mod.rs      AssetServer, LoadBuilder
bevy_asset-0.19.0/src/loader.rs          AssetLoader trait
bevy_audio-0.19.0/src/audio_source.rs    AudioPlayer, PlaybackSettings
bevy_window-0.19.0/src/window.rs         Window, WindowPlugin

迁移指南:
bevy-0.19.0/_release-content/migration-guides/
  resources_as_components.md       Resource is Component
  lifecycle-events.md              on_replace → on_discard
  observer_old_new_archetype.md    EntityComponentsTrigger 新字段
  text_section.md                  TextSection trait
  TextFont_font_and_font_size_changes.md  FontSource / FontSize
  text_layout_renames.md           TextLayout 构造方法 rename
  new_ComputedTextBlock_needs_rerender_parameters.md
  computed_stack_index.md          ComputedStackIndex 独立
  bevy_scene_rename.md             bevy_scene → bevy_world_serialization
  rodio_0_22.md                    rodio 版本升级
  audio_feature.md                 audio feature 不再隐含
  font_from_bytes.md               Font::from_bytes infallible
  load_builder.md                  LoadBuilder API
  assetpath-resolve-semantics.md   AssetPath::resolve
  non_experimental_ui.md           UI feature 默认
  Node_inline_direction.md         Node::direction 新字段
  Core-Prefix-Removal.md           组件 prefix 移除
  entity_allocator.md              entities_allocator → entity_allocator

示例:
bevy-0.19.0/examples/scene/bsn.rs           bsn! 官方示例
bevy-0.19.0/examples/ecs/component_hooks.rs 组件勾子
bevy-0.19.0/examples/ecs/observers.rs       观察者系统
bevy-0.19.0/examples/ui/text/text.rs        文本渲染
bevy-0.19.0/examples/ui/widgets/            新版 UI widgets
bevy-0.19.0/examples/3d/3d_scene.rs         bsn! 3D 场景
```

---

## 附录: 项目 Cargo.toml 模板

```toml
[package]
name = "my-vn-game"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = { version = "0.19", features = [
    "bevy_asset",
    "bevy_audio",
    "jpeg",
] }
bevy-opus-audio = "0.1"
# bevy-vn-core = { path = "../crates/bevy-vn-core" }
# bevy-vn-render = { path = "../crates/bevy-vn-render" }
# bevy-vn-audio = { path = "../crates/bevy-vn-audio" }
# bevy-vn-ui = { path = "../crates/bevy-vn-ui" }
# bevy-vn-save = { path = "../crates/bevy-vn-save" }
# bevy-vn-video = { path = "../crates/bevy-vn-video" }
serde = { version = "1", features = ["derive"] }
ron = "0.8"

# 如果需要视频播放
# [target.'cfg(not(target_os = "android"))'.dependencies]
# gstreamer = "0.25"
# gstreamer-video = "0.25"
# gstreamer-app = "0.25"

# [target.'cfg(target_os = "android")'.dependencies]
# ffmpeg-the-third = { version = "5", features = ["build"] }
```
