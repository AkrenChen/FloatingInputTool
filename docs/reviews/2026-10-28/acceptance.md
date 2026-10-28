# 独立合成样机验收记录

范围：用户批准先验证底层原生合成可行性。实验位于 `experiments/composition-probe`；未迁移正式版、未安装或提交。本记录不代表生产环境验收。

## 当前结论

Windows build 26200 上，Windows.UI.Composition 可与现有 Tauri / 窗口式 WebView2 共存。现阶段没有证据要求更换 Tauri 宿主。独立动态背景窗口的计数和移动块在前方样机中实时更新，界面文字保持清晰。

## 实际检查

| 检查 | 结果 |
| --- | --- |
| 原生 host backdrop + Gaussian 初始化 | 成功 |
| 固定模拟聚焦、底板 0，比较附加模糊 0 / 64 | 可见差异；0 仍有系统基础模糊 |
| 独立染色浓度 | 可调 |
| 阴影范围 0 / 24、浓度 0 / 0.71 / 1 | 可见硬边与扩散变化 |
| 25px 圆角 | 本机静态检查未见灰色直角 |
| 真实聚焦 | 原生读回附加模糊 12、底板 60% |
| 激活背景参照使样机失焦 | 原生读回附加模糊 4、底板 20%，仍有实时磨砂 |
| 再次点击样机 | 恢复聚焦参数 |
| Unicode 中文 / 英文文本 | 前轮输入可显示；不等于 IME 验收 |
| 最新轮多行 / 选区 | 自动化未确认 textarea 编辑焦点，未继续注入，仍待验收 |
| Rust release build / clippy -D warnings / fmt | 通过 |
| 几何测试 | 2 项通过；仅验证 DPI 数学换算与最小化尺寸，不等于实际多显示器测试 |

## 限制与后续门槛

HostBackdrop 提供基础磨砂，附加模糊标准差为 0 无法实现完全清透。这是能力边界，不能用滑块名称掩盖。

正式迁移前仍需：中文 IME 组合与候选、多行和选区、透明阴影边距命中测试、贴边坐标适配、实际高 DPI / 多显示器、生命周期循环、长期资源与功耗、设备丢失和另一台机器验证。

运行时聚焦问题定位到顶层与 WebView 子窗口焦点语义不同；现用前台根 HWND 判断。该变更已经 release 重建并实测聚焦 / 失焦往返。

正式配置 SHA256 前后保持 `2A5E74DACBEBCA42FEDA11FCDFC57F730E800DB1800EB6E6D1AB689C0FE515C0`。样机保留默认参数、处于跟随焦点模式并保持打开。

用户原话对照：“按你的方案来”对应先做独立样机评估底层路线；已经交付实际原生渲染与调参验证，没有将样机包装为正式迁移或完整稳定性验收。

## 整窗淡化追加验证

用户进一步明确选择“整个窗口一起变淡，包括文字和按钮”。新增 `focusedOpacity` / `idleOpacity`（10%–100%），通过 Win32 layered-window alpha 控制主窗口，原生回读当前 alpha。阴影浓度仍为独立 `shadowOpacity`，界面改为百分比显示。

重启前再次核对用户最新参数：聚焦/失焦附加模糊 0、底板 0，阴影范围 10、浓度已由 0.05 调为 0、偏移 2、圆角 24。已保留这些值；新加整窗不透明度默认聚焦 100%、失焦 60%，跟随焦点。快照保存到 `experiments/composition-probe/tuned-parameters.json`。

release 构建、clippy、fmt 和 HTML 内脚本语法检查通过。实际聚焦截图与原生回读为整窗 100%；随后刷新捕获失焦画面与原生回读 60%，文字、按钮和底板一起变淡，后方内容明显可见。期间用户正在操作，自动化激活请求被输入保护中断，未继续争抢焦点，也没有据此宣称重复切换压力测试通过。

整窗 alpha 会同时降低阴影的最终可见浓度。它不移除 HostBackdrop 的基础模糊，也不是只改变底板。正式版尚未迁移。

## 阴影与失焦磨砂回归修正（覆盖上节整窗 alpha 结论）

用户随后发现阴影浓度不生效、低于 100% 只剩四角，以及失焦磨砂消失。旧整窗 alpha 和 LayerVisual 实现不再作为验收通过依据。

现使用离屏圆角遮罩 + 纯阴影 SpriteVisual，阴影浓度控制独立影子视觉；原生 HWND 不再设置 layered alpha。用户进一步明确选择文字按钮单独淡化、磨砂独立保留，新参数 focusedMaterialOpacity / idleMaterialOpacity 控制背景覆盖强度，旧 focusedOpacity / idleOpacity 控制界面文字按钮。

最终实测：阴影 0% 消失、49% 沿四边连续较淡、100% 更重；真实失焦时界面 58%、磨砂覆盖 100%、附加模糊 64，动态网格在窗口内仍明显模糊。验证详情与被否定的路线见 shadow-debugging.md。正式版未迁移。

## Dock follow-up acceptance (candidate)
- User manually tested the candidate and confirmed automatic collapse after leaving/clicking another window.
- User confirmed no additional gray block remains around the dock strip.
- Computer Use read-only snapshot confirms right-docked visible width22. Attempted automated drags were interrupted by physical user input and are not counted as passed checks.
- Frontend build/lint/5 tests and Rust12 tests passed before candidate build. Bundled candidate built with tauri/custom-protocol; production process/config not replaced.
- User narrowed remaining work to dock-strip appearance. No compact-host geometry change was implemented; existing accepted behavior is retained. Visual preference requested.

User accepted dock-strip visual design and requested only font-weight700. This acceptance supersedes earlier pending style preference. Shadow click-through work is outside the user's subsequently narrowed visual-only request.

User explicitly confirmed first-hover then editor-click regression desktop retest passed (response annotation). This closes the pending real first-expansion acceptance item; automated input had been interrupted, so desktop evidence is user verification.
