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
-

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
-

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
|       |            |

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
-

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
-

---
<!-- 
  REMINDER: The 2-Action Rule
  After every 2 view/browser/search operations, you MUST update this file.
  This prevents visual information from being lost when context resets.
-->
*Update this file after every 2 view/browser/search operations*
*This prevents visual information from being lost*

## Findings: Docs/硬件自动化组态规则.md (initial scan)
- SerializeVersion fixed at 0x26; branches for >=0x27/0x36/0x39 are inactive in current runtime.
- MFC CString serialization must follow AfxReadStringLength (u8/u16/u32 length + optional Unicode prefix 0xFF 0xFFFE).
- Count widths differ by list type (e.g., mapping_count u16 vs order/channel u32).
- CModbusSlave::Serialize (ver 0x26) field order: base-chain, then fields A-F, flags, mapping list, orders, channels, extra_len/data.
- OnMakeNewLogicData_Slave (dllDPLogic) is the "god path" for creating devices; requires UI thread (TLS/MFC) and VS2010 MBCS CString by value.
- Data creation updates logic/model only; UI tree requires separate path (OnAddSlave / OnSlave_Operate / AddNodeToCfgTree) to insert tree items and update NameMap.
- CHWContainer (dll_DPFrame) owns CHWDataContainer (+0x248); global instance via sub_100DB560; OnMakeNewLogicData_Slave ECX uses CHWDataContainer*.
- Link/Parent resolution uses GetLinkFromNO, GetDeviceByLogicID / sub_10045E80; NameMap must be updated or tree insert will be skipped.

## Findings: HwHackInject.cpp (protocol add / UI refresh flow)
- Injector runs via UI thread timer callbacks (HandleInjectTimer/HandleDumpTimer) and uses AFX_MANAGE_STATE.
- ContextResolver::SafeResolve provides CHWContainer, CHWDataContainer (ECX), parent device, link, commIdx/linkIdx.
- Function pointers are resolved by module base + offsets: MakeNewLogicData_Slave (dllDPLogic), AddNodeToCfgTree / OnAddSlave / OnSlaveOperate / OnDptreeSlaveOperate + mapping helpers (dll_DPFrame).
- Primary path prefers UI-level OnAddSlave to keep tree + NameMap in sync; fallback MakeSlave path is gated by settings.
- After low-level creation, multiple UI sync strategies exist: OnSlaveOperate, AddNodeToCfgTree, OnDptreeSlaveOperate, SmartInsertNode, and mapping writes (Tree<->ID, Name->ID).
- Uses MapIdToTree/MapTreeToId/MapNameToId to reconcile tree nodes; ensures visibility via TreeView_EnsureVisible.
- Hardcoded test uses typeName="MODBUSSLAVE_TCP" and desc IP string, matching doc/test scenario.

## Findings: HwHackRuntime.cpp (UI tree discovery + injection trigger)
- Runtime locates AutoThink main window by title, then finds SysTreeView32; prefers control ID from settings, else fallback.
- Dumps tree info and child nodes; uses "硬件配置" node to navigate (matches screenshot flow).
- Console workflow prompts for parent node name (examples include LK220/ETHERNET/GROUP1), selects tree item, extracts text variants (full/short/type).
- Sets target item and lParam (parentData) then uses SetTimer on main window to trigger injection on UI thread.

## Findings: HwHackContext.cpp (context resolution logic)
- ContextResolver binds frame/logic functions by module base + offsets (GetGlobal, GetDataContainer, GetLinkFromNO, GetDeviceByMap, GetLogicIdFromName, MapNameToId, MapTreeToId, GetDeviceByLogicID, GetPapaLink, GetLinkIndex*).
- Resolves Parent by priority: raw TreeItem lParam pointer -> TreeItem->PLCDevice -> TreeItem->ID->Device map -> device map via multiple ID sources -> logic ID lookup; uses vtable-in-module checks to validate.
- Builds multiple name variants (targetName, full/short/type from tree text) and uses NameMap + GetLogicIdFromName to find IDs.
- Resolves Link by priority: pre-match by LinkId, pre-link (1,1,0), PapaLink, link index for Modbus/DP/Gateway, comm/sub index scanning via GetLinkFromNO.
- For MASTER-type names, forces Parent=Link to align with protocol hierarchy.
- Uses SEH guarding and stage logging to avoid crashes from invalid pointers and to diagnose thread affinity issues.

## Findings: HwHackConfig.h + HwHackTypes.h (offsets + function signatures)
- Offsets map IDA-verified RVAs: dllDPLogic (OnMakeNewLogicData_Slave @0x59F10, GetDeviceByLogicId @0x50770, GetPapaLink/LinkIndex/CommIndex, GetLogicIdFromName, GetUserName), dll_DPFrame (GetGlobal @0xDB560, GetLinkFromNO @0x117830, GetDataContainer @0x106C60, AddNodeToCfgTree/OnSlaveOperate/OnAddSlave/OnDptreeSlaveOperate, mapping tables).
- FnMakeNewLogicData_Slave signature matches doc: __thiscall, CString by value for typeName/desc, returns char/bool.
- UI-level add path uses FnOnAddSlave(commIdx, linkIdx, typeName, address, count, extra); alternate UI ops include OnSlaveOperate and OnDptreeSlaveOperate.
- Mapping utilities are explicit types: Name->ID map, Tree->ID map, ID->Tree map.
- Settings default to prefer OnAddSlave and disable low-level fallback injection, aligning with UI-sync requirement.

## Findings: MCP/IDA docs
- Docs/本地相关工具及MCP.md lists local reverse-engineering tools and confirms MCP plugins for x64dbg, Cheat Engine, ReClass.NET, IDA.
- Docs/IDA_MCP_多开与端口管理.md defines MCP port mapping: ida-pro-mcp=13337, ida-pro-mcp-1=13338, ida-pro-mcp-2=13339 (matches user’s three IDA instances).
- IDA MCP plugin prioritizes IDA_MCP_URL env var; if set globally, all instances may bind same port.
- ida-mcp-launch.ps1 supports multi-instance + optional auto-start MCP for specific IDB paths (dllDPLogic, dll_DPFrame, dllDPSource).

## Findings: Docs/当前了解的相关函数内容.md
- Summarizes canonical call chain: GetGlobalInstance (sub_100DB560) -> GetDeviceByLogicID (0x50770) -> GetLinkFromNO (0x117830) -> OnMakeNewLogicData_Slave (0x59F10), then UI refresh.
- Emphasizes the goal of auto-resolving ECX and Parent/Link pointers to avoid manual entry.

## Findings: Docs/规则收敛唯一路径和规划.md
- File is currently empty.

## Findings: HwHack.cpp + HwHackContext.h
- DLL entry starts a console thread that drives the injection workflow; timer proc relays to Runtime for UI-thread-safe actions.
- ContextResolver interface exposes Resolve/SafeResolve for container/parent/link resolution.

## Findings: IDA string scans (Modbus)
- dllDPLogic (ida-pro-mcp / 13337): contains CModbusSlave/CModbusOrder/CModbusChannel/CModbusTCPLink symbols + Assign/Check routines (e.g., AssignSlaveOffset_MODBUS, CheckLKModbus*, CheckTCPOrder4TCPLink).
- dll_DPFrame (ida-pro-mcp-1 / 13338): UI-facing strings include MODBUSTCP_MASTER/SLAVE, Modbus TCP/RTU dialog classes, and display labels ("Modbus Slave", "Modbus Master", etc.).

## Findings: IDA string scans (Ethernet + Source)
- dllDPSource (ida-pro-mcp-2 / 13339): holds Modbus source/config handling, e.g., file paths under HardWare\ModbusTCP/ModbusRTU and CModbusInfo/CSourceModbusSlave routines for reading INI params into maps.
- dll_DPFrame (ida-pro-mcp-1 / 13338): contains Ethernet UI/document classes (CEthernetDoc/Frm/TabView/CfgView) and protocol labels: ETHERNET, ETHERNET1/2, ETHERNET PORT FREE PROTOCOL, HOLLITCP_MASTER, MODBUSTCP_MASTER/SLAVE.
- Frame module exports/contains CHWContainer methods like AddEthernetDev2MC, AddHolliTcpSlaveDevice, DeleteEthernetFree, DeleteHollitcp_master, implying protocol node management under Ethernet.

## Findings: IDA decompile CHWFrameContainer::OnAddProcotol (dll_DPFrame, 0x101A697A)
- Handles adding a protocol under a communication device; input is protocol name string (a2).
- If protocol == "HolliTCP_Master", it rewrites to "MODBUSTCP_MASTER" and sets a BaseDPContainer flag (likely to permit Modbus master under HolliTCP).
- Calls CHWContainer::CheckProcotolMasterSourceInfoExist, CheckNumForProcotol, CheckRedunForProcotol before proceeding.
- Retrieves CommunDeviceFromNO (current comm device) and SourceContainer::GetProcotolIDFormName; then calls CHWDataContainer::OnMakeNewLogicData (global+584) to create logic data.
- Sends a message to ProTreeHwnd(1126) to insert/update tree nodes and updates NameMap via sub_1008E4E0.
- Builds default IO function names based on protocol type (HolliTCP vs Modbus TCP Master/Slave) e.g., Sys_ModbusTCP_Master_Send/Recv or Sys_ModbusTCP_Slave_Send/Recv, and pushes them to UI arrays / main frame via WM messages.

## Findings: IDA decompile CHWFrameContainer::OnAddGateWayProtocol (0x101ACEBA)
- Similar flow to OnAddProcotol but operates on CGateWayDevice; verifies type via RTTI.
- If protocol == "HolliTCP_Master", rewrites to "MODBUSTCP_MASTER" and updates BaseDPContainer flag.
- Checks master source info + protocol count; calls CHWDataContainer::OnMakeNewLogicData (global+584) to create data.
- Sends ProTreeHwnd(1126) message to insert node and updates NameMap via sub_1008E4E0.

## Findings: IDA decompile CModbusTCPView::OnInitialUpdate (0x101C672D)
- Parses selected item text (from DBClick string) to decide master/slave context and sets up Modbus TCP UI dialogs accordingly.
- Uses CModbusSlave::GetThisClass to branch on device type and creates table dialogs for channel/order/diag views; suggests UI behavior depends on protocol type string.

## Findings: IDA decompile attempts
- Decompilation failed for CHWContainer::AddHolliTcpSlaveDevice (0x102B6E00) and AddEthernetDev2MC (0x102B6DD7); will use disassembly or other entry points for Ethernet add flow.

## Findings: IDA follow-up on AddHolliTcpSlaveDevice/AddEthernetDev2MC
- Disasm at 0x102B6E00 / 0x102B6DD7 shows .rdata string table of function names, not code. These are not executable addresses.
- lookup_funcs("AddHolliTcpSlaveDevice"/"AddEthernetDev2MC") returned not found; need alternative navigation (xrefs to string table or search for CreateModbusSlave_TCP/OnAddSlave path).

## Findings: IDA lookup attempts (dllDPLogic)
- lookup_funcs for OnMakeNewLogicData_Slave did not return a named function; list_funcs with limit=5 returned empty.
- Next step: resolve image base via __ImageBase and decompile at base+0x59F10 (known RVA).

## Findings: IDA image base (dllDPLogic)
- idaapi.get_imagebase() returns 0x10000000 for dllDPLogic; OnMakeNewLogicData_Slave expected at 0x10059F10 (base + 0x59F10).

## Findings: IDA decompile CHWDataContainer::OnMakeNewLogicData_Slave (dllDPLogic @0x10059F10)
- Wraps CHWDataContainer::MakeNewData and iterates over count to create one or more device entries; writes new IDs to output array.
- Uses CString type/desc parameters and passes link/parent/context pointers through to MakeNewData; returns 1 on success, 0 on failure.
- Behavior differs when a5 flag is set (appears to create two items per count), suggesting internal handling for paired entries.

## Findings: IDA decompile CHWDataContainer::MakeNewData (dllDPLogic @0x10057490)
- Calls CHWSourceContainer::GetProcotolIDFormName to map protocol name -> protocol ID, then switch-case creates specific device types.
- Case 10 creates CModbusSlave: assigns new ID, sets display/desc, sets link-related indices, sets address from desc string, initializes params from source, and writes IP address parameter.
- Cases 1/14/23 create CDPSlave (DP variants): assign offsets, link insertion, add channels, create vars, set modify flag.
- Cases 13/19 create CGateWayDevice with link binding and set modify flag.
- On success, inserts created object into internal maps and sets ModifyLogic; returns new ID or -1 on failure.

## Findings: dllDPSource GetProcotolIDFormName
- Located CHWSourceContainer::GetProcotolIDFormName at 0x100AA4A0 (size 0x103) in dllDPSource.
- String reference for mangled name at 0x1012957B confirms export/name.

## Findings: IDA decompile CHWSourceContainer::GetProcotolIDFormName (dllDPSource @0x100AA4A0)
- Trims/uppercases protocol name, iterates a SourceContainer list via sub_100A7C70 to compare names and return mapped protocol ID.
- Special case: if ID==18 and GetAppVersion()==2, remaps to 12.
- This is the central name->protocol ID resolver used by CHWDataContainer::MakeNewData and OnAddProcotol.

## Findings: OnMakeNewLogicData lookup (dllDPLogic)
- Found mangled name strings for CHWDataContainer::OnMakeNewLogicData at 0x100EF63F / 0x100EF6B3, but lookup_funcs did not return a named function.
- Next step: use IDA Python to search function names containing "OnMakeNewLogicData" and decompile by address.

## Findings: IDA decompile CHWDataContainer::OnMakeNewLogicData (dllDPLogic @0x1005A720)
- Orchestrates creation based on parameters: calls OnMakeNewLogicData_Control, OnMakeNewLogicData_Procotol (protocol/master), or OnMakeNewLogicData_Slave (slave) depending on which pointers/IDs are provided.
- Requires CPU type and app version checks (AppVersion 2 or 4); trims/uppercases protocol name.
- When adding slaves, forwards to OnMakeNewLogicData_Slave with link/parent/context and desc/address; when adding protocol nodes, uses OnMakeNewLogicData_Procotol.

## Findings: IDA decompile OnMakeNewLogicData_Procotol / OnMakeNewLogicData_Control
- OnMakeNewLogicData_Procotol (0x10059E00) loops over count and calls MakeNewData to create protocol/master nodes; supports a flag that doubles count similar to slave path.
- OnMakeNewLogicData_Control (0x10056A40) creates control-level devices (PLC/LK types), with app-version-specific paths (MakeNewData vs MakeNewData_Control_LK) and special handling for LK234/LK235M.

## Findings: IDA decompile CHWFrameContainer::OnAddSlave (dll_DPFrame @0x101A7AF0)
- UI-level add-slave entry: resolves Link via GetLinkFromNO(commIdx/linkIdx), resolves current comm device, checks source info, and reads Modbus channel info into map as needed.
- Builds default address/desc strings depending on link type; for Modbus TCP (link type 10/7), increments IP/addr suffix and checks duplicates via CHWContainer::FindSameAddrSlave.
- Allocates output ID array and calls CHWDataContainer::OnMakeNewLogicData(global+584) to create slave(s); then uses ID->device mapping to update NameMap and tree via ProTreeHwnd(1126).
- Assembles display names based on device class (Gateway, DP slave, Modbus slave) and triggers UI messages to refresh tree nodes.

## Findings: dllDPSource ReadModChannelInfoToMap
- Located mangled strings for CHWSourceContainer::ReadModChannelInfoToMap at 0x1012E180 and 0x1012E201 in dllDPSource.
- lookup_funcs did not resolve function addresses; will locate via IDA Python search and decompile by address.

## Findings: CHWSourceContainer::ReadModChannelInfoToMap (dllDPSource @0x100AFE40)
- Loads Modbus channel definitions from INI (GetModChannelPath) using GetPrivateProfileInt/String (General/Count, ModN sections, ChildMod/ChildModChNum, ChannelNum, etc.).
- Builds/clears internal ModInfo structures and maps channel entries to uppercase keys; deletes entries on missing/invalid sections.
- This is the source-layer prerequisite invoked by OnAddSlave before creating Modbus TCP slaves.

## Findings: CHWContainer checks (dll_DPFrame)
- CheckProcotolMasterSourceInfoExist @0x101089C0: trims/uppercases protocol name, uses CHWSourceContainer::GetProcotolIDFormName; for Modbus (IDs 10/11) uses GetModbusData; for ID 1 uses LookupGSDNameByDeviceName/GetGSDData; shows error dialog if missing.
- Other related checks located via name scan: CheckSlaveSourceInfoExist @0x10142670, CheckNumForProcotol @0x10129480, CheckRedunForProcotol @0x10118800 (not yet decompiled).

## Findings: Additional CHWContainer checks (dll_DPFrame)
- CheckSlaveSourceInfoExist @0x10142670: resolves protocol ID for slave name; for Modbus (10/11) requires CHWSourceContainer::GetModbusData; for DP (1) requires GSD data; shows error if missing.
- CheckNumForProcotol @0x10129480: looks up protocol limits from source config, then compares current count via GetProcotolNum or GetProcotolNumEx; returns true if adding is allowed.

## Findings: Session Status (resume)
- Working tree under CppTest has pending planning file updates (progress.md, task_plan.md) to commit before code changes.
- Baseline commit 977a6c3 already pushed to origin/master.
