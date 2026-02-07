#include "stdafx.h"

#include "HwHackContext.h"

#include <cstring>
#include <iostream>

#include "HwHackConfig.h"
#include "HwHackUtils.h"

namespace hw {
namespace {

/**
 * @brief 通过 Link 索引尝试获取 Link 指针。
 * @param pContainer 全局容器指针。
 * @param index Link 索引。
 * @param getLinkFromNo 取 Link 的函数指针。
 * @param settings 运行时设置（控制日志/扫描范围）。
 * @param logicBase 逻辑模块基址。
 * @param logicSize 逻辑模块大小。
 * @return 命中 Link 指针；失败返回 nullptr。
 */
void* TryGetLinkByIndex(void* pContainer,
                        unsigned int index,
                        FnGetLinkFromNO getLinkFromNo,
                        const Settings& settings,
                        uintptr_t logicBase,
                        size_t logicSize) {
    if (!pContainer || !getLinkFromNo || index == 0) return nullptr;
    // Link 索引可能需要不同的 a2/a4 组合，逐项尝试并验证虚表。
    for (unsigned int a2 = 1; a2 <= 4; ++a2) {
        void* link = getLinkFromNo(pContainer, a2, index, 0);
        if (settings.traceLinkSearch) {
            std::cout << "[DBG] 尝试GetLinkByIndex a2=" << a2 << " a3=" << index
                      << " a4=0 -> 0x" << std::hex << reinterpret_cast<uintptr_t>(link) << std::dec
                      << "\n";
        }
        if (IsVtableInModule(link, logicBase, logicSize)) return link;
        for (unsigned int a4 = 1; a4 <= 4; ++a4) {
            link = getLinkFromNo(pContainer, a2, index, a4);
            if (settings.traceLinkSearch) {
                std::cout << "[DBG] 尝试GetLinkByIndex a2=" << a2 << " a3=" << index
                          << " a4=" << a4 << " -> 0x" << std::hex
                          << reinterpret_cast<uintptr_t>(link) << std::dec << "\n";
            }
            if (IsVtableInModule(link, logicBase, logicSize)) return link;
        }
    }
    return nullptr;
}

/**
 * @brief 按 comm/link/sub 组合尝试获取 Link 指针。
 * @param pContainer 全局容器指针。
 * @param commIdx 通讯索引。
 * @param linkIdx Link 索引。
 * @param subIdx 子索引。
 * @param getLinkFromNo 取 Link 的函数指针。
 * @param settings 运行时设置（控制日志/扫描范围）。
 * @param logicBase 逻辑模块基址。
 * @param logicSize 逻辑模块大小。
 * @return 命中 Link 指针；失败返回 nullptr。
 */
void* TryGetLinkByIndices(void* pContainer,
                          unsigned int commIdx,
                          unsigned int linkIdx,
                          unsigned int subIdx,
                          FnGetLinkFromNO getLinkFromNo,
                          const Settings& settings,
                          uintptr_t logicBase,
                          size_t logicSize) {
    if (!pContainer || !getLinkFromNo || linkIdx == 0) return nullptr;
    // 限定 comm/link/sub 范围进行组合扫描，优先命中有效虚表。
    unsigned int commStart = commIdx ? commIdx : 1;
    unsigned int commEnd = commIdx ? commIdx : 4;
    unsigned int subStart = subIdx ? subIdx : 0;
    unsigned int subEnd = subIdx ? subIdx : 4;
    for (unsigned int a2 = commStart; a2 <= commEnd; ++a2) {
        for (unsigned int a4 = subStart; a4 <= subEnd; ++a4) {
            void* link = getLinkFromNo(pContainer, a2, linkIdx, a4);
            if (settings.traceLinkSearch) {
                std::cout << "[DBG] 尝试GetLinkByIndices a2=" << a2 << " a3=" << linkIdx
                          << " a4=" << a4 << " -> 0x" << std::hex
                          << reinterpret_cast<uintptr_t>(link) << std::dec << "\n";
            }
            if (IsVtableInModule(link, logicBase, logicSize)) return link;
        }
    }
    return nullptr;
}

/**
 * @brief 遍历扫描匹配指定 LinkId 的 Link。
 * @param pContainer 全局容器指针。
 * @param getLinkFromNo 取 Link 的函数指针。
 * @param settings 运行时设置（控制扫描范围）。
 * @param logicBase 逻辑模块基址。
 * @param logicSize 逻辑模块大小。
 * @param targetId 目标 LinkId。
 * @return 匹配结果（包含 Link 与索引）。
 */
LinkMatch FindLinkById(void* pContainer,
                       FnGetLinkFromNO getLinkFromNo,
                       const Settings& settings,
                       uintptr_t logicBase,
                       size_t logicSize,
                       int targetId) {
    LinkMatch match{};
    if (!pContainer || !getLinkFromNo || targetId <= 0) return match;
    // 遍历 Link 组合并读取内部 linkId 进行匹配。
    for (unsigned int a2 = 1; a2 <= settings.maxCommScan; ++a2) {
        for (unsigned int a3 = 1; a3 <= settings.maxLinkScan; ++a3) {
            for (unsigned int a4 = 0; a4 <= settings.maxSubScan; ++a4) {
                void* link = getLinkFromNo(pContainer, a2, a3, a4);
                if (!IsVtableInModule(link, logicBase, logicSize)) continue;
                int linkId = 0;
                if (!ReadI32(link, offsets::kLinkId, &linkId)) continue;
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

/**
 * @brief 使用 SEH 保护的 Link 扫描入口。
 * @param out 输出匹配结果。
 * @param pContainer 全局容器指针。
 * @param getLinkFromNo 取 Link 的函数指针。
 * @param settings 运行时设置。
 * @param logicBase 逻辑模块基址。
 * @param logicSize 逻辑模块大小。
 * @param targetId 目标 LinkId。
 * @return 是否成功完成扫描。
 */
bool TryFindLinkByIdSafe(LinkMatch* out,
                         void* pContainer,
                         FnGetLinkFromNO getLinkFromNo,
                         const Settings& settings,
                         uintptr_t logicBase,
                         size_t logicSize,
                         int targetId) {
    if (!out) return false;
    // SEH 保护：避免非法指针导致崩溃。
    __try {
        *out = FindLinkById(pContainer, getLinkFromNo, settings, logicBase, logicSize, targetId);
        return true;
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        return false;
    }
}

}  // namespace

/**
 * @brief 绑定运行时状态对象。
 * @param state 全局运行时状态引用。
 */
ContextResolver::ContextResolver(AppState& state) : state_(state) {}

/**
 * @brief 阶段标识转中文描述。
 * @param stage 阶段标识字符串。
 * @return 中文描述（未匹配则返回原字符串）。
 */
const char* ContextResolver::StageToZh(const char* stage) const {
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

/**
 * @brief 记录并打印当前阶段。
 * @param stage 阶段标识字符串。
 */
void ContextResolver::SetStage(const char* stage) {
    state_.lastStage = stage;
    if (state_.settings.verbose) {
        std::cout << "[DBG] 阶段=" << StageToZh(stage) << "\n";
    }
}

/**
 * @brief 输出线程与进程信息。
 */
void ContextResolver::LogThreadInfo() const {
    if (!state_.settings.verbose) return;
    DWORD pid = 0;
    DWORD uiTid = state_.mainWnd ? GetWindowThreadProcessId(state_.mainWnd, &pid) : 0;
    DWORD curTid = GetCurrentThreadId();
    std::cout << "[DBG] 线程 cur=" << curTid << " ui=" << uiTid << " pid=" << pid << "\n";
}

/**
 * @brief 解析注入所需上下文（容器/父节点/Link/索引）。
 * @param rawParentData TreeItem 的 lParam 原始值。
 * @param targetName 用户输入的目标名称。
 * @param out 输出解析结果。
 * @return 解析成功返回 true。
 */
bool ContextResolver::Resolve(DWORD rawParentData,
                              const char* targetName,
                              ResolvedContext* out,
                              bool requireLink,
                              bool preferTargetName) {
    if (!out) return false;
    ZeroMemory(out, sizeof(*out));
    SetStage("resolve_start");
    LogThreadInfo();

    HMODULE hFrame = GetModuleHandleA("dll_DPFrame.dll");
    HMODULE hLogic = GetModuleHandleA("dllDPLogic.dll");
    SetStage("module_handles");
    LogModule(state_.settings, "dll_DPFrame", hFrame);
    LogModule(state_.settings, "dllDPLogic", hLogic);
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
    LogModuleRange(state_.settings, "dllDPLogic", logicBase, logicSize);

    SetStage("bind_functions");
    // 从模块基址绑定内部函数指针，解析链路依赖这些入口。
    FnGetGlobalContainer GetGlobal =
        reinterpret_cast<FnGetGlobalContainer>(reinterpret_cast<BYTE*>(hFrame) + offsets::kGetGlobal);
    FnGetLinkFromNO GetLinkFromNO =
        reinterpret_cast<FnGetLinkFromNO>(reinterpret_cast<BYTE*>(hFrame) + offsets::kGetLink);
    FnGetDataContainer GetDataContainer =
        reinterpret_cast<FnGetDataContainer>(reinterpret_cast<BYTE*>(hFrame) + offsets::kGetDataContainer);
    FnGetPlcDeviceDevice GetPlcDeviceDevice =
        reinterpret_cast<FnGetPlcDeviceDevice>(reinterpret_cast<BYTE*>(hFrame) + offsets::kGetPlcDevice);
    FnGetDeviceByMap GetDeviceByMap =
        reinterpret_cast<FnGetDeviceByMap>(reinterpret_cast<BYTE*>(hFrame) + offsets::kGetDeviceByMap);
    LogPtr(state_.settings, "FnGetGlobal", reinterpret_cast<void*>(GetGlobal));
    LogPtr(state_.settings, "FnGetLinkFromNO", reinterpret_cast<void*>(GetLinkFromNO));
    LogPtr(state_.settings, "FnGetDataContainer", reinterpret_cast<void*>(GetDataContainer));
    LogPtr(state_.settings, "FnGetPLCDeviceDevice", reinterpret_cast<void*>(GetPlcDeviceDevice));
    LogPtr(state_.settings, "FnGetDeviceByMap", reinterpret_cast<void*>(GetDeviceByMap));

    FnGetPapaLink GetPapaLink =
        reinterpret_cast<FnGetPapaLink>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetPapaLink);
    FnGetLinkIndex GetLinkIndexModbus =
        reinterpret_cast<FnGetLinkIndex>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetLinkIndexModbus);
    FnGetLinkIndex GetLinkIndexDp =
        reinterpret_cast<FnGetLinkIndex>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetLinkIndexDp);
    FnGetLinkIndex GetLinkIndexGateway =
        reinterpret_cast<FnGetLinkIndex>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetLinkIndexGateway);
    FnGetIndexU32 GetCommunIndex =
        reinterpret_cast<FnGetIndexU32>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetCommIndex);
    FnGetIndexU32 GetSubCommunIndex =
        reinterpret_cast<FnGetIndexU32>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetSubCommIndex);
    FnGetIndexU32 GetCommunIndexDp =
        reinterpret_cast<FnGetIndexU32>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetCommIndexDp);
    FnGetIndexU32 GetCommunIndexGateway =
        reinterpret_cast<FnGetIndexU32>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetCommIndexGateway);
    FnGetThisClass GetThisClassDp =
        reinterpret_cast<FnGetThisClass>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetThisClassDpSlave);
    FnGetThisClass GetThisClassModbus =
        reinterpret_cast<FnGetThisClass>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetThisClassModbusSlave);
    FnGetThisClass GetThisClassGateway =
        reinterpret_cast<FnGetThisClass>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetThisClassGateway);
    FnGetLogicIdFromName GetLogicIdFromName =
        reinterpret_cast<FnGetLogicIdFromName>(reinterpret_cast<BYTE*>(hLogic) + offsets::kGetLogicIdFromName);
    FnMapTreeToId MapTreeToId =
        reinterpret_cast<FnMapTreeToId>(reinterpret_cast<BYTE*>(hFrame) + offsets::kMapTreeToId);
    FnMapNameToId MapNameToId =
        reinterpret_cast<FnMapNameToId>(reinterpret_cast<BYTE*>(hFrame) + offsets::kMapNameToId);
    FnGetCommunNoForLink GetCommunNoForLink =
        reinterpret_cast<FnGetCommunNoForLink>(reinterpret_cast<BYTE*>(hFrame) + offsets::kGetCommunNoForLink);

    void* clsDp = GetThisClassDp ? GetThisClassDp() : nullptr;
    void* clsModbus = GetThisClassModbus ? GetThisClassModbus() : nullptr;
    void* clsGateway = GetThisClassGateway ? GetThisClassGateway() : nullptr;

    auto IsKindOf = [&](void* obj, void* cls) -> bool {
        if (!obj || !cls) return false;
        __try {
            return IsReadablePtr(*(void**)obj) && (*(void**)obj == cls);
        } __except (EXCEPTION_EXECUTE_HANDLER) {
            return false;
        }
    };

    SetStage("get_global");
    // 获取全局/数据容器，失败则无法继续解析。
    void* pContainer = GetGlobal ? GetGlobal() : nullptr;
    LogPtr(state_.settings, "GlobalContainer", pContainer);
    SetStage("get_data_container");
    void* pDataContainer = (pContainer && GetDataContainer) ? GetDataContainer(pContainer) : nullptr;
    LogPtr(state_.settings, "DataContainer", pDataContainer);
    if (!pContainer || !pDataContainer) {
        std::cout << "[-] ResolveContext: 全局容器/数据容器为空。\n";
        return false;
    }

    out->pContainer = pContainer;
    out->pDataContainer = pDataContainer;

    SetStage("pre_link_fixed");
    // 预取默认 Link，用于确认虚表与 LinkId。
    void* preLink = GetLinkFromNO ? GetLinkFromNO(pContainer, 1, 1, 0) : nullptr;
    LogPtr(state_.settings, "PreLinkFixed", preLink);
    const void* expectedVtbl = GetVtablePtr(preLink);
    if (state_.settings.verbose && expectedVtbl) {
        std::cout << "[DBG] 预期Link虚表=0x" << std::hex
                  << reinterpret_cast<uintptr_t>(expectedVtbl) << std::dec << "\n";
    }
    int preLinkId = 0;
    if (preLink && ReadI32(preLink, offsets::kLinkId, &preLinkId)) {
        if (state_.settings.verbose) {
            std::cout << "[DBG] 预取Link_id=0x" << std::hex << preLinkId << std::dec << "\n";
        }
    }
    if (state_.settings.verbose && preLink) {
        unsigned int type = *reinterpret_cast<unsigned char*>(
            reinterpret_cast<unsigned char*>(preLink) + 12);
        std::cout << "[DBG] PreLink类型=0x" << std::hex << type << std::dec << "\n";
    }

    if (state_.settings.verbose) {
        std::cout << "[DBG] 原始TreeData=0x" << std::hex << rawParentData << std::dec << "\n";
        std::cout << "[DBG] 目标名称=" << targetName << "\n";
    }

    SetStage("get_cur_control");
    unsigned int curControlId = 0;
    CString curControlName;
    if (GetDataContainer) {
        FnGetCurControlIdAndName GetCurControlIdAndName =
            reinterpret_cast<FnGetCurControlIdAndName>(reinterpret_cast<BYTE*>(hFrame) +
                                                       offsets::kGetCurControl);
        if (GetCurControlIdAndName) {
            GetCurControlIdAndName(pContainer, &curControlId, &curControlName);
        }
    }
    if (state_.settings.verbose) {
        std::cout << "[DBG] 当前控制ID=0x" << std::hex << curControlId << std::dec
                  << " 当前名称=" << ToUtf8FromMbc(curControlName) << "\n";
    }

    int nameId = 0;
    int fullId = 0;
    int shortId = 0;
    int typeId = 0;

    // 先通过多种名称源获取逻辑 ID，便于后续匹配 Parent/Link。
    SetStage("get_logic_id_from_name");
    if (GetLogicIdFromName && targetName && *targetName) {
        int tmp = GetLogicIdFromName(pDataContainer, targetName);
        if (tmp > 0) nameId = tmp;
        if (state_.settings.verbose) {
            std::cout << "[DBG] 名称转逻辑ID(" << targetName << ")="
                      << (nameId ? "ok" : "未找到") << "\n";
        }
    }

    SetStage("get_logic_id_from_tree");
    if (GetLogicIdFromName && state_.targetNameFull[0]) {
        int tmp = GetLogicIdFromName(pDataContainer, state_.targetNameFull);
        if (tmp > 0) fullId = tmp;
        if (state_.settings.verbose) {
            std::cout << "[DBG] Tree文本转逻辑ID(full)=" << ToUtf8FromAnsi(state_.targetNameFull)
                      << " -> " << (fullId ? "ok" : "未找到") << "\n";
        }
    }
    if (GetLogicIdFromName && state_.targetNameShort[0]) {
        int tmp = GetLogicIdFromName(pDataContainer, state_.targetNameShort);
        if (tmp > 0) shortId = tmp;
        if (state_.settings.verbose) {
            std::cout << "[DBG] Tree文本转逻辑ID(short)=" << ToUtf8FromAnsi(state_.targetNameShort)
                      << " -> " << (shortId ? "ok" : "未找到") << "\n";
        }
    }
    if (GetLogicIdFromName && state_.targetNameType[0]) {
        int tmp = GetLogicIdFromName(pDataContainer, state_.targetNameType);
        if (tmp > 0) typeId = tmp;
        if (state_.settings.verbose) {
            std::cout << "[DBG] Tree文本转逻辑ID(type)=" << ToUtf8FromAnsi(state_.targetNameType)
                      << " -> " << (typeId ? "ok" : "未找到") << "\n";
        }
    }

    SetStage("map_name_to_id");
    // NameMap 再次补齐可能缺失的逻辑 ID。
    int mapName = 0;
    int mapFull = 0;
    int mapShort = 0;
    int mapType = 0;
    if (MapNameToId && pContainer) {
        void* mapThis = reinterpret_cast<void*>(
            reinterpret_cast<BYTE*>(pContainer) + offsets::kNameToIdMapBase);
        if (MapNameToIdUpper(MapNameToId, mapThis, targetName, &mapName)) {
            if (state_.settings.verbose) {
                std::cout << "[DBG] NameMap转ID(" << targetName << ") ok=1 id=0x" << std::hex
                          << mapName << std::dec << "\n";
            }
        }
        if (MapNameToIdUpper(MapNameToId, mapThis, state_.targetNameFull, &mapFull)) {
            if (state_.settings.verbose) {
                std::cout << "[DBG] NameMap转ID(full)=" << ToUtf8FromAnsi(state_.targetNameFull)
                          << " ok=1 id=0x" << std::hex << mapFull << std::dec << "\n";
            }
        }
        if (MapNameToIdUpper(MapNameToId, mapThis, state_.targetNameShort, &mapShort)) {
            if (state_.settings.verbose) {
                std::cout << "[DBG] NameMap转ID(short)=" << ToUtf8FromAnsi(state_.targetNameShort)
                          << " ok=1 id=0x" << std::hex << mapShort << std::dec << "\n";
            }
        }
        if (MapNameToIdUpper(MapNameToId, mapThis, state_.targetNameType, &mapType)) {
            if (state_.settings.verbose) {
                std::cout << "[DBG] NameMap转ID(type)=" << ToUtf8FromAnsi(state_.targetNameType)
                          << " ok=1 id=0x" << std::hex << mapType << std::dec << "\n";
            }
        }
    }

    LinkMatch linkByRaw{};
    if (GetLinkFromNO && pContainer && preLinkId > 0) {
        SetStage("find_link_by_id");
        TryFindLinkByIdSafe(&linkByRaw, pContainer, GetLinkFromNO, state_.settings, logicBase,
                            logicSize, preLinkId);
        if (linkByRaw.link) {
            if (state_.settings.verbose) {
                std::cout << "[DBG] LinkByRaw命中(预取Link匹配) id=0x" << std::hex << preLinkId
                          << " link=0x" << reinterpret_cast<uintptr_t>(linkByRaw.link) << std::dec
                          << "\n";
            }
        }
    }

    SetStage("resolve_parent");
    // Parent 解析：先尝试原始指针，再逐步回退到 TreeItem/映射/逻辑 ID。
    void* pParent = nullptr;
    void* fallbackParent = nullptr;
    if (rawParentData >= 0x100000 &&
        IsVtableInModule(reinterpret_cast<void*>(rawParentData), logicBase, logicSize)) {
        pParent = reinterpret_cast<void*>(rawParentData);
    }

    SetStage("get_plc_device");
    if (!pParent && GetPlcDeviceDevice && state_.targetItem) {
        pParent = GetPlcDeviceDevice(pContainer, state_.targetItem);
        if (state_.settings.verbose) {
            std::cout << "[DBG] TreeItem转设备=0x" << std::hex << reinterpret_cast<uintptr_t>(pParent)
                      << std::dec << "\n";
        }
    }

    SetStage("map_tree_to_id");
    if (!pParent && MapTreeToId && state_.targetItem) {
        void* mapTree = reinterpret_cast<void*>(
            reinterpret_cast<BYTE*>(pContainer) + offsets::kTreeToIdMapBase);
        int* slot = MapTreeToId(mapTree, static_cast<int>(reinterpret_cast<uintptr_t>(state_.targetItem)));
        int mapId = slot ? *slot : 0;
        if (state_.settings.verbose) {
            std::cout << "[DBG] MapTreeToId(TreeItem)=0x" << std::hex << mapId << std::dec << "\n";
        }
        if (mapId > 0 && GetDeviceByMap) {
            void* mapThis = reinterpret_cast<void*>(
                reinterpret_cast<BYTE*>(pContainer) + offsets::kContainerDeviceMap);
            void* candidate = nullptr;
            int ok = GetDeviceByMap(mapThis, mapId, &candidate);
            if (state_.settings.verbose) {
                std::cout << "[DBG] MapGetDevice 查询 id=0x" << std::hex << mapId
                          << " ok=" << ok << " out=0x" << reinterpret_cast<uintptr_t>(candidate)
                          << std::dec << "\n";
            }
            if (ok && candidate) {
                pParent = candidate;
            }
        }
    }

    SetStage("map_get_device");
    if (!pParent && GetDeviceByMap) {
        void* mapThis = reinterpret_cast<void*>(
            reinterpret_cast<BYTE*>(pContainer) + offsets::kContainerDeviceMap);
        LogPtr(state_.settings, "DeviceMapThis", mapThis);
        // 依次尝试多来源 ID，优先命中虚表一致的候选。
        int tryIdsPrefer[] = {nameId, mapName, fullId, mapFull, shortId, mapShort,
                              typeId, mapType, preLinkId, static_cast<int>(curControlId),
                              static_cast<int>(rawParentData)};
        int tryIdsDefault[] = {preLinkId, static_cast<int>(curControlId), nameId, fullId, shortId,
                               typeId, mapName, mapFull, mapShort, mapType,
                               static_cast<int>(rawParentData)};
        int* tryIds = preferTargetName ? tryIdsPrefer : tryIdsDefault;
        size_t tryCount = preferTargetName ? (sizeof(tryIdsPrefer) / sizeof(tryIdsPrefer[0]))
                                           : (sizeof(tryIdsDefault) / sizeof(tryIdsDefault[0]));
        for (size_t i = 0; i < tryCount; ++i) {
            int id = tryIds[i];
            if (id <= 0) continue;
            void* candidate = nullptr;
            int ok = GetDeviceByMap(mapThis, id, &candidate);
            if (state_.settings.verbose) {
                std::cout << "[DBG] MapGetDevice 查询 id=0x" << std::hex << id << " ok=" << ok
                          << " out=0x" << reinterpret_cast<uintptr_t>(candidate) << std::dec << "\n";
            }
            if (ok && candidate) {
                if (IsVtableInModule(candidate, logicBase, logicSize)) {
                    pParent = candidate;
                    break;
                }
                if (!fallbackParent) {
                    fallbackParent = candidate;
                    if (state_.settings.verbose) {
                        std::cout << "[DBG] MapGetDevice 虚表不一致，作为回退候选\n";
                    }
                }
            }
        }
    }

    SetStage("logic_get_device");
    // 最后回退到逻辑层 ID 查询，保留虚表不一致候选用于兜底。
    if (!pParent) {
        FnGetDeviceByLogicID GetDeviceByLogicID =
            reinterpret_cast<FnGetDeviceByLogicID>(reinterpret_cast<BYTE*>(hLogic) +
                                                   offsets::kGetDeviceByLogicId);
        int tryIdsPrefer[] = {nameId, mapName, fullId, mapFull, shortId, mapShort,
                              typeId, mapType, preLinkId, static_cast<int>(curControlId),
                              static_cast<int>(rawParentData)};
        int tryIdsDefault[] = {preLinkId, static_cast<int>(curControlId), nameId, fullId, shortId,
                               typeId, mapName, mapFull, mapShort, mapType,
                               static_cast<int>(rawParentData)};
        int* tryIds = preferTargetName ? tryIdsPrefer : tryIdsDefault;
        size_t tryCount = preferTargetName ? (sizeof(tryIdsPrefer) / sizeof(tryIdsPrefer[0]))
                                           : (sizeof(tryIdsDefault) / sizeof(tryIdsDefault[0]));
        for (size_t i = 0; i < tryCount; ++i) {
            int id = tryIds[i];
            if (id <= 0 || !GetDeviceByLogicID) continue;
            void* candidate = GetDeviceByLogicID(pDataContainer, id);
            if (state_.settings.verbose) {
                std::cout << "[DBG] 逻辑ID取设备(0x" << std::hex << id << ")=0x"
                          << reinterpret_cast<uintptr_t>(candidate) << std::dec << "\n";
            }
            if (candidate) {
                if (IsVtableInModule(candidate, logicBase, logicSize)) {
                    pParent = candidate;
                    break;
                }
                if (!fallbackParent) {
                    fallbackParent = candidate;
                    if (state_.settings.verbose) {
                        std::cout << "[DBG] GetDeviceByLogicID 虚表不一致，作为回退候选\n";
                    }
                }
            }
        }
        if (!pParent && fallbackParent) {
            pParent = fallbackParent;
            if (state_.settings.verbose) {
                std::cout << "[DBG] 使用 Parent 回退候选: 0x" << std::hex
                          << reinterpret_cast<uintptr_t>(pParent) << std::dec << "\n";
            }
        }
    }

    // MASTER 类型优先使用预匹配的 Link 作为 Parent。
    if (IsMasterTypeName(targetName) && linkByRaw.link) {
        if (state_.settings.verbose) {
            std::cout << "[DBG] 目标为 MASTER，强制 Parent=LinkByRaw\n";
        }
        pParent = linkByRaw.link;
    }

    out->pParent = pParent;
    LogPtr(state_.settings, "ParentObj", pParent);
    LogVtable(state_.settings, "ParentObj", pParent);

    unsigned int parentType = 0;
    if (pParent && IsReadablePtr(reinterpret_cast<unsigned char*>(pParent) + 12)) {
        parentType = *(reinterpret_cast<unsigned char*>(pParent) + 12);
        if (state_.settings.verbose) {
            std::cout << "[DBG] Parent类型=0x" << std::hex << parentType << std::dec << "\n";
        }
    }

    if (state_.settings.verbose) {
        std::cout << "[DBG] 类型判断 Modbus=" << (IsKindOf(pParent, clsModbus) ? 1 : 0)
                  << " DP=" << (IsKindOf(pParent, clsDp) ? 1 : 0)
                  << " Gateway=" << (IsKindOf(pParent, clsGateway) ? 1 : 0) << "\n";
    }

    if (!requireLink) {
        if (state_.settings.verbose) {
            std::cout << "[DBG] 跳过Link解析（无需Link）\n";
        }
        out->pLink = nullptr;
        out->commIdx = 0;
        out->linkIdx = 0;
        out->subIdx = 0;
        SetStage("resolve_done");
        return true;
    }

    SetStage("resolve_link");
    // Link 解析优先级：预匹配 -> 预取 -> PapaLink -> 索引推导 -> 组合扫描。
    void* pLink = nullptr;
    unsigned int commIdx = 0;
    unsigned int linkIdx = 0;
    unsigned int subIdx = 0;

    if (linkByRaw.link && IsVtableInModule(linkByRaw.link, logicBase, logicSize)) {
        pLink = linkByRaw.link;
        commIdx = linkByRaw.commIdx;
        linkIdx = linkByRaw.linkIdx;
        subIdx = linkByRaw.subIdx;
    }

    if (!pLink && preLink && IsVtableInModule(preLink, logicBase, logicSize)) {
        pLink = preLink;
        commIdx = 1;
        linkIdx = 1;
        subIdx = 0;
    }

    if (!pLink && pParent && GetPapaLink) {
        void* candidate = GetPapaLink(pParent);
        if (IsVtableInModule(candidate, logicBase, logicSize)) {
            pLink = candidate;
        }
    }

    if (!pLink && pParent && GetLinkIndexModbus) {
        unsigned int idx = GetLinkIndexModbus(pParent);
        pLink = TryGetLinkByIndex(pContainer, idx, GetLinkFromNO, state_.settings, logicBase,
                                  logicSize);
        linkIdx = idx;
    }

    if (!pLink && pParent && GetLinkIndexDp) {
        unsigned int idx = GetLinkIndexDp(pParent);
        pLink = TryGetLinkByIndex(pContainer, idx, GetLinkFromNO, state_.settings, logicBase,
                                  logicSize);
        linkIdx = idx;
    }

    if (!pLink && pParent && GetLinkIndexGateway) {
        unsigned int idx = GetLinkIndexGateway(pParent);
        pLink = TryGetLinkByIndex(pContainer, idx, GetLinkFromNO, state_.settings, logicBase,
                                  logicSize);
        linkIdx = idx;
    }

    if (!pLink && pParent && GetCommunIndex) {
        commIdx = GetCommunIndex(pParent);
    }
    if (!pLink && pParent && GetSubCommunIndex) {
        subIdx = GetSubCommunIndex(pParent);
    }

    if (!pLink && pParent && GetCommunIndexDp) {
        commIdx = GetCommunIndexDp(pParent);
    }
    if (!pLink && pParent && GetCommunIndexGateway) {
        commIdx = GetCommunIndexGateway(pParent);
    }

    if (!pLink && pParent && linkIdx > 0) {
        pLink = TryGetLinkByIndices(pContainer, commIdx, linkIdx, subIdx, GetLinkFromNO,
                                    state_.settings, logicBase, logicSize);
    }

    if (!pLink && pParent && preLinkId > 0) {
        pLink = TryGetLinkByIndex(pContainer, preLinkId, GetLinkFromNO, state_.settings,
                                  logicBase, logicSize);
        linkIdx = preLinkId;
    }

    if (!pLink) {
        std::cout << "[-] ResolveContext: 未找到 Link。\n";
        return false;
    }

    out->pLink = pLink;
    out->commIdx = commIdx;
    out->linkIdx = linkIdx;
    out->subIdx = subIdx;

    if (state_.settings.verbose) {
        std::cout << "[CTX] linkIdx=" << linkIdx << " commIdx=" << commIdx << " subIdx=" << subIdx
                  << "\n";
    }
    LogPtr(state_.settings, "ResolvedLink", pLink);
    LogVtable(state_.settings, "ResolvedLink", pLink);
    if (state_.settings.enableLinkCommProbe && GetCommunNoForLink && pLink) {
        int commNo = GetCommunNoForLink(pContainer, pLink);
        if (state_.settings.verbose) {
            std::cout << "[DBG] GetCommunNoForLink=0x" << std::hex << commNo << std::dec << "\n";
        }
    }
    SetStage("resolve_done");
    return true;
}

/**
 * @brief 带 SEH 保护的上下文解析入口。
 * @param rawParentData TreeItem 的 lParam 原始值。
 * @param targetName 用户输入的目标名称。
 * @param out 输出解析结果。
 * @return 解析成功返回 true。
 */
bool ContextResolver::SafeResolve(DWORD rawParentData,
                                  const char* targetName,
                                  ResolvedContext* out,
                                  bool requireLink,
                                  bool preferTargetName) {
    __try {
        SetStage("seh_enter");
        return Resolve(rawParentData, targetName, out, requireLink, preferTargetName);
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        std::cout << "[-] ResolveContext: 捕获异常，阶段=" << StageToZh(state_.lastStage)
                  << "（可能是无效指针或线程亲和性问题）。\n";
        return false;
    }
}

}  // namespace hw
