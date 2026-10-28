# 窗口与桌面合成方案评估

日期：2026-10-28。范围：评估，不修改应用实现，不迁移框架。

## 结论

可以替换底层，但不建议直接重写整个应用。推荐先验证 Windows 原生合成层，保留 React 界面和 Rust 业务；如果现有 Tauri/WebView2 宿主不能正确叠加效果，再仅替换主窗口宿主。更换框架本身不保证获得可调的真实桌面模糊。

当前稳定的系统圆角仍是基线。曾经提出的 25px 圆角不是本次重新承诺的交付项；可在合成样机中附带验证。

## 目标与成功标准

用户要决策的是：为了真实可调的模糊半径、窗口外阴影范围，是否值得更换窗口或渲染底层。

- 模糊采样自窗口后方真实、动态的其他窗口；不能用壁纸、静态截图、模糊编辑器内容替代。
- 模糊强度与底板染色浓度独立；弱模糊和强模糊有可辨别差异，文字始终清晰。
- 失焦时仍保留磨砂，聚焦/失焦参数可分别设置。
- 外阴影的范围、浓度、偏移能独立调节，不能只有无效 CSS 滑块。
- 圆角外与阴影外保持透明，不再出现四块灰色直角。
- 保留中文输入法、多行编辑、草稿、快捷键、托盘、主窗/设置窗切换及贴边收起。
- 不把阴影边距误算为内容边界或贴边热区。

## 当前代码核查

1. `src-tauri/src/acrylic.rs` 使用 `SetWindowCompositionAttribute` 的 Accent Acrylic，策略字段为 state/flags/tint/animation，没有模糊半径字段。
2. Tauri 2.10.3 能提供主 HWND 与 WebView2 controller/environment。取得 HWND 是接入原生合成的入口，但不等于能直接改造现有浏览器视觉树。
3. 当前 Wry 0.54.4 源码调用 `CreateCoreWebView2ControllerWithOptions` / `CreateCoreWebView2Controller`，是窗口式承载；在其 src 中未找到 CompositionController/RootVisualTarget 接入代码。视觉式承载不能当作现有配置开关。
4. `lib.rs` 将窗口生命周期、快捷键、托盘、设置窗模态关系、贴边几何和业务状态连接到 Tauri。替换主窗需要抽象窗口适配层；纯格式化/存储代码可以保留，AppHandle 相关函数不能原样搬走。
5. 贴边位置和鼠标热区目前基于实际窗口边界。自绘阴影若占用透明边距，必须引入内容矩形与外部窗口矩形的区别。
6. 本机系统内核 build 为 26200；系统产品名称字段仍显示 Windows 10，不能据此误判兼容性。另一台机器版本尚未核实。

## 原生 API 与 TranslucentTB 的边界

TranslucentTB 参考源码 revision：`d4636e439865df0a1a1419db408e055740ce5c74`。

- 其 Accent Acrylic 路径没有半径字段。
- 其自定义 Blur 路径单独使用 `XamlBlurBrush`，内部创建 `GaussianBlurEffect`，设置 `BlurAmount`，再叠加染色。调用端将 `BlurRadius / 3` 传入效果，说明 UI 半径和效果参数也不应不加区分地混用。
- 其 `CreateBackdropBrush` 采样成立依赖任务栏现有 XAML/透明宿主，不能直接推导在本项目普通窗口中效果相同。
- 微软为非 UWP 窗口提供 `DWMWA_USE_HOSTBACKDROPBRUSH`，文档支持起点为 Windows 11 build 22000；配合 `Windows.UI.Composition` 的 host backdrop 是值得验证的官方入口。
- `CreateHostBackdropBrush` 的结果仍受系统透明度/电源策略影响；零效果参数是否能得到完全清透的背景、最低固有模糊程度，需要实测，不能先承诺。
- 不要混淆 `Windows.UI.Composition` 与 WinUI 3 的 `Microsoft.UI.Composition`。后者官方文档说明外部内容与透明 WebView2 存在合成限制，切到 WinUI 3 后叠加一个普通 AcrylicBrush 不构成已验证方案。

## 方案比较

| 方案 | 能力判断 | 迁移范围与主要风险 | 建议 |
| --- | --- | --- | --- |
| A. 保留 Tauri，接入自定义 Windows.UI.Composition 背景/阴影 | 有官方 API 入口，能否与当前窗口式 WebView2 正确叠放尚未验证 | 中等；UI 与业务保留。主要卡点是透明浏览器与原生视觉层的顺序、窗口样式和合成生命周期 | 第一阶段验证 |
| B. 原生 Win32 主窗口 + Composition + 视觉式 WebView2，保留 React/Rust | 对视觉树和浏览器叠放控制更明确，适合以视觉式 WebView2 为基础做样机 | 较高；改宿主、输入/鼠标转发、焦点、DPI、缩放、IPC 及贴边适配。托盘和设置可先留 Tauri，避免一次性迁移全部功能 | 若 A 被宿主限制，优先选择 |
| C. WinUI 3 / WPF 原生 UI 重写 | 可获得原生控件与输入体验，但系统 Acrylic 同样没有现成半径旋钮；真实桌面自定义模糊仍需原生合成验证 | 高；编辑器、主题、设置、事件绑定重写，WinUI 还须处理外部内容限制。WPF 的普通 BlurEffect 不能当作桌面采样 | 仅在长期决定全面原生化时考虑 |
| D. Electron | 可保留 React；其普通背景材质接口不直接提供所需半径和阴影范围，仍需原生实现 | 中高；替换 IPC/托盘/快捷键，另引入打包 Chromium 的分发成本；关键合成问题依旧 | 不推荐为本需求迁移 |

成本为相对工程范围，不是工期承诺。A 成功与否决定 B 是否必要；不能在未实测前称 A 已可落地。

## 推荐验证顺序

1. 建立独立可行性样机，不替换目前使用中的窗口。先只做真实背景采样、可调模糊、染色、圆角裁剪与自绘阴影。
2. 接入一个最小透明 WebView2 文本输入框，验证中文输入、光标、选区和滚动。不能用“无编辑器的玻璃方块”代表完整可行。
3. 优先尝试现有 Tauri 宿主；若确定原生层被窗口式 WebView2 遮挡或透明路径不成立，改测原生 HWND + 视觉式 WebView2。验证阶段不长期维护 Wry 私有分叉。
4. 合成层跑通后才接现有 React 调参页，再验证贴边、DPI、焦点和生命周期；全部通过后决定迁移。

这是可行性样机，属于完整迁移之前的验证交付：可以回答视觉路线是否成立，不能证明生产稳定性或完成全部功能迁移。

## 阴影方案

优先验证单个宿主窗口预留透明边距，在其内绘制圆角内容与阴影。影子在内容外、实际 HWND 内，不能超出宿主像素边界。背景采样层只覆盖内容矩形，不应把透明阴影边距也铺成磨砂。

需要解决透明边距点击行为、内容边缘拖拽缩放、贴边内容坐标、DPI 和阴影被屏幕边缘裁掉的问题。若单窗透明边距无法满足交互，再评估独立无焦点阴影窗；后者增加位置/Z-order/显示隐藏同步成本，不作为默认方案。

## 最小验收矩阵与停止条件

- 在窗口后移动有细字/线条的普通应用，比较多档模糊；确认内容动态更新、变化独立于染色，编辑文字不被模糊。具体 slider 单位待确认 API 语义后命名，暂不承诺 0 等于完全无模糊。
- 聚焦、失焦、设置窗切换，确认无变黑、实色回退或灰角；系统自身禁止透明时的表现单独记录。
- 关闭阴影、弱阴影、宽阴影分别观察；透明边距不能抢输入。
- 测试中文输入法、Ctrl+A、删除、粘贴、多行与光标；阴影和模糊参数变化不能打断输入。
- 100%/150%/200% DPI 与可用的多显示器环境测试移动、缩放、左右贴边、悬停展开、保存收起。
- 重复显示/隐藏、焦点切换和贴边各 100 次，检查灰角、焦点和图形资源是否持续增长；记录空闲及拖动调参时 CPU/GPU/内存，和现版本对照。
- 在用户实际使用的两台机器上确认目标 Windows 版本；若包含 Windows 10，Windows 11 host-backdrop 方案不能被包装成完整覆盖。
- 若只能模糊壁纸/截图、必须关闭中文输入、依赖 Explorer 注入或无法消除灰角，不满足目标。停止该路线并记录原因，不降级后宣称完成。

## 主要来源

- [DWM 窗口属性：DWMWA_USE_HOSTBACKDROPBRUSH 的 Win32/Windows 11 支持](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)
- [Windows.UI.Composition.CreateHostBackdropBrush：采样区域与系统策略边界](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.compositor.createhostbackdropbrush?view=winrt-26100)
- [将 Windows Composition 接入 Win32 HWND](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/using-the-visual-layer-with-win32)
- [WebView2 视觉式承载：RootVisualTarget 与输入转发](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2compositioncontroller)
- [创建视觉式 WebView2 controller](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environment3)
- [透明 WebView2 背景，仅接受 alpha 0 或 255](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2controller2)
- [WinUI 3 Composition 外部内容限制](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/composition)
- [系统背景控制器可调属性](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/system-backdrop-controller)
- [Composition DropShadow 的 BlurRadius/Opacity/Offset](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.dropshadow?view=winrt-26100)
- [Electron 窗口接口](https://www.electronjs.org/docs/latest/api/base-window)
- [TranslucentTB 自定义 Blur 实现（固定参考 revision）](https://github.com/TranslucentTB/TranslucentTB/blob/d4636e439865df0a1a1419db408e055740ce5c74/ExplorerTAP/XamlBlurBrush.cpp)

资料核验使用官方页面与本地依赖/参考源码；本次未编译或运行新的合成样机。API 组件分别存在，不代表它们在本项目宿主中组合后已经通过验证。
