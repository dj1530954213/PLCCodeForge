# TASK-POINTS-FIXES-result.md

## 完成摘要
- 修复批量新增：支持起始 Modbus 地址 + DataType/ByteOrder/Scale 默认值写入每一行，并按 DataType 占用长度自动推进地址。
- 补齐表格填充能力：保留 Fill Down（同值填充），新增 Fill Address（按 DataType 占用自动递增）。
- 修复枚举下拉失效：Select editor 能正确读取列的 `editorOptions` 并写回模型。
- 工程数据持久化：Project `project.v1.json` 升级为“唯一真源”（meta + connections + points + uiState 可选），并新增 `comm_project_load_v1/comm_project_save_v1`；现有 `comm_profiles_* / comm_points_*` 也会同步更新 project 文件。

## 根因分析（问题 3：枚举下拉失效）
- `src/comm/components/revogrid/SelectEditor.vue` 里读取 options 的方式不正确：RevoGrid 传入的 `props.column` 为包装对象，真实列定义在 `props.column.column`，导致 `editorOptions` 读不到（空数组），下拉无可选项/选中不生效。
- 修复：兼容读取 `(props.column as any).column ?? props.column`，从其 `editorOptions` 获取 options。

## 改动清单
### 前端
- `src/comm/services/address.ts`
  - 新增 Modbus 人类地址解析/格式化与跨度计算（`parseHumanAddress/formatHumanAddressFrom0Based/spanForArea/dtypeRegisterSpan/nextAddress`）。
- `src/comm/services/batchAdd.ts`
  - 新增批量新增点位的纯函数 `buildBatchPoints`（地址推进 + 默认值写入 + 越界校验）。
- `src/comm/services/fill.ts`
  - 新增 `computeFillDownEdits/computeFillAddressEdits`（纯算法，UI 负责 apply）。
- `src/comm/pages/Points.vue`
  - 批量新增改为对话框（起始地址/DataType/ByteOrder/Scale/递增或固定模式）。
  - 新增 Fill Address 按钮（确认后按 DataType 占用递增填充地址列）。
  - 地址解析/校验/显示统一改用 `src/comm/services/address.ts`。
- `src/comm/components/revogrid/SelectEditor.vue`
  - 修复 `editorOptions` 读取方式，恢复枚举下拉编辑可用。
- `src/comm/pages/Connection.vue`
  - 打开/切换工程时自动加载 profiles（不再需要手点“加载”才能看到），并补齐 load/save 的错误提示。
- `src/comm/api.ts`
  - 新增 `CommProjectDataV1/CommProjectUiStateV1` 类型与 `commProjectLoadV1/commProjectSaveV1` 调用封装。
- `package.json`, `pnpm-lock.yaml`
  - 新增 `vitest`（仅用于前端纯逻辑测试）。
- `tests/comm/address.test.ts`, `tests/comm/batchAdd.test.ts`
  - 新增最小回归测试：地址推进规则 + 批量新增默认值应用。

### 后端（Rust / Tauri）
- `src-tauri/src/comm/core/model.rs`
  - 新增 `CommProjectDataV1`（meta + connections/points/uiState 可选）与 `CommProjectUiStateV1`。
- `src-tauri/src/comm/adapters/storage/projects.rs`
  - `project.v1.json` 升级为单文件“唯一真源”（创建/复制/软删都保持 connections/points 不丢）。
  - 新增 `load_project_data/save_project_data`。
- `src-tauri/src/comm/tauri_api.rs`
  - 新增 commands：`comm_project_load_v1` / `comm_project_save_v1`（spawn_blocking）。
  - 调整 `comm_profiles_load/save`、`comm_points_load/save`：在 projectId 模式下读写 `project.v1.json`（并同步 legacy split files）。
- `src-tauri/src/lib.rs`
  - 注册新 commands。

## 关键算法说明
### 地址推进（问题 1/2）
- 人类地址解析：`40001` -> `Holding + start0Based=0`（内部 0-based）；`1` -> `Coil + start0Based=0`。
- 占用跨度（Holding/Input）：
  - `Int16/UInt16` -> 1 寄存器
  - `Int32/UInt32/Float32` -> 2 寄存器
- 批量新增（默认递增）：`addr(i) = start + i * span`
  - 例：起始 `40001` + `UInt16` + `3 行` => `40001/40002/40003`
  - 例：起始 `40001` + `Float32` + `3 行` => `40001/40003/40005`
- Fill Address：以选区第一行地址为起点，按“上一行 dataType 占用跨度”递增生成后续行地址。

## 工程持久化说明（问题 4）
- 固定落盘：`AppData/<app-name>/projects/<projectId>/comm/project.v1.json`
- `project.v1.json` 为唯一真源（v1）：顶部 meta 字段与 `CommProjectV1` 保持兼容，并新增：
  - `connections`（ProfilesV1）
  - `points`（PointsV1）
  - `uiState`（可选）
- 新增 commands（最小命令集补齐）：
  - `comm_projects_list` / `comm_project_create`（已存在）
  - `comm_project_load_v1`（新增）
  - `comm_project_save_v1`（新增）
- 兼容策略：
  - 现有 `comm_profiles_* / comm_points_*` 在 projectId 模式下会读写 `project.v1.json`，并同步 `profiles.v1.json/points.v1.json` 作为 legacy split files。

## 验收步骤与证据
### 1) 批量新增地址推进 + 默认值应用
1. 打开 `点位与运行` 页面，选择一个 `Holding` 的连接。
2. 点“批量新增”，输入：
   - 起始地址：`40001`
   - DataType：`Float32`
   - ByteOrder：`DCBA`
   - 行数：`3`
3. 预期：新增 3 行地址为 `40001/40003/40005`，且三行 `byteOrder` 都是 `DCBA`。

### 2) Fill Down + Fill Address
1. 框选多行的 `dataType` 单元格区域 -> 点 `Fill Down` -> 预期全部填充为第一行值。
2. 在选区第一行填写 `40001`，框选多行（至少两行）-> 点 `Fill Address` -> 确认 -> 预期地址按占用长度自动递增。

### 3) 枚举下拉回归
1. 新增一行 -> 点击 `数据类型` 单元格 -> 下拉可打开并选中（如 `Float32`）。
2. 点击 `字节序` -> 可选并写回（如 `DCBA`）。
3. 保存工程 -> 重开工程 -> 值仍正确且下拉仍可用。

### 4) 工程保存与恢复
1. 在连接页配置 TCP/485 profiles（会自动加载已有配置）。
2. 在点位页新增/编辑点位，点击保存。
3. 退出后重新打开该工程：连接与点位应能恢复可见（由 project.v1.json 驱动）。

### 5) 构建/测试输出片段
- `pnpm vitest run`：
  - `tests/comm/address.test.ts (3 tests) ✓`
  - `tests/comm/batchAdd.test.ts (2 tests) ✓`
- `pnpm build`（关键片段）：
  - `vite v6.4.1 building for production...`
  - `✓ built in ...`
- `cargo test --manifest-path src-tauri/Cargo.toml`（关键片段）：
  - `running 39 tests ... ok`
  - `running 2 tests ... ok`
- `cargo tauri dev`（启动片段）：
  - `VITE v6.4.1 ready ...`
  - `Local: http://localhost:61420/`
  - `Running target\\debug\\tauri-app.exe`

## 风险与未决项
- `comm_project_load_v1` 会读写较多内容（包含 connections + points），如果点位规模很大，建议后续在前端用 debounce 保存、并在后端做增量写入/分片。
- 当前 Fill Address 会覆盖选区地址列并二次确认；若后续需要 undo，可评估 RevoGrid 的 undo/redo 插件能力。

