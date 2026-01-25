---
name: plc-reverse:dynamic-debug-hook
description: 动态调试与 Hook 开发。运行时修改内存、拦截函数、注入代码。
---

# plc-reverse:dynamic-debug-hook

## 目标
通过运行时操作验证静态分析结论，并实现功能增强。

## 核心操作
1. **参数嗅探**：在关键函数入口下断点，观察寄存器（x86: ECX/EDX, x64: RCX/RDX）中的参数值。
2. **内存修改 (Patching)**：
   - 运行时修改跳转指令（JZ -> JMP）以改变程序流程（例如：强制进入“成功”分支）。
   - 修改内存中的配置数据。
3. **编写 Hook DLL**：
   - 使用 Detours 或 MinHook 库。
   - 拦截目标函数（如 `EncryptData`），在调用前后打印数据或篡改数据。
   - 实现“中间人攻击”式的功能扩展。

## 场景示例
- **场景**：软件在下载前会检查 PLC 固件版本，版本过低则报错。
- **操作**：定位版本检查函数，Hook 该函数，使其永远返回 `true` 或伪造一个高版本号。
