# 核心能力与原子动作规格

## L0 能力要求
- UIA 会话：OpenSession/CloseSession，STA 单线程执行。
- 元素定位：ElementSelector 支持路径/索引/大小写/空白归一。
- 选择器回退：主窗口失败后回退 Desktop。
- 输入基础：鼠标/键盘/剪贴板。

## L0 细化能力清单
- Session 生命周期：一次 Flow 对应一个 Session，异常时需强制清理。
- 窗口识别：支持 Name/AutomationId/ClassName/ProcessName 组合匹配。
- 焦点控制：SetFocus + 验证 FocusedElement。
- 屏幕信息：分辨率/DPI/缩放比获取（用于 Profile 转换）。

## Action 执行契约
- 参数必须校验，失败返回 InvalidArgument。
- 动作必须写 StepLog（参数摘要、耗时、结果）。
- 失败不抛异常给上层，统一 RpcError。
- 动作必须是幂等或可安全重试。

## 动作生命周期
1) 参数解析与校验。
2) Profile 解析（selector/pos/anchor）。
3) UIA 执行。
4) 结果判断（Success/Warn/Fail）。
5) StepLog 记录与错误归类。

## Action 分类
- 导航类：click_at/click_rel/right_click_at/key_nav。
- 输入类：set_text/send_keys。
- 等待类：wait_until/delay。
- 校验类：ensure_selector/assert。
- 复用类：use_template。

## UIA 模式支持（建议）
- InvokePattern：按钮/菜单。
- ValuePattern：文本输入。
- SelectionPattern：列表/下拉。
- LegacyIAccessible：回退。

## 坐标与锚点
- 坐标来源必须来自 Profile，不允许硬编码。
- 锚点解析顺序：selector -> window -> fallback。
- MFC 自绘区域：以左上角 (0,0) 为坐标原点。
- DPI 处理：Profile 记录 dpiScale，运行时进行换算。

## 自绘区域点击策略
- 优先使用固定坐标点（point）。
- 当 UI 可能小范围漂移时使用 rect 中心或 click_rel。
- 对关键点击建议加入 assert/ensure_selector。

## 输入与焦点
- SetText 优先 ValuePattern，失败时 fallback 键盘输入。
- SendKeys 前尝试 Focus 目标控件。
- KeyNav 支持可配置节流，避免 UI 处理过载。

## 粘贴策略（Ctrl+V）
- 使用剪贴板前先保存原剪贴板内容（可选）。
- 粘贴后等待输入稳定（delay 或 wait_until）。
- 粘贴失败回退为 set_text。

## 幂等性要求
- ensure_selector/assert 不改变 UI 状态。
- click_at/key_nav 在失败后应可重试。
- set_text 建议使用 Replace 模式避免重复输入。

## 动作结果与策略
- 成功：记录 outcome=Success。
- 警告：继续执行但记录 outcome=Warning。
- 失败：触发 policy（retry/fallback/stop）。

## 可观测性要求
- 每个 Action 必须包含 StepId 与参数摘要。
- 失败时必须记录 selector/pos/anchor 信息。
- 支持记录执行前后窗口标题与焦点信息。

## 关键错误场景
- selector 匹配到多个元素：按优先级选第一个并记录警告。
- 目标控件不可用：返回 UnexpectedUIState。
- 目标控件被遮挡：返回 ActionError 并记录坐标。

## 详细参考
- Action 参数与失败模式详见 `12-action-reference.md`。
