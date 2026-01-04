# plc_core Import 映射说明（v1）

> 目标：把 `UnifiedImport v1`（联合表 × 通讯采集核对结果）中的字段，映射成一个**接近 plc_core ImportResult 语义**的可消费结构。  
> 当前阶段不接入 `plc_core` crate，本仓库通过 `comm_unified_export_plc_import_stub_v1` 产出 `plc_import_stub.v1`，作为后续三模块合并的接口适配中间层。

---

## 1) 输入/输出产物（冻结 v1）

### 1.1 输入：UnifiedImport v1

- 生成方式：`comm_merge_import_sources_v1(unionXlsxPath, importResultStubPath)`
- 文件：`outputDir/unified_import/unified_import.v1.<ts>.json`
- 主键：`points[].name`（HMI 变量名称；union xlsx 内必须唯一）

### 1.2 输出：plc_import_stub v1

- 生成方式：`comm_unified_export_plc_import_stub_v1(unifiedImportPath)`
- 文件：`outputDir/plc_import_stub/plc_import.v1.<ts>.json`
- 点位顺序：**按 UnifiedImport.points 原始顺序输出（确定性）**

---

## 2) points 字段映射（UnifiedImport → ImportResult 语义）

> 说明：此处的 “ImportResult” 为 plc_core 侧最终统一数据源的语义目标；当前落盘结构以 `plc_import_stub.v1` 为准。

| UnifiedImport v1 | plc_import_stub v1（≈ ImportResult 语义） | 说明 |
|---|---|---|
| `points[i].name` | `points[i].name` | 点位名称（HMI 变量名称），合并主键 |
| `points[i].design` | `points[i].design` | 来自联合表（工程设计源）的字段快照（MVP 最小集） |
| `points[i].comm.channelName` | `points[i].comm.channelName` | 通讯通道名（用于分组/设备对齐） |
| `points[i].comm.addressSpec` | `points[i].comm.addressSpec` | 地址语义：内部统一 0-based（readArea/absoluteAddress/length 等） |
| `points[i].comm.dataType` | `points[i].comm.dataType` | Bool/Int16/UInt16/Int32/UInt32/Float32 |
| `points[i].comm.endian` | `points[i].comm.endian` | ABCD/BADC/CDAB/DCBA |
| `points[i].comm.scale` | `points[i].comm.scale` | 缩放倍数（导入/展示/回填） |
| `points[i].comm.rw` | `points[i].comm.rw` | 读写属性（MVP 采集为 RO） |
| `points[i].verification.*` | `points[i].verification.*` | 现场核对结果（quality/valueDisplay/timestamp/message） |

---

## 3) 模板/ctx 取值约定（冻结字段名）

未来 plc_core 模板侧需要用到的通讯字段，建议以如下固定路径读取（避免后续改名造成兼容问题）：

- `points[i].comm.channelName`
- `points[i].comm.addressSpec.readArea`
- `points[i].comm.addressSpec.absoluteAddress`
- `points[i].comm.addressSpec.unitLength`
- `points[i].comm.dataType`
- `points[i].comm.endian`
- `points[i].comm.scale`
- `points[i].verification.quality`
- `points[i].verification.valueDisplay`

---

## 4) deviceGroups/hardware 映射占位策略

- `UnifiedImport.deviceGroups` → `plc_import_stub.deviceGroups`（原样透传，当前可为空数组）
- `UnifiedImport.hardware` → `plc_import_stub.hardware`（原样透传，当前可为空对象）

> 后续当联合表解析侧补齐“设备/硬件分组”字段时，应优先 **新增可选字段**，若需要改变语义则 bump `specVersion=v2`。

---

### 4.1 design（工程设计源）字段补齐（v1 仅增可选字段）

`UnifiedImport.points[i].design` 在 TASK-39 起补齐了更多“通讯设计语义”字段，均为 **可选**（不会破坏既有 v1 JSON）：

- `protocolType?` / `deviceId?` / `readArea?`：来自对应 `ConnectionProfile`（按 `channelName` 匹配）。
- `startAddress?` / `length?`：profile 的 base start/len（内部 0-based）。
- `addressOffset?`：点位相对 profile 的偏移（内部 0-based）。
- `pointStartAddress?`：推导值 `startAddress + addressOffset`（内部 0-based），用于快速核对点位绝对地址。
- `inputChannelName?`：原始输入通道名（用于追溯导入前的 channelName；当因 deviceId 去重导致自动拼接后缀时仍可追溯）。
- `tcpIp?` / `tcpPort?`：TCP profile 参数快照（用于交付回溯/脱敏快照）。
- `rtuSerialPort?` / `rtuBaudRate?` / `rtuParity?` / `rtuDataBits?` / `rtuStopBits?`：485 profile 参数快照。
- `timeoutMs?` / `retryCount?` / `pollIntervalMs?`：profile 的运行参数快照。

> 注意：`plc_import_stub.v1` 当前 **原样透传** `design`（不做语义解释），供未来 plc_core 真正接入时作为 ctx/模板字段来源。

### 4.2 deviceGroups（按 profile 聚合，MVP JSON）

`UnifiedImport.deviceGroups[]` 目前由通讯侧按 `ProfilesV1` 聚合生成（每个 profile 一条），结构为可扩展 JSON（后续只加字段）：

- `protocolType` / `channelName` / `deviceId`
- `readArea` / `startAddress` / `length`
- `points`：该通道下的点位名列表（HMI name）
- `connection`：`tcp` 或 `rtu485` 子对象 + `timeoutMs/retryCount/pollIntervalMs`

### 4.3 hardware（解析/通讯统计，MVP JSON）

`UnifiedImport.hardware` 当前包含：

- `hardware.comm.profilesCount` / `hardware.comm.profilesByProtocol`
- `hardware.unionXlsx.parsedColumnsUsed` / `hardware.unionXlsx.missingOptionalColumns`（用于现场确认“哪些列被解析/哪些可选列缺失”）

## 5) v1 演进策略（必须遵守）

- v1：只允许新增可选字段，不得改名/删字段/改语义。
- `readArea` 从 Holding/Coil 扩展到 Input/Discrete 时，**必须 bump specVersion**（避免语义静默变化）。
