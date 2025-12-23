# UIA 自动操作模块（多组态软件适配器）整体设计思路

> 本文描述我们开发的“UIA 自动化引擎 + 目标组态软件适配器”架构，用于操作 AUTOTHINK、Logix5000、TIA Portal（博图）等。  
> 核心目标：**可扩展（新增软件低成本）**、**可回放可定位（现场可用）**、**稳定执行（可重试、可等待、可降级）**。

---

## 1. 业务目标与非目标

### 1.1 业务目标
- 能自动完成常见工程动作：
  - 添加硬件信息
  - 配置通讯模板/驱动参数
  - 导入变量表
  - 创建子程序
  - 导入程序
  - 编译
- 能把 PLC 生成模块交付物（文本/剪贴板/文件）“送入”目标组态软件
- 能输出**步骤日志 + 错误上下文**，现场出错可快速定位

### 1.2 非目标（阶段性不做）
- 不追求“完全无人值守”一次跑通所有客户现场（先做可控闭环）
- 不把 PLC 品牌差异混进 UIA 自动化（UIA 只关心目标软件 UI）
- 不在核心层写死任何 UI 结构（所有 UI 差异都放适配器）

---

## 2. 总体架构：稳定 core + 可插拔 adapters

### 2.1 分层
- **uia_core（稳定核心）**
  - UIA 连接与会话管理（进程/窗口/上下文）
  - 元素定位 DSL（按 AutomationId/Name/ControlType/Pattern 等）
  - 原子动作执行器（Click/DoubleClick/RightClick/Input/Paste）
  - 等待与重试策略（WaitUntil、Timeout、Backoff）
  - 统一错误分类（Find/Action/Timeout/UnexpectedUIState）
  - 统一日志与追踪（StepLog、截图/控件树片段可选）
- **uia_adapters（目标软件适配器，变化隔离）**
  - `uia-autothink`
  - `uia-logix5000`
  - `uia-tiaportal`
  - 每个适配器实现：该软件的 UI 结构、菜单路径、导入向导步骤、编译入口等
- **apps（组合根）**
  - 选择目标软件适配器
  - 注入策略参数（超时、重试次数、语言/主题、路径）
  - 触发执行并展示结果

### 2.2 核心工程规则
- core 内禁止 `if software == ...`
- 新增目标软件 = 新增一个 adapter + 少量组合根注册
- 原子动作在 core 统一实现；流程动作在 adapter 中组织

---

## 3. API 设计：原子动作 + 流程动作（两级抽象）

### 3.1 原子动作（core 提供）
建议形成统一 RPC/接口（无论 C# 进程内调用还是 stdin/stdout JSON-RPC）：

- `FindElement(selector, scope) -> ElementRef`
- `RightClick(elementRef|rect)`
- `Click(elementRef)`
- `DoubleClick(elementRef)`
- `SetText(elementRef, text, mode)`  
  - mode: Replace/Append/CtrlAReplace
- `SendKeys(keys)`  
  - 支持 `CTRL+V`、`ENTER`、组合键
- `WaitUntil(predicate, timeout)`
- `Screenshot(scope)`（可选：失败时自动采集）

> ElementRef 必须是“可失效”的：UI 刷新后自动重新定位或给出明确错误（Stale Element）。

### 3.2 流程动作（adapter 提供）
每个目标软件适配器对外暴露同一组“业务动作接口”（便于上层编排）：

- `AddHardware(hardwareSpec)`
- `ConfigureCommTemplate(commSpec)`
- `ImportVariables(xlsPath)`
- `CreateSubProgram(name, options)`
- `ImportProgram(source)`（文本粘贴/文件导入/剪贴板粘贴）
- `Build()`（编译）

这些动作内部由多个原子动作组成，并对每一步输出 StepLog。

---

## 4. 稳定性设计：等待、重试、幂等、回放

### 4.1 等待策略（必做）
UI 自动化失败常见原因：窗口没激活、控件还没渲染、导入在后台跑。

- 所有流程步骤必须是：
  - `Action` + `WaitForExpectedState`
- Wait 的 predicate 以“可观测状态”为准：
  - 对话框出现/消失
  - 某按钮 enabled
  - 某表格行数变化
  - 某状态栏文本包含关键字

### 4.2 重试策略（必做）
- Find 失败：短周期重试（例如 200ms * N）
- Click 后无变化：按 UI 状态判断是否需要重试或升级为错误
- “一次性风险动作”（导入/编译）：
  - 不盲目重试，需要先识别当前状态（是否已开始、是否在运行中）

### 4.3 幂等语义（强建议）
流程动作尽量设计为可重复执行：
- 创建子程序：若已存在，则跳过或校验一致性
- 导入变量：若已导入，则提示“已存在/覆盖策略”
- 硬件配置：按硬件标识查重

---

## 5. 观测与诊断：现场可用的“证据链”

### 5.1 StepLog（必须）
每个步骤记录：
- stepId、动作名、关键参数摘要（不要泄露敏感路径可脱敏）
- 目标元素 selector（用于复现）
- 开始/结束时间、耗时
- 结果：Success/Warning/Fail
- Fail 时附带：
  - 错误分类（Find/Timeout/UnexpectedState）
  - （可选）截图、控件树片段、当前前台窗口标题

### 5.2 统一错误分类（必须）
用于 UI/验收口径对齐：
- `ConfigError`（路径/进程/版本不匹配）
- `FindError`（控件找不到）
- `TimeoutError`（等待超时）
- `ActionError`（点击/输入失败）
- `UnexpectedUIState`（出现未知弹窗/模式不一致）

---

## 6. 与其它模块的接口（数据交接）

### 6.1 消费 PLC Generator 的交付物
UIA 自动化不理解 PLC 品牌，只消费“交付清单”：
- `.xls`：用于 ImportVariables
- 剪贴板 binary：确保已写入剪贴板后执行粘贴流程
- Memory/Text：粘贴到编辑器或导入窗口

### 6.2 消费联合 xlsx 的硬件信息（可选）
从联合 xlsx 解析出的硬件信息可直接作为 `AddHardware(hardwareSpec)` 的输入。

---

## 7. MVP 实施路径（建议）
1) uia_core：原子动作 + 等待/重试 + StepLog + 错误分类  
2) 先做 `uia-autothink`：跑通“硬件配置 + 子程序 + 导入变量 + 导入程序 + 编译”闭环  
3) 抽象公共流程骨架（wizard、菜单、弹窗处理）沉入 core 工具库  
4) 再扩展 `uia-logix5000`、`uia-tiaportal`

---