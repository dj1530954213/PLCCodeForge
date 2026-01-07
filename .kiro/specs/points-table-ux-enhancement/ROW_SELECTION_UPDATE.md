# 行选择方式更新

## 更改摘要

根据用户反馈，将点位表格的行选择方式从"复选框列"改为"点击行头选择"，提供更直观的交互体验。

## 具体更改

### 1. 移除复选框列

**文件**: `src/comm/pages/Points.vue`

- 移除了 `__selected` 列的复选框实现
- 删除了独立的选择列（原来占用 44px 宽度）

### 2. 添加行头点击事件

**新增功能**:
- 点击行头（行号）可以切换该行的选中状态
- 支持 Ctrl/Cmd + 点击进行多选
- 不按 Ctrl/Cmd 时，点击会清除其他行的选中状态（单选模式）

**实现细节**:
```typescript
function onRowHeaderClick(e: any) {
  const rowIndex = e?.detail?.index;
  const row = gridRows.value[rowIndex];
  
  // 支持 Ctrl/Cmd 多选
  const isMultiSelect = e?.detail?.originalEvent?.ctrlKey || 
                        e?.detail?.originalEvent?.metaKey;
  
  if (!isMultiSelect) {
    // 单选模式：清除其他行的选中状态
    gridRows.value.forEach((r, idx) => {
      if (idx !== rowIndex) {
        r.__selected = false;
      }
    });
  }
  
  // 切换当前行的选中状态
  row.__selected = !row.__selected;
  
  // 触发响应式更新
  gridRows.value = [...gridRows.value];
}
```

### 3. 添加行样式函数

**新增函数**: `getRowClass()`
- 根据行的选中状态动态返回 CSS 类名
- 选中的行会应用 `row-selected` 类

```typescript
function getRowClass(row: any): string {
  const pointRow = row?.model as PointRow | undefined;
  return pointRow?.__selected ? 'row-selected' : '';
}
```

### 4. 修复选中状态保持问题

**问题**: 当 `rebuildGridRows()` 被调用时（例如批量操作后），所有行的选中状态会被重置。

**解决方案**: 修改 `makeRowFromPoint()` 函数，保留现有行的选中状态：

```typescript
function makeRowFromPoint(p: CommPoint): PointRow {
  // ... 其他代码 ...
  
  // 保留现有的选中状态
  const existingRow = gridRows.value.find(r => r.pointKey === p.pointKey);
  const isSelected = existingRow?.__selected ?? false;
  
  return {
    ...p,
    __selected: isSelected,  // 使用保留的状态而不是总是 false
    // ... 其他字段 ...
  };
}
```

### 5. 增强视觉反馈

**新增 CSS 样式**:

1. **行头可点击样式**:
   - 鼠标悬停时显示浅蓝色背景
   - 添加过渡动画效果
   - 设置 `cursor: pointer` 提示可点击

2. **选中行样式**:
   - 选中行的行头：深蓝色背景 + 加粗字体 + 主题色文字
   - 选中行的单元格：浅蓝色背景
   - 使用 `!important` 确保样式优先级

```css
/* 行头可点击 */
:deep(.rgHeaderCell[data-type="rowHeaders"]) {
  cursor: pointer;
  user-select: none;
  transition: background-color 0.2s;
}

:deep(.rgHeaderCell[data-type="rowHeaders"]:hover) {
  background-color: rgba(64, 158, 255, 0.15);
}

/* 选中行 */
:deep(.row-selected) {
  background-color: rgba(64, 158, 255, 0.08) !important;
}

:deep(.row-selected .rgHeaderCell[data-type="rowHeaders"]) {
  background-color: rgba(64, 158, 255, 0.25) !important;
  font-weight: 600;
  color: var(--el-color-primary);
}
```

## 用户体验改进

### 优点

1. **更直观**: 点击行号选择行是表格应用的标准交互模式
2. **节省空间**: 移除了复选框列，为数据列提供更多空间
3. **支持多选**: Ctrl/Cmd + 点击支持多行选择
4. **视觉清晰**: 选中的行有明显的视觉反馈
5. **操作流畅**: 添加了过渡动画，交互更流畅
6. **状态保持**: 批量操作后选中状态不会丢失

### 交互说明

- **单击行头**: 选中该行，取消其他行的选中状态
- **Ctrl/Cmd + 单击行头**: 切换该行的选中状态，保持其他行的状态
- **再次点击已选中的行头**: 取消该行的选中状态

## 修复的问题

### 问题 1: 批量操作后功能失效

**原因**: `makeRowFromPoint()` 函数总是将 `__selected` 设置为 `false`，导致 `rebuildGridRows()` 调用时重置所有选中状态。

**解决**: 在创建新行对象时，从现有行中查找并保留选中状态。

### 问题 2: 响应式更新不触发

**原因**: 直接修改数组元素的属性不会触发 Vue 的响应式更新。

**解决**: 使用 `gridRows.value = [...gridRows.value]` 创建新数组引用，触发响应式更新。

## 兼容性

- ✅ 所有现有的批量操作功能（批量编辑、批量删除等）正常工作
- ✅ `__selected` 属性仍然存在于数据模型中，只是选择方式改变了
- ✅ 键盘快捷键（Ctrl+E、Delete、Ctrl+Z、Ctrl+Y 等）继续基于 `__selected` 属性工作
- ✅ 批量操作后选中状态保持不变
- ✅ 撤销/重做功能正常工作

## 测试建议

1. ✅ 测试单击行头选择单行
2. ✅ 测试 Ctrl/Cmd + 点击多选
3. ✅ 测试选中后执行批量编辑
4. ✅ 测试选中后执行删除操作
5. ✅ 测试批量操作后选中状态是否保持
6. ✅ 测试撤销/重做功能
7. ✅ 测试视觉反馈（悬停、选中状态）
8. ✅ 测试键盘快捷键（Ctrl+B, Ctrl+E, Delete, Ctrl+Z, Ctrl+Y）

## 相关文件

- `src/comm/pages/Points.vue` - 主要修改文件
  - 移除复选框列定义
  - 添加 `onRowHeaderClick` 事件处理
  - 添加 `getRowClass` 样式函数
  - 修复 `makeRowFromPoint` 保留选中状态
  - 添加行头和选中行的 CSS 样式
