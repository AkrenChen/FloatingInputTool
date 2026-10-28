# 样机整窗不透明度

用户明确选择失焦时整个窗口一起变淡，包括文字和按钮。保留当前读回的聚焦/失焦附加模糊 0、底板 0、阴影范围 10、浓度 0.05、偏移 2、圆角 24。只改独立样机。

- [x] Parameters 增加 focusedOpacity / idleOpacity（0.1–1），默认 1 / 0.6；JSON 缺失字段使用默认值。原生 Scene 保存 HWND，通过 WS_EX_LAYERED + SetLayeredWindowAttributes 设置整窗 alpha，GetLayeredWindowAttributes 回读。焦点与模拟模式走已有 apply 路径，失败返回实际错误。
- [x] HTML 增加两组不透明度滑块，阴影浓度改为百分比显示；JSON 保持 0–1 数值。保留已调好参数，重启进入跟随焦点。
- [x] fmt、clippy、release 构建和 JS 语法检查。仅停止独立样机；实际检查聚焦/失焦的原生读回与画面，包括文字和磨砂可见性；出现合成兼容问题时记录并恢复可用版本。
- [x] README 和验收记录说明整窗 alpha 与 Gaussian / tint / 阴影参数的区别，不宣称系统基础模糊可被移除。

依据：Microsoft SetLayeredWindowAttributes 文档规定 LWA_ALPHA 的 0 为全透明、255 为不透明。https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setlayeredwindowattributes

后续回归覆盖本计划的 HWND alpha 架构：用户报告失焦磨砂消失，随后明确批准文字按钮和磨砂独立调节。已撤回 layered alpha；界面由 CSS opacity 淡化，磨砂由 content.Opacity 独立控制。阴影改为离屏遮罩 + 纯投影 SpriteVisual。最终真实图像验证与失败路线见 docs/reviews/2026-10-28/shadow-debugging.md。
