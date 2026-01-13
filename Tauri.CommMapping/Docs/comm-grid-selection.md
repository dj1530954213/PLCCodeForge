# 通讯采集模块：点位编辑数据网格选型（comm-grid-selection）

## 候选方案对比（Vue 3）

### 1) RevoGrid（`@revolist/revogrid` + `@revolist/vue3-datagrid`）
- License：MIT（Revolist OU）。
- 定位：Spreadsheet/Excel-like 数据网格（支持百万级单元格、键盘焦点、Range 操作）。
- 关键能力（与本需求直接相关）：
  - 内置 Range 选择/Range 编辑（含 autofill 思路）。
  - Copy/Paste：官方声明支持从 Excel/Google Sheets 粘贴与复制。
  - 键盘导航：Excel-like focus。
  - 可自定义 editor/cell renderer（Vue3 wrapper 提供 `VGridVueEditor/VGridVueTemplate`）。
- 主要成本/风险：
  - 组件本体为 Web Component（Stencil），需要适配事件与 Vue 响应式更新方式。
  - 我们需要把现有点位 DTO（`PointsV1/CommPoint`）映射为 grid row model（含 UI-only 字段）。

### 2) Tabulator（`tabulator-tables`）
- License：MIT（Oli Folkerd）。
- 定位：Data table + 部分 spreadsheet 能力（模块化）。
- 关键能力（官方文档）：
  - Clipboard module：支持 copy/paste；支持自定义解析器与 paste action（insert/update/replace/range）。
  - 可编辑单元格、键盘操作等。
- 主要成本/风险：
  - Vue3 集成通常需要 wrapper/手写 mount（更偏 DOM 驱动），与我们现有组件体系融合成本较高。
  - Range 选择/填充体验需要更精细的配置与事件编排。

### 3) vxe-table（当前已集成）
- License：MIT（Xu Liangzhan）。
- 定位：企业级表格（Vue 生态），功能面很广。
- 优点：
  - Vue3 集成最顺滑，我们已在项目中使用并跑通构建。
  - 编辑、校验、虚拟滚动等成熟。
- 风险点（对本任务关键）：
  - 官方 README 中将“单元格区域选取 / 单元格复制粘贴 / 全键盘操作”等列为“企业版插件”能力，存在 license/实现边界不清晰风险；本任务要求避免商业 license 争议。

## 最终选择
选择 **RevoGrid（Vue3 wrapper）** 作为点位编辑网格。

理由（按优先级）：
1) 需求是“Excel 级编辑体验”，RevoGrid 原生就是 spreadsheet 方向（Range、Copy/Paste、Keyboard focus）。
2) MIT license，且关键能力在其开源核心中明确提供，降低“企业版插件”争议。
3) 我们的通讯点位数据量在现场可能较大，RevoGrid 的虚拟化与大表格能力更贴合长期需求。

## 需求功能映射表（计划）
| 需求功能 | RevoGrid | 落地方式（本仓库） |
|---|---|---|
| 单元格编辑 | ✅ | 使用内置 editor；对枚举列用 select editor；必要时用 Vue 自定义 editor。 |
| 键盘导航 Tab/Enter | ✅ | 使用 grid 内置 keyboard focus；补充快捷键说明。 |
| 复制/粘贴（TSV） | ✅ | 启用 Copy/Paste；必要时对“Modbus 地址列”做自定义 parser（把 40001 转内部 offset）。 |
| 多选区域 | ✅ | 使用 Range selection；用于 fill down / paste 范围。 |
| 向下填充（fill down） | ✅/可扩展 | 先实现“同值填充”；如内置 autofill 不足则用 range 事件自定义。 |
| 快速新增 N 行 | ✅ | 在 UI 工具条提供 N；批量 append rows（带默认值）。 |
| 行内校验提示 | ✅/可扩展 | UI 侧即时校验（地址/len/dtype），并在 cell 渲染层标红/tooltip；运行前后端再校验。 |
| 运行值回填到每行 | ✅ | 按 `pointKey` 合并最新结果到 row runtime 字段（不落盘）。 |

