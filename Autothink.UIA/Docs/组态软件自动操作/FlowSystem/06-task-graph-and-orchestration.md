# 任务图与编排策略

## 任务图模型
- 每个 Task 具备 requires/produces。
- Orchestrator 通过 DAG 构建可执行顺序。
- 用户可选起点，但必须满足依赖关系。

## 任务状态定义（建议）
- Pending：未准备，依赖未满足。
- Ready：依赖满足，可执行。
- Running：执行中。
- Succeeded：成功完成。
- Failed：失败终止。
- Skipped：按策略跳过。
- Blocked：依赖失败导致阻塞。

## Task 元数据（建议）
- priority：调度优先级（数字越高优先）。
- estDuration：预估耗时（用于提示用户）。
- tags：任务分类（硬件/变量/程序/通讯）。
 - resumePolicy：失败后是否允许重试/继续。

## 依赖类型
- hard dependency：必须先完成，否则阻断执行。
- soft dependency：缺失时提示，但允许继续。

## 依赖校验规则
- hard 依赖失败 → 当前任务 Blocked。
- soft 依赖失败 → 记录 Warning，可继续执行。

## 用户可选顺序策略
- 用户选择要执行的 Task 集合与起点。
- 系统自动补齐依赖任务或提示缺失。
- 支持“只执行子图”与“全量执行”。

## 执行计划生成流程
1) 过滤用户选择的 Task 集合。
2) 补齐 hard 依赖集合。
3) 校验是否存在循环依赖。
4) 拓扑排序生成执行序列。
5) 输出 Dry-Run 计划（可选）。

## 执行计划生成
- 拓扑排序生成执行序列。
- 若依赖不完整，进入“缺失依赖”状态。
- 支持 Dry-Run 输出计划而不执行。

## 任务计划示例（Dry-Run 输出）
```
Plan:
  1) import_variables
  2) add_program_blocks
  3) hardware_config
  4) comm_program
```

## 任务前置/后置校验
- 任务开始前可执行 precheck（例如检查硬件树是否存在）。
- 任务结束后执行 postcheck（例如确认导入对话框关闭）。
- precheck 失败可按 policy 处理（跳过/终止/补救）。

## 状态恢复策略（建议）
- 若检测到弹窗，优先关闭弹窗并回到主窗口。
- 若焦点丢失，执行 focus_main_window 操作。

## 状态机策略（建议）
- UIState: MainWindow / ImportDialog / ProgramEditor / BuildDialog / Unknown
- 状态变化触发规则化处理（弹窗关闭/焦点切换）。

## 失败处理策略
- 任务失败后可选择：终止全部 / 仅终止子图 / 继续后续。
- 对硬件组态类任务建议默认“终止全部”。

## 失败处理策略
- 每个 Task 支持 policy：stop / warn / retry / fallback。
- 失败可选择是否继续后续任务。

## 可恢复执行
- 记录已完成任务与 StepLog。
- 支持从失败节点继续执行（按策略决定是否跳过）。

## 断点续跑规则
- 仅允许从 Task 边界续跑，不支持 Step 级别断点。
- 续跑时必须校验 Profile 与 Flow 版本一致。

## 并发与互斥
- UIA 单线程，默认串行执行。
- 允许“逻辑并发”仅限于前置检查阶段。

## 可观测输出
- Task 开始/结束必须写日志。
- 执行计划必须可导出（json/yaml）。

## 示例（任务图）
```yaml
tasks:
  - id: import_variables
    produces: [vars_imported]
  - id: hardware_config
    requires: [vars_imported]
    produces: [hardware_ready]
  - id: add_program_blocks
    requires: [vars_imported]
    produces: [programs_ready]
  - id: comm_program
    requires: [hardware_ready, programs_ready]
    produces: [comm_ready]
```

## 可观测输出
- 执行计划输出到日志与 evidence。
- 每个任务输出状态摘要与版本信息。
