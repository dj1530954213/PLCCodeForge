# 错误处理与可观测性

## 错误分类（与现有 RpcError 对齐）
- ConfigError：配置/版本/环境不匹配。
- FindError：元素定位失败。
- TimeoutError：等待条件超时。
- ActionError：动作执行失败。
- UnexpectedUIState：出现未知弹窗或异常 UI。
- StaleElement：元素引用失效。
- InvalidArgument：参数不合法。
- NotImplemented：流程/动作未实现。

## 错误字段结构（建议）
- kind：错误类型
- message：错误消息
- detail：可选，包含 selector/anchor/pos
- recoverable：是否可恢复

## StepLog 规范
- StepId：稳定标识，可用于回放与检索。
- Action：动作名称。
- Parameters：摘要参数（避免敏感信息）。
- Selector：可选，便于定位。
- Outcome：Success/Warning/Fail。
- DurationMs：耗时。
- Error：失败时必须写明 Kind 与 Message。

## StepLog 补充字段
- UiState：执行前 UI 状态。
- WindowTitle：当前窗口标题（可选）。
- AnchorResolved：锚点解析结果（可选）。
- ScreenshotId：截图引用（可选）。

## Evidence Pack 结构
- step_logs.json：完整 StepLog。
- summary.json：流程摘要与失败原因。
- profile_version.txt：Profile 版本记录。
- flow_version.txt：Flow 版本记录。
- runtime_context.json：运行环境信息（可选）。

## 运行环境记录建议
- OS 版本、DPI、分辨率、应用版本。
- Profile/Template/Flow 版本。
- 执行时间区间与耗时统计。

## 失败策略
- stop：立即终止。
- warn：记录告警继续执行。
- retry：重试 N 次或退避。
- fallback：执行替代步骤（如从剪贴板回退到键入）。

## 策略优先级
- Step.policy > Task.policy > Flow.policy > 系统默认。

## 诊断建议
- 定位失败必须记录 selector/pos/anchor。
- 坐标失败需输出 anchor 解析结果。
- 记录失败时的 UIState（主窗口/弹窗）。

## 诊断流程（建议）
1) 查看 summary.json 确认失败 Task。
2) 定位 StepLog 中的 errorKind 与 params。
3) 对照 Profile 检查 selector/坐标。
4) 必要时回放失败步骤。

## 运行期指标（建议）
- 成功率（按流程/任务统计）。
- 平均耗时与最大耗时。
- 失败原因分布（按 RpcError.Kind）。

## 告警与通知（可选）
- 同一 Task 连续失败超过阈值时告警。
- UI 版本变化触发 Profile 更新提醒。

## 回放与复现
- 支持按 StepLog 重放动作。
- 允许以只读模式验证 UI 状态。

## 常见恢复策略
- 检测弹窗 → 关闭弹窗 → 回到主窗口。
- 失焦 → Focus 主窗口 → 重试操作。
- 坐标失败 → 退回 click_rel 或备用坐标。
