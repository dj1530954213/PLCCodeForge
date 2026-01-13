# TASK-23-result.md

- **Task 编号与标题**：
  - TASK-23：pointKey 复用键升级：从 hmiName -> (hmiName + channelName + deviceId) + 迁移策略（不破坏旧数据）

- **完成摘要**：
  - 升级 `ImportUnion -> CommPoint` 映射器的 `pointKey` 复用策略：优先按 `(hmiName + channelName + deviceId)`，deviceId 缺失时降级为 `(hmiName + channelName)`，最后才允许 `hmiName`（且仅在旧数据无冲突时）。
  - 对旧的 `points.v1.json` 保持兼容：构建 `keyV2/keyV1` 索引；若旧数据同一 `hmiName` 对应多个 `pointKey`（冲突），则禁止 `hmiName` 回退并产出可观测 warning。
  - warnings 结构化包含：`code/message/hmiName/channelName/deviceId?`，ImportUnion 页继续展示 import + mapper 合并 warnings。

- **改动清单（文件路径 + 关键点）**：
  - Frontend
    - `src/comm/api.ts`
      - 扩展 `CommWarning`：新增可选字段 `channelName?: string`、`deviceId?: number`（向后兼容）。
    - `src/comm/mappers/unionToCommPoints.ts`
      - 新增复用键：`keyV2 = hmi|channel|deviceId`、`keyV2NoDevice = hmi|channel`、`keyV1 = hmi`
      - 旧数据索引：`existingPointKeyByKeyV2` / `existingPointKeyByKeyV2NoDevice` / `existingPointKeyByKeyV1`
      - 冲突检测：`keyV2NoDevice` 冲突、`keyV1` 冲突时禁止回退并输出 warning
      - deviceId 解析：优先从 `channelName` 后缀 `@<id>`，其次从 profiles（同 channelName 且唯一）
    - `src/comm/pages/ImportUnion.vue`
      - 调用 mapper 时同时传入 `importedProfiles` + `existingProfiles`，用于 deviceId 解析与 keyV2 复用。

- **完成证据（build/test）**：
  - `pnpm build`：
    ```text
    > vue-tsc --noEmit && vite build
    ✓ built in 3.50s
    ```

- **映射示例（3 组）**：
  - A：同名不同 channelName（应分别复用各自 pointKey）
    ```json
    {
      "existing": {
        "schemaVersion": 1,
        "points": [
          { "pointKey": "PK1", "hmiName": "温度", "channelName": "tcp-1@1", "dataType": "Int16", "byteOrder": "ABCD", "scale": 1 },
          { "pointKey": "PK2", "hmiName": "温度", "channelName": "tcp-2@1", "dataType": "Int16", "byteOrder": "ABCD", "scale": 1 }
        ]
      },
      "imported": {
        "schemaVersion": 1,
        "points": [
          { "pointKey": "ignored", "hmiName": "温度", "channelName": "tcp-1@1", "dataType": "Int16", "byteOrder": "ABCD", "scale": 1 },
          { "pointKey": "ignored", "hmiName": "温度", "channelName": "tcp-2@1", "dataType": "Int16", "byteOrder": "ABCD", "scale": 1 }
        ]
      },
      "out": {
        "reusedPointKeys": 2,
        "createdPointKeys": 0,
        "points": [
          { "hmiName": "温度", "channelName": "tcp-1@1", "pointKey": "PK1" },
          { "hmiName": "温度", "channelName": "tcp-2@1", "pointKey": "PK2" }
        ]
      }
    }
    ```
  - B：旧数据仅单条 hmiName（允许 keyV1 fallback 复用）
    ```json
    {
      "existing": {
        "schemaVersion": 1,
        "points": [
          { "pointKey": "PK_OLD", "hmiName": "压力", "channelName": "tcp-1@1", "dataType": "Float32", "byteOrder": "ABCD", "scale": 1 }
        ]
      },
      "imported": {
        "schemaVersion": 1,
        "points": [
          { "pointKey": "ignored", "hmiName": "压力", "channelName": "tcp-2@1", "dataType": "Float32", "byteOrder": "ABCD", "scale": 1 }
        ]
      },
      "out": {
        "reusedPointKeys": 1,
        "createdPointKeys": 0,
        "warnings": [
          { "code": "POINTKEY_REUSE_FALLBACK_HMI_ONLY", "hmiName": "压力", "channelName": "tcp-2@1" }
        ],
        "points": [
          { "hmiName": "压力", "channelName": "tcp-2@1", "pointKey": "PK_OLD" }
        ]
      }
    }
    ```
  - C：旧数据同一 hmiName 多条（冲突）→ 禁止 keyV1 fallback，生成新 pointKey 并 warning
    ```json
    {
      "existing": {
        "schemaVersion": 1,
        "points": [
          { "pointKey": "PK_A", "hmiName": "流量", "channelName": "tcp-1@1", "dataType": "UInt16", "byteOrder": "ABCD", "scale": 1 },
          { "pointKey": "PK_B", "hmiName": "流量", "channelName": "tcp-2@1", "dataType": "UInt16", "byteOrder": "ABCD", "scale": 1 }
        ]
      },
      "imported": {
        "schemaVersion": 1,
        "points": [
          { "pointKey": "ignored", "hmiName": "流量", "channelName": "tcp-3@1", "dataType": "UInt16", "byteOrder": "ABCD", "scale": 1 }
        ]
      },
      "out": {
        "reusedPointKeys": 0,
        "createdPointKeys": 1,
        "warnings": [
          { "code": "EXISTING_KEYV1_CONFLICT_NO_FALLBACK", "hmiName": "流量", "channelName": "tcp-3@1" }
        ]
      }
    }
    ```

- **验收自检**：
  - [x] pointKey 复用优先级：`(hmi+channel+deviceId)` → `(hmi+channel)` → `hmi`（仅无冲突时）
  - [x] 旧数据兼容：不修改既有 `points.v1.json` 结构；仅升级导入复用逻辑
  - [x] 冲突可观测：检测到 `hmiName` 冲突时禁用 v1 fallback 并返回结构化 warning
  - [x] 不破坏既有后端 DTO 契约：改动仅在前端 mapper/UI

- **风险与未决项**：
  - 旧数据如果存在“同 channelName 多 deviceId”但未按 `@deviceId` 区分，且 profiles 也无法唯一反推 deviceId 时，只能降级到 `(hmi+channel)` 或生成新 pointKey，并通过 warnings 暴露不确定性。

