# FloatingInputTool 体验修复实施计划

> **For agentic workers:** 使用 superpowers:executing-plans 在当前会话按任务执行。步骤以复选框跟踪；本计划不要求子代理。用户本次只要求输出计划，尚未开始修改应用代码。

**Goal:** 修复正常使用中的呼出、贴边、输入、设置和外观问题，使快速记录流程稳定、界面有序。

**Architecture:** 保留 Tauri + React，不重写框架。由 Rust 统一窗口行为，React 管理编辑状态；仅提取此次修复需要的纯函数和设置面板，不同时重构存储、快捷键和全部入口文件。

**Tech Stack:** Windows、Tauri 2、Rust、React 19、TypeScript、现有 Node 24；前端纯逻辑测试使用 node:test，不引入完整浏览器测试框架。

**Spec:** `docs/superpowers/specs/2026-04-22-side-hide-single-window-design.md`（包含2026-10-27用户调整）；问题依据为 `docs/reviews/2026-10-10/review.md` 和 `docs/reviews/2026-10-27/review.md`。需求优先于审查建议。

## 全局约束与范围

- 单窗口捕获模型；托盘常驻；Capture始终呼出，不切换隐藏。
- 拖到边缘并松开后自动收起，保留把手。
- 从贴边展开后保存成功，立即收回原侧，不等鼠标移出；普通窗口保存成功后完全隐藏。
- Hide和Close保留未保存草稿；Discard才主动丢弃。
- 保留白色、深色、跟随系统和自定义主题，以及中英文、多目标和模板功能。
- 本轮不做磁盘故障回滚、崩溃恢复强化、外部文件并发冲突、安全加固、提交去重服务或草稿存储队列重构。上一轮这些建议记入延期，不算本轮完成条件。
- 保存中输入丢失、IME误提交、设置编辑被覆盖属于正常交互缺陷，纳入修复。
- 不增加成批高级设置；现有JSON贴边参数继续可用，暂不全部搬进UI。
- 不改AHK原型；不以编译通过代替桌面行为验收。
- 界面建议是本计划的推荐设计，尚未制作视觉稿；执行设置重排前先提供静态预览供审阅。

## 批次及依赖

| 批次 | 任务 | 交付 | 依赖 |
| --- | --- | --- | --- |
| 一 | 1—3 | 窗口生命周期、贴边转换及缩放正确 | 任务2依赖1，任务3依赖2 |
| 二 | 4—5 | 输入提交与模板正文正确 | 任务4依赖2的保存后动作；任务5独立 |
| 三 | 6—7 | 设置编辑保留、布局和主题可用 | 任务7沿用任务6的编辑模型 |
| 四 | 8 | 实机验收和可安装构建 | 前述任务完成 |

每个任务独立验证、独立提交。提交范围限本任务文件，不捎带覆盖用户现有更改。执行前读取实际AGENTS.md和最新代码；本计划行号以审查时版本为参考，以函数名定位。

## 任务1：后台启动、系统关闭和设置返回

**修改：** `tauri-app/src-tauri/tauri.conf.json`、`tauri-app/src-tauri/src/lib.rs`、`tauri-app/src/SettingsApp.tsx`。

**接口：** 保留 `show_main_window` 为Capture入口；新增Tauri命令 `close_settings_window`，负责设置隐藏、主窗口启用及编辑器焦点事件。前端不再自行调用未获允许的 `mainWindow.setFocus()`。

- [ ] 记录基线：首次启动是否弹窗、主窗口Alt+F4后能否呼出、设置关闭后能否直接输入。
- [ ] 主窗口配置明确设置：

```json
"visible": false
```

- [ ] 在 `attach_main_window_events` 处理 `WindowEvent::CloseRequested { api, .. }`：先 `api.prevent_close()`，再执行保留草稿的隐藏；真正退出只走托盘Quit。沿用现有flush能力，不新造恢复机制。
- [ ] 注册 `close_settings_window` 并把设置×改为：

```ts
await invoke('close_settings_window')
```

- [ ] 设置原生关闭路径同样恢复主窗口可交互状态。打开设置时取消已有的延迟主窗口聚焦，避免设置刚打开又被抢焦点。
- [ ] Windows人工验证：未打开过设置、打开过并隐藏设置两种情况下，Alt+F4都不退出；关设置后立即打字进入主编辑器；托盘Quit仍退出。

**完成标准：** 不弹启动窗口、不因系统关闭丢失捕获窗口、设置返回能继续输入。

## 任务2：统一贴边状态与保存后收起

**修改：** `lib.rs`、`App.tsx`、`appModel.ts`。
**新增：** `tauri-app/src-tauri/src/window_behavior.rs`（状态决策及单元测试）；主入口保留Tauri/Win32调用。

**接口：** 将现有 `DockSide`、`MainWindowMode`、`StoredWindowBounds`、`MainWindowRuntimeState` 移入 `window_behavior.rs`，以 `pub(crate)`共享。保留现有命令名，避免前后端同步重命名。新增 `finish_capture` 命令：Rust依据当前模式决定收回原侧或完全隐藏。

- [ ] 为重复Hide和恢复属性编写失败用例：左/右docked状态再次Hide不替换normal_bounds；Capture恢复normal且可缩放。
- [ ] `hide_or_dock_main_window_inner` 在已docked时直接返回，不读取屏外位置覆盖恢复位置；统一normal入口恢复 `set_resizable(true)`，docked入口设为false。

```rust
if matches!(mode, MainWindowMode::DockedLeft | MainWindowMode::DockedRight) {
    return Ok(());
}
```

- [ ] 把托盘、快捷键、移动后自动收起、前端收起命令收敛到相同的转换入口。为动画记录转换序号，旧动画和旧聚焦任务执行前检查序号，取消过期操作；移动事件改用一个可重置调度任务，避免每次移动都新建等待线程。
- [ ] 保留拖拽松开后自动贴边；悬停展开取消Rust和React的自动focus。只有Capture和用户点击才主动聚焦。
- [ ] 回缩条件改为“鼠标移出且未在拖拽/输入法组词”，不能把textarea仍有焦点当成永久阻止条件。编辑中的IME合成结束后重新评估回缩，避免候选框尚未确认就收起。
- [ ] `finish_capture` 在ExpandedFromDock时使用保存的dock_side和docked_bounds收回原侧；不能重新猜测另一边。Normal则hide。
- [ ] `handleSaveNote` 成功后调用 `finish_capture`，移除当前expanded分支的重新focus；保存失败不执行收起。
- [ ] 人工验证：左右Hide→Hide→Capture；悬停→移出；拖拽→自动贴边；贴边展开→保存→原侧把手；普通保存→完全隐藏；Capture/Hide重复100轮。

**完成标准：** 窗口位置、可缩放属性和前端模式一致；保存按用户确认规则执行。

## 任务3：修复DPI几何与核查双屏交界

**修改：** `window_behavior.rs`、`lib.rs`、`App.tsx`、`App.css`；必要时更新config帮助文字说明单位。

**接口：** 配置数字统一视为逻辑像素；新增 `logical_px_to_physical(value: u32, scale: f64) -> u32`。Rust几何使用转换后的物理像素，CSS继续使用原逻辑值。

- [ ] 写几何用例，覆盖36逻辑像素在100%/150%/200%对应36/54/72物理像素。

```rust
assert_eq!(logical_px_to_physical(36, 1.0), 36);
assert_eq!(logical_px_to_physical(36, 1.5), 54);
assert_eq!(logical_px_to_physical(36, 2.0), 72);
```

- [ ] 同步转换把手、热区、贴边阈值及光标padding；桌面绝对位置和窗口宽高始终使用物理坐标，禁止二次转换。
- [ ] Capture和hover展开共用恢复区域约束；使用目标显示器的工作区与scale，而非主屏固定scale。
- [ ] 验证不同DPI屏之间移动后的把手、热区及还原位置。
- [ ] 单独实测双屏内部交界处的原生背景及鼠标命中范围。若出现邻屏残留，记录复现并另提窗口方案；本轮不能把“仅外缘可贴边”当作已批准需求，也不擅自引入第二把手窗口或重试旧的区域裁剪方案。

**完成标准：** DPI单位正确；已测试拓扑记录结果。双屏内部交界若确认需要改变窗口模型，作为明确未关闭项，不宣称已根治。

## 任务4：修复中文输入与保存期间交互

**修改：** `App.tsx`、`lib.rs`的 `save_note` 参数、`package.json`。
**新增：** `src/captureBehavior.ts`、`src/captureBehavior.test.ts`。

**接口：** `shouldSubmit(event, mode): boolean`为无React依赖的按键判断，event包含key、ctrlKey、shiftKey、altKey、metaKey、isComposing、keyCode。`save_note`接收文本和明确的targetId，沿用当前格式化/写入，不引入提交ID服务。

- [ ] 使用node:test写IME判断用例，再实现：

```ts
assert.equal(shouldSubmit({
  key: 'Enter', ctrlKey: false, shiftKey: false,
  altKey: false, metaKey: false, isComposing: true, keyCode: 229,
}, 'enter-save'), false)
```

- [ ] composition期间Enter不提交；保留两种提交模式以及对应换行方式。
- [ ] 保存入口先用同步ref锁防止同一页面重复进入，再置React的saving状态；保存期间textarea只读，Discard和目标切换暂停，finally解锁。这个简化方案避免保存完成清掉请求发出后新增的输入，无需同时重构草稿revision。
- [ ] 请求捕获当前targetId，后端按该id查目标，不因保存期间全局Next Target改变本次目的文件。
- [ ] 增加脚本 `test: node --test src/captureBehavior.test.ts src/settingsDraft.test.ts`；任务6的测试文件加入时再加入第二个路径，保证每次提交的命令可运行。
- [ ] 人工验证中文拼音候选确认、两种提交方式、多目标保存及保存中控件状态。

**完成标准：** 不误提交中文候选，不因正常保存期间继续编辑丢字，保存目的文件明确。

## 任务5：模板只插值一次

**修改：** `lib.rs`的 `format_note_block`，必要时提取到 `src-tauri/src/note_formatter.rs`，不改文件写入层。

**接口：** 保持 `format_note_block(note_text: &str, note_template: &str) -> String` 行为与现有三个占位符兼容。

- [ ] 增加能复现正文二次替换的失败测试：

```rust
assert_eq!(
    format_note_block("literal {{text}}", "{{text.callout}}"),
    "> literal {{text}}\r\n\r\n"
);
```

- [ ] 扫描原始模板，从左到右识别 `{{timestamp}}`、`{{text.callout}}`、`{{text}}`；直接追加替代值，不再扫描插入的正文。保留未知占位符和现有行尾格式。
- [ ] 验证默认多行callout、空行、三个占位符混用及正文中出现占位符字样。

**完成标准：** 正文保持原文，现有模板格式兼容。

## 任务6：设置编辑模型和页面布局

**修改：** `SettingsApp.tsx`、`App.css`、`appModel.ts`。
**新增：** `src/settingsDraft.ts`、`src/settingsDraft.test.ts`；按需提取 `src/settings/TargetsPanel.tsx`、`ShortcutsPanel.tsx`、`AppearancePanel.tsx`、`AdvancedPanel.tsx`。

**接口：** 持久化配置与编辑值分开；纯函数 `mergeSettingsDraft<T extends object>(current: T, incoming: T, dirtyKeys: ReadonlySet<keyof T>): T`逐字段合并。UI把targets区域映射到targets/activeTargetId，hotkeys映射到hotkey/hideHotkey/nextTargetHotkey，模板映射到noteTemplate，自定义配色映射到customTheme；dirty字段保留当前编辑，其他字段更新。dirty标记只在该区域保存或取消后清除。

- [ ] 先做静态布局预览，推荐四类：记录、快捷键、外观、高级；模板和署名移到高级，不把所有底层参数变成设置项。完成视觉审阅后再改布局。
- [ ] 写“编辑模板后切主题仍保留模板”的失败用例，覆盖保存目标时其他区域编辑仍在。

```ts
const current = { noteTemplate: '未保存模板', themeMode: 'theme-white' }
const incoming = { noteTemplate: '已保存模板', themeMode: 'theme-dark' }
const dirtyKeys = new Set<keyof typeof current>(['noteTemplate'])
const merged = mergeSettingsDraft(current, incoming, dirtyKeys)
assert.equal(
  merged.noteTemplate,
  '未保存模板',
)
assert.equal(merged.themeMode, 'theme-dark')
```

- [ ] 替换现有 `applyConfig` 的全量重置；快捷键“当前启用”以runtime状态为准，不能用配置值代替。
- [ ] 普通选择即时保存；目标、模板、自定义配色各自保存/取消；取消仅撤销对应区域。关闭设置保留未提交编辑，下次重开仍在；未保存主题预览关闭时结束，重开编辑再预览。
- [ ] 快捷键两列排布，生效说明独占下一行；目标名称和路径分两行。窄窗口按真实断点调整，不仅增加padding。
- [ ] 设置根布局明确header/content/status/footer；状态区域保留，避免反馈出现时跳行；取消为中性色，删除/丢弃保留红色。
- [ ] 在500/560/800逻辑宽度检查中英文、长路径、展开编辑、保存反馈；页面无横向溢出，底部按钮始终可达。

**完成标准：** 设置互不覆盖、保存语义清晰、常用项易找、最小窗口布局稳定。

## 任务7：主题不透明度、把手和圆角一致

**修改：** `App.css`、`index.css`、`App.tsx`、`SettingsApp.tsx`。

- [ ] 自定义windowOpacity定义为主背景的最终不透明度，聚焦/失焦均使用该值；焦点提示使用边框/阴影，避免另乘隐藏系数。

```css
.backdrop-native.theme-custom .floating-window {
  background: var(--custom-window-background);
}
```

- [ ] 删除会覆盖该定义的聚焦固定92%规则，检查编辑器层叠不会使预览与保存结果不同。
- [ ] 补齐自定义dock-handle底色、边框和文字；白/黑/自定义把手均保持可见。
- [ ] 设置窗口采用不透明主题背景，不增加新的背景模式设置；主窗口保留现有Acrylic与fallback。
- [ ] 统一圆角token并沿用已选择的DWM原生圆角方案，不恢复22px原生Region实验；dock状态维持方形边缘。
- [ ] 桌面比较0.35/0.7/1三档不透明度、聚焦/失焦、三种主题、设置打开与关闭以及左右把手。

**完成标准：** 设置值可解释、预览与保存一致；没有因这轮样式重排增加角落材质残留。

## 任务8：回归、安装包和文档

**修改：** `README.md`、`task_plan.md`、相应设计规格、`docs/reviews/2026-10-27/progress.md`。
**新增：** `docs/reviews/2026-10-27/acceptance.md`，记录实际版本、测试环境和结果。

- [ ] 每批运行对应测试；最终在 `tauri-app` 运行 `npm run test`、`npm run lint`、`npm run build`，在 `src-tauri` 运行 `cargo test --offline`、`cargo fmt --check`、`cargo check --offline`。
- [ ] 按下表做Windows实测，并记录失败次数与场景；未具备的双屏/缩放环境标为未测，不能填通过。
- [ ] 在 `tauri-app` 执行 `npm run tauri:build`；确认测试对象为本次构建的实际exe/安装包。替换正在运行的用户版本前另行明确安装动作，不自动覆盖安装。
- [ ] 更新README当前技术栈和能力；旧计划保留历史、标清已替代状态。标记可靠性设计中旧22px区域裁剪附录已由DWM方案取代。
- [ ] 对照用户原话逐项检查：运行稳定、界面不乱、设置排列有序，以及两条已确认的贴边规则。

| 场景 | 达标条件 |
| --- | --- |
| 启动、关闭、重开 | 后台启动；Alt+F4仅隐藏；托盘/快捷键可再次呼出 |
| 左右贴边 | 拖到边缘松开自动收起；重复Hide不漂移；恢复后可缩放 |
| 悬停与焦点 | 悬停不抢原应用输入；移出自动收回；设置关闭后可直接输入 |
| 保存 | 普通保存后隐藏；贴边展开保存后收回原侧；中文确认候选不误提交 |
| 设置 | 编辑模板后切主题不丢编辑；保存/取消只影响对应区域 |
| 布局 | 500/560/800宽度、中英文及反馈出现时无关键控件溢出 |
| 外观 | 不透明度三档可区分；三种主题把手可见；圆角无新增异常 |
| 连续使用 | Capture/Hide/展开/收起100轮，无错位、失焦或错误模式 |

## 明确延期与风险

- 草稿旧写覆盖新写、故障下配置事务、外部文件竞争：按用户要求延期，本轮不能宣称数据存储所有风险已消除。
- 双屏内部交界可能涉及单窗口模型限制：先复现，若需模型变更，单列方案；DPI修复不等于双屏交界根治。
- 窗口动画和Windows材质需要实机确认；先保证转换结果，再恢复平滑动画，不用动画掩盖状态错误。
- 计划不估算工期，避免把未现场验证的问题包装成确定天数。
