---
name: plc-reverse:static-core-logic
description: 深度静态分析。还原控制流图（CFG），推导类继承关系，还原伪代码。
---

# plc-reverse:static-core-logic

## 目标
将汇编代码还原为可读的 C/C++ 伪代码，理解核心算法。

## 重点关注
1. **校验逻辑**：License 校验、CRC 校验、完整性检查。（目标：理解算法以便在自定义工具中复现或规避）。
2. **数据序列化**：Load/Save 函数。理解数据如何在内存和文件间转换。
3. **对象生命周期**：构造函数（Constructor）和析构函数。识别类的成员变量布局。

## 技巧
- **RTTI 恢复**：利用 MSVC 的 RTTI 信息自动重命名类和虚表。
- **虚函数表还原**：手动修复 IDA 中的虚表调用，使其显示为 `call [ecx+Offset]` -> `call CMyClass::Func`。
