#pragma once

#include <windows.h>

#include <string>

#include "HwHackConfig.h"
#include "HwHackTypes.h"

namespace hw {

/// <summary>
/// 获取模块基址与大小。
/// </summary>
bool GetModuleRange(HMODULE hMod, uintptr_t* base, size_t* size);
/// <summary>
/// 判断指针是否可读。
/// </summary>
bool IsReadablePtr(const void* p);
/// <summary>
/// 判断指针是否落在指定模块范围。
/// </summary>
bool PtrInRange(const void* p, uintptr_t base, size_t size);
/// <summary>
/// 判断对象虚表是否位于指定模块内。
/// </summary>
bool IsVtableInModule(const void* obj, uintptr_t base, size_t size);
/// <summary>
/// 获取对象虚表指针（不可读时返回 nullptr）。
/// </summary>
const void* GetVtablePtr(const void* obj);
/// <summary>
/// 判断对象虚表是否为期望类型。
/// </summary>
bool IsExpectedClass(const void* obj, const void* expectedVtbl);
/// <summary>
/// 读取对象内指定偏移的 int32。
/// </summary>
bool ReadI32(const void* base, size_t offset, int* out);

/// <summary>
/// 输出指针日志。
/// </summary>
void LogPtr(const Settings& settings, const char* name, const void* p);
/// <summary>
/// 输出模块句柄日志。
/// </summary>
void LogModule(const Settings& settings, const char* name, HMODULE hMod);
/// <summary>
/// 输出模块范围日志。
/// </summary>
void LogModuleRange(const Settings& settings, const char* name, uintptr_t base, size_t size);
/// <summary>
/// 输出对象虚表日志。
/// </summary>
void LogVtable(const Settings& settings, const char* name, const void* obj);
/// <summary>
/// 输出对象内 byte 值日志。
/// </summary>
void LogU8(const Settings& settings, const char* name, const void* base, size_t offset);

/// <summary>
/// ANSI CString -> UTF-8。
/// </summary>
std::string ToUtf8FromMbc(const CString& s);
/// <summary>
/// Wide -> UTF-8。
/// </summary>
std::string ToUtf8FromWide(const wchar_t* ws);
/// <summary>
/// ANSI C 字符串 -> UTF-8。
/// </summary>
std::string ToUtf8FromAnsi(const char* s);

/// <summary>
/// 读取窗口标题并转为 UTF-8。
/// </summary>
std::string GetWindowTextUtf8(HWND hwnd);
/// <summary>
/// 读取窗口类名并转为 UTF-8。
/// </summary>
std::string GetClassNameUtf8(HWND hwnd);

/// <summary>
/// 带超时的 TreeView 消息发送（避免 UI 卡死）。
/// </summary>
bool TrySendTreeMsg(const Settings& settings, HWND hTree, UINT msg, WPARAM wParam, LPARAM lParam, LRESULT* outResult);

/// <summary>
/// NameMap 查询前统一转大写。
/// </summary>
bool MapNameToIdUpper(FnMapNameToId mapNameToId, void* mapThis, const char* name, int* outId);
/// <summary>
/// 判断类型名是否为 MASTER。
/// </summary>
bool IsMasterTypeName(const char* typeName);

}  // namespace hw
