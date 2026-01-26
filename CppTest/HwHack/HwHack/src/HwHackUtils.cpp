#include "stdafx.h"

#include "HwHackUtils.h"

#include <cstring>
#include <iostream>
#include <psapi.h>

#pragma comment(lib, "Psapi.lib")

namespace hw {

/**
 * @brief 获取模块基址与大小。
 * @param hMod 模块句柄。
 * @param base 输出基址。
 * @param size 输出大小。
 * @return 成功返回 true。
 */
bool GetModuleRange(HMODULE hMod, uintptr_t* base, size_t* size) {
    MODULEINFO mi;
    if (!hMod || !base || !size) return false;
    if (!GetModuleInformation(GetCurrentProcess(), hMod, &mi, sizeof(mi))) return false;
    *base = reinterpret_cast<uintptr_t>(mi.lpBaseOfDll);
    *size = static_cast<size_t>(mi.SizeOfImage);
    return true;
}

/**
 * @brief 判断指针是否可读。
 * @param p 指针。
 * @return 可读返回 true。
 */
bool IsReadablePtr(const void* p) {
    if (!p) return false;
    MEMORY_BASIC_INFORMATION mbi;
    if (!VirtualQuery(p, &mbi, sizeof(mbi))) return false;
    if (mbi.State != MEM_COMMIT) return false;
    if (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)) return false;
    return true;
}

/**
 * @brief 判断指针是否落在指定范围内。
 * @param p 指针。
 * @param base 范围基址。
 * @param size 范围大小。
 * @return 在范围内返回 true。
 */
bool PtrInRange(const void* p, uintptr_t base, size_t size) {
    uintptr_t v = reinterpret_cast<uintptr_t>(p);
    return v >= base && v < (base + size);
}

/**
 * @brief 判断对象虚表是否位于指定模块范围。
 * @param obj 对象指针。
 * @param base 模块基址。
 * @param size 模块大小。
 * @return 命中返回 true。
 */
bool IsVtableInModule(const void* obj, uintptr_t base, size_t size) {
    if (!IsReadablePtr(obj)) return false;
    const void* vtbl = *reinterpret_cast<const void* const*>(obj);
    if (!IsReadablePtr(vtbl)) return false;
    return PtrInRange(vtbl, base, size);
}

/**
 * @brief 获取对象虚表指针。
 * @param obj 对象指针。
 * @return 虚表指针；不可读返回 nullptr。
 */
const void* GetVtablePtr(const void* obj) {
    if (!IsReadablePtr(obj)) return nullptr;
    const void* vtbl = *reinterpret_cast<const void* const*>(obj);
    return IsReadablePtr(vtbl) ? vtbl : nullptr;
}

/**
 * @brief 判断对象是否为期望虚表类型。
 * @param obj 对象指针。
 * @param expectedVtbl 期望虚表指针。
 * @return 匹配返回 true。
 */
bool IsExpectedClass(const void* obj, const void* expectedVtbl) {
    if (!obj || !expectedVtbl) return false;
    const void* vtbl = GetVtablePtr(obj);
    return vtbl == expectedVtbl;
}

/**
 * @brief 读取对象内指定偏移的 int32。
 * @param base 对象指针。
 * @param offset 偏移。
 * @param out 输出值。
 * @return 读取成功返回 true。
 */
bool ReadI32(const void* base, size_t offset, int* out) {
    if (!out) return false;
    *out = 0;
    if (!IsReadablePtr(base) || !IsReadablePtr(reinterpret_cast<const void*>(
            reinterpret_cast<uintptr_t>(base) + offset))) {
        return false;
    }
    *out = *reinterpret_cast<const int*>(reinterpret_cast<const unsigned char*>(base) + offset);
    return true;
}

/**
 * @brief 输出指针日志（受 verbose 控制）。
 * @param settings 运行时设置。
 * @param name 名称。
 * @param p 指针。
 */
void LogPtr(const Settings& settings, const char* name, const void* p) {
    if (!settings.verbose) return;
    std::cout << "[DBG] 指针 " << name << "=0x" << std::hex
              << reinterpret_cast<uintptr_t>(p) << std::dec << "\n";
}

/**
 * @brief 输出模块日志（受 verbose 控制）。
 * @param settings 运行时设置。
 * @param name 名称。
 * @param hMod 模块句柄。
 */
void LogModule(const Settings& settings, const char* name, HMODULE hMod) {
    if (!settings.verbose) return;
    std::cout << "[DBG] 模块 " << name << "=0x" << std::hex
              << reinterpret_cast<uintptr_t>(hMod) << std::dec << "\n";
}

/**
 * @brief 输出模块范围日志（受 verbose 控制）。
 * @param settings 运行时设置。
 * @param name 名称。
 * @param base 基址。
 * @param size 大小。
 */
void LogModuleRange(const Settings& settings, const char* name, uintptr_t base, size_t size) {
    if (!settings.verbose) return;
    std::cout << "[DBG] 模块范围 " << name << "_base=0x" << std::hex << base
              << " size=0x" << size << std::dec << "\n";
}

/**
 * @brief 输出对象虚表日志（受 verbose 控制）。
 * @param settings 运行时设置。
 * @param name 名称。
 * @param obj 对象指针。
 */
void LogVtable(const Settings& settings, const char* name, const void* obj) {
    if (!settings.verbose) return;
    if (!IsReadablePtr(obj)) {
        std::cout << "[DBG] " << name << "_对象=不可读\n";
        return;
    }
    const void* vtbl = *reinterpret_cast<const void* const*>(obj);
    std::cout << "[DBG] " << name << "_虚表=0x" << std::hex
              << reinterpret_cast<uintptr_t>(vtbl) << std::dec << "\n";
}

/**
 * @brief 输出对象内 byte 值日志（受 verbose 控制）。
 * @param settings 运行时设置。
 * @param name 名称。
 * @param base 对象指针。
 * @param offset 偏移。
 */
void LogU8(const Settings& settings, const char* name, const void* base, size_t offset) {
    if (!settings.verbose) return;
    if (!IsReadablePtr(base) || !IsReadablePtr(reinterpret_cast<const void*>(
            reinterpret_cast<uintptr_t>(base) + offset))) {
        std::cout << "[DBG] " << name << "=不可读\n";
        return;
    }
    unsigned int v = *reinterpret_cast<const unsigned char*>(
        reinterpret_cast<const unsigned char*>(base) + offset);
    std::cout << "[DBG] " << name << "=0x" << std::hex << v << std::dec << "\n";
}

/**
 * @brief MBCS CString 转 UTF-8。
 * @param s MBCS 字符串。
 * @return UTF-8 字符串。
 */
std::string ToUtf8FromMbc(const CString& s) {
    if (s.IsEmpty()) return std::string();
    int wlen = MultiByteToWideChar(CP_ACP, 0, s, -1, nullptr, 0);
    if (wlen <= 0) return std::string();
    std::wstring ws(static_cast<size_t>(wlen), L'\0');
    MultiByteToWideChar(CP_ACP, 0, s, -1, &ws[0], wlen);
    int ulen = WideCharToMultiByte(CP_UTF8, 0, ws.c_str(), -1, nullptr, 0, nullptr, nullptr);
    if (ulen <= 0) return std::string();
    std::string out(static_cast<size_t>(ulen), '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws.c_str(), -1, &out[0], ulen, nullptr, nullptr);
    if (!out.empty() && out.back() == '\0') out.pop_back();
    return out;
}

/**
 * @brief Wide 字符串转 UTF-8。
 * @param ws Wide 字符串。
 * @return UTF-8 字符串。
 */
std::string ToUtf8FromWide(const wchar_t* ws) {
    if (!ws || !*ws) return std::string();
    int ulen = WideCharToMultiByte(CP_UTF8, 0, ws, -1, nullptr, 0, nullptr, nullptr);
    if (ulen <= 0) return std::string();
    std::string out(static_cast<size_t>(ulen), '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws, -1, &out[0], ulen, nullptr, nullptr);
    if (!out.empty() && out.back() == '\0') out.pop_back();
    return out;
}

/**
 * @brief ANSI 字符串转 UTF-8。
 * @param s ANSI 字符串。
 * @return UTF-8 字符串。
 */
std::string ToUtf8FromAnsi(const char* s) {
    if (!s || !*s) return std::string();
    int wlen = MultiByteToWideChar(CP_ACP, 0, s, -1, nullptr, 0);
    if (wlen <= 0) return std::string();
    std::wstring ws(static_cast<size_t>(wlen), L'\0');
    MultiByteToWideChar(CP_ACP, 0, s, -1, &ws[0], wlen);
    return ToUtf8FromWide(ws.c_str());
}

/**
 * @brief 获取窗口标题并转为 UTF-8。
 * @param hwnd 窗口句柄。
 * @return UTF-8 标题。
 */
std::string GetWindowTextUtf8(HWND hwnd) {
    if (!hwnd) return std::string();
    if (IsWindowUnicode(hwnd)) {
        wchar_t wbuf[256] = {0};
        GetWindowTextW(hwnd, wbuf, static_cast<int>(sizeof(wbuf) / sizeof(wbuf[0]) - 1));
        return ToUtf8FromWide(wbuf);
    }
    char buf[256] = {0};
    GetWindowTextA(hwnd, buf, static_cast<int>(sizeof(buf) / sizeof(buf[0]) - 1));
    return ToUtf8FromAnsi(buf);
}

/**
 * @brief 获取窗口类名并转为 UTF-8。
 * @param hwnd 窗口句柄。
 * @return UTF-8 类名。
 */
std::string GetClassNameUtf8(HWND hwnd) {
    if (!hwnd) return std::string();
    wchar_t wbuf[128] = {0};
    if (GetClassNameW(hwnd, wbuf, static_cast<int>(sizeof(wbuf) / sizeof(wbuf[0]) - 1)) > 0) {
        return ToUtf8FromWide(wbuf);
    }
    char buf[128] = {0};
    GetClassNameA(hwnd, buf, static_cast<int>(sizeof(buf) / sizeof(buf[0]) - 1));
    return ToUtf8FromAnsi(buf);
}

/**
 * @brief 带超时发送 TreeView 消息。
 * @param settings 运行时设置（超时配置）。
 * @param hTree TreeView 句柄。
 * @param msg 消息 ID。
 * @param wParam wParam 参数。
 * @param lParam lParam 参数。
 * @param outResult 输出结果。
 * @return 发送成功返回 true。
 */
bool TrySendTreeMsg(const Settings& settings, HWND hTree, UINT msg, WPARAM wParam, LPARAM lParam,
                    LRESULT* outResult) {
    DWORD_PTR result = 0;
    // 使用超时消息避免 TreeView 卡死。
    if (!SendMessageTimeout(hTree, msg, wParam, lParam, SMTO_ABORTIFHUNG,
                            settings.treeMsgTimeoutMs, &result)) {
        return false;
    }
    if (outResult) *outResult = static_cast<LRESULT>(result);
    return true;
}

/**
 * @brief NameMap 查询前统一转大写。
 * @param mapNameToId NameMap 查询函数。
 * @param mapThis NameMap this 指针。
 * @param name 待查询名称。
 * @param outId 输出 ID。
 * @return 查询成功返回 true。
 */
bool MapNameToIdUpper(FnMapNameToId mapNameToId, void* mapThis, const char* name, int* outId) {
    if (!mapNameToId || !mapThis || !name || !*name || !outId) return false;
    char buf[256] = {0};
    strncpy_s(buf, name, _TRUNCATE);
    DWORD len = static_cast<DWORD>(lstrlenA(buf));
    // NameMap 内部按大写键存储，先统一转大写再查找。
    if (len) {
        CharUpperBuffA(buf, len);
    }
    int id = 0;
    int ok = mapNameToId(mapThis, buf, &id);
    if (ok) *outId = id;
    return ok != 0;
}

/**
 * @brief 判断类型名是否包含 MASTER。
 * @param typeName 类型名。
 * @return 包含返回 true。
 */
bool IsMasterTypeName(const char* typeName) {
    if (!typeName || !*typeName) return false;
    char buf[128] = {0};
    strncpy_s(buf, typeName, _TRUNCATE);
    DWORD len = static_cast<DWORD>(lstrlenA(buf));
    if (len) {
        CharUpperBuffA(buf, len);
    }
    return strstr(buf, "MASTER") != nullptr;
}

}  // namespace hw
