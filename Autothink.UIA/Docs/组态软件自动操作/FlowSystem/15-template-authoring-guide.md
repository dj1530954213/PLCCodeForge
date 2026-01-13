# 模板编写指南

## 目标
- 通过模板复用复杂步骤，避免重复配置。
- 将模块差异集中在模板内管理。

## 基本规则
- 模板必须参数化，不允许硬编码业务数据。
- 模板只引用 Profile 中的 selector/pos。
- 模板内部步骤应保持幂等与可重试。

## 参数字段规范（建议）
- params 必须显式声明默认值。
- 对复杂对象使用结构化参数（modules/programs）。
- 避免在模板内解析原始点表。

## 参数设计建议
- 将点表解析结果映射为结构化输入（modules/programs）。
- 每个模板定义明确的输入字段与默认值。

## 变量命名规则
- 模板参数使用 camelCase。
- 集合内使用 @item 引用。
- 不使用隐式全局变量。

## 命名规范
- template.id：小写下划线或短横线。
- 参数名：驼峰或下划线，但需统一。

## 模板结构示例
```yaml
template:
  id: program_blocks
  params:
    programs: []
  steps:
    - action: for_each
      args:
        items: "@programs"
        do:
          - action: create_program
            args: { type: "@item.type", name: "@item.name" }
          - action: paste_program
            args: { text: "@item.text" }
```

## 模板文档建议
- 在模板头部注明用途与输入字段说明。
- 标注与 Profile 的依赖（anchors/positions/selectors）。

## 常见模板类型
- hardware_config：硬件配置。
- program_blocks：程序块创建与粘贴。
- comm_program：通讯程序组合模板。

## 常见陷阱
- 直接写坐标导致无法复用。
- set_text 未使用 Replace 导致重复输入。
- 缺少 wait_until 导致 UI 未稳定。

## 模板测试建议
- 使用小规模 inputs 进行 dry-run。
- 保存 StepLog 作为模板验证证据。

## 模板验收标准（建议）
- 依赖的 Profile key 全部存在。
- 对关键步骤有 retry/fallback。
- 参数缺失能给出明确错误提示。

## 版本策略
- 模板版本号与 Profile 无强绑定。
- 模板升级需保持向后兼容或提供迁移指引。
