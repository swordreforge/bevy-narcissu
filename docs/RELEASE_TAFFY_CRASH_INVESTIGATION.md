# Release 版 Taffy Grid 布局崩溃 — 调查报告 / Release Taffy Grid Layout Crash — Investigation Report

> 项目 / Project: bevy-vn-engine (水仙10周年版本 v1.2.0)
> 日期 / Date: 2026-08-07
> 状态 / Status: 已修复并验证 / Fixed & Verified

---

## 1. 摘要 / Summary

### 中文

bevy-vn-example 在 **release** 模式下启动即崩溃,debug 模式完全正常。崩溃点为 taffy 0.10.1 的 `Dimension::maybe_resolve` 中的 `unreachable!`(grid 布局路径)。经过逐层插桩与最小化实验,确认根因是 **rustc 1.99.0-nightly 在 release 优化下对结构体更新语法(struct-update syntax,即 `Style { ..default() }`)的确定性误编译(miscompile)**:bevy_ui 0.19.0 的 `get_or_insert_taffy_viewport_node` 用它构造隐式 grid 根节点时,default 覆盖区中的 `min_size.height` / `max_size.height` / `gap.height` 三个 `CompactLength` 字段被写成非法 tag **8**(合法 tag 集合为 {1,2,3,4,7,15,23,31})。tag=8 满足 `tag & 7 == 0`,被 taffy 的 `is_calc()` 误判为 calc 分支,而 calc feature 未启用,最终触发 `unreachable!`。

修复方式:将 bevy_ui vendored 到 `vendor/bevy_ui`,把该处从「结构体更新」改为「先 `Style::default()` 再逐字段显式赋值」,通过 `[patch.crates-io]` 覆盖依赖。release 构建、`cargo test --workspace`、连续两次 15 秒冒烟运行全部通过(0 panic)。

### English

`bevy-vn-example` crashed at startup in **release** mode while running perfectly fine in debug. The crash was an `unreachable!` inside taffy 0.10.1's `Dimension::maybe_resolve` (grid-layout path). After step-by-step instrumentation and minimization experiments, the root cause was confirmed to be a **deterministic miscompile by rustc 1.99.0-nightly under release optimization of struct-update syntax** (`Style { ..default() }`): bevy_ui 0.19.0's `get_or_insert_taffy_viewport_node` builds the implicit grid root with that syntax, and the defaulted fields `min_size.height` / `max_size.height` / `gap.height` (all `CompactLength`) ended up with the illegal tag **8** (legal tags are {1,2,3,4,7,15,23,31}). Since tag 8 satisfies `tag & 7 == 0`, taffy's `is_calc()` misroutes it to the calc branch, which is not enabled — finally tripping `unreachable!`.

Fix: vendored bevy_ui into `vendor/bevy_ui`, replacing the struct-update with `Style::default()` followed by explicit field mutation, and overriding the dependency via `[patch.crates-io]`. Release build, `cargo test --workspace`, and two consecutive 15-second smoke runs all pass (0 panics).

---

## 2. 环境 / Environment

| 项 / Item | 值 / Value |
|---|---|
| 系统 / OS | Linux x86_64 |
| 工具链 / Toolchain | rustc 1.99.0-nightly (`be8e82435` 2026-07-11) |
| 链接器 / Linker | mold 2.41.0 |
| 编译器后端 / Codegen | clang 22.1.8 (LLVM) |
| Bevy | 0.19.0 |
| bevy_ui | 0.19.0 |
| taffy | 0.10.1 (registry,后端为默认 grid feature) |
| 崩溃目标 / Crashing target | `examples/bevy-vn-example` |
| release profile | 默认:opt-level=3, codegen-units=16, 无 LTO |
| 可用备用工具链 / Fallback toolchains | stable、1.94.1 均已安装 |

---

## 3. 现象 / Symptom

### 中文

- 仅 **release** 崩溃;`cargo run`(debug)正常。
- 首次 UI 布局帧即触发,多次运行 100% 复现。
- panic 消息:

```
thread 'Compute Task Pool (4)' panicked at /home/.../taffy-0.10.1/src/util/resolve.rs:74:13:
internal error: entered unreachable code
```

- 栈帧(插桩后):`compute_grid_layout` → `compute_root_layout` → bevy_ui 的 `ui_layout_system`(系统 `ui_layout::<bevy_vn_render::...>`),线程为 Compute Task Pool(9/10/13/4 等,随运行轮次变化)。

### English

- Crashes **only in release**; `cargo run` (debug) is fine.
- Triggers on the very first UI-layout frame; 100% reproducible across runs.
- Panic message:

```
thread 'Compute Task Pool (4)' panicked at /home/.../taffy-0.10.1/src/util/resolve.rs:74:13:
internal error: entered unreachable code
```

- Stack (after instrumentation): `compute_grid_layout` → `compute_root_layout` → bevy_ui `ui_layout_system` (system `ui_layout::<bevy_vn_render::...>`), on a Compute Task Pool thread (9/10/13/4 depending on run).

---

## 4. 调查过程 / Investigation Process

| 步骤 | 内容 | 结论 |
|---|---|---|
| 1 | 复现:多轮 release 运行,确认 100% 必崩 | 确定性,非偶发 |
| 2 | vendored taffy,在 `new_leaf`、grid compute、`maybe_resolve` 插桩 | 锁定 panic 发生于 grid 布局的尺寸解析 |
| 3 | 转储涉及节点的 `min/max/gap` 字段原始字节 | 发现 viewport 根节点 `min_size.height` / `max_size.height` / `gap.height` 的 tag = **8** |
| 4 | 对照 `CompactLength` 合法 tag 表 | 8 无任何合法构造入口,必为内存损坏或误编译 |
| 5 | 在 bevy_ui 侧做最小化实验(见 §5) | 确认根因在结构体更新语法,与覆盖的字段无关 |
| 6 | 验证 `codegen-units=1` 仍复现 | 排除跨 CGU 内联/常量合并类优化 |
| 7 | 验证 debug 下同样代码正常 | 确认为 release-only 问题 |
| 8 | 验证「先 default 后 mutate」在 release 正常 | 获得可行 workaround,立即落地 |

| Step | Action | Result |
|---|---|---|
| 1 | Reproduce: multiple release runs | Deterministic, 100% reproducible |
| 2 | Vendored taffy; instrumented `new_leaf`, grid compute, `maybe_resolve` | Panic pinned to dimension resolution during grid layout |
| 3 | Dump raw bytes of `min/max/gap` on involved nodes | Viewport root: tag of `min_size.height` / `max_size.height` / `gap.height` = **8** |
| 4 | Compare against legal `CompactLength` tag table | Tag 8 has no legal constructor — corruption or miscompile |
| 5 | Minimized experiments on bevy_ui side (§5) | Root cause is struct-update syntax itself, independent of overridden fields |
| 6 | Verified still broken with `codegen-units=1` | Rules out cross-CGU inlining / constant-folding classes |
| 7 | Same code under debug is fine | Confirmed release-only issue |
| 8 | `default()` then mutate works in release | Viable workaround found and applied |

---

## 5. 关键证据链 / Evidence Chain

### 5.1 CompactLength tag 合法值 / Legal CompactLength tags

`CompactLength` 是 taffy 的 tagged enum(tag 存于低位),合法 tag 如下,且 **8 不存在**:

| tag | 变体 / Variant |
|---|---|
| 1 | Length(f32) |
| 2 | Percent(f32) |
| 3 | Auto |
| 4 | MaxContent |
| 7 | FitContent |
| 15 | MinContent |
| 23 | Flex(f32) |
| 31 | Calc |

`is_calc()` 判断为 `tag & 7 == 0`。tag 8 恰好通过该判断 → 进入未启用的 calc 分支 → `unreachable!`。

### 5.2 最小化实验(bevy_ui 侧,同一构建下) / Minimized experiments (bevy_ui side, same build)

在 release 下用 `Style::default()` 构造后立即打印 `min_size.height` 的 tag:

| 构造方式 / Construction | min_size.height tag | 结果 / Result |
|---|---|---|
| 裸 `Style::default()`(无任何覆盖) | 3 (Auto) | ✅ 正常 |
| `Style { display: Grid, ..default() }`(仅覆盖 display) | **8** | ❌ 损坏 |
| `Style::default()` 后逐字段 mutate | 3 (Auto) | ✅ 正常 |
| 同上三个实验在 **debug** 下 | 全部 3 (Auto) | ✅ 均正常 |

结论:即使**只覆盖一个字段**、其余全部走 `..default()`,release 优化也会把 default 区特定槽位(本结构体中恰好是 `min_size.height`/`max_size.height`/`gap.height`,即位于 struct 偏移 336/352/368 附近的 3 个 `CompactLength`)写坏。覆盖哪个字段无关,问题出在**结构体更新语法本身**。

### 5.3 排除的假设 / Ruled-out hypotheses

| 假设 | 排除方式 |
|---|---|
| bevy_ui 与 taffy 的 Style ABI/布局不一致 | 两侧各自打印:size_offset=336, min=352, max=368, `size_of::<Style>()=536`,features 集合完全相同 |
| feature 错位(calc 未启用但被调用) | tag 本应为 Auto(3),实际为 8 —— 与 feature 无关,是数据本身被写坏 |
| 并行数据竞争(`ui_layout_system` 并发) | taffy 布局为顺序执行,单节点单线程;且损坏模式完全确定 |
| 跨 CGU 优化(常量合并/内联) | `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` 下依旧复现 |

### 5.1 Legal CompactLength tags

`CompactLength` is a tagged enum (tag stored in low bits). Legal tags — **8 does not exist**:

| tag | Variant |
|---|---|
| 1 | Length(f32) |
| 2 | Percent(f32) |
| 3 | Auto |
| 4 | MaxContent |
| 7 | FitContent |
| 15 | MinContent |
| 23 | Flex(f32) |
| 31 | Calc |

`is_calc()` is `tag & 7 == 0`. Tag 8 passes that check → routes into the (disabled) calc branch → `unreachable!`.

### 5.2 Minimized experiments (bevy_ui side, same build)

Printing the `min_size.height` tag right after construction, in release:

| Construction | min_size.height tag | Result |
|---|---|---|
| bare `Style::default()` (no overrides) | 3 (Auto) | ✅ OK |
| `Style { display: Grid, ..default() }` (override only `display`) | **8** | ❌ Corrupted |
| `Style::default()` then mutate fields | 3 (Auto) | ✅ OK |
| Same three experiments under **debug** | all 3 (Auto) | ✅ all OK |

Conclusion: even overriding a **single** field and leaving the rest to `..default()`, release optimization corrupts specific defaulted slots (here exactly the three `CompactLength`s at struct offsets ≈336/352/368: `min_size.height`/`max_size.height`/`gap.height`). The overridden field is irrelevant — the problem is the **struct-update syntax itself**.

### 5.3 Ruled-out hypotheses

| Hypothesis | How ruled out |
|---|---|
| ABI/layout mismatch of `Style` between bevy_ui and taffy | Both sides print: size_offset=336, min=352, max=368, `size_of::<Style>()=536`, identical feature sets |
| Feature mismatch (calc used but not enabled) | Tag should be Auto(3); it is 8 — data itself is corrupted, unrelated to features |
| Parallel data race (`ui_layout_system` concurrency) | Taffy layout runs sequentially on one node/thread; corruption pattern is fully deterministic |
| Cross-CGU optimization (const-fold / inline) | Still reproduces with `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` |

---

## 6. 根因分析 / Root Cause

### 中文

rustc 1.99.0-nightly 的 release 后端在编译 bevy_ui 0.19.0 中形如

```rust
taffy::style::Style { display: Display::Grid, size: ..., align_items: ..., justify_items: ..., ..Default::default() }
```

的结构体更新表达式时,生成错误代码:把「取自 `Default::default()` 的其余字段」中的 3 个 `CompactLength` 槽位写入了非法值(内存中以 tag=8 呈现)。该损坏是**确定性的**(多次运行、不同机器路径、多线程调度下字节完全一致),且**只影响 release**(debug 正常),**与覆盖字段无关**。

由于 tag=8 恰好满足 `tag & 7 == 0`,taffy 的 `is_calc()` 将其误判为 calc 变体,而编译 taffy 时未启用 `calc` feature,`Dimension::maybe_resolve` 对 calc 分支执行 `unreachable!`,进程在首次 grid 布局时崩溃。

性质判断:这是编译器后端(LLVM 经由 rustc 1.99 nightly)的确定性误编译,并非我们代码或依赖的 bug。我们代码无意中踩中了该触发条件。

### English

The rustc 1.99.0-nightly release backend, when compiling bevy_ui 0.19.0's struct-update expression

```rust
taffy::style::Style { display: Display::Grid, size: ..., align_items: ..., justify_items: ..., ..Default::default() }
```

emits incorrect code: three `CompactLength` slots among the fields sourced from `Default::default()` are written with an illegal value (observed as tag 8). The corruption is **deterministic** (byte-identical across runs, paths and thread schedules), **release-only** (debug is fine), and **independent of which fields are overridden**.

Because tag 8 happens to satisfy `tag & 7 == 0`, taffy's `is_calc()` misroutes it to the calc variant; taffy was compiled without the `calc` feature, so `Dimension::maybe_resolve` executes `unreachable!` on that branch, crashing the process on the first grid layout.

Assessment: this is a deterministic miscompile in the compiler backend (LLVM through rustc 1.99 nightly) — not a bug in our code or in the dependency. Our code simply trips the trigger.

---

## 7. 修复方案 / Fix

### 中文

1. **Vendored bevy_ui**:将 registry 中的 bevy_ui-0.19.0 完整拷贝到 `vendor/bevy_ui`(删除 `.cargo-checksum.json`),并在 workspace `Cargo.toml` 增加:

   ```toml
   [patch.crates-io]
   bevy_ui = { path = "vendor/bevy_ui" }
   ```

2. **Workaround**:在 `vendor/bevy_ui/src/layout/ui_surface.rs` 的 `get_or_insert_taffy_viewport_node` 中,把

   ```rust
   let implicit_root = self.taffy.new_leaf(taffy::style::Style {
       display: taffy::style::Display::Grid,
       size: taffy::geometry::Size { width: percent(1.0), height: percent(1.0) },
       align_items: Some(AlignItems::Start),
       justify_items: Some(JustifyItems::Start),
       ..Default::default()
   }).unwrap();
   ```

   改为「先 `default()` 后显式 mutate」(与 §5.2 验证通过的构造方式一致),并保留详细注释说明误编译背景。

3. **清理**:移除调查期间临时使用的 taffy `[patch]` 与 `vendor/taffy`(调试插桩),删除不再使用的 `use bevy_utils::default;` 导入。

### English

1. **Vendored bevy_ui**: copied bevy_ui-0.19.0 from the registry into `vendor/bevy_ui` (dropped `.cargo-checksum.json`) and added to the workspace `Cargo.toml`:

   ```toml
   [patch.crates-io]
   bevy_ui = { path = "vendor/bevy_ui" }
   ```

2. **Workaround**: in `vendor/bevy_ui/src/layout/ui_surface.rs`, `get_or_insert_taffy_viewport_node`, replaced

   ```rust
   let implicit_root = self.taffy.new_leaf(taffy::style::Style {
       display: taffy::style::Display::Grid,
       size: taffy::geometry::Size { width: percent(1.0), height: percent(1.0) },
       align_items: Some(AlignItems::Start),
       justify_items: Some(JustifyItems::Start),
       ..Default::default()
   }).unwrap();
   ```

   with `Style::default()` followed by explicit field mutation (the construction form proven correct in §5.2), keeping a detailed comment documenting the miscompile.

3. **Cleanup**: removed the temporary taffy `[patch]` and `vendor/taffy` (debug instrumentation), and dropped the now-unused `use bevy_utils::default;` import.

---

## 8. 验证结果 / Verification

| 验证项 / Check | 结果 / Result |
|---|---|
| `cargo build --release`(bevy-vn-example) | ✅ 通过 |
| 冒烟运行 1(15s,release) | ✅ `exit=124`(正常超时),**0 panic**,Grid viewport `min_h=3` |
| 冒烟运行 2(15s,release) | ✅ 同上,再次确认 |
| `cargo test --workspace` | ✅ 全部通过(0 failed;各 crate 24/45/6/4/1 等) |
| `cargo check --workspace` | ✅ 干净,仅剩 1 条**预存** warning(`tools/artemis-converter/src/main.rs:165` 未读 `label`,与本次无关) |

---

## 9. 结论与后续建议 / Conclusion & Recommendations

### 中文

本次崩溃是 rustc 1.99.0-nightly release 优化对「结构体更新 + `..default()`」语法的确定性误编译所致,通过将 bevy_ui 中触发点改为「先 default 后逐字段赋值」并 vendor 该 crate 修复。后续建议:

1. **上报上游**:向 rustc 提交该确定性误编译 bug。最小复现:release 下对包含 3 个 `CompactLength` 的 536 字节结构体执行 `Style { display: Grid, ..default() }`,default 区 3 个 tag 被写为 8。
2. **切换 stable 可移除 vendor**:系统已装 stable / 1.94.1 工具链。若确认该误编译为 nightly 回归,可将项目切到 stable,届时可删除 `vendor/` 与 `[patch]`,恢复上游 bevy_ui。
3. **警惕同类模式**:仓库代码中如还有对 taffy/其他 tagged-enum 结构体使用「覆盖部分字段 + `..default()`」的写法,且仅 release 出现异常,应优先怀疑同类误编译,改用显式构造。

### English

The crash was a deterministic miscompile by rustc 1.99.0-nightly of the struct-update + `..default()` pattern under release optimization. It is fixed by switching the trigger site in bevy_ui to "default-then-mutate" and vendoring the crate. Recommendations:

1. **Report upstream**: file a bug against rustc. Minimal repro: in release, on a 536-byte struct containing three `CompactLength`s, `Style { display: Grid, ..default() }` writes tag 8 into three defaulted slots.
2. **Drop vendor on stable**: stable / 1.94.1 toolchains are already installed. If the miscompile is confirmed as a nightly regression, switch the project to stable and remove `vendor/` + `[patch]` to go back to upstream bevy_ui.
3. **Watch for the same pattern**: if any other code in this repo uses "override some fields + `..default()`" on taffy or other tagged-enum structs and misbehaves only in release, suspect the same class of miscompile and switch to explicit construction.
