# Technical Notes

> 记录硬核技术细节：地址、偏移、原型。

## 1. Global Offsets (基址偏移)
| Name | RVA (Offset) | Type | Description |
|---|---|---|---|
| `g_AppInstance` | `0x......` | `CWinApp*` | 全局 App 单例 |
| `Fn_CheckLicense`| `0x......` | `bool(__cdecl*)`| 校验逻辑（需分析） |

## 2. Class Structures (ReClass)
### `CProjectData` (Size: 0x...)
- `+0x00` vtable
- `+0x04` m_ProjectName (CString)
- `+0x...`

## 3. Function Prototypes
```cpp
// 示例：核心通信函数
int __stdcall SendCommand(void* pContext, char* buffer, int len);
```

## 4. Hook Points
计划 Hook 的位置：
寄存器上下文要求：
