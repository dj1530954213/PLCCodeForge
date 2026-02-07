#pragma once

#include <windows.h>

namespace hw {

/// <summary>
/// 反编译得到的关键函数/字段偏移（按模块划分）。
/// </summary>
namespace offsets {
// dllDPLogic.dll
constexpr DWORD kMakeNew = 0x59F10;
constexpr DWORD kOnMakeNewLogicData = 0x5A824;
constexpr DWORD kGetDeviceByLogicId = 0x50770;

// dll_DPFrame.dll
constexpr DWORD kGetGlobal = 0xDB560;
constexpr DWORD kGetLink = 0x117830;
constexpr DWORD kGetDataContainer = 0x106C60;
constexpr DWORD kGetCurControl = 0x106C80;
constexpr DWORD kUpdateView = 0x106E00;
constexpr DWORD kGetPlcDevice = 0x125CB0;
constexpr DWORD kGetCommunDeviceFromNO = 0x117760;
constexpr DWORD kContainerDeviceMap = 0x250;
constexpr DWORD kGetDeviceByMap = 0x45E80;
constexpr DWORD kMapNameToId = 0x45E00;
constexpr DWORD kAddNodeToCfgTree = 0x150940;
constexpr DWORD kMapTreeToId = 0x149D80;
constexpr DWORD kMapIdToTree = 0x149DF0;
constexpr DWORD kNameToIdMapBase = 0x1FC;
constexpr DWORD kTreeToIdMapBase = 0x9B8;
constexpr DWORD kIdToTreeMapBase = 0x9D4;
constexpr DWORD kOnSlaveOperate = 0x155D70;
constexpr DWORD kGetCommunNoForLink = 0x1293B0;
constexpr DWORD kOnDptreeSlaveOperate = 0x167AB0;
constexpr DWORD kOnAddProcotol = 0x1A697A;
constexpr DWORD kOnAddSlave = 0x1A7AF0;
constexpr DWORD kFrameContainer = 0x640;
constexpr DWORD kLinkId = 0x10;

// dllDPLogic.dll
constexpr DWORD kGetPapaLink = 0x2E90;
constexpr DWORD kGetLinkIndexModbus = 0x2810;
constexpr DWORD kGetLinkIndexDp = 0x2CC0;
constexpr DWORD kGetCommIndex = 0x2830;
constexpr DWORD kGetSubCommIndex = 0x2850;
constexpr DWORD kGetCommIndexDp = 0x2DF0;
constexpr DWORD kGetCommIndexGateway = 0x37E0;
constexpr DWORD kGetLinkIndexGateway = 0x37C0;
constexpr DWORD kGetThisClassDpSlave = 0x30820;
constexpr DWORD kGetThisClassModbusSlave = 0x67010;
constexpr DWORD kGetThisClassGateway = 0x3AC10;
constexpr DWORD kGetLogicIdFromName = 0x484D0;
constexpr DWORD kGetUserName = 0x1E30;
}  // namespace offsets

/// <summary>
/// 运行时配置项：控制日志、树扫描、注入策略与安全开关。
/// </summary>
struct Settings {
    /// 是否输出详细调试日志。
    bool verbose = true;
    /// 是否追踪 Link 搜索过程。
    bool traceLinkSearch = true;
    /// 启动时是否 Dump 整棵树。
    bool dumpTreeOnStart = true;
    /// DumpTree 最大节点数（0 表示不限制）。
    int dumpTreeMaxNodes = 0;
    /// DumpTree 最大深度（0 表示不限制）。
    int dumpTreeMaxDepth = 0;
    /// 注入后是否延迟 Dump 目标子节点。
    bool dumpTreeAfterInject = true;
    /// DumpTreeChildren 打印子节点上限。
    int dumpTreeChildrenLimit = 20;
    /// 是否优先使用设备显示名来定位节点。
    bool tryDeviceDisplayName = false;
    /// 是否偏好 AddNodeToCfgTree 插入路径。
    bool preferAddNodeToCfgTree = false;
    /// 是否启用 OnSlaveOperate 路径。
    bool enableOnSlaveOperate = false;
    /// 是否启用 OnDPTreeSlaveOperate 路径。
    bool enableOnDptreeOperate = false;
    /// 是否启用 SmartInsert 路径（仅 UI 层插入）。
    bool enableSmartInsert = false;
    /// 是否进行设备对象探测（可能阻塞 UI）。
    bool enableDeviceIntrospection = false;
    /// 是否探测 Link->Comm（可能阻塞 UI）。
    bool enableLinkCommProbe = false;
    /// 是否优先走 OnAddSlave UI 入口。
    bool preferOnAddSlave = true;
    /// 是否允许回退低层 MakeSlave 注入。
    bool enableFallbackInjection = false;
    /// 是否优先使用无弹窗协议添加路径。
    bool preferSilentAddProtocol = true;
    /// 是否允许回退调用 OnAddProcotol（可能弹窗）。
    bool enableOnAddProcotolFallback = false;
    /// 是否在添加协议时尝试聚焦弹窗。
    bool focusProtocolDialog = true;
    /// 是否自动关闭添加协议的弹窗（默认关闭）。
    bool autoCloseProtocolDialog = false;
    /// 弹窗监测超时（毫秒）。
    DWORD protocolDialogTimeoutMs = 8000;
    /// 弹窗监测轮询间隔（毫秒）。
    DWORD protocolDialogPollMs = 200;
    /// Comm 扫描上限。
    unsigned int maxCommScan = 64;
    /// Link 扫描上限。
    unsigned int maxLinkScan = 64;
    /// Sub 扫描上限。
    unsigned int maxSubScan = 4;
    /// Tree 消息超时（防卡 UI）。
    DWORD treeMsgTimeoutMs = 200;
    /// 注入定时器 ID。
    UINT_PTR injectTimerId = 7777;
    /// 注入后 Dump 定时器 ID。
    UINT_PTR dumpAfterTimerId = 7778;
    /// 期望的树控件 ID。
    int treeCtrlIdWanted = 1558;
};

}  // namespace hw
