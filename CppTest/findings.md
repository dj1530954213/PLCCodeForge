# Findings & Decisions
<!-- 
  WHAT: Your knowledge base for the task. Stores everything you discover and decide.
  WHY: Context windows are limited. This file is your "external memory" - persistent and unlimited.
  WHEN: Update after ANY discovery, especially after 2 view/browser/search operations (2-Action Rule).
-->

## Requirements
<!-- 
  WHAT: What the user asked for, broken down into specific requirements.
  WHY: Keeps requirements visible so you don't forget what you're building.
  WHEN: Fill this in during Phase 1 (Requirements & Discovery).
  EXAMPLE:
    - Command-line interface
    - Add tasks
    - List all tasks
    - Delete tasks
    - Python implementation
-->
<!-- Captured from user request -->
- Review `HwHack`, `plc-mfc-reverse-suite`, and `Docs` to understand context
- Use MCP/IDA tools to analyze and update `Docs/当前了解的相关函数内容.md`
- User has three IDA instances with MCP on ports 13337, 13338, 13339

## Research Findings
<!-- 
  WHAT: Key discoveries from web searches, documentation reading, or exploration.
  WHY: Multimodal content (images, browser results) doesn't persist. Write it down immediately.
  WHEN: After EVERY 2 view/browser/search operations, update this section (2-Action Rule).
  EXAMPLE:
    - Python's argparse module supports subcommands for clean CLI design
    - JSON module handles file persistence easily
    - Standard pattern: python script.py <command> [args]
-->
<!-- Key discoveries during exploration -->
- No `AGENTS.md` found under `C:/Program Files/Git/code/PLCCodeForge/CppTest`
- Planning files initialized from templates in project root
- session-catchup script executed from `.codex` path; no unsynced context reported
- `Docs/IDA_MCP_多开与端口管理.md` maps MCP server names to ports 13337-13341
- IDA MCP plugin priority: `IDA_MCP_URL` > `IDA_MCP_HOST`+`IDA_MCP_PORT` > default `127.0.0.1:13337`
- Recommended launch script: `ida-mcp-launch.ps1` with per-instance ports/IDBs
- MCP connectivity confirmed: `ida-pro-mcp`/`ida-pro-mcp-1`/`ida-pro-mcp-2` respond to `imports` calls
- `HwHack/HwHack.cpp` implements UI TreeView search and injects nodes via `dllDPLogic.dll` offsets (0x59F10, 0x50770)
- `HwHack/HwHack.cpp` uses `OnDPTreeSlaveOperate` and falls back to `UpdateView` when tree insertion fails
- `HwHack/HwHack.cpp` only queries Name->ID map (`MapNameToId`), no write/update path found for NameMap
- `HwHack/HwHack.cpp` defaults: `kPreferAddNodeToCfgTree=true`, `kEnableOnSlaveOperate=false`, `kEnableOnDPTreeOperate=true`, `kEnableSmartInsert=false`
- Updated `HwHack/HwHack.cpp`: run `OnSlaveOperate` first, then fallback to `AddNodeToCfgTree` only if no TreeItem mapping exists; avoids duplicate UI insertion while still allowing fallback
- Added tree message timeouts in `HwHack/HwHack.cpp` to avoid UI-thread hangs during TreeView enumeration (`SendMessageTimeout` around TVM_GETNEXTITEM/TVM_GETITEM)
- Moved target tree dump out of injection path: dump `target_before` when target selected, schedule `target_after` via timer after injection to avoid blocking UI inserts
- Disabled device introspection (`GetCommunIndex`/`GetLinkIndex`/`GetUserNameA`) and link->comm probe by default to avoid potential UI-thread stalls before `OnSlaveOperate`
- Added `OnAddSlave` UI entry path: call `CHWFrameContainer::OnAddSlave` first (commIdx/linkIdx/type/address) to let UI manage insert/mappings; fallback to MakeSlave path only if it fails
- `plc-mfc-reverse-suite` is a skill suite for MFC PLC reverse engineering with a staged workflow (case-init, triage, UI map, static core, dynamic hook, config, protocol, delivery)
- `Docs/当前了解的相关函数内容.md` describes a call chain in `dllDPLogic.dll`: GetGlobalInstance (RVA 0xDB560) -> GetDevice (RVA 0x50770) -> GetLinkFromNO (RVA 0x117830) -> OnMakeNewLogicData_Slave (RVA 0x59F10)
- `Docs/本地相关工具及MCP.md` lists local reverse tooling and MCP plugin locations (IDA, x64dbg, Cheat Engine, ReClass.NET, radare2, etc.)
- IDA MCP mapping: `ida-pro-mcp` -> `dllDPLogic.dll` (imagebase 0x10000000)
- IDA MCP mapping: `ida-pro-mcp-1` -> `dll_DPFrame.dll` (imagebase 0x10000000)
- IDA MCP mapping: `ida-pro-mcp-2` -> `dllDPSource.dll` (imagebase 0x10000000)
- IDA `dllDPLogic.dll` lookup: 0x10050770 resolves to `CHWDataContainer::GetDeviceByLogicID`; 0x10059F10 resolves to `CHWDataContainer::OnMakeNewLogicData_Slave`
- IDA `dllDPLogic.dll` lookup: 0x100DB560 is in `.rdata` (not a function); 0x10117830 not defined as a function in this IDB
- `dllDPLogic.dll` has `GetStation` symbols: `?GetStation@@YAPAVCStation@@XZ` (0x1002D920) and `?GetStation@CAppGlobalFunc@@SAPAVCStation@@XZ` (0x100AE3A6)
- No code refs/xrefs found to 0x100AE3A6 via MCP queries (may be unused, inlined, or missing refs)
- `list_funcs` shows many `CHWDataContainer` methods are symbolized; no `CHWContainer` symbols found with that filter
- `CAppGlobalFunc::GetStation` (0x100AE3A6) is a thunk to `__imp_?GetStation@CAppGlobalFunc@@SAPAVCStation@@XZ`; global `GetStation` (0x1002D920) just calls `CAppGlobalFunc::GetStation`
- `CHWContainer::GetLinkFromNO` is in `dll_DPFrame.dll` at 0x10117830 (found via `ida-pro-mcp-1`)
- Decompile `CHWContainer::GetLinkFromNO` shows it calls `sub_100DB560()` (in `dll_DPFrame.dll`) and uses helper functions `sub_10043690` / `sub_10043610`
- `CHWDataContainer::GetDeviceByLogicID` signature confirmed: `CDevice* __thiscall GetDeviceByLogicID(CHWDataContainer* this, int id)`
- `dll_DPFrame.dll` `sub_100DB560` returns `*(CAppGlobalFunc::GetStation() + 0xA7C)`; likely the global container pointer (matches doc's "GetGlobalInstance")
- IDA prototypes: `CHWContainer::GetLinkFromNO` is `CLink* __thiscall(CHWContainer*, unsigned int, unsigned int, unsigned int)`
- IDA prototypes: `CHWDataContainer::OnMakeNewLogicData_Slave` has 10 params; IDA shows `char __fastcall(int*, int, int, unsigned int, int**, int, CDPLink*, int**, int, void*, int**)` (types need refinement)
- `Docs/硬件自动化组态规则.md` 已包含两块内容：协议序列化规则与 API 注入层规则（含 UI 刷新待解决）
- `HwHack/HwHack.cpp` 通过 TreeView 查找+手动输入 ECX/Link 完成注入与临时 UI 插入
- `dll_DPFrame.dll` 中 `CHWContainer` 具有明显“视图/控制”职责（SetView/SetInfoView/UpdateView 等）
- `CHWContainer::UpdateView` 调用 `CHWFrameContainer::UpdataView`（对象偏移 +1600），说明视图更新通过 FrameContainer 实现
- `CHWContainer::UpdateView` (0x10106E00) only calls `CHWFrameContainer::UpdataView(this+1600, a2)` and returns 1
- `dll_DPFrame.dll` 的 `sub_10043610`/`sub_10043690` 是相同的“按索引取节点”函数：`this[1]` 为链表头，`this[3]` 为计数，遍历 `next` 指针得到元素
- `CHWContainer::GetDataContainer` 返回 `this + 584`，说明 `CHWDataContainer` 作为子对象嵌入在 `CHWContainer` 内
- `CHWContainer::SetView` 将 `CHardWareView*` 写入偏移 144，体现 View 指针由 Container 持有
- `CHWContainer::GetCurControlIDAndName` 读取 `this[416]` 作为当前控制 ID，并通过 `CHWFrameContainer::GetControlName` 返回名称
- `CHWContainer::SetCurControlIDAndName` 将当前控制 ID/名称写入 `this[416]` 与 `this[415]`（CString）
- `CHWDataContainer::OnMakeNewLogicData_Slave` 内部主要调用 `CHWDataContainer::MakeNewData` 循环创建数据，失败返回 0
- `OnMakeNewLogicData_Slave` 有上层封装 `CHWDataContainer::OnMakeNewLogicData`（xref 0x1005A87C）
- `CHWDataContainer::OnMakeNewLogicData` 对 CPU 类型与 AppVersion 做判定后分流：协议/控制/从站三条路径
- `CHWDataContainer` 存在多种 `MakeNewData` 重载（带 CControl/CLink/tagCFG_MOD_INFO 等参数）
- `CHWDataContainer::MakeNewData`（含 CControl/CLink 参数）按协议类型创建不同设备对象（如 CDPSlave、CModbusSlave、CGateWayDevice），设置名称/描述/ID，并插入容器与 Link 关系
- `MakeNewData` 创建完成后会调用 `CBaseDPContainer::SetModifyLogic`，标记数据层已修改
- `CHWContainer::GetLinkFromNO` 汇编显示：`sub_100DB560()` 返回全局对象指针；随后读取 `[obj+0x288]`，并在其 `+0x84`/`+0x68` 上执行链表索引获取 Link
- `sub_100DB560` 汇编确认返回 `*(CAppGlobalFunc::GetStation() + 0xA7C)`
- `CHWFrameContainer::OnAddSlave` 中调用 `CHWContainer::GetLinkFromNO(Global, a2, a3, 0)`，说明 a2/a3 为上层传入的“Link 选择参数”（0 表示不走第三层选择）
- `CHWContainer::CreateModbusSlave_TCP` 中调用 `GetLinkFromNO(Global, 1, v79, 0)`，a2 固定为 1，a3 为枚举到的 Link 序号
- `CHWFrameContainer::OnAddCommun` 中调用 `GetLinkFromNO(Global, p_p_wParam, a4, 0)`，a2/a3 来自上层传参（与通信链路选择相关）
- `CHWContainer::GetCommunDeviceFromNO(a1, name)`：若 name 为空则从 `[global+648]` 的 `+132` 链表按 `a1-1` 取；若 name 非空则遍历 `+136` 链表按名称匹配
- `CModbusSlave::GetLinkIndex` 返回偏移 +124 的字节（LinkIndex）
- `CDPSlave::GetPapaLink` 返回 `this + 34*4`（偏移 0x88）的 `CDPLink*`
- `CDPSlave::GetLinkIndex` 返回偏移 +132 的字节（LinkIndex）
- `CModbusSlave::SetLinkIndex` 设置偏移 +124 的 LinkIndex
- Context Resolver 策略：优先使用 CDPSlave::GetPapaLink；否则读取 LinkIndex 并调用 `GetLinkFromNO` 搜索匹配 Link
- `CModbusSlave` 提供 `GetCommunIndex`/`GetSubCommunIndex`/`GetLinkIndex` 三类索引字段，可用于推断 GetLinkFromNO 的 a2/a3/a4
- `CModbusSlave::GetCommunIndex` 返回 `this[32]`；`GetSubCommunIndex` 返回 `this[33]`
- `CDPSlave::GetCommunIndex` 返回 `this[44]`
- `CGateWayDevice::GetCommunIndex` 返回 `this[43]`，`GetLinkIndex` 为偏移 +168
- `CHWDataContainer::GetLogicIDFromName` (RVA 0x484D0) 可用于名称 -> 逻辑 ID 反查
- `CHWContainer::GetCurControlIDAndName` 可提供当前选中节点的逻辑 ID
- ResolveContext 在控制台线程可能触发崩溃，已调整到 UI 线程（Timer 回调）执行并加 SEH 防护
- `dll_DPFrame.dll` 高层 UI 路径函数：`CHWFrameContainer::OnAddSlave` (0x101A7AF0), `OnAddCommun` (0x101AEEC0), `OnAddProcotol` (0x101A68C0), `OnAddModbusSlave_RTU` (0x101AB7A0)
- `dll_DPFrame.dll` Tree 操作入口：`CHWContainer::OnDPTree_Slave_Operate` (0x10167AB0), `OnDPTree_CommunDevice_Operate` (0x101529F0), `OnDPTree_Procotol_Operate` (0x10153BD0)
- `dll_DPFrame.dll` 刷新接口：`CHWContainer::UpdateView` (0x10106E00), `CHWFrameContainer::RefreshDPTreeForAdd` (0x10199090)
- `dllDPLogic.dll` 低层创建入口：`OnMakeNewLogicData_Slave` (0x10059F10), `OnMakeNewLogicData` (0x1005A720), `MakeNewData` 重载 0x10057490
- `dllDPSource.dll` 资源/协议入口：`GetSourceContainer` (0x10083540), `GetProcotolIDFormName` (0x100AA4A0), `ReadModChannelInfoToMap` (0x100AF6C0)
- `CHWFrameContainer::OnAddSlave` (0x101A7AF0) 反编译显示：入参包含 a2/a3（Link/Comm 序号）与多个 CString（名称/地址），内部调用 `CHWContainer::GetLinkFromNO(Global, a2, a3, 0)` + `GetCommunDeviceFromNO(a2, ...)`，并走 `CHWDataContainer::OnMakeNewLogicData(Global+584)` 创建数据
- `CHWContainer::OnDPTree_Slave_Operate` (0x10167AB0) 会规范化名称、解析括号文本，再调用 `CHWContainer::OnSlave_Operate`，其中 `GetLinkFromNO(this, a4, a5, 0)` 仍依赖上层传入的 Link/Comm 序号
- `CHWContainer::OnDPTree_Slave_Operate` uses NameMap (`sub_10045E00`) to resolve name->ID, then `sub_10045E80` to resolve ID->device, and calls `OnSlave_Operate` only when both link+device exist
- `OnDPTree_Slave_Operate` 反汇编确认：`sub_10045E00` 调用前 `ecx=CHWContainer+0x1FC`（名称→ID 映射表），随后 `sub_10045E80` 使用 `ecx=CHWContainer+0x250`（ID→Device 映射），说明 Tree 名称解析不走 `CHWDataContainer::GetLogicIDFromName`
- `CHWContainer::GetLinkFromNO` 的 `a2/a3/a4` 依次为 commIdx/linkIdx/subIdx，`CHWContainer::GetCommunNoForLink` 通过 `link+0x10` 的 linkId 匹配通信号，暗示 TreeItem.lParam 可能是 linkId
- 运行日志显示 `NameMap转ID(TESTAAA)=0x17`，与 `GetLinkFromNO(1,1,0)` 的 `linkId=0x17` 一致，说明 TESTAAA 对应的 Master Link 可直接用预取 Link 命中
- 日志中的 `0xffffffff` 为 `-1`（未找到）返回值，来自 `GetLogicIDFromName`；后续已在日志中标记为“未找到”并归一化为 0
- `CHWContainer::GetCommunNoForLink` (0x101293B0) 可从 Link 反推 commIdx，用于修正 OnSlaveOperate 的通信索引参数
- `OnDPTree_Slave_Operate` 签名经 demangle 确认为 `OnDPTree_Slave_Operate(char, CString, int, int, CString, CString, uint)`，可作为 UI 插入与映射更新入口
- `sub_10046D70`（疑似 AddDeviceDlg::OnOK）从 TreeView 当前项 + 对话框字段（this[68]/[69]/[70]/[71]）取 Link/Comm 信息，最终调用 `CHWFrameContainer::OnAddSlave/OnAddCommun/OnAddModbusSlave_RTU` 等高层 UI 入口
- `CHWContainer::GetPLCDeviceDevice` (0x10125CB0) 以 `_TREEITEM*` 为入参，先通过 `sub_100F7290` 查出逻辑 ID，再经 `sub_10045E80` 映射到 `CDevice*`
- `sub_10045E80` 反汇编确认 `retn 8`（两参），且 `CHWContainer::GetPLCDeviceDevice` 在调用前将 `ecx` 置为 `this+0x250`，因此其正确形态为 `int __thiscall sub_10045E80(void* mapThis, int id, void** out)`；需要传入 `pContainer+0x250` 作为 this
- `sub_100F7290`/`sub_100F7030` 实际是 TreeItem->ID 的哈希表查找逻辑（`sub_100F7030` 使用 `this[1]/this[2]` 作为桶表/计数），表明 TreeView 句柄可能比 lParam 更可靠
- 定位到 `CHWContainer::CreateNodeData` (0x10167760)，疑似 TreeItem/NodeData 的创建入口，可能包含 lParam/ID 写入逻辑
- 定位到 `CHWContainer::AddNodeToCfgTree` (0x10150940)，可能负责 TreeView 插入与 TreeItem 数据绑定
- `AddNodeToCfgTree` 内部调用 `CTreeCtrl::InsertItem(..., lParam)`，随后通过 `sub_10149D80` 将 `insertedTreeItem -> DeviceID` 写入映射，再用 `sub_10149DF0` 记录 `DeviceID -> TreeItem`，说明 lParam 并非唯一可信的逻辑 ID 来源
- `CHWContainer::AddNodeToCfgTree` (0x10150940) ensures unique name (renames when Caption), inserts via `CTreeCtrl::InsertItem`, then writes TreeItem<->ID mapping using `sub_10149D80(inserted)` and `sub_10149DF0(deviceId)`
- `sub_10149D80` / `sub_10149DF0` 为 `__thiscall`，this 分别是 `CHWContainer+0x9B8` 与 `CHWContainer+0x9D4` 的两张映射表；返回值为槽位地址（`return eax+4`），可直接写入 ID/TreeItem
- `CHWContainer::FillHwCfgTree` (0x10150D12) 调用 `AddNodeToCfgTree` 并使用 `this+0x250` 作为映射表，说明通过该接口插入节点可保证双击/属性窗口正常
- `CHWContainer::UpdateView` 仅调用 `CHWFrameContainer::UpdataView`，不会补齐 TreeItem/ID 映射，因此仅靠 UpdateView 不能修复图标/属性窗口问题
- `sub_10149D80`/`sub_10149DF0` 结构与 `CMap::operator[]` 类似，分别维护 TreeItem/ID 双向映射（依赖 `sub_100F7030` 与 `sub_10112190` 的哈希桶查找）
- `CAppGlobalFunc::GetProTreeHwnd` 在 `OnAddSlave` 里通过 IAT 调用（`call ds:__imp_?GetProTreeHwnd@CAppGlobalFunc@@SAPAUHWND__@@XZ`），说明可从 `dll_DPFrame.dll` Import 表找到其 IAT 槽位并间接调用
- 运行时证据（x32）：`dll_DPFrame.dll` 基址 0x7A320000，`dllDPLogic.dll` 基址 0x77600000
- `GetLinkFromNO` 入口实测（从 `OnDPTree_Slave_Operate` 触发）：ECX=0x11B65460，args=1/1/0，返回 `CLink*`=0x11AC2E38
- 返回 `CLink*` 的 vftable=0x776C84CC（落在 `dllDPLogic.dll`），`[link+0xC]=0x64`，对应 `CModbusTCPLink` / ModbusTCP 类型
- `GetPLCDeviceDevice` 断点未触发，说明当前 UI 路径未直接调用该函数
- `CHWContainer::OnSlave_Operate` 在 `dll_DPFrame.dll` 中存在两套重载：DPLink 版 (0x101557F0) 与 ModbusTCPLink 版 (0x10155D70)
- `OnSlave_Operate` (ModbusTCP 版) 的 `a4` 被当作 `CDevice*` 使用，且仅检查 `a3/a4` 非空；结合运行时数据，`GetLinkFromNO` 与 `sub_10045E80` 都返回 `CModbusTCPLink`，说明 Modbus 路径下 Link 对象本身即作为“父设备/CDevice”使用
- `OnSlave_Operate` (ModbusTCP 版) 反编译：签名为 `this, op, link, device, commIdx, linkIdx, displayText, typeName`（后两个为 `CString`），`op=1` 为新增；要求 `commIdx/linkIdx >= 1`，且返回 1 并不等价于 UI 树插入成功
- `CHWContainer::OnSlave_Operate` (0x10155D70) 在 `op=1` 路径会：规范化/大写名称、写 NameMap(`sub_1008E4E0`)、发送 `GetProTreeHwnd(1126)` 消息插树，并调用 `CBaseDPContainer::SetModifyUnLogic` 标记修改
- `OnSlave_Operate` (ModbusTCP 版) 在新增路径中会解析 `displayText` 中的括号字符串并使用 `typeName` 参与设备命名与 UI 消息构造
- `CHWContainer::OnDPTree_Slave_Operate` (0x10167AB0) 调用 `GetLinkFromNO(this, a4, a5, 0)` 并将 `a4/a5` 继续传给 `OnSlave_Operate`，表明 `commIdx/linkIdx` 的顺序为先 comm 后 link
- `CHWFrameContainer::RefreshDPTreeForAdd` (0x10199090) 需要 `CSlot*` 入参，内部用 `slot->id` 取设备并发送树消息刷新（依赖 `GetProTreeHwnd(1126)`），无法直接用 `CDevice*` 调用
- Repo search found no `CSlot` usage; need IDA to locate slot getter for `RefreshDPTreeForAdd`
- `CHWFrameContainer::OnAddSlave` (0x101A7AF0) uses `GetLinkFromNO(a2,a3,0)` + `GetCommunDeviceFromNO` then calls `CHWDataContainer::OnMakeNewLogicData(global+584)`; after creation it maps ID->device via `sub_10045E80` and sends a `GetProTreeHwnd(1126)` message to refresh the tree (not `UpdateView`)
- `CHWFrameContainer::RefreshDPTreeForAdd` confirms it needs `CSlot*` and uses `slot->id` to resolve `CPLCDevice*` before sending tree messages; also emits additional child nodes via `GetPrmIdxByID`
- Latest test log: NameMap can resolve TESTAAA -> id 0x20, but TreeItem->device mapping still fails; UI tree flickers yet no new node shown (need full inject segment to see OnSlaveOperate/AddNode outcome)
- `Docs/硬件自动化组态规则.md` notes UI refresh is still pending and calls out `UpdateView -> UpdataView` as the refresh path to study
- Reviewed `Docs/硬件自动化组态规则.md`: UI refresh analysis is in section 5/6; will extend with OnAddSlave/RefreshDPTreeForAdd details and concrete UI insert path
- `CHWContainer::FillHwCfgTree` (0x10150D12) 签名为 `this, CTreeCtrl*, HTREEITEM parent`，遍历 `this+323` 列表并对每个设备调用 `AddNodeToCfgTree`，不会自动清空 TreeView
- `CHWContainer::AddNodeToCfgTree` (0x10150940) 通过 IAT 调用 `CPLCDevice::GetUserNameA`，插入 TreeView 后写入 TreeItem↔ID 映射（`sub_10149D80/sub_10149DF0`）
- `AddNodeToCfgTree` 只要 `device` 与 `CTreeCtrl*` 非空就会调用 `CTreeCtrl::InsertItem` 插入；parent 由入参 `HTREEITEM` 指定，插入后立即写入 TreeItem↔ID 映射
- `AddNodeToCfgTree` 在设备名等于 `Caption` 时会遍历 parent 子节点生成唯一名称，再调用 `CPLCDevice::SetUserName`
- `CHWFrameContainer::OnAddSlave` 以 `commIdx/linkIdx` + `typeName`(CString) + `address`(CString) 为主要输入；内部调用 `GetLinkFromNO`、`GetCommunDeviceFromNO`、`CHWDataContainer::OnMakeNewLogicData`，并通过 `GetProTreeHwnd(1126)` 发送消息刷新树
- `sub_10046D70` 调用链显示 `OnAddSlave` 入参顺序为：commIdx、linkIdx、typeName(CString)、address(CString)、count(整数)、extra(字符串/指针)，第 6 参不是 owner 指针
- `sub_10046D70` 反编译中 `OnAddSlave(v50+1600, this[68], this[70], v88, i, this[71], this[78])`，且 `OnAddSlave` 内部以 `a6` 作为循环上限使用（`if (++v156 >= (unsigned int)a6)`），进一步确认第 6 参为数量
- `HwHack.cpp` 当前 `OnAddSlave` 调用将第 6/7 参传为 `NULL`，会导致 `OnAddSlave` 直接失败（期望 count/extra），需要改为传 `count` 与可选 extra 字符串
- 最新运行日志：`OnAddSlave` 返回 1，目标节点 `target_after` 子节点数从 0 变为 1，树新增 `MODBUSSLAVE_TCP(192.168.2.39:MODBUSSLAVE_TCP)`，UI 刷新成功
- 运行时证据（x32, 0x59F10）：
  - `ECX`=0x01BFE970（CHWDataContainer 实例）
  - Arg1=`"MODBUSSLAVE_TCP"`，Arg2=1，Arg3=1
  - Arg4=0x1A780B68（pOutID，返回后写入 0x12）
  - Arg5=0x11AC2E38（Link 指针，来自 `GetLinkFromNO(1,1,0)`）
  - Arg6=0x19190FC0（Parent 指针，来自 `sub_10045E80`）
  - Arg7=desc (CString*，IP/名称)
  - Arg8=count=1
  - Arg9=pContext=0x19190FC0（与 Arg6 相等）
- Arg5/Arg6 均为 `CModbusTCPLink` 类型，但为不同对象
- `sub_10045E80` 末尾为 `ret 8`（x32 实测），调用约定应为 `__stdcall`
- 最新运行：终端操作后树控件出现刷新但新设备不显示；保存重开后出现两个同名同IP设备，且重复现象仍在，说明数据层可能重复创建而 UI 映射未写入或刷新挂载失败
- IDA 反汇编/反混淆确认 `CHWDataContainer::OnMakeNewLogicData_Slave` 真正签名为 `CString, unsigned int, char, unsigned int*, CControl*, CLink*, CString, unsigned int, CControl*`；当第 3 参数( char )非 0 时会进入 2*count 循环，因此传入 `1` 会导致一次调用创建两个设备

## Technical Decisions
<!-- 
  WHAT: Architecture and implementation choices you've made, with reasoning.
  WHY: You'll forget why you chose a technology or approach. This table preserves that knowledge.
  WHEN: Update whenever you make a significant technical choice.
  EXAMPLE:
    | Use JSON for storage | Simple, human-readable, built-in Python support |
    | argparse with subcommands | Clean CLI: python todo.py add "task" |
-->
<!-- Decisions made with rationale -->
| Decision | Rationale |
|----------|-----------|
|          |           |

## Issues Encountered
<!-- 
  WHAT: Problems you ran into and how you solved them.
  WHY: Similar to errors in task_plan.md, but focused on broader issues (not just code errors).
  WHEN: Document when you encounter blockers or unexpected challenges.
  EXAMPLE:
    | Empty file causes JSONDecodeError | Added explicit empty file check before json.load() |
-->
<!-- Errors and how they were resolved -->
| Issue | Resolution |
|-------|------------|
| PowerShell `Get-ChildItem -Filter` does not accept array | Use `Test-Path` or separate calls |
| `Docs/本地相关工具及MCP` path missing | Use `Docs/本地相关工具及MCP.md` instead |
| `lookup_funcs` with newline-separated queries returned not found | Retry with comma-separated queries or address-based lookup |
| Documented RVAs (0xDB560, 0x117830) do not map to functions in current IDB | Need to locate equivalent functions via name/xrefs or adjust for version |
| MCP ida-pro-mcp-1 tool call failed (connection error) | Defer IDA check; ask user to confirm MCP server on 13338 |

## Resources
<!-- 
  WHAT: URLs, file paths, API references, documentation links you've found useful.
  WHY: Easy reference for later. Don't lose important links in context.
  WHEN: Add as you discover useful resources.
  EXAMPLE:
    - Python argparse docs: https://docs.python.org/3/library/argparse.html
    - Project structure: src/main.py, src/utils.py
-->
<!-- URLs, file paths, API references -->
- `Docs/当前了解的相关函数内容.md`
- MCP endpoints: `http://127.0.0.1:13337/mcp`, `http://127.0.0.1:13338/mcp`, `http://127.0.0.1:13339/mcp`
- MCP config file: `C:\Users\DELL\.codex\config.toml`
- MCP multi-instance doc: `Docs/IDA_MCP_多开与端口管理.md`
- Launch script: `ida-mcp-launch.ps1`
- Source: `HwHack/HwHack/HwHack.cpp`
- Suite README: `plc-mfc-reverse-suite/README.md`
- Current analysis doc: `Docs/当前了解的相关函数内容.md`
- Tools inventory: `Docs/本地相关工具及MCP.md`

## Visual/Browser Findings
<!-- 
  WHAT: Information you learned from viewing images, PDFs, or browser results.
  WHY: CRITICAL - Visual/multimodal content doesn't persist in context. Must be captured as text.
  WHEN: IMMEDIATELY after viewing images or browser results. Don't wait!
  EXAMPLE:
    - Screenshot shows login form has email and password fields
    - Browser shows API returns JSON with "status" and "data" keys
-->
<!-- CRITICAL: Update after every 2 view/browser operations -->
<!-- Multimodal content must be captured as text immediately -->
- Accessibility 树视图路径显示：窗口“AutoThink - TEST.hpf” → 窗格“工程管理” → 树“树” → 节点“TEST / 硬件配置 / LK220 / ETHERNET / TESTAAA(MODBUSTCP_MASTER)”
- 选中树节点“MODBUSSLAVE_TCP(192.168.2.39:MODBUSSLAVE_TCP)”时，Properties 显示 ControlType=TreeItem(50024)、FrameworkId=Win32、ExpandCollapsePattern 状态可见（截图中为 1 与 3）
- Accessibility 中树节点文本与注入显示文本一致，可能存在重复节点（截图中同名 TreeItem 出现两条）
- SysTreeView32 控件信息：NativeWindowHandle=0x096D18B6 (十进制 158144694)，AutomationId=1558，FrameworkId=Win32，ClassName=SysTreeView32，Name 空
- TreeItem 句柄为 0（UIA 正常现象），RuntimeId 包含 Tree 句柄（2A, 96D18B6, 4, 13D26A48），表明节点寄宿于该 Tree 控件
- 多个 SysTreeView32 同时存在，非主树控件的根节点文本在 ANSI 输出下出现乱码，需改为 Unicode 读取以便比对树结构
- 枚举到 3 个 SysTreeView32：id=1558（parentTitle=工程管理，root=TEST，count=31）、id=8197（root=设备库，count=56）、id=12002（root=库管理器，count=445）
- TreeDump 已确认工程管理树结构与截图一致（含“硬件配置→LK220→ETHERNET→TESTAAA”），但 `GetPLCDeviceDevice(TreeItem)` 返回 0，导致 Parent 回退到当前控制ID（LK220）

---
<!-- 
  REMINDER: The 2-Action Rule
  After every 2 view/browser/search operations, you MUST update this file.
  This prevents visual information from being lost when context resets.
-->
*Update this file after every 2 view/browser/search operations*
*This prevents visual information from being lost*
