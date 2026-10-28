# 焦点底板主题设计

用户批准：聚焦时底板约85%，失焦约30%，200ms过渡；可调参数放theme中。目标是使用时文字清楚，闲置时少遮挡桌面。

仅主输入窗口普通/贴边展开状态调整底板，不对整窗或文字设置opacity。保留系统圆角、原生Acrylic、浅阴影和设置窗口的可读性。DWM的实际桌面模糊半径不做可调承诺，通过底板浓度提供焦点区分。

参数统一定义于src/theme.css：聚焦/失焦底板透明度、编辑区透明度、过渡时长。白/黑主题默认85%/30%；自定义主题以已有windowOpacity作为聚焦值，失焦使用主题闲置值且不高于聚焦值。遵从prefers-reduced-motion；无原生材质的环境保持原有不透明回退；贴边把手不套用底板规则。不新增设置项或原生合成补丁。

验收：两种焦点状态明确变化且文字不淡化，切换平滑；主题参数集中；原生回退、贴边样式不被覆盖；前端lint/测试/build通过，最终打开正式构建查看。

## 用户实测反馈与TranslucentTB参考
用户确认当前失焦丢失磨砂。参考TranslucentTB revision d4636e439865df0a1a1419db408e055740ce5c74 的ExplorerTAP/taskbarappearanceservice.cpp，作者明确使用Backdrop而非HostBackdrop绕过inactive回退；该路径依赖任务栏XAML，不能直接移植到Tauri。其旧兼容路径taskbarattributeworker.cpp使用SetWindowCompositionAttribute+AccentPolicy。先在系统圆角、无region条件下验证Accent Acrylic是否保持当前窗口失焦磨砂。不得把CSS底板参数当成真正桌面模糊强度，不直接移植Explorer注入/任务栏hook。此改动为既定焦点体验的必要bug修复，先候选验证，不提前宣称成功。
