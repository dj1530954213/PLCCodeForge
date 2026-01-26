#include "stdafx.h"

#include "HwHackInject.h"

#include <commctrl.h>
#include <iostream>

#include "HwHackConfig.h"
#include "HwHackUtils.h"

namespace hw {

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

    HMODULE hDll = GetModuleHandleA("dllDPLogic.dll");
    HMODULE hFrame = GetModuleHandleA("dll_DPFrame.dll");

    if (!hDll || !hFrame) {
        std::cout << "[-] 模块缺失，无法注入。\n";
        return;
    }

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
            void* pFrame = reinterpret_cast<void*>(reinterpret_cast<BYTE*>(state_.params.addrContainer) +
                                                  offsets::kFrameContainer);
            if (IsReadablePtr(pFrame)) {
                std::cout << "[DBG] 调用 OnAddSlave commIdx=0x" << std::hex
                          << state_.params.commIdx << " linkIdx=0x" << state_.params.linkIdx
                          << std::dec << " count=" << count
                          << " extra=" << (extra ? extra : "(null)") << "\n";
                char uiOk = OnAddSlave(pFrame, state_.params.commIdx, state_.params.linkIdx,
                                       typeName, strDesc, count, extra);
                std::cout << "[DBG] OnAddSlave 结果=" << static_cast<int>(uiOk) << "\n";
                if (uiOk) {
                    if (state_.settings.dumpTreeAfterInject && state_.treeView &&
                        state_.targetItem) {
                        state_.pendingDumpTarget = state_.targetItem;
                        ::SetTimer(hwnd, state_.settings.dumpAfterTimerId, 50,
                                   state_.timerProc);
                    }
                    Beep(1500, 100);
                    return;
                }
            } else {
                std::cout << "[DBG] OnAddSlave 跳过：Frame 指针不可读\n";
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
