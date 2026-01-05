# Implementation Plan: Points Table UX Enhancement

## Overview

本实现计划遵循SOLID原则和优良的软件架构设计思想，将点位表格用户体验增强功能分解为可执行的任务。每个任务都注重单一职责、开放封闭、依赖倒置等设计原则，确保代码的可维护性和可扩展性。

## SOLID Principles Application

### Single Responsibility Principle (SRP)
- 每个服务类只负责一个功能领域（如地址计算、模板渲染、批量编辑）
- 组件职责明确分离（配置组件、运行组件、对话框组件）

### Open/Closed Principle (OCP)
- 使用接口和抽象类定义扩展点
- 新增数据类型不需要修改现有代码，只需扩展类型定义

### Liskov Substitution Principle (LSP)
- 所有数据类型实现统一的接口
- 撤销操作使用统一的UndoableAction接口

### Interface Segregation Principle (ISP)
- 定义细粒度的接口，避免臃肿的接口
- 组件只依赖它们需要的接口

### Dependency Inversion Principle (DIP)
- 高层模块（组件）依赖抽象接口，不依赖具体实现
- 使用依赖注入传递服务实例

## Tasks

- [x] 1. 设置项目基础和类型定义
  - 创建新的服务模块目录结构
  - 定义核心接口和类型
  - 遵循SRP：每个类型定义文件只负责一个领域
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 1.1 扩展数据类型定义（前端）
  - 在 `src/comm/api.ts` 中添加 Int64、UInt64、Float64 类型
  - 更新 DATA_TYPES 常量数组
  - 确保类型定义的完整性和一致性
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 1.2 创建数据类型工具服务
  - 创建 `src/comm/services/dataTypes.ts`
  - 实现 `getDataTypeInfo()` 函数
  - 实现 `getRegisterSpan()` 函数
  - 实现 `isValidForArea()` 函数
  - 遵循SRP：只负责数据类型相关的工具函数
  - _Requirements: 3.4, 3.5, 3.6_

- [ ]* 1.3 编写数据类型工具函数的单元测试
  - 测试所有数据类型的步长计算
  - 测试64位类型返回步长4
  - 测试类型与区域的兼容性判断
  - _Requirements: 3.4_

- [x] 2. 扩展地址计算服务
  - 扩展 `src/comm/services/address.ts`
  - 遵循OCP：扩展而不修改现有函数
  - _Requirements: 2.3, 6.1, 6.2_

- [x] 2.1 更新 spanForArea 函数支持64位类型
  - 添加 Int64、UInt64、Float64 的步长计算（返回4）
  - 保持向后兼容性
  - _Requirements: 3.4, 3.5_

- [x] 2.2 实现智能地址推断函数
  - 实现 `inferNextAddress()` 函数
  - 根据上一行的地址和数据类型计算下一个地址
  - 处理边界情况（无上一行、地址为空等）
  - _Requirements: 1.4, 6.1_

- [x] 2.3 实现地址范围验证函数
  - 实现 `validateAddressRange()` 函数
  - 验证批量生成的地址是否在连接配置范围内
  - 提供详细的错误信息
  - _Requirements: 3.6, 6.2, 6.4_

- [ ]* 2.4 编写地址计算函数的属性测试
  - **Property 2: 地址递增计算的正确性**
  - **Validates: Requirements 2.3, 2.4, 2.5, 2.6, 2.7**
  - 生成随机起始地址、数据类型和数量
  - 验证地址序列正确递增且无重复

- [ ]* 2.5 编写64位类型步长的属性测试
  - **Property 3: 64位数据类型的步长计算**
  - **Validates: Requirements 3.4**
  - 生成随机64位数据类型
  - 验证步长始终为4

- [ ]* 2.6 编写地址推断的属性测试
  - **Property 13: 地址自动推断的正确性**
  - **Validates: Requirements 1.4, 6.1**
  - 生成随机上一行数据
  - 验证推断的地址等于上一行地址加步长

- [ ] 3. Checkpoint - 确保基础服务测试通过
  - 确保所有测试通过，询问用户是否有问题


- [x] 4. 实现批量编辑服务
  - 创建 `src/comm/services/batchEdit.ts`
  - 遵循SRP：只负责批量编辑逻辑
  - 遵循DIP：依赖抽象的数据接口
  - _Requirements: 4.5, 4.6, 4.7_

- [x] 4.1 定义批量编辑接口
  - 定义 `BatchEditRequest` 接口
  - 定义 `BatchEditResult` 接口
  - 定义 `BatchEditPreview` 接口
  - _Requirements: 4.4_

- [x] 4.2 实现批量编辑计算函数
  - 实现 `computeBatchEdits()` 函数
  - 计算需要修改的字段和行
  - 支持可选的数据类型、字节序、缩放倍数修改
  - _Requirements: 4.5, 4.6, 4.7_

- [x] 4.3 实现批量编辑应用函数
  - 实现 `applyBatchEdits()` 函数
  - 将计算的编辑应用到点位数据
  - 确保原子性：要么全部成功，要么全部失败
  - _Requirements: 4.8_

- [ ]* 4.4 编写批量编辑的属性测试
  - **Property 9: 批量编辑的数据类型应用**
  - **Validates: Requirements 4.5**
  - **Property 10: 批量编辑的字节序应用**
  - **Validates: Requirements 4.6**
  - **Property 11: 批量编辑的缩放倍数应用**
  - **Validates: Requirements 4.7**
  - 生成随机选中行和目标值
  - 验证所有行正确更新

- [ ]* 4.5 编写批量操作原子性测试
  - **Property 15: 批量操作的原子性**
  - **Validates: Requirements 4.8**
  - 模拟部分失败场景
  - 验证状态保持不变或全部应用

- [-] 5. 实现撤销/重做管理器
  - 创建 `src/comm/services/undoRedo.ts`
  - 遵循SRP：只负责历史记录管理
  - 遵循LSP：所有操作实现统一的UndoableAction接口
  - _Requirements: 9.1, 9.2, 9.3_

- [x] 5.1 定义撤销操作接口
  - 定义 `UndoableAction` 接口
  - 定义 `UndoHistoryEntry` 接口
  - 确保接口的通用性和可扩展性
  - _Requirements: 9.1, 9.2_

- [x] 5.2 实现 UndoManager 类
  - 实现历史记录栈管理
  - 实现 `push()`, `undo()`, `redo()` 方法
  - 实现 `canUndo()`, `canRedo()` 方法
  - 限制历史记录大小为20条
  - _Requirements: 9.3, 9.5_

- [-] 5.3 实现状态快照工具
  - 实现 `createSnapshot()` 函数（深拷贝）
  - 实现 `createBatchAddUndoAction()` 工厂函数
  - 实现 `createBatchEditUndoAction()` 工厂函数
  - _Requirements: 9.1, 9.2_

- [ ]* 5.4 编写撤销恢复的属性测试
  - **Property 12: 撤销操作的状态恢复**
  - **Validates: Requirements 9.3**
  - 生成随机操作序列
  - 验证撤销后状态与操作前一致

- [ ]* 5.5 编写撤销管理器的单元测试
  - 测试历史记录大小限制
  - 测试 canUndo/canRedo 状态
  - 测试边界情况（空历史、满历史）
  - _Requirements: 9.5_

- [ ] 6. 扩展批量添加服务
  - 扩展 `src/comm/services/batchAdd.ts`
  - 遵循OCP：扩展现有功能而不修改
  - _Requirements: 2.1, 2.2, 2.8, 2.9, 2.10_

- [ ] 6.1 增强模板渲染函数
  - 支持 `{{number}}` 占位符（从1开始）
  - 支持 `{{addr}}` 占位符（当前地址）
  - 添加模板语法验证
  - _Requirements: 2.1, 2.2_

- [ ] 6.2 增强批量添加验证
  - 集成地址范围验证
  - 集成数据类型兼容性验证
  - 提供详细的错误信息和建议
  - _Requirements: 6.2, 6.3, 6.4, 6.5_

- [ ]* 6.3 编写模板生成的属性测试
  - **Property 1: 模板变量名称生成的一致性**
  - **Validates: Requirements 2.1, 2.2**
  - 生成随机模板和数量
  - 验证生成的名称符合规则且唯一

- [ ]* 6.4 编写批量添加一致性的属性测试
  - **Property 4: 批量添加的数据类型一致性**
  - **Validates: Requirements 2.8**
  - **Property 5: 批量添加的字节序一致性**
  - **Validates: Requirements 2.9**
  - **Property 6: 批量添加的缩放倍数一致性**
  - **Validates: Requirements 2.10**
  - 生成随机批量参数
  - 验证所有点位属性一致

- [ ]* 6.5 编写地址验证的属性测试
  - **Property 7: 地址范围验证的正确性**
  - **Validates: Requirements 3.6, 6.2, 6.4**
  - **Property 8: 地址与数据类型的匹配性**
  - **Validates: Requirements 6.3**
  - 生成随机越界和不匹配场景
  - 验证正确检测错误

- [ ] 7. Checkpoint - 确保核心服务测试通过
  - 确保所有测试通过，询问用户是否有问题


- [ ] 8. 实现批量编辑对话框组件
  - 创建 `src/comm/components/BatchEditDialog.vue`
  - 遵循SRP：只负责批量编辑UI
  - 遵循DIP：依赖批量编辑服务接口
  - _Requirements: 4.2, 4.3, 4.4_

- [ ] 8.1 创建对话框基础结构
  - 实现对话框布局和样式
  - 添加数据类型、字节序、缩放倍数输入字段
  - 添加预览区域
  - _Requirements: 4.4_

- [ ] 8.2 实现实时预览功能
  - 监听输入变化
  - 调用 `computeBatchEdits()` 计算预览
  - 显示将要修改的行数和字段数
  - _Requirements: 7.4_

- [ ] 8.3 实现确认和取消逻辑
  - 确认时触发批量编辑
  - 创建撤销操作并推入历史
  - 显示成功提示信息
  - _Requirements: 4.8, 7.5_

- [ ] 8.4 添加键盘快捷键支持
  - Enter键确认
  - Esc键取消
  - _Requirements: 8.3, 8.4_

- [ ] 9. 重构批量添加对话框组件
  - 重构 `src/comm/pages/Points.vue` 中的批量添加对话框
  - 提取为独立组件（可选）
  - 遵循SRP和DIP原则
  - _Requirements: 1.1, 1.2, 1.4, 1.5_

- [ ] 9.1 实现智能默认值
  - 自动推断起始地址（调用 `inferNextAddress()`）
  - 自动继承上一行的数据类型
  - 自动继承上一行的字节序
  - 自动继承上一行的缩放倍数
  - _Requirements: 1.4, 1.5_

- [ ] 9.2 增强实时预览
  - 使用防抖优化性能（50ms）
  - 显示前10行完整信息
  - 显示错误信息（如果有）
  - _Requirements: 7.1, 7.2, 7.3_

- [ ] 9.3 集成撤销功能
  - 确认时创建撤销操作
  - 推入撤销历史
  - _Requirements: 9.1_

- [ ]* 9.4 编写预览一致性的属性测试
  - **Property 14: 预览数据的一致性**
  - **Validates: Requirements 7.1, 7.2**
  - 生成随机批量参数
  - 验证预览与实际结果一致

- [ ] 10. 实现键盘快捷键系统
  - 创建 `src/comm/composables/useKeyboardShortcuts.ts`
  - 遵循SRP：只负责键盘事件处理
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 10.1 定义快捷键接口
  - 定义 `KeyboardShortcut` 接口
  - 定义快捷键配置
  - _Requirements: 8.1, 8.2_

- [ ] 10.2 实现快捷键注册和注销
  - 实现 `useKeyboardShortcuts()` composable
  - 处理键盘事件
  - 防止默认行为
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 10.3 集成到Points组件
  - 注册 Ctrl+B（批量添加）
  - 注册 Ctrl+E（批量编辑）
  - 注册 Delete（删除选中行）
  - 注册 Ctrl+Z（撤销）
  - 注册 Ctrl+Shift+Z / Ctrl+Y（重做）
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 11. 重构Points页面（配置页面）
  - 重构 `src/comm/pages/Points.vue`
  - 遵循SRP：只负责点位配置
  - _Requirements: 5.1, 5.2, 5.3, 5.5_

- [ ] 11.1 移除运行相关功能
  - 移除运行控制按钮（开始、停止、重启）
  - 移除运行状态显示
  - 移除运行统计信息
  - 移除运行日志折叠面板
  - 移除高级工具折叠面板
  - _Requirements: 5.1, 5.2, 5.3_

- [ ] 11.2 添加批量编辑按钮
  - 添加"批量编辑"按钮到工具栏
  - 根据选中行数量启用/禁用
  - 点击时打开批量编辑对话框
  - _Requirements: 4.2, 4.3_

- [ ] 11.3 添加撤销/重做按钮
  - 添加"撤销"和"重做"按钮到工具栏
  - 根据历史状态启用/禁用
  - 显示撤销/重做的操作描述（tooltip）
  - _Requirements: 9.3, 9.4, 9.5_

- [ ] 11.4 集成批量编辑对话框
  - 引入 BatchEditDialog 组件
  - 处理确认事件
  - 更新表格数据
  - _Requirements: 4.3, 4.8_

- [ ] 11.5 集成撤销管理器
  - 创建 UndoManager 实例
  - 在批量操作时推入历史
  - 实现撤销/重做处理函数
  - _Requirements: 9.1, 9.2, 9.3_

- [ ] 11.6 集成键盘快捷键
  - 使用 useKeyboardShortcuts composable
  - 注册所有快捷键
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 12. 创建PointsRun页面（运行页面）
  - 创建 `src/comm/pages/PointsRun.vue`
  - 遵循SRP：只负责点位运行和监控
  - _Requirements: 5.4, 5.6_

- [ ] 12.1 实现基础布局
  - 连接选择（只读显示）
  - 运行控制按钮区域
  - 实时数据表格区域
  - 统计信息区域
  - 日志区域
  - _Requirements: 5.6_

- [ ] 12.2 迁移运行控制功能
  - 从 Points.vue 迁移开始/停止/重启按钮
  - 迁移运行状态显示
  - 迁移轮询间隔设置
  - _Requirements: 5.4_

- [ ] 12.3 迁移实时数据显示
  - 迁移实时数据表格
  - 迁移运行统计信息
  - 迁移运行日志
  - _Requirements: 5.6_

- [ ] 12.4 迁移诊断工具
  - 迁移 Plan 生成工具
  - 迁移 Fill 工具
  - _Requirements: 5.6_

- [ ] 13. 更新路由配置
  - 更新 `src/router/index.ts`
  - 添加 PointsRun 路由
  - _Requirements: 5.4, 5.6_

- [ ] 13.1 添加运行页面路由
  - 添加 `/projects/:projectId/comm/run` 路由
  - 配置 PointsRun 组件
  - _Requirements: 5.4_

- [ ] 13.2 更新导航
  - 在 ProjectWorkspace 中添加"运行"标签
  - 更新标签切换逻辑
  - _Requirements: 5.6_

- [ ] 14. Checkpoint - 确保UI组件测试通过
  - 确保所有测试通过，询问用户是否有问题


- [ ] 15. 后端支持：扩展数据类型（Rust）
  - 更新 `src-tauri/src/comm/model.rs`
  - 遵循OCP：扩展而不修改现有代码
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 15.1 扩展 DataType 枚举
  - 添加 Int64、UInt64、Float64 变体
  - 更新序列化/反序列化
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 15.2 更新 register_span 方法
  - 为64位类型返回步长4
  - 保持向后兼容性
  - _Requirements: 3.4_

- [ ] 15.3 更新编解码器
  - 更新 `src-tauri/src/comm/codec.rs`
  - 添加64位类型的编解码逻辑
  - _Requirements: 3.1, 3.2, 3.3_

- [ ]* 15.4 编写后端数据类型的单元测试
  - 测试所有数据类型的步长计算
  - 测试64位类型的编解码
  - 测试序列化/反序列化
  - _Requirements: 3.4_

- [ ] 16. 后端支持：地址验证
  - 更新 `src-tauri/src/comm/plan.rs`
  - 添加地址范围验证逻辑
  - _Requirements: 6.2, 6.4_

- [ ] 16.1 实现地址范围验证
  - 在 plan 构建时验证地址范围
  - 返回详细的验证错误
  - _Requirements: 6.2, 6.4_

- [ ] 16.2 更新错误类型
  - 添加地址验证相关的错误类型
  - 提供详细的错误信息
  - _Requirements: 6.5_

- [ ]* 16.3 编写后端地址验证的单元测试
  - 测试正常范围内的地址
  - 测试越界地址
  - 测试边界情况
  - _Requirements: 6.2, 6.4_

- [ ] 17. 性能优化
  - 优化批量操作性能
  - 优化表格渲染性能
  - _Requirements: 所有_

- [ ] 17.1 实现防抖优化
  - 为实时预览添加防抖（50ms）
  - 为地址验证添加防抖
  - _Requirements: 7.1_

- [ ] 17.2 优化状态快照
  - 使用结构化克隆API（如果可用）
  - 优化大数据量的深拷贝性能
  - _Requirements: 9.1, 9.2_

- [ ] 17.3 优化表格更新
  - 使用增量更新而不是全量刷新
  - 只更新受影响的行
  - _Requirements: 所有_

- [ ] 18. 文档和示例
  - 编写用户文档
  - 编写开发者文档
  - _Requirements: 所有_

- [ ] 18.1 编写用户指南
  - 批量添加功能使用说明
  - 批量编辑功能使用说明
  - 键盘快捷键列表
  - 撤销/重做功能说明
  - _Requirements: 所有_

- [ ] 18.2 编写开发者文档
  - 架构设计文档
  - API接口文档
  - SOLID原则应用说明
  - 扩展指南
  - _Requirements: 所有_

- [ ] 18.3 创建示例项目
  - 创建示例配置
  - 演示所有新功能
  - _Requirements: 所有_

- [ ] 19. 最终测试和验收
  - 执行完整的测试套件
  - 进行用户验收测试
  - _Requirements: 所有_

- [ ] 19.1 执行所有单元测试
  - 确保所有单元测试通过
  - 代码覆盖率 > 80%
  - _Requirements: 所有_

- [ ] 19.2 执行所有属性测试
  - 确保所有属性测试通过（100次迭代）
  - 验证所有正确性属性
  - _Requirements: 所有_

- [ ] 19.3 执行集成测试
  - 测试完整的用户流程
  - 测试组件间交互
  - _Requirements: 所有_

- [ ] 19.4 性能测试
  - 批量添加500行 < 500ms
  - 批量编辑100行 < 200ms
  - 撤销操作 < 100ms
  - 表格渲染1000行 < 1s
  - _Requirements: 所有_

- [ ] 19.5 用户验收测试
  - 邀请用户测试新功能
  - 收集反馈
  - 修复发现的问题
  - _Requirements: 所有_

- [ ] 20. Final Checkpoint - 完成验收
  - 确保所有功能正常工作
  - 确保所有测试通过
  - 确保性能指标达标
  - 询问用户是否满意

## Notes

### SOLID Principles Checklist

每个任务完成时，请检查是否遵循了以下原则：

- [ ] **Single Responsibility**: 每个类/函数只有一个职责
- [ ] **Open/Closed**: 对扩展开放，对修改封闭
- [ ] **Liskov Substitution**: 子类可以替换父类
- [ ] **Interface Segregation**: 接口细粒度，不臃肿
- [ ] **Dependency Inversion**: 依赖抽象而不是具体实现

### Code Quality Standards

- 所有函数都有类型注解
- 所有公共API都有JSDoc注释
- 所有复杂逻辑都有注释说明
- 遵循项目的代码风格指南
- 通过ESLint和Prettier检查

### Testing Standards

- 单元测试覆盖率 > 80%
- 属性测试最少100次迭代
- 所有边界情况都有测试
- 所有错误路径都有测试
- 测试代码清晰易懂

### Performance Standards

- 批量添加500行 < 500ms
- 批量编辑100行 < 200ms
- 撤销操作 < 100ms
- 预览更新 < 50ms（防抖后）
- 表格渲染1000行 < 1s

### Documentation Standards

- 所有公共API都有文档
- 所有复杂算法都有说明
- 提供使用示例
- 提供架构图和流程图
- 保持文档与代码同步
