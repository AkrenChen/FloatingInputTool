# 阴影浓度问题排查

用户报告浓度滑块无视觉效果。复现时将浓度由 0 调到 80%，前端和 DropShadow 原生回读均更新，但轮廓附近阴影近乎不变，证实不能把属性回读当作像素验收。

对照一：关闭 DWM 非客户区渲染后，0% 阴影仍存在，假设不成立，已撤回该诊断改动。

对照二：0% 时隐藏整个 LayerVisual，固定阴影消失；再次设 100% 显示该层，阴影出现。因此问题定位在自绘合成层路径，尚不把具体驱动或系统内部原因当作定论。

修正：DropShadow 保持固定强度；由 LayerVisual.Opacity 控制浓度，0% 时 SetIsVisible(false)，原生浓度回读改为有效控制对象 LayerVisual。保留用户最新失焦不透明度 58%、附加模糊 64。

整层透明度版本虽可区分 0% / 100%，但用户指出中间浓度仅四角有阴影，且失焦磨砂异常。49% 实际截图也显示边缘不一致，因此该路线未通过，不能交付为修复。

根因分析：旧 LayerVisual 把白色投影源直接作为可见子树放在 HostBackdrop 后方，导致投影源、阴影与背景混合，而不是纯阴影。接下来改为离屏 CompositionVisualSurface 提供圆角 alpha mask，SpriteVisual 只绘制投影，不把白色形状插入可见树。模糊问题需独立对照整窗 layered alpha；不预先认定已解决。

失焦附加模糊 64 / 整窗 58% 在实际网格上仍可见清晰背景，另一次窗口捕获甚至只剩背后窗口。整窗 layered alpha 不是稳定的 HostBackdrop 集成方式，撤回该路径。新对照使用 Composition 根节点 Opacity 加 WebView 面板 CSS opacity 同步淡化，保留原 HWND 合成方式。尚待实际验证，不将原生参数回读当作效果证据。

## 最终实现和实测

纯阴影：离屏 ShapeVisual → CompositionVisualSurface → SurfaceBrush alpha mask → 仅承载阴影的 SpriteVisual。投影源不加入可见树；DropShadow 固定强度，由影子 SpriteVisual.Opacity 调节浓度，0% 隐藏。实际 0%、49%、100% 对照：0% 无自绘阴影，中间值沿四边连续减淡，100% 更重，未再出现只剩四角的情况。

根节点整体淡化也会混入未模糊桌面，这是视觉透明度与背景采样的混合问题。用户明确选择“保留明显磨砂，文字按钮单独变淡；两项分别调节”。因此不再淡化 HWND 或合成根节点，而是分别控制 HTML 面板 opacity 和原生 content.Opacity；影子保持独立浓度。新增 focusedMaterialOpacity / idleMaterialOpacity，默认 1 / 1。原 focusedOpacity / idleOpacity 改为界面淡化参数，标签与说明同步更新，避免继续标为整窗 alpha。

最终 release 实际验证：聚焦界面 100%、磨砂 100%、附加模糊 0；激活独立动态网格参照窗口后，界面降到 58%，磨砂保持 100%、附加模糊 64，后方网格和文字仍明显模糊，界面文字/按钮一起变淡。该结论来自实际图像和状态联合检查，而非仅回读参数。

保留用户最新值：阴影 10 / 49% / 2，圆角 24，聚焦/失焦 tint 均为 0。clippy、fmt、release、JS 语法检查通过。只改独立样机，未迁移或安装正式版。
