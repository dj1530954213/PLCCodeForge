# 原子动作与控制流参考

## 约定字段
- action：动作名称。
- args：动作参数。
- policy：失败策略（可选）。
- stepId：可选，未提供则由运行时生成。

## 通用参数规范
- selector：Profile 中定义的逻辑选择器 key。
- anchor：Profile 中定义的锚点 key。
- pos：Profile 中定义的坐标 key。
- timeoutMs：等待超时（默认 5000ms）。
- intervalMs：节流间隔（默认 80ms）。

## 通用返回字段（ActionResult）
- ok：是否成功
- warn：是否警告
- error：错误信息（失败时）

## 动作分类
- 原子动作：直接触发 UIA 行为。
- 控制流动作：if/for_each/try/retry 等。
- 领域宏动作：由模板展开（不属于核心动作）。

## 原子动作

### ensure_selector
- 作用：验证元素存在。
- args：selector（Profile 逻辑选择器名）。
- 成功：元素找到。
- 失败：FindError。
 - 注意：不改变 UI 状态，适合 precheck。

### click_at
- 作用：在锚点坐标系点击。
- args：anchor（Profile 锚点名）、pos（Profile 坐标名）。
- 成功：点击成功。
- 失败：ActionError/ConfigError。
 - 前置：anchor 解析成功。

### click_rel
- 作用：在锚点矩形内按比例点击。
- args：anchor、xRatio、yRatio。
- 成功：点击成功。
- 失败：ActionError/ConfigError。
 - 约束：xRatio/yRatio 范围 0-1。

### right_click_at
- 作用：右键点击。
- args：anchor、pos。
- 成功：点击成功。
- 失败：ActionError/ConfigError。

### key_nav
- 作用：方向键导航序列。
- args：seq（Profile navSequence 名），intervalMs（可选）。
- 成功：导航序列完成。
- 失败：ActionError。
 - 建议：对关键步骤加 verify。

### set_text
- 作用：向控件输入文本。
- args：selector、text、mode（Replace/Append/CtrlAReplace）。
- 成功：输入完成。
- 失败：ActionError。
 - 备注：优先 ValuePattern，失败回退键盘输入。

### send_keys
- 作用：发送组合键或文本。
- args：keys（如 CTRL+V/ENTER）。
- 成功：按键完成。
- 失败：InvalidArgument/ActionError。
 - 建议：发送前确保目标控件已 Focus。

### wait_until
- 作用：等待条件满足。
- args：kind（ElementExists/ElementNotExists/ElementEnabled）、selector、timeoutMs。
- 成功：条件满足。
- 失败：TimeoutError。
 - 备注：用于等待弹窗关闭/元素出现。

### assert
- 作用：断言条件满足。
- args：kind、selector。
- 成功：条件满足。
- 失败：FindError/UnexpectedUIState。
 - 备注：失败即终止当前 Step。

### delay
- 作用：显式等待。
- args：ms。
- 成功：延时结束。
 - 备注：用于稳定 UI。

## 控制流动作

### if
- args：condition, then, else。
- condition：表达式结果为 true/false。
 - 备注：then/else 为 Step 列表。

### for_each
- args：items, do。
- items：数组或集合。
 - 备注：每次循环绑定 @item。

### try
- args：do, fallback。
- 作用：执行失败后进入 fallback。
 - 备注：do/fallback 为 Step 列表。

### retry
- args：times, intervalMs, step。
 - 备注：仅对单一步骤重试，推荐用于 click/ensure。

### use_template
- args：template, params（可选）。
- 作用：将模板展开为步骤。
 - 备注：展开后继续执行。

## 领域宏动作（建议）
> 宏动作应通过模板展开实现，避免在核心动作层固化业务逻辑。

- menu_import_xls
- select_node
- add_device
- ensure_protocol
- add_module
- configure_module
- create_program
- paste_program

## StepLog 记录建议
- 参数摘要：必须包含 selector/anchor/pos/key。
- 失败记录：必须包含 RpcError.Kind 与 Message。
 - 可选记录：窗口标题、UIState、截图引用。

## 参考
- Action 参数校验与错误分类必须与 `03-core-capabilities.md` 对齐。
