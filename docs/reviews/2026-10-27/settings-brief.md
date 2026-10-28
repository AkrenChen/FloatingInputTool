# 设置实施边界

工作目录D:/Claude/Project/FloatingInputTool，分支codex/experience-fixes。
只修改SettingsApp.tsx、App.css、index.css、settingsDraft.ts/test.ts，不改App.tsx、lib.rs、appModel.ts或package.json。
执行计划任务6—7：四类tabs（记录/快捷键/外观/高级），目标两行和快捷键两列；dirty保留各区域编辑；关闭使用invoke('close_settings_window')；配置事件不能重置其他编辑；当前生效快捷键来自runtime；反馈固定区域；自定义透明度实际生效并补齐把手，设置不透明背景，圆角统一原生DWM约10px。
先写失败测试验证dirty merge，再实现。不要创建子代理、不要提交或改其他文件。完成报告验证结果。默认允许直接采用计划推荐布局，不等用户。
