---
name: plc-reverse:triage-windows-binary
description: 快速扫描二进制特性，识别编译器版本、库依赖、加密/压缩算法，为代码还原做准备。
---

# plc-reverse:triage-windows-binary

## 目标
快速识别目标的“技术栈”，以便选择正确的开发工具（如 VS 版本）和符号库。

## 关键动作
1. **编译器识别**：VS2010 (v100) / VS2008 (v90)？这决定了 `CString` 等 MFC 类的内存布局。
2. **库依赖分析**：
   - 是否使用了 OpenSSL？（寻找 `libcrypto`/`libssl` 或静态链接特征）
   - 是否使用了 zlib/lzo？（用于解压工程文件）
   - 是否使用了 Protobuf/XML/JSON？
3. **保护壳探测**：是否存在加壳？（UPX/VMP/Themida）。如果是，首要任务是脱壳（Unpacking）以恢复原始逻辑。

## 输出
- `technical_notes.md` -> Binary Info
- 确定后续开发用的 SDK 版本（如 MFC 10.0）。
