---
name: plc-reverse:protocol-implementation
description: 通信协议逆向与重实现。编写第三方客户端与 PLC 通信。
---

# plc-reverse:protocol-implementation

## 目标
完全理解上位机与 PLC 之间的通信报文，并编写自己的驱动库。

## 核心操作
1. **抓包与关联**：Wireshark 抓包 + 动态调试 `send/recv` 函数。将报文内容与内存数据对应。
2. **加密/校验还原**：
   - 找到计算 Checksum/CRC 的函数。
   - 找到会话密钥生成的逻辑。
3. **状态机重现**：
   - 握手 -> 认证 -> 保持心跳 -> 读写数据。
   - 在代码中重写这个状态机。

## 产出
- 一个 Python 脚本或 C# 库，可以在不打开原软件的情况下，直接读写 PLC 变量。
