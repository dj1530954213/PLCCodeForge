# TASK-39-result.md（补齐联合 xlsx 字段到 UnifiedImport.design + 映射一致性测试）

## 1) 完成摘要

- 扩展 `UnifiedImport v1`：在 `points[].design` 中补齐联合表解析出的通讯设计字段（**仅新增可选字段**），并填充 `deviceGroups/hardware`（MVP JSON）。
- 新增缺列可观测：联合表缺少可选列时输出 `warnings(code=MISSING_COLUMN)`，避免静默漏导入/默认值不可解释。
- 增加“映射一致性”单测：锁定 `UnifiedImport -> plc_import_stub` 过程中 `comm/verification` 字段不漂移（防回归）。
- `MergeImportSourcesSummary` 增补可选 `parsedColumnsUsed`（便于现场确认“读到了哪些列”）。

## 2) 改动清单（文件路径 + 关键点）

- `Tauri.CommMapping/src-tauri/src/comm/union_xlsx_parser.rs`
  - 从 `import_union_xlsx` 的 `diagnostics.detectedColumns` 推导 `parsedColumnsUsed/missingOptionalColumns`。
  - 生成 `MISSING_COLUMN` warnings（缺失可选列逐条可观测）。
  - 生成 `deviceGroups`（按 profile 聚合）与 `hardware`（通讯统计 + union 列使用信息）。
- `Tauri.CommMapping/src-tauri/src/comm/merge_unified_import.rs`
  - `UnifiedImportV1PointDesign` 新增可选字段：`protocolType/deviceId/readArea/startAddress/length/pointStartAddress/*tcp/*rtu/timeoutMs/retryCount/pollIntervalMs/...`。
  - 合并时调用 `union_xlsx_parser`：填充 `deviceGroups/hardware`、追加 `MISSING_COLUMN` warnings、返回 `summary.parsedColumnsUsed`。
  - 新增单测：`mapping_consistency_unified_to_plc_stub_keeps_comm_and_verification`。
- `src/comm/api.ts`
  - `MergeImportSourcesSummary` TS 类型新增可选 `parsedColumnsUsed?: string[]`（对齐后端）。
- `Tauri.CommMapping/Docs/Integration/plc_core_import_mapping.v1.md`
  - 追加 `design/deviceGroups/hardware` 字段映射说明（v1 仅增内容）。

## 3) build/test 证据

### 3.1 cargo build

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.54s
```

### 3.2 cargo test（包含映射一致性单测）

```text
running 41 tests
...
test comm::merge_unified_import::tests::mapping_consistency_unified_to_plc_stub_keeps_comm_and_verification ... ok
...
test result: ok. 41 passed; 0 failed
```

## 4) UnifiedImport v1 样例片段（design/deviceGroups/hardware）

（字段均为 v1 冻结；本任务只新增可选字段，不改变既有语义）

```json
{
  "schemaVersion": 1,
  "specVersion": "v1",
  "points": [
    {
      "name": "P1",
      "design": {
        "channelName": "tcp-1",
        "protocolType": "TCP",
        "deviceId": 1,
        "readArea": "Holding",
        "startAddress": 100,
        "addressOffset": 2,
        "pointStartAddress": 102,
        "tcpIp": "192.168.1.10",
        "tcpPort": 502,
        "timeoutMs": 1000,
        "retryCount": 1,
        "pollIntervalMs": 500
      }
    }
  ],
  "deviceGroups": [
    {
      "protocolType": "TCP",
      "channelName": "tcp-1",
      "deviceId": 1,
      "readArea": "Holding",
      "startAddress": 100,
      "length": 120,
      "points": ["P1"]
    }
  ],
  "hardware": {
    "comm": { "profilesCount": 1 },
    "unionXlsx": {
      "parsedColumnsUsed": ["变量名称（HMI）", "数据类型", "字节序", "通道名称", "协议类型", "设备标识"],
      "missingOptionalColumns": ["TCP:IP", "TCP:端口"]
    }
  }
}
```

## 5) 缺列 warnings 示例（MISSING_COLUMN）

```json
{
  "code": "MISSING_COLUMN",
  "message": "union xlsx missing column 'TCP:IP' (values may be defaulted)",
  "pointKey": null,
  "hmiName": null
}
```

## 6) 映射文档新增片段

文件：`Tauri.CommMapping/Docs/Integration/plc_core_import_mapping.v1.md`

```text
UnifiedImport.points[i].design.protocolType? / deviceId? / readArea? / startAddress? / length?
UnifiedImport.deviceGroups[]: { protocolType/channelName/deviceId/readArea/startAddress/length/points/connection }
UnifiedImport.hardware.unionXlsx.parsedColumnsUsed / missingOptionalColumns
```

## 7) summary/manifest 片段（unionXlsxDigest + parsedColumnsUsed）

（示例：`unionXlsxDigest` 为 sha256 前缀格式；`parsedColumnsUsed` 为 v1 规范列名的逐字匹配列表）

```json
{
  "unionXlsxDigest": "sha256:4db8d530b1e60acb7f4e3edecbaa7a5035acf8cfa54027914a50d99c6791f20b",
  "parsedColumnsUsed": ["变量名称（HMI）", "数据类型"]
}
```

## 8) 自检清单（逐条勾选）

- [x] UnifiedImport v1 仅新增可选字段（不改名/不删字段/不改语义）
- [x] 联合表缺列不 panic：通过 `warnings(code=MISSING_COLUMN)` 可观测
- [x] 新增映射一致性单测，锁定 `comm/verification` 不漂移
- [x] `parsedColumnsUsed` 可用于现场确认“哪些列被读取/参与映射”

## 9) 风险与未决项

- `MISSING_COLUMN` 当前按“缺失每个可选列一条 warning”输出，现场文件列很少时 warning 数量可能偏多（但满足“不静默默认”的约束）。
- `deviceGroups/hardware` 当前为 MVP JSON，后续扩展只能新增字段；若需改变语义必须 bump `specVersion=v2`。
