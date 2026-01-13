# 检查清单与验收标准

## L0/L1 能力检查
- UIA Session 可建立与关闭。
- 原子动作具备参数校验与错误分类。
- StepLog 完整覆盖动作执行。
- 关键动作具备 fallback 与 retry 策略。
- MFC 自绘区坐标点击可复现。
- 粘贴与输入操作具备可替代路径。

## Profile 检查
- anchors 可解析并可定位。
- positions 坐标可点击且可复现。
- selectors 别名具备回退策略。
- Profile 版本与环境匹配。
- overlay 合并结果符合预期。
- profile_check_report.json 中 Missing/Invalid 列表为空。

## DSL 检查
- Flow Schema 校验通过。
- 变量引用可解析。
- 控制流结构合法。
- 模板引用可解析。
- use_template 展开后无未解析变量。

## 编排检查
- 任务图依赖可解析。
- 用户起点选择时能提示缺失依赖。
- 失败策略按任务生效。
- Dry-Run 输出与实际执行一致。

## 运行回归检查
- 核心流程至少 1 条全量执行成功。
- 关键失败案例可回放。
- Evidence Pack 输出完整。
- 版本号与变更记录一致。
- 性能指标满足基线（平均耗时/失败率）。

## 发布检查
- Profile/Template/Flow 版本已登记。
- 变更说明与回滚方案齐备。
- 现场校准结果已归档。
- 兼容矩阵更新（Flow-Template-Profile）。
