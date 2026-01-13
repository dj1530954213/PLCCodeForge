# 版本策略与发布流程

## 版本对象
- Profile 版本：UI 版本变化或坐标变化。
- Template 版本：流程逻辑变化。
- Flow 版本：任务组合与参数变化。

## 版本标识建议
- Profile：app/version/lang/resolution/dpi
- Template：id + semver
- Flow：id + semver

## 版本规则
- major：结构变化，可能影响兼容性。
- minor：功能增强，兼容旧版本。
- patch：修复，无行为变化。

## 兼容矩阵
- Flow 依赖 Template 版本范围。
- Template 依赖 Profile 版本范围。

## 兼容示例
- Flow v1.2.0 兼容 Template v1.x。
- Template v1.1.0 兼容 Profile v1.0.x。

## 发布流程
1) 更新 Profile/Template/Flow。
2) 通过校验与回归检查。
3) 生成 Evidence Pack。
4) 发布版本并记录变更说明。

## 发布产物
- 版本清单（Profile/Template/Flow）。
- 变更说明与风险提示。
- Evidence Pack 与回放记录。

## 回滚策略
- Profile 与 Flow 均支持快速回滚。
- 版本选择由 Runner 参数控制。

## 废弃与兼容策略
- 旧版本保留至少 2 个小版本。
- 重大版本升级需提供迁移说明。
