---
name: plc-reverse:mfc-ui-surface-map
description: 映射 MFC 消息处理函数。从 UI 行为反推核心逻辑入口（OnCmdMsg, MessageMap）。
---

# plc-reverse:mfc-ui-surface-map

## 目标
找到用户点击按钮后，真正执行业务逻辑的 C++ 函数地址。

## 关键技术点
1. **资源逆向**：通过 Resource Hacker 找到 Menu/Dialog 的 ID（如 `ID_DOWNLOAD_PROJECT = 32771`）。
2. **消息映射搜索**：
   - 在 IDA 中搜索该 ID（常数）。
   - 定位 `AFX_MSGMAP` 表或 `OnCommand`/`OnCmdMsg` 虚函数。
3. **定位 Handler**：找到最终处理该 Command ID 的成员函数地址。

## 价值
这是**动态调试的起点**。在此处下断点，可以捕获完整的调用栈，从而分析核心业务逻辑。
