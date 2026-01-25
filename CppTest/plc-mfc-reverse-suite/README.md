# plc-mfc-reverse-suite (Engineering Edition)

面向 Windows 下基于 C++/MFC 的 PLC 组态软件的“深度逆向工程与二次开发”Skill 套件。

## 工程背景
针对源代码丢失或文档缺失的遗留工业软件，进行深度分析以实现：
- **功能扩展**：编写插件或 Hook 模块以增强原有功能。
- **互操作性**：分析内部协议以实现第三方客户端对接。
- **系统维护**：修复原有 Bug 或移除由于环境变更导致的兼容性障碍。

## 核心原则
1) **代码即真理**：不依赖猜测，通过反汇编还原核心逻辑。
2) **完全控制**：不仅观察，更要具备修改内存、Hook 函数、重定向流程的能力。
3) **工程化输出**：产出可编译的代码（SDK/Header/Lib）、可用的协议文档和自动化工具。

## 典型工作流
- plc-reverse:case-init (工程初始化)
- plc-reverse:triage-windows-binary (架构分析)
- plc-reverse:mfc-ui-surface-map (UI 交互逻辑)
- plc-reverse:static-core-logic (核心算法还原)
- plc-reverse:dynamic-debug-hook (动态调试与 Hook 验证)
- plc-reverse:config-file-engineering (文件格式解析与生成)
- plc-reverse:protocol-implementation (协议栈重写)
- plc-reverse:delivery-package (交付物打包)
