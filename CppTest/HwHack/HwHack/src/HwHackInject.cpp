#include "stdafx.h"

#include "HwHackInject.h"

#include <commctrl.h>
#include <iostream>
#include <memory>
#include <string>
#include <vector>

#include "HwHackConfig.h"
#include "HwHackUtils.h"

namespace hw {
namespace {

struct DialogWatchParams {
    DWORD pid = 0;
    HWND owner = nullptr;
    bool focus = false;
    bool autoClose = false;
    DWORD timeoutMs = 0;
    DWORD pollMs = 0;
};

struct DialogFoundContext {
    DialogWatchParams* params = nullptr;
    HWND found = nullptr;
    char title[256] = {};
};

BOOL CALLBACK EnumDialogProc(HWND hwnd, LPARAM lParam) {
    auto* ctx = reinterpret_cast<DialogFoundContext*>(lParam);
    if (!ctx || !ctx->params) return TRUE;
    DWORD pid = 0;
    GetWindowThreadProcessId(hwnd, &pid);
    if (pid != ctx->params->pid) return TRUE;
    if (!IsWindowVisible(hwnd)) return TRUE;
    char className[64] = {};
    if (!GetClassNameA(hwnd, className, static_cast<int>(sizeof(className)))) return TRUE;
    if (lstrcmpA(className, "#32770") != 0) return TRUE;
    if (ctx->params->owner) {
        HWND owner = GetWindow(hwnd, GW_OWNER);
        HWND rootOwner = GetAncestor(hwnd, GA_ROOTOWNER);
        if (owner && owner != ctx->params->owner && rootOwner != ctx->params->owner) {
            return TRUE;
        }
    }
    GetWindowTextA(hwnd, ctx->title, static_cast<int>(sizeof(ctx->title) - 1));
    ctx->found = hwnd;
    return FALSE;
}

DWORD WINAPI ProtocolDialogWatchThread(LPVOID param) {
    std::unique_ptr<DialogWatchParams> params(reinterpret_cast<DialogWatchParams*>(param));
    if (!params) return 0;
    DWORD start = GetTickCount();
    while (GetTickCount() - start < params->timeoutMs) {
        DialogFoundContext ctx;
        ctx.params = params.get();
        EnumWindows(EnumDialogProc, reinterpret_cast<LPARAM>(&ctx));
        if (ctx.found) {
            std::cout << "[DBG] 发现协议弹窗 hwnd=0x" << std::hex
                      << reinterpret_cast<uintptr_t>(ctx.found) << std::dec;
            if (ctx.title[0]) {
                std::cout << " title=" << ctx.title;
            }
            std::cout << "\n";
            if (params->focus) {
                SetForegroundWindow(ctx.found);
                SetActiveWindow(ctx.found);
            }
            if (params->autoClose) {
                SendMessage(ctx.found, WM_COMMAND, IDOK, 0);
            } else {
                break;
            }
        }
        Sleep(params->pollMs);
    }
    return 0;
}

void StartProtocolDialogWatch(const Settings& settings, HWND owner) {
    if ((!settings.focusProtocolDialog && !settings.autoCloseProtocolDialog) || !owner) {
        return;
    }
    auto* params = new DialogWatchParams();
    params->pid = GetCurrentProcessId();
    params->owner = owner;
    params->focus = settings.focusProtocolDialog;
    params->autoClose = settings.autoCloseProtocolDialog;
    params->timeoutMs = settings.protocolDialogTimeoutMs;
    params->pollMs = settings.protocolDialogPollMs;
    HANDLE hThread = CreateThread(nullptr, 0, ProtocolDialogWatchThread, params, 0, nullptr);
    if (hThread) {
        CloseHandle(hThread);
    } else {
        delete params;
    }
}

}  // namespace

/**
 * @brief 构造注入器并绑定依赖组件。
 * @param state 全局运行时状态。
 * @param tree 树扫描器。
 * @param resolver 上下文解析器。
 */
Injector::Injector(AppState& state, TreeScanner& tree, ContextResolver& resolver)
    : state_(state), tree_(tree), resolver_(resolver) {}

/**
 * @brief 分发定时器事件。
 * @param hwnd 主窗口句柄。
 * @param idEvent 定时器 ID。
 */
void Injector::HandleTimer(HWND hwnd, UINT_PTR idEvent) {
    if (idEvent == state_.settings.dumpAfterTimerId) {
        HandleDumpTimer(hwnd);
        return;
    }
    if (idEvent == state_.settings.injectTimerId) {
        HandleInjectTimer(hwnd);
        return;
    }
}

/**
 * @brief 处理注入后延迟 Dump 的定时器。
 * @param hwnd 主窗口句柄。
 */
void Injector::HandleDumpTimer(HWND hwnd) {
    KillTimer(hwnd, state_.settings.dumpAfterTimerId);
    AFX_MANAGE_STATE(AfxGetStaticModuleState());
    if (state_.pendingDumpTarget && state_.treeView) {
        tree_.DumpTargetChildren(state_.pendingDumpTarget, "target_after");
    }
    state_.pendingDumpTarget = nullptr;
}

/**
 * @brief 处理注入主流程的定时器。
 * @param hwnd 主窗口句柄。
 */
void Injector::HandleInjectTimer(HWND hwnd) {
    KillTimer(hwnd, state_.settings.injectTimerId);
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    if (!state_.treeView || !state_.targetItem) {
        std::cout << "[-] TreeView 或目标节点无效。\n";
        return;
    }

    HMODULE hDll = GetModuleHandleA("dllDPLogic.dll");
    HMODULE hFrame = GetModuleHandleA("dll_DPFrame.dll");

    if (!hDll || !hFrame) {
        std::cout << "[-] 模块缺失，无法注入。\n";
        return;
    }

    FnGetGlobalContainer GetGlobal =
        reinterpret_cast<FnGetGlobalContainer>(reinterpret_cast<DWORD>(hFrame) +
                                               offsets::kGetGlobal);
    FnOnAddProcotol OnAddProcotol =
        reinterpret_cast<FnOnAddProcotol>(reinterpret_cast<DWORD>(hFrame) +
                                          offsets::kOnAddProcotol);
    FnGetCommunDeviceFromNO GetCommunDeviceFromNO =
        reinterpret_cast<FnGetCommunDeviceFromNO>(reinterpret_cast<DWORD>(hFrame) +
                                                  offsets::kGetCommunDeviceFromNO);
    FnOnMakeNewLogicData OnMakeNewLogicData =
        reinterpret_cast<FnOnMakeNewLogicData>(reinterpret_cast<DWORD>(hDll) +
                                               offsets::kOnMakeNewLogicData);
    FnMakeNewLogicData_Slave MakeSlave =
        reinterpret_cast<FnMakeNewLogicData_Slave>(reinterpret_cast<DWORD>(hDll) +
                                                   offsets::kMakeNew);
    FnAddNodeToCfgTree AddNodeToCfgTree =
        reinterpret_cast<FnAddNodeToCfgTree>(reinterpret_cast<DWORD>(hFrame) +
                                             offsets::kAddNodeToCfgTree);
    FnGetDeviceByMap GetDeviceByMap =
        reinterpret_cast<FnGetDeviceByMap>(reinterpret_cast<DWORD>(hFrame) +
                                           offsets::kGetDeviceByMap);
    FnMapTreeToId MapTreeToId =
        reinterpret_cast<FnMapTreeToId>(reinterpret_cast<DWORD>(hFrame) + offsets::kMapTreeToId);
    FnMapIdToTree MapIdToTree =
        reinterpret_cast<FnMapIdToTree>(reinterpret_cast<DWORD>(hFrame) + offsets::kMapIdToTree);
    FnMapNameToId MapNameToId =
        reinterpret_cast<FnMapNameToId>(reinterpret_cast<DWORD>(hFrame) + offsets::kMapNameToId);
    FnOnSlaveOperate OnSlaveOperate =
        reinterpret_cast<FnOnSlaveOperate>(reinterpret_cast<DWORD>(hFrame) +
                                           offsets::kOnSlaveOperate);
    FnOnAddSlave OnAddSlave =
        reinterpret_cast<FnOnAddSlave>(reinterpret_cast<DWORD>(hFrame) + offsets::kOnAddSlave);
    FnOnDptreeSlaveOperate OnDptreeSlaveOperate =
        reinterpret_cast<FnOnDptreeSlaveOperate>(reinterpret_cast<DWORD>(hFrame) +
                                                 offsets::kOnDptreeSlaveOperate);
    FnGetUserNameA GetUserNameA =
        reinterpret_cast<FnGetUserNameA>(reinterpret_cast<DWORD>(hDll) + offsets::kGetUserName);

    void* pContainer = GetGlobal ? GetGlobal() : nullptr;
    if (!pContainer) {
        std::cout << "[-] 获取全局容器失败。\n";
        return;
    }
    void* pFrame = reinterpret_cast<void*>(reinterpret_cast<BYTE*>(pContainer) +
                                           offsets::kFrameContainer);
    if (!IsReadablePtr(pFrame)) {
        std::cout << "[-] Frame 容器指针无效。\n";
        return;
    }

    auto UpdateTargetFromItem = [&](HTREEITEM item, const char* preferredName) -> bool {
        if (!item || !state_.treeView) return false;
        TVITEMA tvi = {};
        tvi.mask = TVIF_PARAM | TVIF_HANDLE;
        tvi.hItem = item;
        LRESULT ok = 0;
        if (!TrySendTreeMsg(state_.settings, state_.treeView, TVM_GETITEMA, 0,
                            reinterpret_cast<LPARAM>(&tvi), &ok) ||
            !ok) {
            return false;
        }

        state_.params.valParentData = static_cast<DWORD>(tvi.lParam);
        std::string fullName = tree_.GetTreeItemTextMbc(item);
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

        const char* chosenName = (preferredName && *preferredName) ? preferredName
                                                                   : shortName.c_str();
        strncpy_s(state_.targetName, sizeof(state_.targetName), chosenName, _TRUNCATE);
        strncpy_s(state_.targetNameFull, sizeof(state_.targetNameFull), fullName.c_str(),
                  _TRUNCATE);
        strncpy_s(state_.targetNameShort, sizeof(state_.targetNameShort), shortName.c_str(),
                  _TRUNCATE);
        strncpy_s(state_.targetNameType, sizeof(state_.targetNameType), typeName.c_str(),
                  _TRUNCATE);
        state_.targetItem = item;
        if (state_.settings.verbose) {
            std::cout << "[DBG] TreeItem文本(full)=" << ToUtf8FromAnsi(state_.targetNameFull)
                      << " short=" << ToUtf8FromAnsi(state_.targetNameShort)
                      << " type=" << ToUtf8FromAnsi(state_.targetNameType) << "\n";
        }
        return true;
    };

    bool targetIsMaster = IsMasterTypeName(state_.targetName) ||
                          IsMasterTypeName(state_.targetNameFull) ||
                          IsMasterTypeName(state_.targetNameShort) ||
                          IsMasterTypeName(state_.targetNameType);

    if (!targetIsMaster) {
        ResolvedContext baseCtx;
        if (!resolver_.SafeResolve(state_.params.valParentData, state_.targetName, &baseCtx, false,
                                   true)) {
            std::cout << "[-] MASTER 前置上下文解析失败。\n";
            return;
        }

        std::vector<HTREEITEM> beforeChildren;
        std::vector<HTREEITEM> afterChildren;
        bool beforeOk = tree_.CollectChildren(state_.targetItem, &beforeChildren);
        if (!beforeOk && state_.settings.verbose) {
            std::cout << "[DBG] CollectChildren(before) 失败\n";
        }

        CString protocolName = "MODBUSTCP_MASTER";
        unsigned int masterId = 0;
        bool masterCreated = false;

        if (state_.settings.preferSilentAddProtocol && OnMakeNewLogicData) {
            if (!baseCtx.pParent || !baseCtx.pDataContainer || !baseCtx.pContainer) {
                std::cout << "[-] MASTER 创建缺少必要上下文：parent/dataContainer/container。\n";
                return;
            }
            void* pControl = baseCtx.pParent;
            if (GetCommunDeviceFromNO) {
                unsigned int commNo = baseCtx.commIdx ? baseCtx.commIdx : 1;
                CString commName = state_.targetName;
                void* commDevice = GetCommunDeviceFromNO(baseCtx.pContainer, commNo, commName);
                if (commDevice) {
                    pControl = commDevice;
                }
                if (state_.settings.verbose) {
                    std::cout << "[DBG] CommunDeviceFromNO commNo=" << commNo
                              << " ptr=0x" << std::hex << reinterpret_cast<uintptr_t>(commDevice)
                              << std::dec << "\n";
                }
            }
            CString emptyDesc = "";
            std::cout << "[DBG] 调用 OnMakeNewLogicData(Procotol) name="
                      << static_cast<LPCTSTR>(protocolName) << "\n";
            char ok = OnMakeNewLogicData(baseCtx.pDataContainer, protocolName, 1, 0, &masterId,
                                         pControl, baseCtx.pLink, emptyDesc, 0, pControl);
            std::cout << "[DBG] OnMakeNewLogicData 结果=" << static_cast<int>(ok)
                      << " newID=" << masterId << "\n";
            masterCreated = ok != 0;
        }

        if (!masterCreated && state_.settings.enableOnAddProcotolFallback) {
            if (!OnAddProcotol) {
                std::cout << "[-] OnAddProcotol 指针无效。\n";
                return;
            }
            StartProtocolDialogWatch(state_.settings, state_.mainWnd);
            std::cout << "[DBG] 调用 OnAddProcotol name=" << static_cast<LPCTSTR>(protocolName)
                      << "\n";
            char addOk = OnAddProcotol(pFrame, protocolName);
            std::cout << "[DBG] OnAddProcotol 结果=" << static_cast<int>(addOk) << "\n";
            if (!addOk) {
                std::cout << "[-] OnAddProcotol 失败。\n";
                return;
            }
            masterCreated = true;
        }

        if (!masterCreated) {
            std::cout << "[-] MASTER 创建失败，已跳过 OnAddProcotol 回退。\n";
            return;
        }

        HTREEITEM newMaster = nullptr;
        if (masterId > 0 && AddNodeToCfgTree && GetDeviceByMap && baseCtx.pContainer) {
            void* mapThis = reinterpret_cast<void*>(
                reinterpret_cast<BYTE*>(baseCtx.pContainer) + offsets::kContainerDeviceMap);
            void* device = nullptr;
            if (GetDeviceByMap(mapThis, static_cast<int>(masterId), &device) && device) {
                CTreeCtrl treeCtrl;
                if (treeCtrl.Attach(state_.treeView)) {
                    newMaster = AddNodeToCfgTree(baseCtx.pContainer, device, &treeCtrl,
                                                 state_.targetItem);
                    treeCtrl.Detach();
                }
            }
        }
        if (!newMaster && state_.settings.verbose) {
            std::cout << "[DBG] AddNodeToCfgTree 未返回新节点\n";
        }

        bool afterOk = tree_.CollectChildren(state_.targetItem, &afterChildren);
        int newCount = -1;
        if (!newMaster && beforeOk && afterOk) {
            newMaster = tree_.FindNewChildByDiff(beforeChildren, afterChildren, &newCount);
            if (!newMaster && state_.settings.verbose) {
                std::cout << "[DBG] 子节点差分未命中 newCount=" << newCount << "\n";
            }
        }

        if (!newMaster) {
            newMaster = tree_.FindNodeByText(state_.targetItem, "MODBUSTCP_MASTER");
        }
        if (!newMaster && state_.treeView) {
            HTREEITEM sel = TreeView_GetSelection(state_.treeView);
            if (sel && sel != state_.targetItem) {
                newMaster = sel;
            }
        }
        if (!newMaster) {
            std::cout << "[-] 未定位到新建 MASTER 节点。\n";
            return;
        }

        tree_.DumpTreePath(newMaster, "new_master");
        if (!UpdateTargetFromItem(newMaster, "MODBUSTCP_MASTER")) {
            std::cout << "[-] 更新 MASTER 上下文失败。\n";
            return;
        }
    }

    ResolvedContext ctx;
    // 解析容器/父节点/Link/索引，失败则直接退出。
    if (!resolver_.SafeResolve(state_.params.valParentData, state_.targetName, &ctx)) {
        std::cout << "[-] 上下文解析失败，请检查节点选择与模块状态。\n";
        return;
    }

    state_.params.addrContainer = reinterpret_cast<DWORD>(ctx.pContainer);
    state_.params.addrInstance = reinterpret_cast<DWORD>(ctx.pDataContainer);
    state_.params.addrLink = reinterpret_cast<DWORD>(ctx.pLink);
    state_.params.valParentData = reinterpret_cast<DWORD>(ctx.pParent);
    state_.params.commIdx = ctx.commIdx;
    state_.params.linkIdx = ctx.linkIdx;

    std::cout << "[OK] 上下文解析完成：Container=0x" << std::hex << state_.params.addrContainer
              << " ECX=0x" << state_.params.addrInstance << " Link=0x" << state_.params.addrLink
              << std::dec << "\n";

    pFrame = reinterpret_cast<void*>(reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                                     offsets::kFrameContainer);
    if (!IsReadablePtr(pFrame)) {
        std::cout << "[-] OnAddSlave 跳过：Frame 指针不可读\n";
        return;
    }

    void* pRealParent = reinterpret_cast<void*>(state_.params.valParentData);
    void* pRealLink = reinterpret_cast<void*>(state_.params.addrLink);
    if (!pRealParent || !pRealLink) {
        std::cout << "[-] Parent/Link 指针无效。\n";
        return;
    }
    if (*reinterpret_cast<void**>(pRealParent) != *reinterpret_cast<void**>(pRealLink)) {
        std::cout << "[DBG] Parent/Link 虚表不一致，parent=0x" << std::hex
                  << reinterpret_cast<uintptr_t>(pRealParent) << " link=0x"
                  << reinterpret_cast<uintptr_t>(pRealLink) << std::dec << "\n";
    }

    CString typeName = "MODBUSSLAVE_TCP";
    CString strDesc = "192.168.2.39";
    unsigned int newID = 0;
    unsigned int count = 1;
    unsigned int extraFlag = 1;
    const char* extra = nullptr;
    char dupFlag = 0;

    try {
        // 优先走 UI 侧 OnAddSlave，确保界面与内部映射同步。
        if (state_.settings.preferOnAddSlave && OnAddSlave && state_.params.addrContainer) {
            std::cout << "[DBG] 调用 OnAddSlave commIdx=0x" << std::hex
                      << state_.params.commIdx << " linkIdx=0x" << state_.params.linkIdx
                      << std::dec << " count=" << count
                      << " extra=" << (extra ? extra : "(null)") << "\n";
            char uiOk = OnAddSlave(pFrame, state_.params.commIdx, state_.params.linkIdx,
                                   typeName, strDesc, count, extra);
            std::cout << "[DBG] OnAddSlave 结果=" << static_cast<int>(uiOk) << "\n";
            if (uiOk) {
                if (state_.settings.dumpTreeAfterInject && state_.treeView && state_.targetItem) {
                    state_.pendingDumpTarget = state_.targetItem;
                    ::SetTimer(hwnd, state_.settings.dumpAfterTimerId, 50, state_.timerProc);
                }
                Beep(1500, 100);
                return;
            }
        }

        // 回退路径默认关闭，避免 UI 与数据不同步。
        if (!state_.settings.enableFallbackInjection) {
            std::cout << "[DBG] OnAddSlave 失败且回退已禁用，终止注入。\n";
            return;
        }

        if (!MakeSlave) {
            std::cout << "[-] MakeSlave 指针无效，无法回退注入。\n";
            return;
        }

        // 低层注入路径：直接调用 MakeSlave。
        std::cout << "[DBG] 调用 MakeSlave type=" << static_cast<LPCTSTR>(typeName)
                  << " link=0x" << std::hex << reinterpret_cast<uintptr_t>(pRealLink)
                  << " parent=0x" << reinterpret_cast<uintptr_t>(pRealParent)
                  << " count=0x" << count << " dupFlag=0x" << std::hex
                  << static_cast<int>(dupFlag) << " extra=0x" << extraFlag << std::dec
                  << "\n";
        char result = MakeSlave(reinterpret_cast<void*>(state_.params.addrInstance), typeName,
                                count, dupFlag, &newID, pRealLink, pRealParent, strDesc,
                                extraFlag, pRealParent);
        std::cout << "[DBG] MakeSlave 结果=" << static_cast<int>(result) << " newID=" << newID
                  << "\n";

        if (!result && pRealParent != pRealLink) {
            std::cout << "[DBG] MakeSlave 失败，尝试 parent=link 重试...\n";
            result = MakeSlave(reinterpret_cast<void*>(state_.params.addrInstance), typeName,
                                count, dupFlag, &newID, pRealLink, pRealLink, strDesc,
                                extraFlag, pRealLink);
            std::cout << "[DBG] MakeSlave(Parent=Link) 结果=" << static_cast<int>(result)
                      << " newID=" << newID << "\n";
        }

        if (!result) {
            std::cout << "[FAIL] 注入返回 0，newID=" << newID << "\n";
            return;
        }

        void* pDeviceObj = nullptr;
        if (state_.params.addrContainer && newID > 0 && MapIdToTree) {
            // 若 UI 已生成节点，优先复用现有 TreeItem。
            void* mapIdToTree = reinterpret_cast<void*>(
                reinterpret_cast<BYTE*>(state_.params.addrContainer) + offsets::kIdToTreeMapBase);
            int* slot2 = MapIdToTree(mapIdToTree, newID);
            if (slot2 && *slot2) {
                HTREEITEM hExisting = reinterpret_cast<HTREEITEM>(*slot2);
                TreeView_EnsureVisible(state_.treeView, hExisting);
                std::cout << "[DBG] ID->Tree 已有节点=0x" << std::hex
                          << reinterpret_cast<uintptr_t>(hExisting) << std::dec << "\n";
            }
        }

        if (state_.params.addrContainer && newID > 0 && IsReadablePtr(
                reinterpret_cast<void*>(state_.params.addrContainer + offsets::kContainerDeviceMap))) {
            void* mapThis = reinterpret_cast<void*>(state_.params.addrContainer +
                                                    offsets::kContainerDeviceMap);
            if (state_.settings.enableDeviceIntrospection && IsReadablePtr(mapThis)) {
                void* candidate = nullptr;
                if (GetDeviceByMap(mapThis, newID, &candidate)) {
                    pDeviceObj = candidate;
                }
            }
        }

        if (state_.settings.enableDeviceIntrospection && pDeviceObj && GetUserNameA) {
            CString displayName;
            if (GetUserNameA(pDeviceObj, &displayName)) {
                std::cout << "[DBG] DeviceDisplay=" << ToUtf8FromMbc(displayName) << "\n";
            }
        }

        HTREEITEM hTarget = state_.targetItem ? state_.targetItem : TreeView_GetSelection(state_.treeView);
        if (state_.settings.enableOnSlaveOperate && OnSlaveOperate && state_.params.commIdx &&
            state_.params.linkIdx) {
            std::cout << "[DBG] 调用 OnSlaveOperate commIdx=0x" << std::hex
                      << state_.params.commIdx << " linkIdx=0x" << state_.params.linkIdx << std::dec
                      << "\n";
            char uiOk = OnSlaveOperate(reinterpret_cast<void*>(state_.params.addrContainer), 1,
                                       pRealLink, pRealParent, state_.params.commIdx,
                                       state_.params.linkIdx, strDesc, typeName);
            std::cout << "[DBG] OnSlaveOperate 添加结果=" << static_cast<int>(uiOk) << "\n";
            if (uiOk && MapIdToTree) {
                void* mapId = reinterpret_cast<void*>(
                    reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                    offsets::kIdToTreeMapBase);
                int* slot2 = MapIdToTree(mapId, newID);
                HTREEITEM hNewItem = slot2 ? reinterpret_cast<HTREEITEM>(*slot2) : nullptr;
                LogPtr(state_.settings, "OnSlaveOperateItem", hNewItem);
                if (hNewItem) {
                    TreeView_EnsureVisible(state_.treeView, hNewItem);
                }
            }
        } else if (state_.settings.enableOnSlaveOperate && OnSlaveOperate) {
            std::cout << "[DBG] OnSlaveOperate 跳过：索引无效 commIdx=0x" << std::hex
                      << state_.params.commIdx << " linkIdx=0x" << state_.params.linkIdx << std::dec
                      << "\n";
        }

        bool inserted = false;
        if (!inserted && state_.settings.preferAddNodeToCfgTree && AddNodeToCfgTree &&
            state_.treeView && state_.params.addrContainer) {
            if (MapIdToTree && newID > 0) {
                // 映射表已有节点则不再重复插入。
                void* mapIdToTree = reinterpret_cast<void*>(
                    reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                    offsets::kIdToTreeMapBase);
                int* slot2 = MapIdToTree(mapIdToTree, newID);
                HTREEITEM hExisting = slot2 ? reinterpret_cast<HTREEITEM>(*slot2) : nullptr;
                LogPtr(state_.settings, "AddNodeExistingItem", hExisting);
                if (hExisting) {
                    TreeView_EnsureVisible(state_.treeView, hExisting);
                    inserted = true;
                }
            }
            if (!inserted) {
                std::cout << "[DBG] 回退 AddNodeToCfgTree\n";
                CTreeCtrl treeCtrl;
                if (treeCtrl.Attach(state_.treeView)) {
                    HTREEITEM hNewItem = AddNodeToCfgTree(
                        reinterpret_cast<void*>(state_.params.addrContainer), pRealParent,
                        &treeCtrl, hTarget);
                    LogPtr(state_.settings, "AddNodeToCfgTreeItem", hNewItem);
                    if (hNewItem) {
                        TreeView_Expand(state_.treeView, hTarget, TVE_EXPAND);
                        TreeView_EnsureVisible(state_.treeView, hNewItem);
                        inserted = true;
                    } else {
                        std::cout << "[DBG] AddNodeToCfgTree 失败\n";
                    }
                    treeCtrl.Detach();
                } else {
                    std::cout << "[DBG] TreeCtrl 绑定失败\n";
                }
            }
        }

        if (!inserted && state_.settings.enableOnDptreeOperate && OnDptreeSlaveOperate &&
            state_.params.commIdx && state_.params.linkIdx) {
            void* mapName = reinterpret_cast<void*>(
                reinterpret_cast<BYTE*>(state_.params.addrContainer) + offsets::kNameToIdMapBase);
            int nameId = 0;
            int ok = MapNameToId ? MapNameToId(mapName, state_.targetName, &nameId) : 0;
            std::cout << "[DBG] OnDPTreeSlaveOperate 预检 NameMap ok=" << ok
                      << " id=0x" << std::hex << nameId << std::dec << "\n";
            if (ok && nameId == static_cast<int>(newID)) {
                std::cout << "[DBG] 调用 OnDPTreeSlaveOperate commIdx=0x" << std::hex
                          << state_.params.commIdx << " linkIdx=0x" << state_.params.linkIdx
                          << std::dec << "\n";
                char treeOk = OnDptreeSlaveOperate(reinterpret_cast<void*>(state_.params.addrContainer),
                                                   1, strDesc, state_.params.commIdx,
                                                   state_.params.linkIdx, strDesc, typeName, 0);
                std::cout << "[DBG] OnDPTreeSlaveOperate 结果=" << static_cast<int>(treeOk) << "\n";
                if (treeOk && MapIdToTree) {
                    void* mapIdToTree = reinterpret_cast<void*>(
                        reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                        offsets::kIdToTreeMapBase);
                    int* slot2 = MapIdToTree(mapIdToTree, newID);
                    HTREEITEM hNewItem = slot2 ? reinterpret_cast<HTREEITEM>(*slot2) : nullptr;
                    LogPtr(state_.settings, "OnDPTreeItem", hNewItem);
                    if (hNewItem) {
                        TreeView_EnsureVisible(state_.treeView, hNewItem);
                        inserted = true;
                    }
                }
            } else if (ok) {
                std::cout << "[DBG] OnDPTreeSlaveOperate 跳过：NameMap ID 与 newID 不一致 id=0x"
                          << std::hex << nameId << " newID=0x" << newID << std::dec << "\n";
            }
        }

        if (!inserted && state_.settings.enableSmartInsert && hTarget) {
            // SmartInsert 仅用于 UI 层补节点，需要手动写入映射关系。
            int image = tree_.GetSiblingImageIndex(hTarget);
            if (image < 0) image = 4;
            HTREEITEM hNewItem = tree_.SmartInsertNode(hTarget, typeName, strDesc, image, 0);
            LogPtr(state_.settings, "SmartInsertItem", hNewItem);
            if (hNewItem && MapTreeToId && MapIdToTree && state_.params.addrContainer && newID > 0) {
                void* mapTree = reinterpret_cast<void*>(
                    reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                    offsets::kTreeToIdMapBase);
                void* mapId = reinterpret_cast<void*>(
                    reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                    offsets::kIdToTreeMapBase);
                int key = static_cast<int>(reinterpret_cast<intptr_t>(hNewItem));
                int* slot = MapTreeToId(mapTree, key);
                int* slot2 = MapIdToTree(mapId, newID);
                if (slot) *slot = newID;
                if (slot2) *slot2 = key;
                std::cout << "[DBG] 已写入 TreeItem<->ID 映射 newID=" << newID << "\n";
                inserted = true;
            } else {
                std::cout << "[DBG] SmartInsertNode 插入但未写映射\n";
            }
        }

        if (!inserted && state_.settings.tryDeviceDisplayName && state_.settings.enableDeviceIntrospection &&
            pDeviceObj && IsReadablePtr(pDeviceObj)) {
            CString displayName;
            if (GetUserNameA && GetUserNameA(pDeviceObj, &displayName)) {
                std::string deviceDisplay = ToUtf8FromMbc(displayName);
                HTREEITEM searchRoot = hTarget ? hTarget : TreeView_GetRoot(state_.treeView);
                HTREEITEM found = tree_.FindNodeByText(searchRoot, deviceDisplay.c_str());
                if (!found && searchRoot != TreeView_GetRoot(state_.treeView)) {
                    found = tree_.FindNodeByText(TreeView_GetRoot(state_.treeView),
                                                 deviceDisplay.c_str());
                }
                if (found && MapTreeToId && state_.params.addrContainer && newID > 0) {
                    void* mapTree = reinterpret_cast<void*>(
                        reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                        offsets::kTreeToIdMapBase);
                    int key = static_cast<int>(reinterpret_cast<intptr_t>(found));
                    int* slot = MapTreeToId(mapTree, key);
                    if (slot && *slot == 0) {
                        *slot = newID;
                    }
                    if (MapIdToTree) {
                        void* mapId = reinterpret_cast<void*>(
                            reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                            offsets::kIdToTreeMapBase);
                        int* slot2 = MapIdToTree(mapId, newID);
                        if (slot2 && *slot2 == 0) {
                            *slot2 = key;
                        }
                    }
                    TreeView_EnsureVisible(state_.treeView, found);
                }
            }
        }

        if (!inserted && state_.settings.dumpTreeAfterInject && state_.treeView && hTarget) {
            state_.pendingDumpTarget = hTarget;
            ::SetTimer(hwnd, state_.settings.dumpAfterTimerId, 50, state_.timerProc);
        }

    } catch (...) {
        std::cout << "[崩溃]\n";
    }
}

}  // namespace hw
