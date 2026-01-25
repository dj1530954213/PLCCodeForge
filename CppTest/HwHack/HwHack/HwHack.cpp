#include "stdafx.h"

#include <afxwin.h>
#include <afxcmn.h>
#include <afxcmn.h>

#include <commctrl.h>

#include <iostream>

#include <cstdio>

#include <cstdlib>

#include <string>
#include <vector>
#include <algorithm>
#include <psapi.h>

#pragma comment(lib, "Psapi.lib")

  

// ============================================================================

// 1. 配置区域

// ============================================================================

const DWORD OFFSET_MAKE_NEW = 0x59F10;

const DWORD OFFSET_GET_DEVICE = 0x50770; // 务必确认此偏移

// dll_DPFrame.dll
const DWORD OFFSET_GET_GLOBAL = 0xDB560;
const DWORD OFFSET_GET_LINK = 0x117830;
const DWORD OFFSET_GET_DATA_CONTAINER = 0x106C60;
const DWORD OFFSET_GET_CUR_CONTROL = 0x106C80;
const DWORD OFFSET_UPDATE_VIEW = 0x106E00;
const DWORD OFFSET_GET_PLC_DEVICE = 0x125CB0;
const DWORD OFFSET_CONTAINER_DEVICE_MAP = 0x250;
const DWORD OFFSET_GET_DEVICE_BY_MAP = 0x45E80;
const DWORD OFFSET_MAP_NAME_TO_ID = 0x45E00;
const DWORD OFFSET_ADD_NODE_TO_CFG_TREE = 0x150940;
const DWORD OFFSET_MAP_TREE_TO_ID = 0x149D80;
const DWORD OFFSET_MAP_ID_TO_TREE = 0x149DF0;
const DWORD OFFSET_NAME_TO_ID_MAP_BASE = 0x1FC;
const DWORD OFFSET_TREE_TO_ID_MAP_BASE = 0x9B8;
const DWORD OFFSET_ID_TO_TREE_MAP_BASE = 0x9D4;
const DWORD OFFSET_ON_SLAVE_OPERATE = 0x155D70;
const DWORD OFFSET_GET_COMM_NO_FOR_LINK = 0x1293B0;
const DWORD OFFSET_ON_DPTREE_SLAVE_OPERATE = 0x167AB0;
const DWORD OFFSET_ON_ADD_SLAVE = 0x1A7AF0;
const DWORD OFFSET_FRAME_CONTAINER = 0x640;
const DWORD OFFSET_LINK_ID = 0x10;

// dllDPLogic.dll
const DWORD OFFSET_GET_PAPA_LINK = 0x2E90;
const DWORD OFFSET_GET_LINK_INDEX_MODBUS = 0x2810;
const DWORD OFFSET_GET_LINK_INDEX_DP = 0x2CC0;
const DWORD OFFSET_GET_COMM_INDEX = 0x2830;
const DWORD OFFSET_GET_SUB_COMM_INDEX = 0x2850;
const DWORD OFFSET_GET_COMM_INDEX_DP = 0x2DF0;
const DWORD OFFSET_GET_COMM_INDEX_GATEWAY = 0x37E0;
const DWORD OFFSET_GET_LINK_INDEX_GATEWAY = 0x37C0;
const DWORD OFFSET_GET_THISCLASS_DP_SLAVE = 0x30820;
const DWORD OFFSET_GET_THISCLASS_MODBUS_SLAVE = 0x67010;
const DWORD OFFSET_GET_THISCLASS_GATEWAY = 0x3AC10;
const DWORD OFFSET_GET_LOGIC_ID_FROM_NAME = 0x484D0;
const DWORD OFFSET_GET_USER_NAME = 0x1E30;

  

// ============================================================================

// 2. 类型定义

// ============================================================================

typedef char (__thiscall *FnMakeNewLogicData_Slave)(
    void* pThis,
    CString typeName,
    unsigned int countOrMode,
    char dupFlag,
    unsigned int* pOutIDs,
    void* pLink,
    void* pParent,
    CString desc,
    unsigned int extraFlag,
    void* pContext
);

typedef void* (__thiscall *FnGetDeviceByLogicID)(void* pThis, unsigned int id);
typedef void* (__thiscall *FnGetPLCDeviceDevice)(void* pThis, void* hItem);
typedef int (__thiscall *FnGetDeviceByMap)(void* pThis, int id, void** outDevice);
typedef int (__thiscall *FnMapNameToId)(void* pThis, const char* name, int* outId);

typedef void* (__cdecl *FnGetGlobalContainer)();
typedef void* (__thiscall *FnGetLinkFromNO)(void* pThis, unsigned int a2, unsigned int a3, unsigned int a4);
typedef void* (__thiscall *FnGetDataContainer)(void* pThis);
typedef void (__thiscall *FnGetCurControlIDAndName)(void* pThis, unsigned int* pOutId, CString* pOutName);
typedef char (__thiscall *FnUpdateView)(void* pThis, unsigned int a2);
typedef HTREEITEM (__thiscall *FnAddNodeToCfgTree)(void* pThis, void* pDevice, CTreeCtrl* pTree, HTREEITEM hParent);
typedef int* (__thiscall *FnMapTreeToId)(void* pThis, int key);
typedef int* (__thiscall *FnMapIdToTree)(void* pThis, int key);
typedef char (__thiscall *FnOnSlaveOperate)(
    void* pThis,
    int op,
    void* pLink,
    void* pDevice,
    int commIdx,
    int linkIdx,
    CString name,
    CString typeName);
typedef char (__thiscall *FnOnAddSlave)(
    void* pThis,
    unsigned int commIdx,
    unsigned int linkIdx,
    CString typeName,
    CString address,
    unsigned int count,
    const char* extra);
typedef char (__thiscall *FnOnDPTreeSlaveOperate)(
    void* pThis,
    char op,
    CString name,
    int commIdx,
    int linkIdx,
    CString commName,
    CString linkName,
    unsigned int subIdx);
typedef CString* (__thiscall *FnGetDeviceDisplayName)(void* pThis, CString* outName);
typedef void* (__thiscall *FnGetPapaLink)(void* pThis);
typedef unsigned char (__thiscall *FnGetLinkIndex)(void* pThis);
typedef unsigned int (__thiscall *FnGetIndexU32)(void* pThis);
typedef void* (__cdecl *FnGetThisClass)();
typedef int (__thiscall *FnGetLogicIDFromName)(void* pThis, CString name);
typedef int (__thiscall *FnGetCommunNoForLink)(void* pThis, void* pLink);
typedef CString* (__thiscall *FnGetUserNameA)(void* pThis, CString* outName);

  

struct InjectionParams {

    DWORD addrContainer;

    DWORD addrInstance;

    DWORD valParentData; // 存 ID 或 指针

    DWORD addrLink;

    DWORD commIdx;

    DWORD linkIdx;

} g_Params;

  

HWND g_hMainWnd = NULL;

HWND g_hTreeView = NULL;
HWND g_hTreeViewFallback = NULL;
static const int kTreeCtrlIdWanted = 1558;

static const bool kVerbose = true;
static const bool kTraceLinkSearch = true;
static const bool kDumpTreeOnStart = true;
static const int kDumpTreeMaxNodes = 0; // 0 = no limit
static const int kDumpTreeMaxDepth = 0; // 0 = no limit
static const bool kDumpTreeAfterInject = true;
static const int kDumpTreeChildrenLimit = 20;
static const bool kTryDeviceDisplayName = false;
static const bool kPreferAddNodeToCfgTree = true;
static const bool kEnableOnSlaveOperate = true;
static const bool kEnableOnDPTreeOperate = false;
static const bool kEnableSmartInsert = false;
static const bool kEnableDeviceIntrospection = false;
static const bool kEnableLinkCommProbe = false;
static const bool kPreferOnAddSlave = true;
static const unsigned int kMaxCommScan = 64;
static const unsigned int kMaxLinkScan = 64;
static const unsigned int kMaxSubScan = 4;
static char g_TargetName[256] = {0};
static char g_TargetNameFull[256] = {0};
static char g_TargetNameShort[256] = {0};
static char g_TargetNameType[256] = {0};
static HTREEITEM g_TargetItem = NULL;
static const char* g_LastStage = "init";
static const UINT_PTR kInjectTimerId = 7777;
static const UINT_PTR kDumpAfterTimerId = 7778;
static HTREEITEM g_PendingDumpTarget = NULL;

struct ResolvedContext {
    void* pContainer;
    void* pDataContainer;
    void* pParent;
    void* pLink;
    unsigned int commIdx;
    unsigned int linkIdx;
    unsigned int subIdx;
};

struct LinkMatch {
    void* link;
    unsigned int commIdx;
    unsigned int linkIdx;
    unsigned int subIdx;
};

static bool GetModuleRange(HMODULE hMod, uintptr_t* base, size_t* size) {
    MODULEINFO mi;
    if (!hMod || !base || !size) return false;
    if (!GetModuleInformation(GetCurrentProcess(), hMod, &mi, sizeof(mi))) return false;
    *base = (uintptr_t)mi.lpBaseOfDll;
    *size = (size_t)mi.SizeOfImage;
    return true;
}

static bool IsReadablePtr(const void* p) {
    if (!p) return false;
    MEMORY_BASIC_INFORMATION mbi;
    if (!VirtualQuery(p, &mbi, sizeof(mbi))) return false;
    if (mbi.State != MEM_COMMIT) return false;
    if (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)) return false;
    return true;
}

static bool PtrInRange(const void* p, uintptr_t base, size_t size) {
    uintptr_t v = (uintptr_t)p;
    return v >= base && v < (base + size);
}

static bool IsVtableInModule(const void* obj, uintptr_t base, size_t size) {
    if (!IsReadablePtr(obj)) return false;
    const void* vtbl = *(const void* const*)obj;
    if (!IsReadablePtr(vtbl)) return false;
    return PtrInRange(vtbl, base, size);
}

static void LogPtr(const char* name, const void* p) {
    if (!kVerbose) return;
    std::cout << "[DBG] 指针 " << name << "=0x" << std::hex << (uintptr_t)p << std::dec << "\n";
}

static void LogModule(const char* name, HMODULE hMod) {
    if (!kVerbose) return;
    std::cout << "[DBG] 模块 " << name << "=0x" << std::hex << (uintptr_t)hMod << std::dec << "\n";
}

static void LogModuleRange(const char* name, uintptr_t base, size_t size) {
    if (!kVerbose) return;
    std::cout << "[DBG] 模块范围 " << name << "_base=0x" << std::hex << base
              << " size=0x" << size << std::dec << "\n";
}

static void LogVtable(const char* name, const void* obj) {
    if (!kVerbose) return;
    if (!IsReadablePtr(obj)) {
        std::cout << "[DBG] " << name << "_对象=不可读\n";
        return;
    }
    const void* vtbl = *(const void* const*)obj;
    std::cout << "[DBG] " << name << "_虚表=0x" << std::hex << (uintptr_t)vtbl << std::dec << "\n";
}

static const void* GetVtablePtr(const void* obj) {
    if (!IsReadablePtr(obj)) return NULL;
    const void* vtbl = *(const void* const*)obj;
    return IsReadablePtr(vtbl) ? vtbl : NULL;
}

static bool IsExpectedClass(const void* obj, const void* expectedVtbl) {
    if (!obj || !expectedVtbl) return false;
    const void* vtbl = GetVtablePtr(obj);
    return vtbl == expectedVtbl;
}

static void LogU8(const char* name, const void* base, size_t offset) {
    if (!kVerbose) return;
    if (!IsReadablePtr(base) || !IsReadablePtr((const void*)((uintptr_t)base + offset))) {
        std::cout << "[DBG] " << name << "=不可读\n";
        return;
    }
    unsigned int v = *(const unsigned char*)((const unsigned char*)base + offset);
    std::cout << "[DBG] " << name << "=0x" << std::hex << v << std::dec << "\n";
}

static bool ReadI32(const void* base, size_t offset, int* out) {
    if (!out) return false;
    *out = 0;
    if (!IsReadablePtr(base) || !IsReadablePtr((const void*)((uintptr_t)base + offset))) {
        return false;
    }
    *out = *(const int*)((const unsigned char*)base + offset);
    return true;
}

static std::string ToUtf8FromMbc(const CString& s) {
    if (s.IsEmpty()) return std::string();
    int wlen = MultiByteToWideChar(CP_ACP, 0, s, -1, NULL, 0);
    if (wlen <= 0) return std::string();
    std::wstring ws(wlen, L'\0');
    MultiByteToWideChar(CP_ACP, 0, s, -1, &ws[0], wlen);
    int ulen = WideCharToMultiByte(CP_UTF8, 0, ws.c_str(), -1, NULL, 0, NULL, NULL);
    if (ulen <= 0) return std::string();
    std::string out(ulen, '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws.c_str(), -1, &out[0], ulen, NULL, NULL);
    if (!out.empty() && out.back() == '\0') out.pop_back();
    return out;
}

static std::string ToUtf8FromWide(const wchar_t* ws) {
    if (!ws || !*ws) return std::string();
    int ulen = WideCharToMultiByte(CP_UTF8, 0, ws, -1, NULL, 0, NULL, NULL);
    if (ulen <= 0) return std::string();
    std::string out(ulen, '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws, -1, &out[0], ulen, NULL, NULL);
    if (!out.empty() && out.back() == '\0') out.pop_back();
    return out;
}

static std::string ToUtf8FromAnsi(const char* s) {
    if (!s || !*s) return std::string();
    int wlen = MultiByteToWideChar(CP_ACP, 0, s, -1, NULL, 0);
    if (wlen <= 0) return std::string();
    std::wstring ws(wlen, L'\0');
    MultiByteToWideChar(CP_ACP, 0, s, -1, &ws[0], wlen);
    return ToUtf8FromWide(ws.c_str());
}

static std::string GetWindowTextUtf8(HWND hwnd) {
    if (!hwnd) return std::string();
    if (IsWindowUnicode(hwnd)) {
        wchar_t wbuf[256] = {0};
        GetWindowTextW(hwnd, wbuf, (int)(sizeof(wbuf) / sizeof(wbuf[0]) - 1));
        return ToUtf8FromWide(wbuf);
    }
    char buf[256] = {0};
    GetWindowTextA(hwnd, buf, sizeof(buf) - 1);
    return ToUtf8FromAnsi(buf);
}

static std::string GetClassNameUtf8(HWND hwnd) {
    if (!hwnd) return std::string();
    wchar_t wbuf[128] = {0};
    if (GetClassNameW(hwnd, wbuf, (int)(sizeof(wbuf) / sizeof(wbuf[0]) - 1)) > 0) {
        return ToUtf8FromWide(wbuf);
    }
    char buf[128] = {0};
    GetClassNameA(hwnd, buf, sizeof(buf) - 1);
    return ToUtf8FromAnsi(buf);
}

static const DWORD kTreeMsgTimeoutMs = 200;

static bool TrySendTreeMsg(HWND hTree, UINT msg, WPARAM wParam, LPARAM lParam, LRESULT* outResult) {
    DWORD_PTR result = 0;
    if (!SendMessageTimeout(hTree, msg, wParam, lParam, SMTO_ABORTIFHUNG,
                            kTreeMsgTimeoutMs, &result)) {
        return false;
    }
    if (outResult) *outResult = (LRESULT)result;
    return true;
}

static std::string GetTreeItemTextUtf8(HWND hTree, HTREEITEM hItem) {
    if (!hTree || !hItem) return std::string();
    LRESULT ok = 0;
    if (IsWindowUnicode(hTree)) {
        wchar_t wbuf[256] = {0};
        TVITEMW tvi = {};
        tvi.mask = TVIF_TEXT | TVIF_HANDLE;
        tvi.hItem = hItem;
        tvi.pszText = wbuf;
        tvi.cchTextMax = (int)(sizeof(wbuf) / sizeof(wbuf[0]) - 1);
        if (!TrySendTreeMsg(hTree, TVM_GETITEMW, 0, (LPARAM)&tvi, &ok) || !ok) {
            return std::string();
        }
        return ToUtf8FromWide(wbuf);
    }
    char buf[256] = {0};
    TVITEMA tvi = {};
    tvi.mask = TVIF_TEXT | TVIF_HANDLE;
    tvi.hItem = hItem;
    tvi.pszText = buf;
    tvi.cchTextMax = sizeof(buf) - 1;
    if (!TrySendTreeMsg(hTree, TVM_GETITEMA, 0, (LPARAM)&tvi, &ok) || !ok) {
        return std::string();
    }
    return ToUtf8FromAnsi(buf);
}

static std::string GetTreeItemTextMbc(HWND hTree, HTREEITEM hItem) {
    if (!hTree || !hItem) return std::string();
    LRESULT ok = 0;
    char buf[256] = {0};
    TVITEMA tvi = {};
    tvi.mask = TVIF_TEXT | TVIF_HANDLE;
    tvi.hItem = hItem;
    tvi.pszText = buf;
    tvi.cchTextMax = sizeof(buf) - 1;
    if (!TrySendTreeMsg(hTree, TVM_GETITEMA, 0, (LPARAM)&tvi, &ok) || !ok) {
        return std::string();
    }
    return std::string(buf);
}

static void DumpTreePath(HWND hTree, HTREEITEM hItem, const char* label) {
    if (!hTree || !hItem) return;
    std::vector<std::string> parts;
    HTREEITEM cur = hItem;
    while (cur) {
        std::string text = GetTreeItemTextUtf8(hTree, cur);
        if (!text.empty()) parts.push_back(text);
        cur = (HTREEITEM)::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_PARENT, (LPARAM)cur);
    }
    std::reverse(parts.begin(), parts.end());
    std::cout << "[DBG] TreePath(" << label << ")=";
    for (size_t i = 0; i < parts.size(); ++i) {
        if (i) std::cout << " / ";
        std::cout << parts[i];
    }
    std::cout << "\n";
}

static void DumpTreeChildren(HWND hTree, HTREEITEM hParent, const char* label, int maxCount) {
    if (!hTree || !hParent) return;
    int printed = 0;
    std::cout << "[DBG] TreeChildren(" << label << ")\n";
    LRESULT res = 0;
    if (!TrySendTreeMsg(hTree, TVM_GETNEXTITEM, TVGN_CHILD, (LPARAM)hParent, &res)) {
        std::cout << "[DBG] TreeChildren(" << label << ") timeout\n";
        return;
    }
    HTREEITEM child = (HTREEITEM)res;
    while (child && printed < maxCount) {
        std::string text = GetTreeItemTextUtf8(hTree, child);
        std::cout << "[DBG]  - child[" << printed << "] handle=0x"
                  << std::hex << (uintptr_t)child << std::dec
                  << " text=" << text << "\n";
        ++printed;
        if (!TrySendTreeMsg(hTree, TVM_GETNEXTITEM, TVGN_NEXT, (LPARAM)child, &res)) {
            std::cout << "[DBG] TreeChildren(" << label << ") timeout\n";
            return;
        }
        child = (HTREEITEM)res;
    }
    if (child) {
        std::cout << "[DBG]  - ... more\n";
    }
}

static int CountTreeChildren(HWND hTree, HTREEITEM hParent) {
    if (!hTree || !hParent) return 0;
    int count = 0;
    LRESULT res = 0;
    if (!TrySendTreeMsg(hTree, TVM_GETNEXTITEM, TVGN_CHILD, (LPARAM)hParent, &res)) {
        return -1;
    }
    HTREEITEM child = (HTREEITEM)res;
    while (child) {
        ++count;
        if (!TrySendTreeMsg(hTree, TVM_GETNEXTITEM, TVGN_NEXT, (LPARAM)child, &res)) {
            return -1;
        }
        child = (HTREEITEM)res;
    }
    return count;
}

static int GetTreeCountSafe(HWND hTree) {
    if (!hTree) return 0;
    LRESULT res = 0;
    if (!TrySendTreeMsg(hTree, TVM_GETCOUNT, 0, 0, &res)) {
        return -1;
    }
    return (int)res;
}

static void DumpTargetChildren(const char* label, HWND hTree, HTREEITEM hTarget) {
    if (!hTree || !hTarget) return;
    int treeCount = GetTreeCountSafe(hTree);
    int childCount = CountTreeChildren(hTree, hTarget);
    if (childCount >= 0) {
        std::cout << "[DBG] Target 子节点(" << label << ") count=" << childCount << "\n";
    } else {
        std::cout << "[DBG] Target 子节点(" << label << ") count=timeout\n";
    }
    DumpTreeChildren(hTree, hTarget, label, kDumpTreeChildrenLimit);
    if (treeCount >= 0) {
        std::cout << "[DBG] TreeCount(" << label << ")=" << treeCount << "\n";
    } else {
        std::cout << "[DBG] TreeCount(" << label << ")=timeout\n";
    }
}

static void DumpTreeRecursive(HWND hTree, HTREEITEM hItem, int depth, int* count,
                              int maxNodes, int maxDepth) {
    if (!hTree || !hItem || !count) return;
    if (maxNodes > 0 && *count >= maxNodes) return;
    if (maxDepth > 0 && depth > maxDepth) return;
    std::string text = GetTreeItemTextUtf8(hTree, hItem);
    std::string indent((size_t)(depth * 2), ' ');
    std::cout << "[DBG] TreeNode " << indent
              << "handle=0x" << std::hex << (uintptr_t)hItem << std::dec
              << " text=" << text << "\n";
    ++(*count);
    HTREEITEM child = (HTREEITEM)::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_CHILD, (LPARAM)hItem);
    while (child) {
        DumpTreeRecursive(hTree, child, depth + 1, count, maxNodes, maxDepth);
        if (maxNodes > 0 && *count >= maxNodes) return;
        child = (HTREEITEM)::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_NEXT, (LPARAM)child);
    }
}

static void DumpTreeAll(HWND hTree, int maxNodes, int maxDepth) {
    if (!hTree) return;
    int count = 0;
    std::cout << "[DBG] TreeDump start\n";
    HTREEITEM root = (HTREEITEM)::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_ROOT, 0);
    while (root) {
        DumpTreeRecursive(hTree, root, 0, &count, maxNodes, maxDepth);
        if (maxNodes > 0 && count >= maxNodes) break;
        root = (HTREEITEM)::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_NEXT, (LPARAM)root);
    }
    std::cout << "[DBG] TreeDump end count=" << count << "\n";
    if (maxNodes > 0 && count >= maxNodes) {
        std::cout << "[DBG] TreeDump reached maxNodes=" << maxNodes << "\n";
    }
}

static const char* StageToZh(const char* stage) {
    if (!stage) return "";
    if (!strcmp(stage, "seh_enter")) return "进入SEH保护";
    if (!strcmp(stage, "resolve_start")) return "开始解析上下文";
    if (!strcmp(stage, "module_handles")) return "获取模块句柄";
    if (!strcmp(stage, "module_range")) return "获取模块范围";
    if (!strcmp(stage, "bind_functions")) return "绑定函数";
    if (!strcmp(stage, "get_global")) return "获取全局容器";
    if (!strcmp(stage, "get_data_container")) return "获取数据容器";
    if (!strcmp(stage, "pre_link_fixed")) return "预取默认Link";
    if (!strcmp(stage, "get_cur_control")) return "获取当前控制ID";
    if (!strcmp(stage, "get_logic_id_from_name")) return "名称转逻辑ID";
    if (!strcmp(stage, "get_logic_id_from_tree")) return "树文本转逻辑ID";
    if (!strcmp(stage, "map_name_to_id")) return "名称映射表取ID";
    if (!strcmp(stage, "resolve_parent")) return "解析Parent";
    if (!strcmp(stage, "get_plc_device")) return "TreeItem转设备";
    if (!strcmp(stage, "map_get_device")) return "映射表取设备";
    if (!strcmp(stage, "map_tree_to_id")) return "TreeItem映射表取ID";
    if (!strcmp(stage, "logic_get_device")) return "逻辑ID取设备";
    if (!strcmp(stage, "resolve_link")) return "解析Link";
    if (!strcmp(stage, "find_link_by_id")) return "按原始ID查Link";
    if (!strcmp(stage, "get_papa_link")) return "获取PapaLink";
    if (!strcmp(stage, "get_link_fixed")) return "固定索引取Link";
    if (!strcmp(stage, "get_link_indices")) return "多索引取Link";
    if (!strcmp(stage, "get_link_index")) return "单索引取Link";
    if (!strcmp(stage, "resolve_done")) return "解析完成";
    return stage;
}

static void SetStage(const char* stage) {
    g_LastStage = stage;
    if (kVerbose) {
        std::cout << "[DBG] 阶段=" << StageToZh(stage) << "\n";
    }
}

static void LogThreadInfo() {
    if (!kVerbose) return;
    DWORD pid = 0;
    DWORD uiTid = g_hMainWnd ? GetWindowThreadProcessId(g_hMainWnd, &pid) : 0;
    DWORD curTid = GetCurrentThreadId();
    std::cout << "[DBG] 线程 cur=" << curTid << " ui=" << uiTid << " pid=" << pid << "\n";
}

static void* TryGetLinkByIndex(
    void* pContainer,
    unsigned int index,
    FnGetLinkFromNO GetLinkFromNO,
    uintptr_t logicBase,
    size_t logicSize) {
    if (!pContainer || !GetLinkFromNO || index == 0) return NULL;
    for (unsigned int a1 = 1; a1 <= 4; ++a1) {
        void* link = GetLinkFromNO(pContainer, a1, index, 0);
        if (kTraceLinkSearch) {
            std::cout << "[DBG] 尝试GetLinkByIndex a2=" << a1 << " a3=" << index
                      << " a4=0 -> 0x" << std::hex << (uintptr_t)link << std::dec << "\n";
        }
        if (IsVtableInModule(link, logicBase, logicSize)) return link;
        for (unsigned int a3 = 1; a3 <= 4; ++a3) {
            link = GetLinkFromNO(pContainer, a1, index, a3);
            if (kTraceLinkSearch) {
                std::cout << "[DBG] 尝试GetLinkByIndex a2=" << a1 << " a3=" << index
                          << " a4=" << a3 << " -> 0x" << std::hex << (uintptr_t)link << std::dec << "\n";
            }
            if (IsVtableInModule(link, logicBase, logicSize)) return link;
        }
    }
    return NULL;
}

static void* TryGetLinkByIndices(
    void* pContainer,
    unsigned int commIdx,
    unsigned int linkIdx,
    unsigned int subIdx,
    FnGetLinkFromNO GetLinkFromNO,
    uintptr_t logicBase,
    size_t logicSize) {
    if (!pContainer || !GetLinkFromNO || linkIdx == 0) return NULL;
    unsigned int commStart = commIdx ? commIdx : 1;
    unsigned int commEnd = commIdx ? commIdx : 4;
    unsigned int subStart = subIdx ? subIdx : 0;
    unsigned int subEnd = subIdx ? subIdx : 4;
    for (unsigned int a2 = commStart; a2 <= commEnd; ++a2) {
        for (unsigned int a4 = subStart; a4 <= subEnd; ++a4) {
            void* link = GetLinkFromNO(pContainer, a2, linkIdx, a4);
            if (kTraceLinkSearch) {
                std::cout << "[DBG] 尝试GetLinkByIndices a2=" << a2 << " a3=" << linkIdx
                          << " a4=" << a4 << " -> 0x" << std::hex << (uintptr_t)link << std::dec << "\n";
            }
            if (IsVtableInModule(link, logicBase, logicSize)) return link;
        }
    }
    return NULL;
}

static bool MapNameToIdUpper(FnMapNameToId MapNameToId, void* mapThis, const char* name, int* outId) {
    if (!MapNameToId || !mapThis || !name || !*name || !outId) return false;
    char buf[256] = {0};
    strncpy_s(buf, name, _TRUNCATE);
    DWORD len = (DWORD)lstrlenA(buf);
    if (len) {
        CharUpperBuffA(buf, len);
    }
    int id = 0;
    int ok = MapNameToId(mapThis, buf, &id);
    if (ok) *outId = id;
    return ok != 0;
}

static bool IsMasterTypeName(const char* typeName) {
    if (!typeName || !*typeName) return false;
    char buf[128] = {0};
    strncpy_s(buf, typeName, _TRUNCATE);
    DWORD len = (DWORD)lstrlenA(buf);
    if (len) {
        CharUpperBuffA(buf, len);
    }
    return strstr(buf, "MASTER") != NULL;
}

static LinkMatch FindLinkById(
    void* pContainer,
    FnGetLinkFromNO GetLinkFromNO,
    uintptr_t logicBase,
    size_t logicSize,
    int targetId) {
    LinkMatch match = {0};
    if (!pContainer || !GetLinkFromNO || targetId <= 0) return match;
    for (unsigned int a2 = 1; a2 <= kMaxCommScan; ++a2) {
        for (unsigned int a3 = 1; a3 <= kMaxLinkScan; ++a3) {
            for (unsigned int a4 = 0; a4 <= kMaxSubScan; ++a4) {
                void* link = GetLinkFromNO(pContainer, a2, a3, a4);
                if (!IsVtableInModule(link, logicBase, logicSize)) continue;
                int linkId = 0;
                if (!ReadI32(link, OFFSET_LINK_ID, &linkId)) continue;
                if (linkId == targetId) {
                    match.link = link;
                    match.commIdx = a2;
                    match.linkIdx = a3;
                    match.subIdx = a4;
                    return match;
                }
            }
        }
    }
    return match;
}

static bool TryFindLinkByIdSafe(
    LinkMatch* out,
    void* pContainer,
    FnGetLinkFromNO GetLinkFromNO,
    uintptr_t logicBase,
    size_t logicSize,
    int targetId) {
    if (!out) return false;
    __try {
        *out = FindLinkById(pContainer, GetLinkFromNO, logicBase, logicSize, targetId);
        return true;
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        return false;
    }
}

static bool ResolveContext(DWORD rawParentData, const char* targetName, ResolvedContext* out) {
    if (!out) return false;
    ZeroMemory(out, sizeof(*out));
    SetStage("resolve_start");
    LogThreadInfo();

    HMODULE hFrame = GetModuleHandleA("dll_DPFrame.dll");
    HMODULE hLogic = GetModuleHandleA("dllDPLogic.dll");
    SetStage("module_handles");
    LogModule("dll_DPFrame", hFrame);
    LogModule("dllDPLogic", hLogic);
    if (!hFrame || !hLogic) {
        std::cout << "[-] ResolveContext: 模块缺失。\n";
        return false;
    }

    uintptr_t logicBase = 0;
    size_t logicSize = 0;
    SetStage("module_range");
    if (!GetModuleRange(hLogic, &logicBase, &logicSize)) {
        std::cout << "[-] ResolveContext: GetModuleRange 失败。\n";
        return false;
    }
    LogModuleRange("dllDPLogic", logicBase, logicSize);

    SetStage("bind_functions");
    FnGetGlobalContainer GetGlobal = (FnGetGlobalContainer)((BYTE*)hFrame + OFFSET_GET_GLOBAL);
    FnGetLinkFromNO GetLinkFromNO = (FnGetLinkFromNO)((BYTE*)hFrame + OFFSET_GET_LINK);
    FnGetDataContainer GetDataContainer = (FnGetDataContainer)((BYTE*)hFrame + OFFSET_GET_DATA_CONTAINER);
    FnGetPLCDeviceDevice GetPLCDeviceDevice =
        (FnGetPLCDeviceDevice)((BYTE*)hFrame + OFFSET_GET_PLC_DEVICE);
    FnGetDeviceByMap GetDeviceByMap =
        (FnGetDeviceByMap)((BYTE*)hFrame + OFFSET_GET_DEVICE_BY_MAP);
    LogPtr("FnGetGlobal", (void*)GetGlobal);
    LogPtr("FnGetLinkFromNO", (void*)GetLinkFromNO);
    LogPtr("FnGetDataContainer", (void*)GetDataContainer);
    LogPtr("FnGetPLCDeviceDevice", (void*)GetPLCDeviceDevice);
    LogPtr("FnGetDeviceByMap", (void*)GetDeviceByMap);

    FnGetPapaLink GetPapaLink = (FnGetPapaLink)((BYTE*)hLogic + OFFSET_GET_PAPA_LINK);
    FnGetLinkIndex GetLinkIndexModbus = (FnGetLinkIndex)((BYTE*)hLogic + OFFSET_GET_LINK_INDEX_MODBUS);
    FnGetLinkIndex GetLinkIndexDp = (FnGetLinkIndex)((BYTE*)hLogic + OFFSET_GET_LINK_INDEX_DP);
    FnGetLinkIndex GetLinkIndexGateway = (FnGetLinkIndex)((BYTE*)hLogic + OFFSET_GET_LINK_INDEX_GATEWAY);
    FnGetIndexU32 GetCommunIndex = (FnGetIndexU32)((BYTE*)hLogic + OFFSET_GET_COMM_INDEX);
    FnGetIndexU32 GetSubCommunIndex = (FnGetIndexU32)((BYTE*)hLogic + OFFSET_GET_SUB_COMM_INDEX);
    FnGetIndexU32 GetCommunIndexDp = (FnGetIndexU32)((BYTE*)hLogic + OFFSET_GET_COMM_INDEX_DP);
    FnGetIndexU32 GetCommunIndexGateway = (FnGetIndexU32)((BYTE*)hLogic + OFFSET_GET_COMM_INDEX_GATEWAY);
    FnGetThisClass GetThisClassDp = (FnGetThisClass)((BYTE*)hLogic + OFFSET_GET_THISCLASS_DP_SLAVE);
    FnGetThisClass GetThisClassModbus = (FnGetThisClass)((BYTE*)hLogic + OFFSET_GET_THISCLASS_MODBUS_SLAVE);
    FnGetThisClass GetThisClassGateway = (FnGetThisClass)((BYTE*)hLogic + OFFSET_GET_THISCLASS_GATEWAY);
    FnGetLogicIDFromName GetLogicIDFromName = (FnGetLogicIDFromName)((BYTE*)hLogic + OFFSET_GET_LOGIC_ID_FROM_NAME);
    FnMapTreeToId MapTreeToId = (FnMapTreeToId)((BYTE*)hFrame + OFFSET_MAP_TREE_TO_ID);
    FnMapNameToId MapNameToId = (FnMapNameToId)((BYTE*)hFrame + OFFSET_MAP_NAME_TO_ID);
    FnGetCommunNoForLink GetCommunNoForLink =
        (FnGetCommunNoForLink)((BYTE*)hFrame + OFFSET_GET_COMM_NO_FOR_LINK);

    void* clsDp = GetThisClassDp ? GetThisClassDp() : NULL;
    void* clsModbus = GetThisClassModbus ? GetThisClassModbus() : NULL;
    void* clsGateway = GetThisClassGateway ? GetThisClassGateway() : NULL;

    auto IsKindOf = [&](void* obj, void* cls) -> bool {
        if (!obj || !cls) return false;
        if (!IsVtableInModule(obj, logicBase, logicSize)) return false;
        return ((CObject*)obj)->IsKindOf((CRuntimeClass*)cls) != FALSE;
    };

    SetStage("get_global");
    void* pContainer = GetGlobal ? GetGlobal() : NULL;
    SetStage("get_data_container");
    void* pDataContainer = pContainer && GetDataContainer ? GetDataContainer(pContainer) : NULL;
    LogPtr("GlobalContainer", pContainer);
    LogPtr("DataContainer", pDataContainer);
    if (!pContainer || !pDataContainer) {
        std::cout << "[-] ResolveContext: 全局容器/数据容器为空。\n";
        return false;
    }

    void* preLink = NULL;
    const void* expectedLinkVtbl = NULL;
    int linkIdFromLink = 0;
    if (GetLinkFromNO) {
        SetStage("pre_link_fixed");
        preLink = GetLinkFromNO(pContainer, 1, 1, 0);
        LogPtr("PreLinkFixed", preLink);
        expectedLinkVtbl = GetVtablePtr(preLink);
        if (expectedLinkVtbl) {
            std::cout << "[DBG] 预期Link虚表=0x" << std::hex
                      << (uintptr_t)expectedLinkVtbl << std::dec << "\n";
        }
        if (ReadI32(preLink, OFFSET_LINK_ID, &linkIdFromLink)) {
            std::cout << "[DBG] 预取Link_id=0x" << std::hex << linkIdFromLink << std::dec << "\n";
        }
        LogU8("PreLink类型", preLink, 0xC);
    }

    std::cout << "[DBG] 原始TreeData=0x" << std::hex << rawParentData << std::dec << "\n";
    if (targetName && *targetName) {
        std::cout << "[DBG] 目标名称=" << targetName << "\n";
    }

    unsigned int curId = 0;
    CString curName;
    FnGetCurControlIDAndName GetCurControlIDAndName =
        (FnGetCurControlIDAndName)((BYTE*)hFrame + OFFSET_GET_CUR_CONTROL);
    if (GetCurControlIDAndName) {
        SetStage("get_cur_control");
        GetCurControlIDAndName(pContainer, &curId, &curName);
        if (kVerbose) {
            std::cout << "[DBG] 当前控制ID=0x" << std::hex << curId << std::dec
                      << " 当前名称=" << ToUtf8FromMbc(curName) << "\n";
        }
    }

    int idByName = 0;
    if (GetLogicIDFromName && targetName && *targetName) {
        CString name(targetName);
        SetStage("get_logic_id_from_name");
        idByName = GetLogicIDFromName(pDataContainer, name);
        if (kVerbose) {
            if (idByName < 0) {
                std::cout << "[DBG] 名称转逻辑ID(" << targetName << ")=未找到\n";
            } else {
            std::cout << "[DBG] 名称转逻辑ID(" << targetName << ")=0x"
                      << std::hex << idByName << std::dec << "\n";
            }
        }
        if (idByName < 0) idByName = 0;
    }
    int idByFull = 0;
    int idByShort = 0;
    int idByType = 0;
    if (GetLogicIDFromName && g_TargetNameFull[0]) {
        CString fullName(g_TargetNameFull);
        SetStage("get_logic_id_from_tree");
        idByFull = GetLogicIDFromName(pDataContainer, fullName);
        if (idByFull < 0) {
            std::cout << "[DBG] Tree文本转逻辑ID(full)=" << ToUtf8FromAnsi(g_TargetNameFull)
                      << " -> 未找到\n";
        } else {
            std::cout << "[DBG] Tree文本转逻辑ID(full)=" << ToUtf8FromAnsi(g_TargetNameFull)
                      << " -> 0x" << std::hex << idByFull << std::dec << "\n";
        }
        if (idByFull < 0) idByFull = 0;
    }
    if (GetLogicIDFromName && g_TargetNameShort[0]) {
        CString shortName(g_TargetNameShort);
        idByShort = GetLogicIDFromName(pDataContainer, shortName);
        if (idByShort < 0) {
            std::cout << "[DBG] Tree文本转逻辑ID(short)=" << ToUtf8FromAnsi(g_TargetNameShort)
                      << " -> 未找到\n";
        } else {
            std::cout << "[DBG] Tree文本转逻辑ID(short)=" << ToUtf8FromAnsi(g_TargetNameShort)
                      << " -> 0x" << std::hex << idByShort << std::dec << "\n";
        }
        if (idByShort < 0) idByShort = 0;
    }
    if (GetLogicIDFromName && g_TargetNameType[0]) {
        CString typeName(g_TargetNameType);
        idByType = GetLogicIDFromName(pDataContainer, typeName);
        if (idByType < 0) {
            std::cout << "[DBG] Tree文本转逻辑ID(type)=" << ToUtf8FromAnsi(g_TargetNameType)
                      << " -> 未找到\n";
        } else {
            std::cout << "[DBG] Tree文本转逻辑ID(type)=" << ToUtf8FromAnsi(g_TargetNameType)
                      << " -> 0x" << std::hex << idByType << std::dec << "\n";
        }
        if (idByType < 0) idByType = 0;
    }

    int idByMapName = 0;
    int idByMapFull = 0;
    int idByMapShort = 0;
    int idByMapType = 0;
    if (MapNameToId && pContainer) {
        SetStage("map_name_to_id");
        void* mapName = (void*)((BYTE*)pContainer + OFFSET_NAME_TO_ID_MAP_BASE);
        if (!IsReadablePtr(mapName)) {
            std::cout << "[DBG] NameMapThis 不可读，跳过名称映射\n";
        } else {
            if (targetName && *targetName) {
                int ok = MapNameToIdUpper(MapNameToId, mapName, targetName, &idByMapName);
                std::cout << "[DBG] NameMap转ID(" << targetName << ") ok=" << ok
                          << " id=0x" << std::hex << idByMapName << std::dec << "\n";
            }
            if (g_TargetNameFull[0]) {
                int ok = MapNameToIdUpper(MapNameToId, mapName, g_TargetNameFull, &idByMapFull);
                std::cout << "[DBG] NameMap转ID(full)=" << ToUtf8FromAnsi(g_TargetNameFull)
                          << " ok=" << ok << " id=0x" << std::hex << idByMapFull
                          << std::dec << "\n";
            }
            if (g_TargetNameShort[0]) {
                int ok = MapNameToIdUpper(MapNameToId, mapName, g_TargetNameShort, &idByMapShort);
                std::cout << "[DBG] NameMap转ID(short)=" << ToUtf8FromAnsi(g_TargetNameShort)
                          << " ok=" << ok << " id=0x" << std::hex << idByMapShort
                          << std::dec << "\n";
            }
            if (g_TargetNameType[0]) {
                int ok = MapNameToIdUpper(MapNameToId, mapName, g_TargetNameType, &idByMapType);
                std::cout << "[DBG] NameMap转ID(type)=" << ToUtf8FromAnsi(g_TargetNameType)
                          << " ok=" << ok << " id=0x" << std::hex << idByMapType
                          << std::dec << "\n";
            }
        }
    }

    bool targetIsMaster = IsMasterTypeName(g_TargetNameType);

    int linkIdCandidate = 0;
    if (idByMapName > 0) {
        linkIdCandidate = idByMapName;
    } else if (idByMapShort > 0) {
        linkIdCandidate = idByMapShort;
    } else if (rawParentData > 0 && rawParentData < 0x100000) {
        linkIdCandidate = (int)rawParentData;
    }

    LinkMatch linkByRaw = {0};
    if (linkIdCandidate && preLink && linkIdFromLink == linkIdCandidate) {
        linkByRaw.link = preLink;
        linkByRaw.commIdx = 1;
        linkByRaw.linkIdx = 1;
        linkByRaw.subIdx = 0;
        std::cout << "[DBG] LinkByRaw命中(预取Link匹配) id=0x" << std::hex
                  << linkIdCandidate << " link=0x" << (uintptr_t)preLink
                  << std::dec << "\n";
    } else if (GetLinkFromNO && linkIdCandidate > 0) {
        SetStage("find_link_by_id");
        if (!TryFindLinkByIdSafe(&linkByRaw, pContainer, GetLinkFromNO, logicBase, logicSize, linkIdCandidate)) {
            std::cout << "[DBG] LinkByRaw扫描异常，跳过 id=0x" << std::hex
                      << linkIdCandidate << std::dec << "\n";
        }
        if (linkByRaw.link) {
            std::cout << "[DBG] LinkByRaw命中 link=0x" << std::hex
                      << (uintptr_t)linkByRaw.link
                      << " commIdx=0x" << linkByRaw.commIdx
                      << " linkIdx=0x" << linkByRaw.linkIdx
                      << " subIdx=0x" << linkByRaw.subIdx
                      << std::dec << "\n";
        } else {
            std::cout << "[DBG] LinkByRaw未命中 id=0x" << std::hex
                      << linkIdCandidate << std::dec << "\n";
        }
    }

    FnGetDeviceByLogicID GetDevice = (FnGetDeviceByLogicID)((BYTE*)hLogic + OFFSET_GET_DEVICE);
    void* pParent = NULL;
    void* fallbackParent = NULL;
    SetStage("resolve_parent");
    if (!pParent && GetPLCDeviceDevice && g_TargetItem) {
        SetStage("get_plc_device");
        void* candidate = GetPLCDeviceDevice(pContainer, g_TargetItem);
        if (kVerbose) {
            std::cout << "[DBG] TreeItem转设备=0x"
                      << std::hex << (uintptr_t)candidate << std::dec << "\n";
        }
            if (IsVtableInModule(candidate, logicBase, logicSize)) {
            if (!expectedLinkVtbl || IsExpectedClass(candidate, expectedLinkVtbl)) {
                pParent = candidate;
            } else if (!fallbackParent) {
                std::cout << "[DBG] GetPLCDeviceDevice 虚表不一致，作为回退候选\n";
                fallbackParent = candidate;
            }
        }
    }
    if (!pParent && MapTreeToId && g_TargetItem) {
        SetStage("map_tree_to_id");
        void* mapTree = (void*)((BYTE*)pContainer + OFFSET_TREE_TO_ID_MAP_BASE);
        int* slot = MapTreeToId(mapTree, (int)g_TargetItem);
        int mapId = slot ? *slot : 0;
        std::cout << "[DBG] MapTreeToId(TreeItem)=0x" << std::hex << mapId << std::dec << "\n";
        if (mapId > 0 && GetDevice) {
            void* candidate = GetDevice(pDataContainer, mapId);
            if (kVerbose) {
                std::cout << "[DBG] MapTreeToId->GetDevice=0x"
                          << std::hex << (uintptr_t)candidate << std::dec << "\n";
            }
            if (candidate && IsVtableInModule(candidate, logicBase, logicSize)) {
                if (expectedLinkVtbl && !IsExpectedClass(candidate, expectedLinkVtbl)) {
                    std::cout << "[DBG] MapTreeToId 设备虚表不一致，作为回退候选\n";
                    if (!fallbackParent) fallbackParent = candidate;
                } else {
                    pParent = candidate;
                }
            }
        }
    }
    if (!pParent && GetDeviceByMap && pContainer) {
        SetStage("map_get_device");
        void* mapThis = (void*)((BYTE*)pContainer + OFFSET_CONTAINER_DEVICE_MAP);
        LogPtr("DeviceMapThis", mapThis);
        if (!IsReadablePtr(mapThis)) {
            std::cout << "[DBG] DeviceMapThis 不可读，跳过 map_get_device\n";
        } else {
        const DWORD tryIds[11] = {
            (DWORD)linkIdFromLink,
            curId,
            (DWORD)idByName,
            (DWORD)idByFull,
            (DWORD)idByShort,
            (DWORD)idByType,
            (DWORD)idByMapName,
            (DWORD)idByMapFull,
            (DWORD)idByMapShort,
            (DWORD)idByMapType,
            rawParentData
        };
        if (kVerbose) {
            std::cout << "[DBG] 尝试ID linkId=0x" << std::hex << tryIds[0]
                      << " curId=0x" << tryIds[1]
                      << " nameId=0x" << tryIds[2]
                      << " fullId=0x" << tryIds[3]
                      << " shortId=0x" << tryIds[4]
                      << " typeId=0x" << tryIds[5]
                      << " mapName=0x" << tryIds[6]
                      << " mapFull=0x" << tryIds[7]
                      << " mapShort=0x" << tryIds[8]
                      << " mapType=0x" << tryIds[9]
                      << " raw=0x" << tryIds[10] << std::dec << "\n";
        }
        for (int i = 0; i < 11 && !pParent; ++i) {
            DWORD id = tryIds[i];
            if (!id || id >= 0x100000) continue;
            void* candidate = NULL;
            int ok = GetDeviceByMap(mapThis, (int)id, &candidate);
            if (kVerbose) {
                std::cout << "[DBG] MapGetDevice 查询 id=0x" << std::hex << id
                          << " ok=" << ok << " out=0x" << (uintptr_t)candidate
                          << std::dec << "\n";
            }
            if (!ok || !IsVtableInModule(candidate, logicBase, logicSize)) {
                continue;
            }
            if (expectedLinkVtbl && !IsExpectedClass(candidate, expectedLinkVtbl)) {
                std::cout << "[DBG] MapGetDevice 虚表不一致，作为回退候选\n";
                if (!fallbackParent) fallbackParent = candidate;
                continue;
            }
            if (ok) {
                pParent = candidate;
                break;
            }
        }
        }
    }
    if (rawParentData >= 0x100000 && IsVtableInModule((void*)rawParentData, logicBase, logicSize)) {
        pParent = (void*)rawParentData;
    } else {
        const DWORD tryIds[11] = {
            (DWORD)linkIdFromLink,
            curId,
            (DWORD)idByName,
            (DWORD)idByFull,
            (DWORD)idByShort,
            (DWORD)idByType,
            (DWORD)idByMapName,
            (DWORD)idByMapFull,
            (DWORD)idByMapShort,
            (DWORD)idByMapType,
            rawParentData
        };
        for (int i = 0; i < 11 && !pParent; ++i) {
            DWORD id = tryIds[i];
            if (!id || id >= 0x100000) continue;
            if (!pParent && GetDevice) {
                SetStage("logic_get_device");
                void* candidate = GetDevice(pDataContainer, id);
                if (kVerbose) {
                    std::cout << "[DBG] 逻辑ID取设备(0x" << std::hex << id
                              << ")=0x" << (uintptr_t)candidate << std::dec << "\n";
                }
                if (!candidate || !IsVtableInModule(candidate, logicBase, logicSize)) {
                    continue;
                }
                if (expectedLinkVtbl && !IsExpectedClass(candidate, expectedLinkVtbl)) {
                    std::cout << "[DBG] GetDeviceByLogicID 虚表不一致，作为回退候选\n";
                    if (!fallbackParent) fallbackParent = candidate;
                    continue;
                }
                pParent = candidate;
                break;
            }
        }
    }
    if (!pParent && fallbackParent) {
        pParent = fallbackParent;
        std::cout << "[DBG] 使用 Parent 回退候选: 0x" << std::hex
                  << (uintptr_t)pParent << std::dec << "\n";
    }
    if (!pParent && linkByRaw.link) {
        pParent = linkByRaw.link;
        std::cout << "[DBG] 使用 LinkByRaw 作为 Parent: 0x" << std::hex
                  << (uintptr_t)pParent << std::dec << "\n";
    }
    if (targetIsMaster && linkByRaw.link && !IsExpectedClass(pParent, expectedLinkVtbl)) {
        pParent = linkByRaw.link;
        std::cout << "[DBG] 目标为 MASTER，强制 Parent=LinkByRaw\n";
    }
    LogPtr("ParentObj", pParent);
    LogVtable("ParentObj", pParent);
    LogU8("Parent类型", pParent, 0xC);
    if (!pParent) {
        std::cout << "[-] ResolveContext: GetDeviceByLogicID 失败，ID=0x"
                  << std::hex << rawParentData << std::dec << "\n";
        return false;
    }

    bool isModbus = IsKindOf(pParent, clsModbus);
    bool isDp = IsKindOf(pParent, clsDp);
    bool isGateway = IsKindOf(pParent, clsGateway);
    if (kVerbose) {
        std::cout << "[DBG] 类型判断 Modbus=" << isModbus
                  << " DP=" << isDp
                  << " Gateway=" << isGateway << "\n";
    }

    void* pLink = NULL;
    SetStage("resolve_link");
    if (linkByRaw.link && IsVtableInModule(linkByRaw.link, logicBase, logicSize)) {
        pLink = linkByRaw.link;
    }
    if (!pLink && preLink && IsVtableInModule(preLink, logicBase, logicSize)) {
        pLink = preLink;
    }
    if (GetPapaLink && isDp) {
        SetStage("get_papa_link");
        void* candidate = GetPapaLink(pParent);
        LogPtr("PapaLink", candidate);
        LogVtable("PapaLink", candidate);
        if (IsVtableInModule(candidate, logicBase, logicSize)) {
            pLink = candidate;
        }
    }
    if (!pLink && GetLinkFromNO) {
        SetStage("get_link_fixed");
        void* candidate = GetLinkFromNO(pContainer, 1, 1, 0);
        if (kTraceLinkSearch) {
            std::cout << "[DBG] 固定参数取Link a2=1 a3=1 a4=0 -> 0x"
                      << std::hex << (uintptr_t)candidate << std::dec << "\n";
        }
        if (IsVtableInModule(candidate, logicBase, logicSize)) {
            pLink = candidate;
        }
    }
    unsigned int linkIdx = linkByRaw.linkIdx;
    unsigned int commIdx = linkByRaw.commIdx;
    unsigned int subCommIdx = linkByRaw.subIdx;
    bool hasLinkIndices = linkIdx != 0;
    if (!hasLinkIndices) {
        if (isModbus) {
            commIdx = GetCommunIndex ? GetCommunIndex(pParent) : 0;
            subCommIdx = GetSubCommunIndex ? GetSubCommunIndex(pParent) : 0;
            linkIdx = GetLinkIndexModbus ? GetLinkIndexModbus(pParent) : 0;
        } else if (isDp) {
            commIdx = GetCommunIndexDp ? GetCommunIndexDp(pParent) : 0;
            linkIdx = GetLinkIndexDp ? GetLinkIndexDp(pParent) : 0;
        } else if (isGateway) {
            commIdx = GetCommunIndexGateway ? GetCommunIndexGateway(pParent) : 0;
            linkIdx = GetLinkIndexGateway ? GetLinkIndexGateway(pParent) : 0;
        } else {
            if (GetLinkIndexModbus) linkIdx = GetLinkIndexModbus(pParent);
            if (!linkIdx && GetLinkIndexDp) linkIdx = GetLinkIndexDp(pParent);
            commIdx = GetCommunIndex ? GetCommunIndex(pParent) : 0;
            subCommIdx = GetSubCommunIndex ? GetSubCommunIndex(pParent) : 0;
        }
    }
    std::cout << "[CTX] linkIdx=" << std::hex << linkIdx
              << " commIdx=" << commIdx << " subIdx=" << subCommIdx
              << std::dec << "\n";
    if (!pLink && linkIdx) {
        SetStage("get_link_indices");
        pLink = TryGetLinkByIndices(pContainer, commIdx, linkIdx, subCommIdx, GetLinkFromNO, logicBase, logicSize);
    }
    if (!pLink && linkIdx) {
        SetStage("get_link_index");
        pLink = TryGetLinkByIndex(pContainer, linkIdx, GetLinkFromNO, logicBase, logicSize);
    }
    LogPtr("ResolvedLink", pLink);
    LogVtable("ResolvedLink", pLink);
    LogU8("Link类型", pLink, 0xC);
    if (!pLink) {
        std::cout << "[-] ResolveContext: 未找到 Link。\n";
        return false;
    }
    if (GetCommunNoForLink) {
        int commFromLink = GetCommunNoForLink(pContainer, pLink);
        if (commFromLink > 0) {
            commIdx = (unsigned int)commFromLink;
            std::cout << "[DBG] GetCommunNoForLink=0x" << std::hex << commIdx << std::dec << "\n";
        }
    }

    out->pContainer = pContainer;
    out->pDataContainer = pDataContainer;
    out->pParent = pParent;
    out->pLink = pLink;
    out->commIdx = commIdx;
    out->linkIdx = linkIdx;
    out->subIdx = subCommIdx;
    SetStage("resolve_done");
    return true;
}

static bool SafeResolveContext(DWORD rawParentData, const char* targetName, ResolvedContext* out) {
    __try {
        SetStage("seh_enter");
        return ResolveContext(rawParentData, targetName, out);
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        std::cout << "[-] ResolveContext: 捕获异常，阶段=" << StageToZh(g_LastStage)
                  << "（可能是无效指针或线程亲和性问题）。\n";
        return false;
    }
}

  

// ============================================================================

// 3. 基础查找工具

// ============================================================================

BOOL IsConsoleWindow(HWND hwnd) {

    char className[256];

    if (GetClassNameA(hwnd, className, sizeof(className))) {

        if (strstr(className, "Console") != NULL) return TRUE;

    }

    return FALSE;

}

  

BOOL CALLBACK FindRealMainWindowCallback(HWND hwnd, LPARAM lParam) {

    DWORD lpdwProcessId;

    GetWindowThreadProcessId(hwnd, &lpdwProcessId);

    if (lpdwProcessId == GetCurrentProcessId()) {

        if (IsWindowVisible(hwnd) && GetParent(hwnd) == NULL) {

            if (IsConsoleWindow(hwnd)) return TRUE;

            char title[256];

            GetWindowTextA(hwnd, title, sizeof(title));

            if (strstr(title, "AutoThink") != NULL) {

                if (strstr(title, "-") != NULL || strlen(title) > 10) {

                    g_hMainWnd = hwnd;

                    return FALSE;

                }

            }

        }

    }

    return TRUE;

}

  

BOOL CALLBACK FindTreeViewCallback(HWND hwnd, LPARAM lParam) {

    char className[256];

    GetClassNameA(hwnd, className, sizeof(className));

    if (strstr(className, "SysTreeView32") != NULL) {
        if (IsWindowVisible(hwnd)) {
            int ctrlId = GetDlgCtrlID(hwnd);
            if (ctrlId == kTreeCtrlIdWanted) {
                g_hTreeView = hwnd;
                return FALSE;
            }
            if (!g_hTreeViewFallback) {
                g_hTreeViewFallback = hwnd;
            }
        }
    }

    return TRUE;

}

static void DumpTreeItemSummary(HWND hTree, const char* label, HTREEITEM hItem) {
    if (!hTree || !hItem) return;
    std::string text = GetTreeItemTextUtf8(hTree, hItem);
    std::cout << "[DBG] TreeItem " << label << " handle=0x"
              << std::hex << (uintptr_t)hItem << std::dec
              << " text=" << text << "\n";
}

static void DumpTreeInfo(HWND hwnd, const char* tag) {
    if (!hwnd) return;
    std::string className = GetClassNameUtf8(hwnd);
    std::string title = GetWindowTextUtf8(hwnd);
    std::string parentTitle;
    RECT rc = {};
    RECT crc = {};
    GetWindowRect(hwnd, &rc);
    GetClientRect(hwnd, &crc);
    HWND hParent = GetParent(hwnd);
    if (hParent) parentTitle = GetWindowTextUtf8(hParent);
    DWORD pid = 0;
    DWORD tid = GetWindowThreadProcessId(hwnd, &pid);
    LONG_PTR style = GetWindowLongPtr(hwnd, GWL_STYLE);
    LONG_PTR exStyle = GetWindowLongPtr(hwnd, GWL_EXSTYLE);
    int count = (int)::SendMessage(hwnd, TVM_GETCOUNT, 0, 0);
    HTREEITEM hRoot = (HTREEITEM)::SendMessage(hwnd, TVM_GETNEXTITEM, TVGN_ROOT, 0);
    HTREEITEM hSel = (HTREEITEM)::SendMessage(hwnd, TVM_GETNEXTITEM, TVGN_CARET, 0);
    std::cout << "[DBG] TreeInfo(" << tag << ") hwnd=0x" << std::hex
              << (uintptr_t)hwnd << " id=" << std::dec << GetDlgCtrlID(hwnd)
              << " class=" << className
              << " title=" << title
              << " parent=0x" << std::hex << (uintptr_t)hParent
              << " parentTitle=" << parentTitle
              << " pid=" << std::dec << pid
              << " tid=" << tid
              << " style=0x" << std::hex << (uintptr_t)style
              << " exStyle=0x" << (uintptr_t)exStyle << std::dec
              << " rect=(" << rc.left << "," << rc.top << "," << rc.right << "," << rc.bottom << ")"
              << " client=(" << crc.left << "," << crc.top << "," << crc.right << "," << crc.bottom << ")"
              << " count=" << count << "\n";
    DumpTreeItemSummary(hwnd, "root", hRoot);
    DumpTreeItemSummary(hwnd, "sel", hSel);
}

BOOL CALLBACK DumpTreeViewCallback(HWND hwnd, LPARAM lParam) {
    char className[256] = {0};
    GetClassNameA(hwnd, className, sizeof(className));
    if (strstr(className, "SysTreeView32") != NULL) {
        DumpTreeInfo(hwnd, "candidate");
    }
    return TRUE;
}

  

// ============================================================================

// 4. [新增] 树节点搜索算法 (递归)

// ============================================================================

HTREEITEM FindNodeByText(HTREEITEM hStart, const char* targetText) {

    if (!hStart) return NULL;

  

    HTREEITEM hCurrent = hStart;

    while (hCurrent) {

        // 1. 获取当前节点的文字

        std::string text = GetTreeItemTextUtf8(g_hTreeView, hCurrent);

  

        // 2. 匹配检查 (模糊匹配)

        // 例如输入 "LK220"，如果节点叫 "LK220 (LK220)" 也能匹配

        if (!text.empty() && targetText && *targetText) {
            if (text.find(targetText) != std::string::npos) {
                return hCurrent; // 找到了！
            }
        }

  

        // 3. 递归查找子节点

        HTREEITEM hChild = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_CHILD, (LPARAM)hCurrent);

        if (hChild) {

            HTREEITEM hResult = FindNodeByText(hChild, targetText);

            if (hResult) return hResult; // 在子树里找到了

        }

  

        // 4. 查找下一个兄弟节点

        hCurrent = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_NEXT, (LPARAM)hCurrent);

    }

    return NULL;

}

  

// ============================================================================

// 5. 智能插入节点

// ============================================================================

static HTREEITEM FindNodeById(HTREEITEM hStart, void* mapTree, FnMapTreeToId MapTreeToId, int targetId) {
    if (!hStart || !mapTree || !MapTreeToId) return NULL;
    HTREEITEM hCurrent = hStart;
    while (hCurrent) {
        int* slot = MapTreeToId(mapTree, (int)hCurrent);
        if (slot && *slot == targetId) {
            return hCurrent;
        }
        HTREEITEM hChild = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_CHILD, (LPARAM)hCurrent);
        if (hChild) {
            HTREEITEM hResult = FindNodeById(hChild, mapTree, MapTreeToId, targetId);
            if (hResult) return hResult;
        }
        hCurrent = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_NEXT, (LPARAM)hCurrent);
    }
    return NULL;
}

static int GetSiblingImageIndex(HTREEITEM hParent) {
    if (!g_hTreeView || !hParent) return -1;
    HTREEITEM hChild = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_CHILD, (LPARAM)hParent);
    if (!hChild) return -1;
    TVITEMA tvi;
    ZeroMemory(&tvi, sizeof(tvi));
    tvi.mask = TVIF_IMAGE | TVIF_SELECTEDIMAGE | TVIF_HANDLE;
    tvi.hItem = hChild;
    if (!::SendMessage(g_hTreeView, TVM_GETITEMA, 0, (LPARAM)&tvi)) return -1;
    return tvi.iImage;
}

HTREEITEM SmartInsertNode(HTREEITEM hParent, CString name, CString desc, int image, LPARAM lParam) {
    if (hParent) {
        TVINSERTSTRUCT tvi;
        tvi.hParent = hParent;
        tvi.hInsertAfter = TVI_LAST;
        tvi.item.mask = TVIF_TEXT | TVIF_PARAM | TVIF_IMAGE | TVIF_SELECTEDIMAGE;
        CString displayText;
        displayText.Format("%s(%s:%s)", name, desc, name);
        tvi.item.pszText = (LPSTR)(LPCTSTR)displayText;
        tvi.item.iImage = image;
        tvi.item.iSelectedImage = image;
        tvi.item.lParam = lParam;
        HTREEITEM hNewItem = (HTREEITEM)::SendMessage(g_hTreeView, TVM_INSERTITEM, 0, (LPARAM)&tvi);
        if (hNewItem) {
            TreeView_Expand(g_hTreeView, hParent, TVE_EXPAND);
            TreeView_EnsureVisible(g_hTreeView, hNewItem);
        }
        return hNewItem;
    }
    return NULL;
}

  

// ============================================================================

// 6. 执行注入 (Timer Callback)

// ============================================================================

void CALLBACK MyTimerProc(HWND hwnd, UINT uMsg, UINT_PTR idEvent, DWORD dwTime)

{

    if (idEvent == kDumpAfterTimerId)
    {
        KillTimer(hwnd, idEvent);
        AFX_MANAGE_STATE(AfxGetStaticModuleState());
        if (g_PendingDumpTarget && g_hTreeView) {
            DumpTargetChildren("target_after", g_hTreeView, g_PendingDumpTarget);
        }
        g_PendingDumpTarget = NULL;
        return;
    }

    if (idEvent == kInjectTimerId)

    {

        KillTimer(hwnd, idEvent);

        AFX_MANAGE_STATE(AfxGetStaticModuleState());

  

        ResolvedContext ctx;
        if (!SafeResolveContext(g_Params.valParentData, g_TargetName, &ctx)) {
            std::cout << "[-] 上下文解析失败，请检查节点选择与模块状态。\n";
            return;
        }
        g_Params.addrContainer = (DWORD)ctx.pContainer;
        g_Params.addrInstance = (DWORD)ctx.pDataContainer;
        g_Params.addrLink = (DWORD)ctx.pLink;
        g_Params.valParentData = (DWORD)ctx.pParent;
        g_Params.commIdx = ctx.commIdx;
        g_Params.linkIdx = ctx.linkIdx;
        std::cout << "[OK] 上下文解析完成：Container=0x" << std::hex
                  << g_Params.addrContainer << " ECX=0x" << g_Params.addrInstance
                  << " Link=0x" << g_Params.addrLink << std::dec << "\n";

        HMODULE hDll = GetModuleHandleA("dllDPLogic.dll");
        HMODULE hFrame = GetModuleHandleA("dll_DPFrame.dll");

        if (hDll) {

            FnMakeNewLogicData_Slave MakeSlave = (FnMakeNewLogicData_Slave)((DWORD)hDll + OFFSET_MAKE_NEW);

            FnGetDeviceByLogicID GetDevice = (FnGetDeviceByLogicID)((DWORD)hDll + OFFSET_GET_DEVICE);
            FnGetDeviceByMap GetDeviceByMap =
                hFrame ? (FnGetDeviceByMap)((DWORD)hFrame + OFFSET_GET_DEVICE_BY_MAP) : NULL;
            FnGetLinkIndex GetLinkIndexModbus =
                hDll ? (FnGetLinkIndex)((DWORD)hDll + OFFSET_GET_LINK_INDEX_MODBUS) : NULL;
            FnGetIndexU32 GetCommunIndex =
                hDll ? (FnGetIndexU32)((DWORD)hDll + OFFSET_GET_COMM_INDEX) : NULL;
            FnUpdateView UpdateView = hFrame ? (FnUpdateView)((DWORD)hFrame + OFFSET_UPDATE_VIEW) : NULL;
            FnAddNodeToCfgTree AddNodeToCfgTree =
                hFrame ? (FnAddNodeToCfgTree)((DWORD)hFrame + OFFSET_ADD_NODE_TO_CFG_TREE) : NULL;
            FnMapTreeToId MapTreeToId =
                hFrame ? (FnMapTreeToId)((DWORD)hFrame + OFFSET_MAP_TREE_TO_ID) : NULL;
            FnMapIdToTree MapIdToTree =
                hFrame ? (FnMapIdToTree)((DWORD)hFrame + OFFSET_MAP_ID_TO_TREE) : NULL;
            FnMapNameToId MapNameToId =
                hFrame ? (FnMapNameToId)((DWORD)hFrame + OFFSET_MAP_NAME_TO_ID) : NULL;
            FnOnSlaveOperate OnSlaveOperate =
                hFrame ? (FnOnSlaveOperate)((DWORD)hFrame + OFFSET_ON_SLAVE_OPERATE) : NULL;
            FnOnAddSlave OnAddSlave =
                hFrame ? (FnOnAddSlave)((DWORD)hFrame + OFFSET_ON_ADD_SLAVE) : NULL;
            FnGetCommunNoForLink GetCommunNoForLink =
                hFrame ? (FnGetCommunNoForLink)((DWORD)hFrame + OFFSET_GET_COMM_NO_FOR_LINK) : NULL;
            FnOnDPTreeSlaveOperate OnDPTreeSlaveOperate =
                hFrame ? (FnOnDPTreeSlaveOperate)((DWORD)hFrame + OFFSET_ON_DPTREE_SLAVE_OPERATE) : NULL;
            FnGetUserNameA GetUserNameA =
                hDll ? (FnGetUserNameA)((DWORD)hDll + OFFSET_GET_USER_NAME) : NULL;

  

            void* pRealParent = (void*)g_Params.valParentData;
            void* pRealLink = (void*)g_Params.addrLink;
            if (!pRealParent || !pRealLink) {
                std::cout << "[-] Parent/Link 指针无效。\n";
                return;
            }
            if (*(void**)pRealParent != *(void**)pRealLink) {
                std::cout << "[DBG] Parent/Link 虚表不一致，parent=0x"
                          << std::hex << (uintptr_t)pRealParent << " link=0x"
                          << (uintptr_t)pRealLink << std::dec << "\n";
            }

            // 2. 注入
            CString typeName = "MODBUSSLAVE_TCP";
            CString strDesc = "192.168.2.39";
            unsigned int newID = 0;
            unsigned int count = 1;
            unsigned int extraFlag = 1;
            const char* extra = NULL;
            char dupFlag = 0;

  

            try {
                if (kPreferOnAddSlave && OnAddSlave && g_Params.addrContainer) {
                    void* pFrame = (void*)((BYTE*)g_Params.addrContainer + OFFSET_FRAME_CONTAINER);
                    if (IsReadablePtr(pFrame)) {
                        std::cout << "[DBG] 调用 OnAddSlave commIdx=0x" << std::hex
                                  << g_Params.commIdx << " linkIdx=0x" << g_Params.linkIdx << std::dec
                                  << " count=" << count << " extra=" << (extra ? extra : "(null)") << "\n";
                        char uiOk = OnAddSlave(
                            pFrame,
                            g_Params.commIdx,
                            g_Params.linkIdx,
                            typeName,
                            strDesc,
                            count,
                            extra
                        );
                        std::cout << "[DBG] OnAddSlave 结果=" << (int)uiOk << "\n";
                        if (uiOk) {
                            if (kDumpTreeAfterInject && g_hTreeView && g_TargetItem) {
                                g_PendingDumpTarget = g_TargetItem;
                                ::SetTimer(hwnd, kDumpAfterTimerId, 50, (TIMERPROC)MyTimerProc);
                            }
                            Beep(1500, 100);
                            return;
                        }
                    } else {
                        std::cout << "[DBG] OnAddSlave 跳过：Frame 指针不可读\n";
                    }
                }

                std::cout << "[DBG] 调用 MakeSlave type=" << (LPCTSTR)typeName
                          << " link=0x" << std::hex << (uintptr_t)pRealLink
                          << " parent=0x" << (uintptr_t)pRealParent
                          << " count=0x" << count
                          << " dupFlag=0x" << std::hex << (int)dupFlag
                          << " extra=0x" << extraFlag << std::dec << "\n";
                char result = MakeSlave(
                    (void*)g_Params.addrInstance,
                    typeName,
                    count,
                    dupFlag,
                    &newID,
                    pRealLink,
                    pRealParent,
                    strDesc,
                    extraFlag,
                    pRealParent
                );
                std::cout << "[DBG] MakeSlave 结果=" << (int)result
                          << " newID=" << newID << "\n";

                if (!result && pRealParent != pRealLink) {
                    std::cout << "[DBG] 重试 MakeSlave（parent=link）\n";
                    result = MakeSlave(
                        (void*)g_Params.addrInstance,
                        typeName,
                        count,
                        dupFlag,
                        &newID,
                        pRealLink,
                        pRealLink,
                        strDesc,
                        extraFlag,
                        pRealLink
                    );
                    std::cout << "[DBG] MakeSlave(重试) 结果=" << (int)result
                              << " newID=" << newID << "\n";
                }

                if (result) {
                    void* pDeviceObj = NULL;
                    void* mapThis = (void*)((BYTE*)g_Params.addrContainer + OFFSET_CONTAINER_DEVICE_MAP);
                    if (GetDeviceByMap && IsReadablePtr(mapThis) && newID > 0) {
                        int ok = GetDeviceByMap(mapThis, newID, &pDeviceObj);
                        std::cout << "[DBG] MapGetDevice(newID) 查询 id=" << newID
                                  << " ok=" << ok << " out=0x" << std::hex
                                  << (uintptr_t)pDeviceObj << std::dec << "\n";
                    }
                    if (!pDeviceObj && GetDevice && newID > 0) {
                        pDeviceObj = GetDevice((void*)g_Params.addrInstance, newID);
                        std::cout << "[DBG] 逻辑ID取设备(newID) out=0x"
                                  << std::hex << (uintptr_t)pDeviceObj << std::dec << "\n";
                    }

                    if (pDeviceObj) {
                        HTREEITEM hTarget = g_TargetItem ? g_TargetItem : TreeView_GetSelection(g_hTreeView);
                        LogPtr("TargetItem", hTarget);
                        bool inserted = false;
                        unsigned int uiCommIdx = g_Params.commIdx;
                        unsigned int uiLinkIdx = g_Params.linkIdx;
                        unsigned int devCommIdx = 0;
                        unsigned int devLinkIdx = 0;
                        CString deviceName;
                        if (kEnableDeviceIntrospection && IsReadablePtr(pDeviceObj)) {
                            if (GetCommunIndex) devCommIdx = GetCommunIndex(pDeviceObj);
                            if (GetLinkIndexModbus) devLinkIdx = GetLinkIndexModbus(pDeviceObj);
                            if (GetUserNameA) {
                                GetUserNameA(pDeviceObj, &deviceName);
                                if (!deviceName.IsEmpty()) {
                                    std::cout << "[DBG] 设备名称="
                                              << ToUtf8FromMbc(deviceName) << "\n";
                                }
                            }
                        } else if (!kEnableDeviceIntrospection) {
                            std::cout << "[DBG] Skip device introspection\n";
                        }
                        if (kEnableLinkCommProbe && GetCommunNoForLink && pRealLink) {
                            int commFromLink = GetCommunNoForLink((void*)g_Params.addrContainer, pRealLink);
                            if (commFromLink > 0) {
                                uiCommIdx = (unsigned int)commFromLink;
                                std::cout << "[DBG] Link推导commIdx=0x" << std::hex
                                          << uiCommIdx << std::dec << "\n";
                            }
                        }
                        if (!uiCommIdx && devCommIdx) uiCommIdx = devCommIdx;
                        if (!uiLinkIdx && devLinkIdx) uiLinkIdx = devLinkIdx;
                        std::cout << "[DBG] UI索引 commIdx=0x" << std::hex << uiCommIdx
                                  << " linkIdx=0x" << uiLinkIdx
                                  << " (devComm=0x" << devCommIdx
                                  << " devLink=0x" << devLinkIdx
                                  << ")" << std::dec << "\n";

                        CString deviceDisplay;
                        if (kTryDeviceDisplayName && IsReadablePtr(pDeviceObj)) {
                            void** vtbl = *(void***)pDeviceObj;
                            FnGetDeviceDisplayName GetDisplayName =
                                vtbl ? (FnGetDeviceDisplayName)vtbl[9] : NULL;
                            if (GetDisplayName) {
                                GetDisplayName(pDeviceObj, &deviceDisplay);
                                if (!deviceDisplay.IsEmpty()) {
                                    std::cout << "[DBG] 设备显示名="
                                              << ToUtf8FromMbc(deviceDisplay) << "\n";
                                }
                            }
                        }

                        if (!inserted && kEnableOnSlaveOperate && OnSlaveOperate
                            && uiCommIdx > 0 && uiLinkIdx > 0) {
                            CString displayText;
                            displayText.Format("%s(%s:%s)", typeName, strDesc, typeName);
                            std::cout << "[DBG] 调用 OnSlaveOperate commIdx=0x" << std::hex
                                      << uiCommIdx << " linkIdx=0x" << uiLinkIdx << std::dec << "\n";
                            char uiOk = OnSlaveOperate(
                                (void*)g_Params.addrContainer,
                                1,
                                pRealLink,
                                pDeviceObj,
                                (int)uiCommIdx,
                                (int)uiLinkIdx,
                                displayText,
                                typeName);
                            std::cout << "[DBG] OnSlaveOperate 添加结果=" << (int)uiOk << "\n";
                            if (uiOk) {
                                bool located = false;
                                void* mapId = MapIdToTree
                                    ? (void*)((BYTE*)g_Params.addrContainer + OFFSET_ID_TO_TREE_MAP_BASE)
                                    : NULL;
                                void* mapTree = MapTreeToId
                                    ? (void*)((BYTE*)g_Params.addrContainer + OFFSET_TREE_TO_ID_MAP_BASE)
                                    : NULL;
                                int* slot2 = (MapIdToTree && mapId) ? MapIdToTree(mapId, newID) : NULL;
                                HTREEITEM hNewItem = slot2 ? (HTREEITEM)(*slot2) : NULL;
                                LogPtr("OnSlaveOperateItem", hNewItem);
                                if (hNewItem) {
                                    TreeView_EnsureVisible(g_hTreeView, hNewItem);
                                    located = true;
                                }
                                if (!located && MapTreeToId && mapTree) {
                                    HTREEITEM hRoot = TreeView_GetRoot(g_hTreeView);
                                    HTREEITEM hById = FindNodeById(hRoot, mapTree, MapTreeToId, newID);
                                    LogPtr("OnSlaveOperateFindById", hById);
                                    if (hById) {
                                        if (slot2) *slot2 = (int)hById;
                                        TreeView_EnsureVisible(g_hTreeView, hById);
                                        located = true;
                                    }
                                }
                                if (!located) {
                                    HTREEITEM searchRoot = hTarget ? hTarget : TreeView_GetRoot(g_hTreeView);
                                    HTREEITEM found = NULL;
                                    if (!deviceDisplay.IsEmpty()) {
                                        found = FindNodeByText(searchRoot, (LPCTSTR)deviceDisplay);
                                        if (!found && searchRoot != TreeView_GetRoot(g_hTreeView)) {
                                            found = FindNodeByText(TreeView_GetRoot(g_hTreeView),
                                                                   (LPCTSTR)deviceDisplay);
                                        }
                                    }
                                    if (!found) {
                                        found = FindNodeByText(searchRoot, (LPCTSTR)strDesc);
                                    }
                                    if (!found && searchRoot != TreeView_GetRoot(g_hTreeView)) {
                                        found = FindNodeByText(TreeView_GetRoot(g_hTreeView), (LPCTSTR)strDesc);
                                    }
                                    if (!found) {
                                        found = FindNodeByText(searchRoot, (LPCTSTR)typeName);
                                    }
                                    if (!found && searchRoot != TreeView_GetRoot(g_hTreeView)) {
                                        found = FindNodeByText(TreeView_GetRoot(g_hTreeView), (LPCTSTR)typeName);
                                    }
                                    LogPtr("OnSlaveOperateSearch", found);
                                    if (found) {
                                        if (MapTreeToId && mapTree) {
                                            int* slot = MapTreeToId(mapTree, (int)found);
                                            int curId = slot ? *slot : 0;
                                            if (slot && (curId == 0 || curId == newID)) {
                                                *slot = newID;
                                            } else if (slot) {
                                                std::cout << "[DBG] 文本匹配节点已有ID=0x"
                                                          << std::hex << curId << std::dec
                                                          << "，跳过回填\n";
                                            }
                                        }
                                        if (slot2) {
                                            int curItem = *slot2;
                                            if (!curItem || curItem == (int)found) {
                                                *slot2 = (int)found;
                                            } else {
                                                std::cout << "[DBG] ID->Tree 已有节点=0x"
                                                          << std::hex << curItem << std::dec
                                                          << "，跳过回填\n";
                                            }
                                        }
                                        TreeView_EnsureVisible(g_hTreeView, found);
                                        located = true;
                                    }
                                }
                                if (!located) {
                                    std::cout << "[DBG] OnSlaveOperate 未定位树节点，继续回退\n";
                                } else {
                                    inserted = true;
                                }
                            }
                        } else if (!inserted && kEnableOnSlaveOperate && OnSlaveOperate) {
                            std::cout << "[DBG] OnSlaveOperate 跳过：索引无效 commIdx=0x"
                                      << std::hex << uiCommIdx << " linkIdx=0x"
                                      << uiLinkIdx << std::dec << "\n";
                        }
                        if (!inserted && kPreferAddNodeToCfgTree && AddNodeToCfgTree && g_hTreeView
                            && g_Params.addrContainer && hTarget) {
                            HTREEITEM hExisting = NULL;
                            if (MapIdToTree && newID > 0) {
                                void* mapIdToTree = (void*)((BYTE*)g_Params.addrContainer + OFFSET_ID_TO_TREE_MAP_BASE);
                                int* slot2 = MapIdToTree(mapIdToTree, newID);
                                hExisting = slot2 ? (HTREEITEM)(*slot2) : NULL;
                                LogPtr("AddNodeExistingItem", hExisting);
                            }
                            if (hExisting) {
                                TreeView_EnsureVisible(g_hTreeView, hExisting);
                                inserted = true;
                            } else {
                                std::cout << "[DBG] 回退 AddNodeToCfgTree\n";
                                CTreeCtrl treeCtrl;
                                if (treeCtrl.Attach(g_hTreeView)) {
                                    HTREEITEM hNewItem = AddNodeToCfgTree(
                                        (void*)g_Params.addrContainer,
                                        pDeviceObj,
                                        &treeCtrl,
                                        hTarget);
                                    treeCtrl.Detach();
                                    LogPtr("AddNodeToCfgTreeItem", hNewItem);
                                    if (hNewItem) {
                                        TreeView_Expand(g_hTreeView, hTarget, TVE_EXPAND);
                                        TreeView_EnsureVisible(g_hTreeView, hNewItem);
                                        inserted = true;
                                    } else {
                                        std::cout << "[DBG] AddNodeToCfgTree 失败\n";
                                    }
                                } else {
                                    std::cout << "[DBG] TreeCtrl 绑定失败\n";
                                }
                            }
                        }
                        if (!inserted && kEnableOnDPTreeOperate && OnDPTreeSlaveOperate
                            && uiCommIdx > 0 && uiLinkIdx > 0
                            && !deviceName.IsEmpty()) {
                            CString empty;
                            int mapId = 0;
                            int ok = 0;
                            void* mapName = (void*)((BYTE*)g_Params.addrContainer + OFFSET_NAME_TO_ID_MAP_BASE);
                            if (MapNameToId && IsReadablePtr(mapName)) {
                                ok = MapNameToIdUpper(MapNameToId, mapName, (LPCSTR)deviceName, &mapId);
                            }
                            std::cout << "[DBG] OnDPTreeSlaveOperate 预检 NameMap ok=" << ok
                                      << " id=0x" << std::hex << mapId << std::dec << "\n";
                            if (ok && mapId > 0 && mapId != (int)newID) {
                                std::cout << "[DBG] OnDPTreeSlaveOperate 跳过：NameMap ID 与 newID 不一致 id=0x"
                                          << std::hex << mapId << " newID=0x" << newID << std::dec << "\n";
                            } else if (ok && mapId > 0) {
                                std::cout << "[DBG] 调用 OnDPTreeSlaveOperate commIdx=0x" << std::hex
                                          << uiCommIdx << " linkIdx=0x" << uiLinkIdx
                                          << " name=" << ToUtf8FromMbc(deviceName)
                                          << std::dec << "\n";
                                char treeOk = OnDPTreeSlaveOperate(
                                    (void*)g_Params.addrContainer,
                                    1,
                                    deviceName,
                                    (int)uiCommIdx,
                                    (int)uiLinkIdx,
                                    empty,
                                    empty,
                                    0);
                                std::cout << "[DBG] OnDPTreeSlaveOperate 结果=" << (int)treeOk << "\n";
                                if (treeOk && MapIdToTree) {
                                    void* mapIdToTree = (void*)((BYTE*)g_Params.addrContainer + OFFSET_ID_TO_TREE_MAP_BASE);
                                    int* slot2 = MapIdToTree(mapIdToTree, newID);
                                    HTREEITEM hNewItem = slot2 ? (HTREEITEM)(*slot2) : NULL;
                                    LogPtr("OnDPTreeItem", hNewItem);
                                    if (hNewItem) {
                                        TreeView_EnsureVisible(g_hTreeView, hNewItem);
                                        inserted = true;
                                    }
                                }
                            }
                        }
                        if (!inserted && kEnableSmartInsert && hTarget) {
                            int image = GetSiblingImageIndex(hTarget);
                            if (image < 0) image = 4;
                            HTREEITEM hNewItem = SmartInsertNode(hTarget, typeName, strDesc, image, 0);
                            LogPtr("SmartInsertItem", hNewItem);
                            if (hNewItem && MapTreeToId && MapIdToTree && g_Params.addrContainer && newID > 0) {
                                void* mapTree = (void*)((BYTE*)g_Params.addrContainer + OFFSET_TREE_TO_ID_MAP_BASE);
                                void* mapId = (void*)((BYTE*)g_Params.addrContainer + OFFSET_ID_TO_TREE_MAP_BASE);
                                int* slot = MapTreeToId(mapTree, (int)hNewItem);
                                int* slot2 = MapIdToTree(mapId, newID);
                                if (slot) *slot = newID;
                                if (slot2) *slot2 = (int)hNewItem;
                                std::cout << "[DBG] 已写入 TreeItem<->ID 映射 newID=" << newID << "\n";
                            } else {
                                std::cout << "[DBG] SmartInsertNode 插入但未写映射\n";
                            }
                        } else if (!inserted && !kEnableSmartInsert) {
                            std::cout << "[DBG] SmartInsertNode 已禁用，避免产生不可编辑节点\n";
                        }
                        std::cout << "[SUCCESS] 已注入 " << (LPCTSTR)typeName
                                  << " (ID: " << newID << ")\n";
                        if (kDumpTreeAfterInject && g_hTreeView && hTarget) {
                            g_PendingDumpTarget = hTarget;
                            ::SetTimer(hwnd, kDumpAfterTimerId, 50, (TIMERPROC)MyTimerProc);
                        }
                        if (!inserted && UpdateView && g_Params.addrContainer) {
                            UpdateView((void*)g_Params.addrContainer, 0);
                        } else if (inserted) {
                            std::cout << "[DBG] 已插入树节点，跳过 UpdateView\n";
                        }

                        Beep(1500, 100);

                    } else {
                        std::cout << "[WARN] MakeSlave 成功但设备指针获取失败，newID="
                                  << newID << "\n";
                        HTREEITEM hTarget = g_TargetItem ? g_TargetItem : TreeView_GetSelection(g_hTreeView);
                        LogPtr("TargetItem", hTarget);
                        bool inserted = false;
                        if (hTarget && MapTreeToId && MapIdToTree && g_Params.addrContainer && newID > 0) {
                            int image = GetSiblingImageIndex(hTarget);
                            if (image < 0) image = 4;
                            HTREEITEM hNewItem = SmartInsertNode(hTarget, typeName, strDesc, image, 0);
                            if (hNewItem) {
                                void* mapTree = (void*)((BYTE*)g_Params.addrContainer + OFFSET_TREE_TO_ID_MAP_BASE);
                                void* mapId = (void*)((BYTE*)g_Params.addrContainer + OFFSET_ID_TO_TREE_MAP_BASE);
                                int* slot = MapTreeToId(mapTree, (int)hNewItem);
                                int* slot2 = MapIdToTree(mapId, newID);
                                if (slot) *slot = newID;
                                if (slot2) *slot2 = (int)hNewItem;
                                std::cout << "[DBG] 已写入 TreeItem<->ID 映射 newID=" << newID << "\n";
                                inserted = true;
                            }
                        }
                        if (!inserted && UpdateView && g_Params.addrContainer) {
                            UpdateView((void*)g_Params.addrContainer, 0);
                        } else if (inserted) {
                            std::cout << "[DBG] 已插入树节点，跳过 UpdateView\n";
                        }
                    }

                } else {
                    std::cout << "[FAIL] 注入返回 0，newID=" << newID << "\n";
                }

            }

            catch (...) { std::cout << "[崩溃]\n"; }

        }

    }

}

  

// ============================================================================

// 7. 工程化控制台 (Search Mode)

// ============================================================================

DWORD WINAPI ConsoleThread(LPVOID lpParam)

{

    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    AllocConsole();

    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);

    FILE* f; freopen_s(&f, "CONIN$", "r", stdin); freopen_s(&f, "CONOUT$", "w", stdout);

    std::cout << "=== ICS 自动组态 V11.0（工程模式） ===\n";

  

    // 1. 找窗口

    while (!g_hMainWnd) {

        EnumWindows(FindRealMainWindowCallback, 0);

        if (!g_hMainWnd) Sleep(1000);

    }

    std::cout << "[OK] 主窗口已锁定。\n";

    // 2. 找 TreeView

    EnumChildWindows(g_hMainWnd, DumpTreeViewCallback, 0);

    while (!g_hTreeView) {
        g_hTreeViewFallback = NULL;
        EnumChildWindows(g_hMainWnd, FindTreeViewCallback, 0);
        if (!g_hTreeView && g_hTreeViewFallback) {
            g_hTreeView = g_hTreeViewFallback;
        }
        if (!g_hTreeView) Sleep(1000);
    }

    std::cout << "[OK] 已找到树控件。hwnd=0x" << std::hex << (uintptr_t)g_hTreeView
              << " id=" << std::dec << GetDlgCtrlID(g_hTreeView) << "\n";
    DumpTreeInfo(g_hTreeView, "selected");
    HTREEITEM hRoot = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_ROOT, 0);
    if (hRoot) {
        DumpTreeChildren(g_hTreeView, hRoot, "root", 20);
        std::string hwText = ToUtf8FromAnsi("硬件配置");
        HTREEITEM hHw = FindNodeByText(hRoot, hwText.c_str());
        if (hHw) {
            DumpTreePath(g_hTreeView, hHw, "硬件配置");
            DumpTreeChildren(g_hTreeView, hHw, "硬件配置", 20);
        }
    }
    if (kDumpTreeOnStart) {
        DumpTreeAll(g_hTreeView, kDumpTreeMaxNodes, kDumpTreeMaxDepth);
    }

    std::cout << "----------------------------------------\n";

    // 3. 配置一次 (后续可以写死在配置文件里)

    std::cout << "[AUTO] 已启用上下文解析器。\n";

  

    std::cout << "----------------------------------------\n";

    std::cout << "系统就绪，请输入父节点名称以注入。\n";

    std::cout << "示例：LK220、ETHERNET、GROUP1\n";

    std::cout << "----------------------------------------\n";

  

    char targetName[256];

    while (true) {

        std::cout << "\n目标父节点名称 > ";

        std::cin.getline(targetName, 256);

  

        if (strlen(targetName) == 0) continue;

        if (strcmp(targetName, "exit") == 0) break;

  

        std::cout << "[*] 正在查找节点：'" << targetName << "'...\n";

  

        // 4. 自动遍历查找

        // ✅ 正确写法

        HTREEITEM hRoot = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_ROOT, 0);

        HTREEITEM hFound = FindNodeByText(hRoot, targetName);

  

        if (hFound) {

            // 选中它 (为了视觉反馈，也为了 SmartInsertNode 能用)

            TreeView_SelectItem(g_hTreeView, hFound);

            TreeView_EnsureVisible(g_hTreeView, hFound);

  

            // 获取 ID

            TVITEM tvi;

            tvi.mask = TVIF_PARAM | TVIF_HANDLE;

            tvi.hItem = hFound;

            ::SendMessage(g_hTreeView, TVM_GETITEM, 0, (LPARAM)&tvi);

            DWORD parentData = (DWORD)tvi.lParam;

            std::cout << "[+] 已找到节点！Data: " << std::hex << parentData << "\n";

  

            // 触发注入

            strncpy_s(g_TargetName, sizeof(g_TargetName), targetName, _TRUNCATE);
            std::string fullName = GetTreeItemTextMbc(g_hTreeView, hFound);
            strncpy_s(g_TargetNameFull, sizeof(g_TargetNameFull), fullName.c_str(), _TRUNCATE);
            std::string shortName = fullName;
            std::string typeName;
            size_t lp = fullName.find('(');
            if (lp != std::string::npos) {
                shortName = fullName.substr(0, lp);
                size_t rp = fullName.find(')', lp + 1);
                if (rp != std::string::npos && rp > lp + 1) {
                    typeName = fullName.substr(lp + 1, rp - lp - 1);
                }
            }
            strncpy_s(g_TargetNameShort, sizeof(g_TargetNameShort), shortName.c_str(), _TRUNCATE);
            strncpy_s(g_TargetNameType, sizeof(g_TargetNameType), typeName.c_str(), _TRUNCATE);
            if (kVerbose) {
                std::cout << "[DBG] TreeItem文本(full)=" << ToUtf8FromAnsi(g_TargetNameFull)
                          << " short=" << ToUtf8FromAnsi(g_TargetNameShort)
                          << " type=" << ToUtf8FromAnsi(g_TargetNameType) << "\n";
            }
            g_TargetItem = hFound;
            g_Params.addrContainer = 0;
            g_Params.addrInstance = 0;
            g_Params.valParentData = parentData;
            g_Params.addrLink = 0;

  

            if (kDumpTreeAfterInject && g_hTreeView && g_TargetItem) {
                DumpTargetChildren("target_before", g_hTreeView, g_TargetItem);
            }
            ::SetTimer(g_hMainWnd, kInjectTimerId, 10, (TIMERPROC)MyTimerProc);

        } else {

            std::cout << "[-] 未找到节点，请检查名称拼写。\n";

        }

    }

  

    FreeConsole(); FreeLibraryAndExitThread((HMODULE)lpParam, 0);

    return 0;

}

  

class CHwHackApp : public CWinApp {

public: CHwHackApp() {}

    virtual BOOL InitInstance() { CWinApp::InitInstance(); ::CreateThread(NULL, 0, ConsoleThread, m_hInstance, 0, NULL); return TRUE; }

};

CHwHackApp theApp;
