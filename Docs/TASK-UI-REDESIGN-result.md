# TASK-UI-REDESIGN-result.md

## 完成摘要
- 表格组件确认：当前点位表格使用的是 RevoGrid（`@revolist/vue3-datagrid`，MIT），不是 `el-table`。
- 修复“无法编辑/无法选区”：补上 `defineCustomElements(window)`，使 RevoGrid WebComponent 正常注册，编辑/选区/复制粘贴恢复可用。
- 行选择可用：新增左侧 checkbox 列；“删除选中行 / Apply 批量设置”仅作用于勾选行，可现场验收。
- 工具栏强制收敛：编辑工具条仅保留 6 项（批量新增 / 删除选中 / dtype / endian / scale 表达式 / Apply）；Fill/Plan/诊断移入折叠的“高级/工具”。
- 批量新增模板化：支持 `{{i}}`、`{{addr}}` 占位符 + 地址步长自动推导（由 dtype span 决定）+ 右侧预览前 10 行 + 插入到末尾/选中行后。
- 批量缩放表达式：支持 `+ - * /` 与小括号；`{{x}}` 表示旧 scale；纯数字表示直接赋值；解析失败会 toast 报错。
- 模板持久化到工程：新增 `comm_project_ui_state_patch_v1`，将 `activeChannelName` 与 `pointsBatchTemplate` 写入 `project.v1.json` 的 `uiState`（不自动落盘 points，避免误覆盖未保存编辑）。

## 现状定位（B0）
- 表格组件：`src/comm/pages/Points.vue` 使用 `Grid`（`@revolist/vue3-datagrid`）。
- 根因：之前未调用 `@revolist/revogrid/loader` 的 `defineCustomElements()`，导致 `<revo-grid>` 未注册（UI 能显示但交互不可用）。

## 改动清单（路径 + 说明）
- `src/main.ts`：调用 `defineCustomElements(window)`，让 RevoGrid 生效。
- `src/comm/pages/Points.vue`：
  - 增加 checkbox 选择列（`__selected`）+ 选中行删除。
  - 编辑工具栏收敛为 6 项；Fill/Plan/日志移入折叠“高级/工具”。
  - 批量 Apply 改为“只作用于勾选行”，并支持 scale 表达式。
  - 批量新增改为“模板化对话框 + 预览 + 插入位置”。
  - 从 `comm_project_load_v1` 读取 `uiState.pointsBatchTemplate` 以复用模板。
- `src/comm/services/scaleExpr.ts`：新增 scale 表达式编译器（安全的四则/括号/`{{x}}`）。
- `src/comm/services/batchAdd.ts`：新增 `buildBatchPointsTemplate/previewBatchPointsTemplate`（HMI 模板 + scale 模板 + preview）。
- `src/comm/api.ts`：
  - 新增 `CommPointsBatchTemplateV1`、扩展 `CommProjectUiStateV1`。
  - 新增 API：`commProjectUiStatePatchV1`。
- `src-tauri/src/comm/core/model.rs`：新增 `CommPointsBatchTemplateV1`，并扩展 `CommProjectUiStateV1.pointsBatchTemplate`（可选字段）。
- `src-tauri/src/comm/tauri_api.rs`：新增 command：`comm_project_ui_state_patch_v1(projectId, patch)`（只更新 uiState，不改 points/profiles）。
- `src-tauri/src/lib.rs`：注册 `comm_project_ui_state_patch_v1`。

## 交互验收（可复现步骤）
### 1) 行选择可用（验收：选 3 行 → 删除只删这 3 行）
1. 打开工程 → 进入“点位与运行”页。
2. 在表格最左侧 checkbox 勾选 3 行。
3. 点击工具栏 `删除选中行（3）` → 确认 → 仅删除这 3 行。

### 2) 批量缩放表达式（验收：{{x}}*10 / {{x}}/2）
1. 勾选 5 行（初始 `scale=1`）。
2. 在工具栏“缩放表达式”输入：`{{x}}*10` → 点击 `Apply（对选中行）` → 5 行 scale 变为 10。
3. 不取消勾选，输入：`{{x}}/2` → Apply → 5 行 scale 变为 5。
4. 输入非法表达式（例如 `{{y}}*10`）→ toast 提示“缩放表达式错误”，不会静默修改。

### 3) 批量新增模板化（验收：Float32 步长=2；HMI 模板；预览；插入位置）
1. 点击 `批量新增` 打开模板对话框：
   - `行数=3`
   - `起始地址=40001`
   - `数据类型=Float32`（step=2）
   - `HMI 模板=test{{i}}`
2. 右侧预览应为：
   - 地址：40001 / 40003 / 40005
   - HMI：test1 / test2 / test3
3. 选择插入位置：
   - “追加到末尾”：插入到表格末尾
   - “插入到选中行之后”：先勾选一行，再生成，会插入到该行之后
4. 点击 `生成并插入`：插入后行可继续编辑/可勾选/可删除。

### 4) 模板持久化（工程级）
1. 执行一次“批量新增”并生成（会写入 `project.v1.json.uiState.pointsBatchTemplate`）。
2. 关闭应用/切换工程后再打开该工程。
3. 再次点击 `批量新增`：模板字段会从工程 `uiState` 恢复（复用上次模板）。

## Grid 选型说明（B4，简版对比）
- RevoGrid（当前使用，MIT）：原生支持选区/复制粘贴/大数据量；本次仅补齐 customElements 注册与 UI 工具链即可落地。
- vxe-table（MIT）：Excel 操作友好，但需要整套组件生态与样式集成，迁移成本更高。
- Tabulator（MIT）：轻量、易用，但“选区 + 复制粘贴 + 填充”等要额外验证/补齐，且与现有 spreadsheet 交互模型不如 RevoGrid 原生。

## 构建证据（pnpm build）
```text
> pnpm build
> vue-tsc --noEmit && vite build
vite v6.4.1 building for production...
✓ built in 5.37s
```

