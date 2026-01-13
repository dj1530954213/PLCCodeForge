# 流程编排落地指南（从总体到细节）

## 适用对象
- 配置人员：组装模板与流程。
- 开发/运维：维护 Profile 与模板库。

## 端到端落地流程
1) 明确目标流程与用户可选顺序。
2) 确认 UI 版本与分辨率，生成 Profile。
3) 定义输入契约（modules/programs/racks）。
4) 组装模板库（按模块类型拆分）。
5) 设计 TaskGraph（依赖与输出标签）。
6) 配置策略（retry/fallback/skip）。
7) Dry-Run 校验与回放。
8) 发布版本并归档证据。

## 关键资产清单
- Profile：anchors/positions/selectors/navSequences。
- Templates：hardware_config/program_blocks/comm_program。
- Flow：任务图与参数绑定。
- Evidence Pack：执行证据与版本记录。

## 典型变更场景与处理
- UI 变化（按钮位置/对话框标题）：
  - 只更新 Profile。
  - 不改模板与 DSL。
- 模块类型变化：
  - 新增模块模板。
  - 更新 commTemplates 映射。
- 执行顺序变化：
  - 只改 TaskGraph 依赖或用户选择，不改模板。
- 参数规则变化：
  - 更新输入参数映射或模板参数解析。

## TaskGraph 设计建议
- import_variables 独立任务，产出 vars_imported。
- hardware_config 与 add_program_blocks 可并行（逻辑上）但执行仍串行。
- comm_program 依赖 hardware_ready + programs_ready。
- 保留 precheck/postcheck 提升稳定性。

## 模板组合策略
- 模块差异通过模板拆分，不在动作层判断。
- 复杂模块参数配置单独为 sub-template。
- 通讯程序模板通过 use_template 组合。

## 策略配置建议
- dialog 类步骤加入 retry。
- 坐标点击加入 fallback（click_rel）。
- 关键任务失败默认 stop，非关键任务可 warn。

## 交付与验收
- 版本与兼容矩阵更新。
- Evidence Pack 完整输出。
- 关键路径回放通过。

