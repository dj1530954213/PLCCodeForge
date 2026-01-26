#pragma once

#include <afxcmn.h>
#include <afxwin.h>

namespace hw {

/// <summary>
/// 与目标模块成员函数匹配的函数指针类型。
/// </summary>
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
    void* pContext);

/// <summary>
/// 通过逻辑 ID 获取设备对象。
/// </summary>
typedef void* (__thiscall *FnGetDeviceByLogicID)(void* pThis, unsigned int id);
/// <summary>
/// 由 TreeItem 取设备对象。
/// </summary>
typedef void* (__thiscall *FnGetPlcDeviceDevice)(void* pThis, void* hItem);
/// <summary>
/// 从映射表按 ID 获取设备对象。
/// </summary>
typedef int (__thiscall *FnGetDeviceByMap)(void* pThis, int id, void** outDevice);
/// <summary>
/// 名称映射表：name -> id。
/// </summary>
typedef int (__thiscall *FnMapNameToId)(void* pThis, const char* name, int* outId);

/// <summary>
/// 获取全局容器。
/// </summary>
typedef void* (__cdecl *FnGetGlobalContainer)();
/// <summary>
/// 通过索引获取 Link。
/// </summary>
typedef void* (__thiscall *FnGetLinkFromNO)(void* pThis, unsigned int a2, unsigned int a3, unsigned int a4);
/// <summary>
/// 获取数据容器。
/// </summary>
typedef void* (__thiscall *FnGetDataContainer)(void* pThis);
/// <summary>
/// 获取当前控制 ID 与名称。
/// </summary>
typedef void (__thiscall *FnGetCurControlIdAndName)(void* pThis, unsigned int* pOutId, CString* pOutName);
/// <summary>
/// 刷新界面视图。
/// </summary>
typedef char (__thiscall *FnUpdateView)(void* pThis, unsigned int a2);
/// <summary>
/// 向配置树插入节点。
/// </summary>
typedef HTREEITEM (__thiscall *FnAddNodeToCfgTree)(void* pThis, void* pDevice, CTreeCtrl* pTree, HTREEITEM hParent);
/// <summary>
/// TreeItem -> 逻辑 ID 映射。
/// </summary>
typedef int* (__thiscall *FnMapTreeToId)(void* pThis, int key);
/// <summary>
/// 逻辑 ID -> TreeItem 映射。
/// </summary>
typedef int* (__thiscall *FnMapIdToTree)(void* pThis, int key);
/// <summary>
/// UI 侧添加/操作从站。
/// </summary>
typedef char (__thiscall *FnOnSlaveOperate)(
    void* pThis,
    int op,
    void* pLink,
    void* pDevice,
    int commIdx,
    int linkIdx,
    CString name,
    CString typeName);
/// <summary>
/// UI 侧添加从站（推荐路径）。
/// </summary>
typedef char (__thiscall *FnOnAddSlave)(
    void* pThis,
    unsigned int commIdx,
    unsigned int linkIdx,
    CString typeName,
    CString address,
    unsigned int count,
    const char* extra);
/// <summary>
/// DP 树操作入口。
/// </summary>
typedef char (__thiscall *FnOnDptreeSlaveOperate)(
    void* pThis,
    char op,
    CString name,
    int commIdx,
    int linkIdx,
    CString commName,
    CString linkName,
    unsigned int subIdx);
/// <summary>
/// 获取设备显示名。
/// </summary>
typedef CString* (__thiscall *FnGetDeviceDisplayName)(void* pThis, CString* outName);
/// <summary>
/// 获取父级 Link。
/// </summary>
typedef void* (__thiscall *FnGetPapaLink)(void* pThis);
/// <summary>
/// 获取 Link 索引。
/// </summary>
typedef unsigned char (__thiscall *FnGetLinkIndex)(void* pThis);
/// <summary>
/// 获取通用索引（u32）。
/// </summary>
typedef unsigned int (__thiscall *FnGetIndexU32)(void* pThis);
/// <summary>
/// 获取静态类对象（类型判断）。
/// </summary>
typedef void* (__cdecl *FnGetThisClass)();
/// <summary>
/// 名称 -> 逻辑 ID（逻辑层）。
/// </summary>
typedef int (__thiscall *FnGetLogicIdFromName)(void* pThis, CString name);
/// <summary>
/// Link -> 通讯号。
/// </summary>
typedef int (__thiscall *FnGetCommunNoForLink)(void* pThis, void* pLink);
/// <summary>
/// 获取设备用户显示名（ANSI）。
/// </summary>
typedef CString* (__thiscall *FnGetUserNameA)(void* pThis, CString* outName);

/// <summary>
/// 注入前后的关键指针与索引快照。
/// </summary>
struct InjectionParams {
    DWORD addrContainer = 0;
    DWORD addrInstance = 0;
    DWORD valParentData = 0;
    DWORD addrLink = 0;
    DWORD commIdx = 0;
    DWORD linkIdx = 0;
};

/// <summary>
/// 上下文解析结果（容器/父节点/Link/索引）。
/// </summary>
struct ResolvedContext {
    void* pContainer = nullptr;
    void* pDataContainer = nullptr;
    void* pParent = nullptr;
    void* pLink = nullptr;
    unsigned int commIdx = 0;
    unsigned int linkIdx = 0;
    unsigned int subIdx = 0;
};

/// <summary>
/// Link 命中信息。
/// </summary>
struct LinkMatch {
    void* link = nullptr;
    unsigned int commIdx = 0;
    unsigned int linkIdx = 0;
    unsigned int subIdx = 0;
};

}  // namespace hw
