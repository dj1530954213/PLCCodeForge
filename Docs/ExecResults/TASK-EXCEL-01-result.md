# TASK-EXCEL-01-result：点位编辑页升级为 Excel-like（vxe-table）

## 1) 完成摘要
- 点位配置页从 ElementPlus Table（弹窗编辑）升级为 `vxe-table` 网格编辑：双击编辑单元格、支持鼠标框选区域、支持 Ctrl+C/Ctrl+V（单元格区域复制粘贴）。
- 增加“向下填充（按选区首行）”与“对勾选行批量设置”（DataType/Endian/channelName/scale）以满足现场快速录入。
- 必填项缺失高亮（HMI 名称/通道名称/scale 非法），保存时 fail-fast 提示并自动聚焦到首个错误单元格。
- 点位 DTO 与后端保存格式保持不变（仍为 `points.v1.json`，`schemaVersion: 1`）。

## 2) 改动清单（文件路径 + 关键点）
- `src/comm/pages/Points.vue`
  - 替换为 `vxe-table` 单元格编辑。
  - 启用 `mouse-config.area + keyboard-config.isClip` 支持区域选择与复制粘贴。
  - 新增：`向下填充（按选区首行）`、`删除选中行`、`对勾选行批量设置`。
  - 新增：必填高亮与保存校验（聚焦到错误单元格）。
- `package.json`
  - 依赖：`vxe-table`、`xe-utils`（用于 Excel-like grid 能力）。
- `src/main.ts`
  - 全局注册 `vxe-table` 并引入样式（`vxe-table/lib/style.css`）。

## 3) 验收证据
### 3.1 采用的表格库与 License
- 方案：A（`vxe-table`）
- License：MIT（见 `node_modules/.pnpm/vxe-table@*/node_modules/vxe-table/LICENSE`）

### 3.2 `pnpm build` 输出片段
```text
vite v6.4.1 building for production...
✓ 1769 modules transformed.
✓ built in 4.56s
```

### 3.3 现场操作步骤与预期 UI 结果（录屏/截图要点）
1. 工程工作区 → “点位配置”
2. 点击 `加载 Demo（mock）`（或 `新增行` 多次）
3. 在表格中：
   - 用鼠标框选一块单元格区域，按 `Ctrl+C` 再到其它区域按 `Ctrl+V`：应完成区域粘贴
   - 双击单元格进入编辑，修改 `变量名称（HMI）/通道名称/数据类型/字节序/缩放倍数`
4. 批量设置：
   - 勾选多行 → 设置 DataType/Endian/（可选）channelName/scale → 点击 `对勾选行批量设置`：应批量更新
5. 向下填充：
   - 框选一个包含至少两行的区域（例如选中 `channelName` 列的多行）→ 点击 `向下填充（按选区首行）`：下方行应被首行同列值覆盖
6. 校验高亮：
   - 清空某行 `变量名称（HMI）` 或 `通道名称` → 该单元格背景应高亮
   - 点击 `保存`：应提示错误并聚焦到首个错误单元格（不落盘）
7. 保存一致性：
   - 填好必填项后点击 `保存`，再点击 `加载`：数据应保持一致

## 4) 风险与未决项
- 复制粘贴的兼容性：当前依赖 vxe-table 的区域剪贴板能力（更偏“表格内复制粘贴”）；若现场需要“从 Excel 外部粘贴 TSV/CSV 到表格”，建议后续补一个 paste 文本解析适配（不影响现有实现）。
- 表格块体积：引入 vxe-table 会增加前端 bundle 体积（目前已有 chunk warning，但不影响功能）。

## 5) 回滚点说明（如何恢复到旧结构）
- 直接回滚 `src/comm/pages/Points.vue` 到 ElementPlus Table 版本，并移除 `vxe-table/xe-utils` 依赖与 `src/main.ts` 的 `VXETable` 注册即可。

