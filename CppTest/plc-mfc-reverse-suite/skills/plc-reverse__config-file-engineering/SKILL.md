---
name: plc-reverse:config-file-engineering
description: 工程文件格式全解析。实现脱离原软件的工程文件读取与生成。
---

# plc-reverse:config-file-engineering

## 目标
编写一个能够读写该软件工程文件（.prj, .mcp 等）的独立工具。

## 方法论
1. **File Format Parsing**：
   - 识别头部 Magic Number。
   - 识别压缩/加密层（利用 Triage 阶段的发现）。
   - 识别序列化格式（MFC `CArchive` 序列化是重灾区，需重点还原）。
2. **编写 Parser**：
   - 使用 Python (Kaitai Struct) 或 C++ 编写解析器。
   - 目标是能提取出 Tag 列表、逻辑代码段。
3. **Fuzzing (可选)**：如果解析器崩溃，使用 Fuzzing 技术测试文件结构的健壮性。
