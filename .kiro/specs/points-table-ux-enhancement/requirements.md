# Requirements Document

## Introduction

本规范定义了通讯点位表格（Points Table）的用户体验增强需求。当前系统已经具备基本的点位配置和批量添加功能，但在实际使用中，用户需要更高效的批量编辑能力，包括从任意行开始批量添加、快速选择区域进行批量编辑、以及更简洁的界面布局。

## Glossary

- **Points_Table**: 通讯点位配置表格，用于配置 Modbus 通讯点位的核心 UI 组件
- **Batch_Add**: 批量添加功能，允许用户一次性创建多个点位
- **Template_Fill**: 模板填充，使用规则自动生成点位属性值
- **Selection_Range**: 选择区域，用户在表格中框选的单元格范围
- **Point_Row**: 点位行，表格中的一行数据，代表一个通讯点位
- **HMI_Name**: 变量名称（HMI），点位的业务标识名称
- **Modbus_Address**: Modbus 地址，点位在 Modbus 协议中的寄存器地址
- **Data_Type**: 数据类型，如 Bool、Int16、UInt16、Int32、UInt32、Int64、UInt64、Float32、Float64
- **Byte_Order**: 字节序，32位和64位数据的字节排列顺序（ABCD、BADC、CDAB、DCBA）
- **Scale_Factor**: 缩放倍数，用于数据转换的乘数

## Requirements

### Requirement 1: 从任意行快速批量添加点位

**User Story:** 作为用户，我希望能够从表格的任意行位置快速批量添加点位，这样我可以在现有点位之间插入新的点位组，而不必总是追加到末尾。

#### Acceptance Criteria

1. WHEN 用户在表格中选中一行或多行 THEN THE System SHALL 在工具栏显示"从此处批量添加"按钮
2. WHEN 用户点击"从此处批量添加"按钮 THEN THE System SHALL 打开批量添加对话框，并将插入位置默认设置为选中行之后
3. WHEN 用户未选中任何行时点击批量添加 THEN THE System SHALL 将插入位置默认设置为表格末尾
4. WHEN 批量添加对话框打开 THEN THE System SHALL 自动推断起始地址为选中行的下一个地址
5. WHEN 批量添加对话框打开 THEN THE System SHALL 自动继承选中行的数据类型和字节序设置

### Requirement 2: 模板化批量填充规则

**User Story:** 作为用户，我希望使用模板规则批量生成点位属性，这样我可以快速创建具有规律性命名和地址的点位组。

#### Acceptance Criteria

1. WHEN 用户在批量添加对话框中输入变量名称模板 THEN THE System SHALL 支持 `{{number}}` 占位符生成递增序号
2. WHEN 变量名称模板为 "XXXX_{{number}}" THEN THE System SHALL 生成 "XXXX_1"、"XXXX_2"、"XXXX_3" 等名称
3. WHEN 用户设置 Modbus 地址起始值和步长 THEN THE System SHALL 根据数据类型自动计算地址递增
4. WHEN 数据类型为 UInt16 且起始地址为 40001 THEN THE System SHALL 生成地址序列 40001、40002、40003（步长=1）
5. WHEN 数据类型为 Int32 且起始地址为 40001 THEN THE System SHALL 生成地址序列 40001、40003、40005（步长=2）
6. WHEN 数据类型为 Int64 且起始地址为 40001 THEN THE System SHALL 生成地址序列 40001、40005、40009（步长=4）
7. WHEN 数据类型为 Float64 且起始地址为 40001 THEN THE System SHALL 生成地址序列 40001、40005、40009（步长=4）
6. WHEN 用户选择统一的数据类型 THEN THE System SHALL 将该类型应用到所有新增点位（支持 Bool、Int16、UInt16、Int32、UInt32、Int64、UInt64、Float32、Float64）
7. WHEN 用户选择统一的字节序 THEN THE System SHALL 将该字节序应用到所有新增点位
8. WHEN 用户输入缩放倍数 THEN THE System SHALL 将该倍数应用到所有新增点位

### Requirement 3: 64位数据类型支持

**User Story:** 作为用户，我希望系统支持64位整型和浮点型数据类型，这样我可以处理更大范围的数据和更高精度的浮点数。

#### Acceptance Criteria

1. THE System SHALL 支持 Int64 数据类型（64位有符号整数）
2. THE System SHALL 支持 UInt64 数据类型（64位无符号整数）
3. THE System SHALL 支持 Float64 数据类型（64位双精度浮点数）
4. WHEN 数据类型为 Int64、UInt64 或 Float64 THEN THE System SHALL 自动计算地址步长为 4（占用4个寄存器）
5. WHEN 用户选择64位数据类型 THEN THE System SHALL 在批量添加时正确计算地址递增
6. WHEN 用户选择64位数据类型 THEN THE System SHALL 验证地址范围是否足够容纳该数据类型

### Requirement 4: 快速选择区域进行批量编辑

**User Story:** 作为用户，我希望能够快速选中表格中的一个区域并进行批量编辑，这样我可以高效地修改多个点位的属性。

#### Acceptance Criteria

1. WHEN 用户在表格中框选多行 THEN THE System SHALL 高亮显示选中的行
2. WHEN 用户选中多行后 THEN THE System SHALL 在工具栏显示"批量编辑选中区域"按钮
3. WHEN 用户点击"批量编辑选中区域"按钮 THEN THE System SHALL 打开批量编辑对话框
4. WHEN 批量编辑对话框打开 THEN THE System SHALL 显示数据类型、字节序、缩放倍数的批量设置选项
5. WHEN 用户在批量编辑对话框中选择数据类型 THEN THE System SHALL 将该类型应用到选中区域的所有行
6. WHEN 用户在批量编辑对话框中选择字节序 THEN THE System SHALL 将该字节序应用到选中区域的所有行
7. WHEN 用户在批量编辑对话框中输入缩放倍数 THEN THE System SHALL 将该倍数应用到选中区域的所有行
8. WHEN 用户在批量编辑对话框中确认修改 THEN THE System SHALL 更新选中区域的所有行并标记为已修改

### Requirement 5: 简化界面布局

**User Story:** 作为用户，我希望界面更加简洁，只显示核心功能，这样我可以专注于点位配置工作，不被无关信息干扰。

#### Acceptance Criteria

1. WHEN 用户打开点位配置页面 THEN THE System SHALL 隐藏运行时统计信息（Total、OK、Timeout 等）
2. WHEN 用户打开点位配置页面 THEN THE System SHALL 隐藏运行日志折叠面板
3. WHEN 用户打开点位配置页面 THEN THE System SHALL 隐藏高级工具折叠面板（Fill/Plan/诊断）
4. WHEN 用户打开点位配置页面 THEN THE System SHALL 将运行控制按钮（开始运行、停止）移至独立的"运行"标签页
5. WHEN 用户打开点位配置页面 THEN THE System SHALL 只显示点位表格和核心编辑工具
6. WHEN 用户需要查看运行状态 THEN THE System SHALL 提供独立的"运行"标签页显示实时数据和统计信息

### Requirement 6: 地址自动推断和验证

**User Story:** 作为用户，我希望系统能够智能推断和验证 Modbus 地址，这样我可以减少手动输入错误，提高配置效率。

#### Acceptance Criteria

1. WHEN 用户在批量添加时未指定起始地址 THEN THE System SHALL 自动推断为上一行地址的下一个有效地址
2. WHEN 用户输入的地址超出连接配置的地址范围 THEN THE System SHALL 显示错误提示并阻止保存
3. WHEN 用户输入的地址与数据类型不匹配 THEN THE System SHALL 显示错误提示
4. WHEN 用户批量添加点位时 THEN THE System SHALL 验证所有生成的地址都在有效范围内
5. WHEN 地址验证失败 THEN THE System SHALL 在对话框中显示具体的错误原因和建议

### Requirement 7: 批量编辑预览

**User Story:** 作为用户，我希望在确认批量操作之前能够预览结果，这样我可以确保操作符合预期，避免错误。

#### Acceptance Criteria

1. WHEN 用户在批量添加对话框中修改参数 THEN THE System SHALL 实时更新预览表格
2. WHEN 预览表格显示 THEN THE System SHALL 显示前 10 行的完整信息（变量名称、地址、数据类型、字节序、缩放倍数）
3. WHEN 批量添加参数无效 THEN THE System SHALL 在预览区域显示错误信息
4. WHEN 用户在批量编辑对话框中修改参数 THEN THE System SHALL 显示将要修改的行数和字段数
5. WHEN 用户确认批量操作 THEN THE System SHALL 显示操作成功的提示信息，包括实际修改的行数

### Requirement 8: 键盘快捷操作支持

**User Story:** 作为用户，我希望能够使用键盘快捷键快速执行常用操作，这样我可以提高操作效率，减少鼠标点击。

#### Acceptance Criteria

1. WHEN 用户在表格中按下 Ctrl+B THEN THE System SHALL 打开批量添加对话框
2. WHEN 用户选中多行后按下 Ctrl+E THEN THE System SHALL 打开批量编辑对话框
3. WHEN 用户在对话框中按下 Enter THEN THE System SHALL 确认并执行操作
4. WHEN 用户在对话框中按下 Esc THEN THE System SHALL 取消并关闭对话框
5. WHEN 用户在表格中按下 Delete THEN THE System SHALL 删除选中的行（需要确认）

### Requirement 9: 批量操作撤销支持

**User Story:** 作为用户，我希望能够撤销批量操作，这样我可以在发现错误时快速恢复到之前的状态。

#### Acceptance Criteria

1. WHEN 用户执行批量添加操作 THEN THE System SHALL 记录操作前的状态
2. WHEN 用户执行批量编辑操作 THEN THE System SHALL 记录操作前的状态
3. WHEN 用户点击"撤销"按钮 THEN THE System SHALL 恢复到上一次操作前的状态
4. WHEN 用户执行撤销操作 THEN THE System SHALL 显示撤销成功的提示信息
5. WHEN 没有可撤销的操作时 THEN THE System SHALL 禁用"撤销"按钮
