# Release 版 Taffy Grid 布局崩溃 — 调查报告 / Release Taffy Grid Layout Crash — Investigation Report

> 项目 / Project: bevy-vn-engine (水仙10周年版本 v1.2.0)
> 日期 / Date: 2026-08-07
> 状态 / Status: 已修复并验证 / Fixed & Verified
> 后续更新 / Follow-up: rustc 已在后续 nightly 修复该误编译,本项目已移除 vendor workaround(见 §10)

---

## 0. 后续更新 / Follow-up (2026-08-07)

### 中文

在初始修复落地后,rustup 工具链更新到最新 nightly(`rustc 1.99.0-nightly (84b36a78a 2026-08-06)`)。用最小独立复现(MRE,见 §10)在 **三个工具链** 上验证原始构造代码:

| 工具链 / Toolchain | `min/max/gap.height` CompactLength tag | 旧 tag=8 破坏 |
|---|---|---|
| 旧 nightly `be8e82435` (2026-07-11) | min=**8**(非法) | ❌ 存在(崩溃根因) |
| 新 nightly `84b36a78a` (2026-08-06) | [3, 3, 1] 全部合法 | ✅ 已修复 |
| stable 1.97.1 | [3, 3, 1] 全部合法 | ✅ 已修复 |
| 1.94.1 | [3, 3, 1] 全部合法 | ✅ 已修复 |

结论:**rustc 已在 2026-08-06 的 nightly 中修复该确定性误编译**(stable 1.97.1、1.94.1 亦无此问题,仅旧 nightly 受影响)。据此:

1. 移除了 `[patch.crates-io] bevy_ui = { path = "vendor/bevy_ui" }` 与整个 `vendor/` 目录,恢复 registry 上游 bevy_ui 0.19.0。
2. 用新工具链重新构建 release:✅ 通过(0 error)。
3. 连续两次 15 秒冒烟运行:✅ `exit=124`(正常超时)、**0 panic**、viewport `min_w_tag=3 min_h_tag=3`(合法 Auto)。
4. `cargo check --workspace`:✅ 干净(仅预存 artemis-converter warning)。

因此该 miscompile **不需要**上报 rustc(已在上游修复),也 **不需要**向 bevy 提交 workaround 建议(编译器已修复,无需改依赖写法)。升级工具链到 2026-08-06 之后的 nightly(或任意较新 stable)即可规避。

**上游 issue 引用(已验证存在)**:本 bug 即 [rust-lang/rust#159116](https://github.com/rust-lang/rust/issues/159116)(2026-07-11 报告,`I-miscompile` / `P-high`,由 [PR #159148](https://github.com/rust-lang/rust/pull/159148) 于 07-12 修复);其下游崩溃报告为 [bevyengine/bevy#24952](https://github.com/bevyengine/bevy/issues/24952),崩溃点与本次调查完全一致(`taffy resolve.rs:68` `unreachable!` / `Compute Task Pool` / `ui_layout_system`)。详见 §11。

### English

After the initial fix landed, the rustup toolchain was updated to the latest nightly (`rustc 1.99.0-nightly (84b36a78a 2026-08-06)`). A minimal standalone reproducer (MRE, §10) was used to verify the original construction code on **three toolchains**:

| Toolchain | `min/max/gap.height` CompactLength tag | Old tag=8 corruption |
|---|---|---|
| Old nightly `be8e82435` (2026-07-11) | min=**8** (illegal) | ❌ present (crash root cause) |
| New nightly `84b36a78a` (2026-08-06) | [3, 3, 1] all legal | ✅ fixed |
| stable 1.97.1 | [3, 3, 1] all legal | ✅ fixed |
| 1.94.1 | [3, 3, 1] all legal | ✅ fixed |

Conclusion: **rustc fixed this deterministic miscompile in the 2026-08-06 nightly** (stable 1.97.1 and 1.94.1 are also clean; only the old nightly is affected). Accordingly:

1. Removed `[patch.crates-io] bevy_ui = { path = "vendor/bevy_ui" }` and the entire `vendor/` directory, reverting to upstream registry bevy_ui 0.19.0.
2. Rebuilt release with the new toolchain: ✅ passed (0 errors).
3. Two consecutive 15-second smoke runs: ✅ `exit=124` (normal timeout), **0 panics**, viewport `min_w_tag=3 min_h_tag=3` (legal Auto).
4. `cargo check --workspace`: ✅ clean (only the pre-existing artemis-converter warning).

Consequently this miscompile does **not** need to be reported to rustc (already fixed upstream), nor does bevy need a workaround suggestion (the compiler is fixed, no dependency change required). Simply upgrade to the 2026-08-06 nightly or any newer stable to avoid it.

**Upstream issue references (verified)**: this bug is [rust-lang/rust#159116](https://github.com/rust-lang/rust/issues/159116) (filed 2026-07-11, `I-miscompile` / `P-high`, fixed by [PR #159148](https://github.com/rust-lang/rust/pull/159148) on 07-12); its downstream crash report is [bevyengine/bevy#24952](https://github.com/bevyengine/bevy/issues/24952), whose crash site matches this investigation exactly (`taffy resolve.rs:68` `unreachable!` / `Compute Task Pool` / `ui_layout_system`). See §11.

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

本次崩溃是 rustc 1.99.0-nightly release 优化对「结构体更新 + `..default()`」语法的确定性误编译所致,通过将 bevy_ui 中触发点改为「先 default 后逐字段赋值」并 vendor 该 crate 修复。**该误编译已在 2026-08-06 的 nightly 中由 rustc 上游修复**(见 §0 与 §10),故本项目已移除 vendor 与 `[patch]`,回归上游 bevy_ui。后续建议:

1. **保持工具链更新**:升级到 2026-08-06 之后的 nightly 或任意较新 stable 即可规避该问题,无需任何代码改动。
2. **无需上报**:误编译已在上游修复,不需要向 rustc 提交 issue,也不需要向 bevy 提交 workaround 建议。
3. **警惕同类模式**:如仓库代码对 taffy/其他 tagged-enum 结构体使用「覆盖部分字段 + `..default()`」且仅 release 异常,应优先怀疑同类误编译,并记录当时使用的工具链版本。

### English

The crash was a deterministic miscompile by rustc 1.99.0-nightly of the struct-update + `..default()` pattern under release optimization, initially fixed by switching the trigger site in bevy_ui to "default-then-mutate" and vendoring the crate. **The miscompile has since been fixed upstream by rustc in the 2026-08-06 nightly** (see §0 and §10), so this project has removed `vendor/` and `[patch]` and reverted to upstream bevy_ui. Recommendations:

1. **Keep the toolchain current**: upgrading to the 2026-08-06 nightly or any newer stable avoids the issue with zero code changes.
2. **No upstream report needed**: the miscompile is already fixed upstream — no rustc issue and no bevy workaround request required.
3. **Watch for the same pattern**: if any code in this repo uses "override some fields + `..default()`" on taffy or other tagged-enum structs and misbehaves only in release, suspect the same class of miscompile and record the exact toolchain version in use.

---

## 10. 附录:MRE 验证 / Appendix: MRE Verification

### 中文

为验证误编译是否已在更新后的工具链修复,编写了独立最小复现(位于 `/tmp/opencode/mre`,不依赖本项目,vendor 之外直接用 registry taffy 0.10.1):

- **复刻内容**:逐字复刻 bevy_ui 0.19.0 `get_or_insert_taffy_viewport_node` 的构造代码 —— `Style { display: Grid, size: {100%}, align_items: Start, justify_items: Start, ..Default::default() }`。
- **判定方法**:同一 `Style` 用两种方式构造(结构体更新 vs 先 default 后 mutate),二者语义必须逐字节一致;再直接读取 `min_size.height` / `max_size.height` / `gap.height` 三个 `CompactLength` 的 tag(低 8 位,`AUTO_TAG=3`)。
- **关键字节布局**(x86_64,`size_of::<Style>()=536`):`min_size @ 0x0160`、`max_size @ 0x0170`、`gap @ 0x01e0`,每个 `Size<T>` 16 字节,`.height` 即 `offset+8`。

三个工具链 release 下的结果(各跑 10 轮,完全确定):

| 工具链 | tags (min/max/gap.height) | 两侧字节一致 | 判定 |
|---|---|---|---|
| 旧 nightly `be8e82435` | min=8(max=3, gap=1) | ❌ tag 区损坏 | **误编译仍在** |
| 新 nightly `84b36a78a` | [3, 3, 1] | ✅(仅 0x00ac padding 2 字节差异) | ✅ 已修复 |
| stable 1.97.1 | [3, 3, 1] | ✅(仅 padding 差异) | ✅ 已修复 |
| 1.94.1 | [3, 3, 1] | ✅(仅 padding 差异) | ✅ 已修复 |

> 注:新工具链上仍存在的 2 字节差异位于 `aspect_ratio: Option<f32>`(8 字节,`@ 0x00a8`)内部 0x00ac/0x00ad —— 该字段两侧的 Debug 输出与 `PartialEq` 均相等,属未初始化 padding 字节差异,语义无害,不影响任何行为。

MRE 源码核心:

```rust
let mut mutated: Style = Style::default();
mutated.display = Display::Grid;
mutated.size = Size { width: percent(1.0), height: percent(1.0) };
mutated.align_items = Some(AlignItems::Start);
mutated.justify_items = Some(JustifyItems::Start);

let updated: Style = Style {
    display: Display::Grid,
    size: Size { width: percent(1.0), height: percent(1.0) },
    align_items: Some(AlignItems::Start),
    justify_items: Some(JustifyItems::Start),
    ..Default::default()
};
// 断言:两个 Style 逐字节一致,且三个 CompactLength tag 均为 3
```

### English

To verify whether the miscompile was fixed by the updated toolchain, a minimal standalone reproducer was written (at `/tmp/opencode/mre`, independent of this project, using registry taffy 0.10.1):

- **What it replicates**: verbatim copy of bevy_ui 0.19.0 `get_or_insert_taffy_viewport_node`'s construction — `Style { display: Grid, size: {100%}, align_items: Start, justify_items: Start, ..Default::default() }`.
- **How it judges**: the same `Style` built two ways (struct-update vs default-then-mutate) must be byte-identical; additionally the `CompactLength` tags (low 8 bits, `AUTO_TAG=3`) of `min_size.height` / `max_size.height` / `gap.height` are read directly.
- **Key byte layout** (x86_64, `size_of::<Style>()=536`): `min_size @ 0x0160`, `max_size @ 0x0170`, `gap @ 0x01e0`, each `Size<T>` is 16 bytes, so `.height` is at `offset+8`.

Release results across three toolchains (10 rounds each, fully deterministic):

| Toolchain | tags (min/max/gap.height) | byte-identical | Verdict |
|---|---|---|---|
| Old nightly `be8e82435` | min=8 (max=3, gap=1) | ❌ tag region corrupted | **miscompile present** |
| New nightly `84b36a78a` | [3, 3, 1] | ✅ (only 2 padding bytes @0x00ac) | ✅ fixed |
| stable 1.97.1 | [3, 3, 1] | ✅ (padding only) | ✅ fixed |
| 1.94.1 | [3, 3, 1] | ✅ (padding only) | ✅ fixed |

> Note: the 2 remaining differing bytes on new toolchains sit inside `aspect_ratio: Option<f32>` (8 bytes, `@ 0x00a8`) at 0x00ac/0x00ad — Debug output and `PartialEq` are equal on both sides; this is an uninitialized-padding byte difference, semantically harmless.

MRE core:

```rust
let mut mutated: Style = Style::default();
mutated.display = Display::Grid;
mutated.size = Size { width: percent(1.0), height: percent(1.0) };
mutated.align_items = Some(AlignItems::Start);
mutated.justify_items = Some(JustifyItems::Start);

let updated: Style = Style {
    display: Display::Grid,
    size: Size { width: percent(1.0), height: percent(1.0) },
    align_items: Some(AlignItems::Start),
    justify_items: Some(JustifyItems::Start),
    ..Default::default()
};
// assert: both Styles byte-identical AND all three CompactLength tags == 3
```

---

## 11. 上游 issue 定位 / Upstream Issue Identification

### 中文

经检索 rust-lang/rust 与 bevyengine 的 issue 追踪器,本 bug 已在上游被完整报告、定位并修复,无需我们再提交任何 issue:

| 仓库 | Issue | 状态 | 说明 |
|---|---|---|---|
| rust-lang/rust | [#159116](https://github.com/rust-lang/rust/issues/159116) "memory corruption with nightly-2026-07-10" | CLOSED(由 PR #159148 修复) | **根因 issue**。`I-miscompile` / `A-codegen` / `P-high` / `regression-from-stable-to-nightly` |
| bevyengine/bevy | [#24952](https://github.com/bevyengine/bevy/issues/24952) "Memory corruption bug in rustc causes UI panics on nightly" | CLOSED | **下游崩溃报告**,崩溃点与本调查完全一致 |

**根因细节**(rust#159116):

- **引入**:PR [#158666](https://github.com/rust-lang/rust/pull/158666) "Carry the `b_offset` inside `BackendRepr::ScalarPair`",提交 [`4ccf0ea`](https://github.com/rust-lang/rust/commit/4ccf0eadf7087640a20d0b5e09ea4b73b4d67033),2026-07-09 合并。
- **修复**:PR [#159148](https://github.com/rust-lang/rust/pull/159148) "Fix offset used to read the second part of scalar pairs from a `const`",2026-07-12 合并。
- **机制**:rustc codegen 在从 `const` 读取 ScalarPair(双值表示)的第二部分时使用了错误的偏移("global vs relative coordinate space" 混淆,`b_offset` 计算多加了偏移)。Release 优化下,经结构体更新语法构造的 tagged enum(如 taffy `CompactLength`)第二个槽位被读出错误字节 → tag 变为非法值 → 触发下游 `unreachable!`。

**与我们调查的对应关系**:

- 我们崩溃使用的 nightly `be8e82435`(2026-07-11)正好位于引入(07-09)与修复(07-12)之间的受影响窗口。
- MRE 验证的 4 个工具链结果与修复时间线完全一致:旧 nightly 坏、新 nightly(08-06,已含 07-12 修复)好、stable 1.97.1 / 1.94.1 好。
- bevy#24952 报告者同样复现于 `taffy-0.10.1/src/util/resolve.rs:68:18`(`unreachable!`)、`Compute Task Pool` 线程、`bevy_ui::layout::ui_layout_system` 系统 —— 与我们在插桩前的崩溃现场完全吻合(我们插桩后行号漂移至 74)。

### English

A search of the rust-lang/rust and bevyengine issue trackers shows this bug was already fully reported, bisected, and fixed upstream — no further issue submission is needed from us:

| Repo | Issue | State | Notes |
|---|---|---|---|
| rust-lang/rust | [#159116](https://github.com/rust-lang/rust/issues/159116) "memory corruption with nightly-2026-07-10" | CLOSED (fixed by PR #159148) | **Root-cause issue**. `I-miscompile` / `A-codegen` / `P-high` / `regression-from-stable-to-nightly` |
| bevyengine/bevy | [#24952](https://github.com/bevyengine/bevy/issues/24952) "Memory corruption bug in rustc causes UI panics on nightly" | CLOSED | **Downstream crash report**; crash site matches this investigation exactly |

**Root-cause details** (rust#159116):

- **Introduced by**: PR [#158666](https://github.com/rust-lang/rust/pull/158666) "Carry the `b_offset` inside `BackendRepr::ScalarPair`", commit [`4ccf0ea`](https://github.com/rust-lang/rust/commit/4ccf0eadf7087640a20d0b5e09ea4b73b4d67033), merged 2026-07-09.
- **Fixed by**: PR [#159148](https://github.com/rust-lang/rust/pull/159148) "Fix offset used to read the second part of scalar pairs from a `const`", merged 2026-07-12.
- **Mechanism**: rustc codegen used the wrong offset when reading the second part of a ScalarPair (two-value representation) from a `const` — a "global vs relative coordinate space" mixup with an extra `offset +` in `b_offset`. Under release optimization, tagged enums built via struct-update syntax (e.g. taffy `CompactLength`) read corrupted bytes into the second slot → illegal tag → downstream `unreachable!`.

**Correspondence with our investigation**:

- Our crashing nightly `be8e82435` (2026-07-11) sits exactly in the affected window between introduction (07-09) and fix (07-12).
- The MRE's four-toolchain results match the fix timeline precisely: old nightly broken; new nightly (08-06, includes the 07-12 fix) clean; stable 1.97.1 / 1.94.1 clean.
- The bevy#24952 reporter hit the identical site: `taffy-0.10.1/src/util/resolve.rs:68:18` (`unreachable!`), `Compute Task Pool` thread, `bevy_ui::layout::ui_layout_system` — matching our pre-instrumentation crash exactly (post-instrumentation line drifted to 74).
