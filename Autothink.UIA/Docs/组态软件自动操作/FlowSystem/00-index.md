# 自动化流程体系文档索引

> 本目录定义“流程可配置 + UI 变化可控”的完整实施方案。文档从底层能力到上层编排逐层展开，目标是让用户创建/调整流程时只改配置而不改代码。

## 范围与边界
- 覆盖 UIA 自动化从底层能力到上层编排的全过程。
- 覆盖 Profile/Template/Flow 的资产化管理与版本策略。
- 不覆盖地址计算与点表解析细节（外部输入提供）。

## 适用场景
- UI 频繁变化但流程稳定的工程软件自动化。
- MFC 自绘区域无法直接 UIA 识别的场景。
- 用户希望“改配置不改代码”的现场交付模式。

## 适用对象
- 开发：搭建底层能力、DSL 引擎与执行器。
- 现场/运维：维护 UI Profile、坐标与选择器资产。
- 流程配置人员：通过模板与 DSL 组合流程。

## 文档导航
- 01-vision-and-goals.md：目标、边界、约束与术语体系。
- 02-architecture.md：分层架构与模块边界（L0-L5）。
- 03-core-capabilities.md：原子动作与 UIA 能力规格。
- 04-ui-profile-spec.md：UI Profile 的结构、版本与校准流程。
- 05-flow-dsl-spec.md：流程 DSL 结构、控制流与参数绑定规范。
- 06-task-graph-and-orchestration.md：任务图/DAG 与用户可选顺序执行策略。
- 07-error-handling-and-observability.md：错误模型、证据链与运行期观测。
- 08-implementation-plan.md：阶段化实施计划与验收标准。
- 09-migration-and-adoption.md：迁移路径与兼容策略。
- 10-templates-and-examples.md：模板与样例（硬件/变量/程序/通讯）。
- 11-checklists-and-acceptance.md：实施与发布检查清单。
- 12-action-reference.md：原子动作与控制流动作完整参考。
- 13-profile-calibration-guide.md：坐标与锚点校准流程。
- 14-dsl-schema.md：DSL 数据模型与 JSON Schema 参考。
- 15-template-authoring-guide.md：模板编写规范与示例。
- 16-directory-layout.md：资产目录结构与命名规范。
- 17-policy-recipes.md：常用策略与降级配方。
- 18-versioning-and-release.md：版本策略与发布流程。
- 19-evidence-pack-format.md：证据包结构与字段说明。
- 20-task-specs.md：四大任务规格与依赖说明。
- 21-input-contracts.md：输入契约与参数规范。
- 22-flow-authoring-playbook.md：流程编排落地指南。
- 23-csharp-uia-inventory.md：C# UIA 资产盘点（P0）。

## 推荐阅读顺序
- 架构与目标：01 -> 02
- 规则与资产：03 -> 04 -> 05
- 编排与执行：06 -> 07
- 落地与迁移：08 -> 09 -> 10 -> 11
- 细节参考：12 -> 13 -> 14 -> 15 -> 16 -> 17 -> 18 -> 19
- 扩展落地：20 -> 21 -> 22
- 现状盘点：23

## 角色视角阅读
- 开发人员：01/02/03/05/06/07/08/12/14/18
- 运维人员：01/02/04/07/11/13/16/18
- 流程配置人员：01/04/05/10/11/15/17

## 文档变更规范
- 结构性变更必须同步更新 00-index/01/02/05。
- Profile/Template 结构调整必须更新 04/05 并提供样例。
- 实施计划变更必须更新 08 与 11。
- 任何新增 Action 必须更新 03/12。

## 维护原则
- 所有流程逻辑必须通过 DSL 与模板表达，禁止把业务流程写死在代码里。
- UI 变化只改 Profile（锚点/坐标/选择器），不改 DSL。
- 规则变更优先改模板与参数映射，不改原子动作实现。
- 每次发布必须包含可回放证据（StepLog）与版本标识。

## 快速使用路径
- 开发：先看 `02-architecture.md` + `03-core-capabilities.md`。
- 现场：先看 `04-ui-profile-spec.md` + `13-profile-calibration-guide.md`。
- 配置：先看 `05-flow-dsl-spec.md` + `10-templates-and-examples.md`。
