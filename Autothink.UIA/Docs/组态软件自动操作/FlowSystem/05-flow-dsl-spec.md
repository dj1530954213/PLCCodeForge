# Flow DSL 规范

## 设计目标
- 可读、可配置、可组合。
- 约束明确，避免任意脚本执行。
- 与 UI Profile/模板解耦。

## DSL 顶层结构
- flow：流程元信息（id/name/version）。
- inputs：输入参数定义与默认值。
- tasks：任务定义（可被 DAG 编排）。
- templates：模板引用或内联定义。

## flow 元信息字段（建议）
- id：唯一标识（必填）
- name：名称（可选）
- version：语义版本号（必填）
- description：说明（可选）
- tags：场景标签（可选）

## 顶层字段约定
- flow.id：必填，字符串，唯一标识。
- flow.version：必填，语义版本号。
- inputs：可选，参数默认值与类型提示。
- tasks：必填，任务列表。

## inputs 字段规范（建议）
- inputs.<key>.type：string/int/bool/list/object
- inputs.<key>.required：true/false
- inputs.<key>.default：默认值
- inputs.<key>.desc：字段说明

## Task 结构
- id：任务标识。
- requires：依赖任务列表。
- produces：输出标签列表。
- steps：步骤序列。
- policies：失败策略/重试策略。

## Task 扩展字段（建议）
- name：任务名称（可选）
- tags：任务分类（硬件/变量/程序/通讯）
- precheck：前置检查步骤（可选）
- postcheck：后置检查步骤（可选）
- outputs：写入上下文的变量（可选）

## Step 与 Action
- step 由一个 action + args 组成。
- action 必须是内置动作或可扩展动作。

## Step 扩展字段（建议）
- stepId：稳定标识（用于回放/定位）
- policy：覆盖默认策略
- saveAs：将动作结果写入上下文

## 控制流
- if / else
- switch / case
- for_each（集合迭代）
- try / fallback / retry
- wait（显式等待）

## 控制流边界
- for_each 只允许迭代有限集合（来自 inputs/上下文）。
- condition 只能使用表达式，不允许自定义脚本。

## 控制流语法示例
```yaml
- action: if
  args:
    condition: "@{length(@modules) > 0}"
    then:
      - action: for_each
        args:
          items: "@modules"
          do:
            - action: add_module
              args: { type: "@item.type" }
    else:
      - action: log
        args: { message: "No modules" }
```

## 变量与参数
- 变量引用：`@var` 或 `@{expr}`。
- 变量来源：inputs、运行时上下文、任务输出。
- 禁止执行任意脚本，仅允许内置表达式函数。

## 变量命名空间（建议）
- @inputs.xxx：显式读取输入参数。
- @task.xxx：读取当前任务输出。
- @flow.xxx：读取流程级变量。
- @profile.xxx：读取 Profile 元数据（只读）。

## 变量解析顺序
1) Step 本地变量
2) Task 级变量
3) Flow inputs
4) 系统上下文

## 表达式规则（建议）
- 支持基本比较：==, !=, >, <, >=, <=
- 支持逻辑：and, or, not
- 支持空值：null
- 支持安全函数（无副作用）

## 模板扩展语义
- use_template 仅做展开，不执行。
- 展开后成为 steps 的一部分。
- 模板参数在展开时绑定。

## policy 字段结构（建议）
- policy.onFail：stop/warn/retry/fallback
- policy.retry.times：重试次数
- policy.retry.intervalMs：重试间隔
- policy.fallback：替代步骤

## 内置表达式函数（建议）
- coalesce(a,b)
- concat(a,b)
- now()
- length(list)
- to_int(x)
- to_string(x)

## 内置动作清单（建议）
- ensure_selector
- click_at
- click_rel
- right_click_at
- key_nav
- set_text
- send_keys
- wait_until
- assert
- delay
- use_template

## Action 参数示例
```yaml
- action: click_at
  args: { anchor: "mfcPanel", pos: "baudRateField" }
- action: wait_until
  args: { kind: "ElementNotExists", selector: "importDialog", timeoutMs: 5000 }
```

## 失败策略
- fail：立即终止任务。
- warn：记录告警，继续执行。
- retry：按策略重试。
- fallback：失败后进入降级步骤。

## Step 输出约定
- saveAs 可以将 ActionResult 写入上下文。
- 输出仅允许结构化字段（ok/error/extra）。

## 校验规则
- Flow Schema 必须完整。
- Selector/Position 必须能在 Profile 解析。
- 变量引用必须可解析。

## 运行时保护
- 每个任务设置最大耗时（可选）。
- 每个动作设置最大等待时长。
- 避免无限循环（for_each 限定数组）。

## 用户可选顺序支持
- DSL 仅描述 Task 依赖，不固定执行顺序。
- Orchestrator 根据用户选择生成执行计划。

## 详细参考
- JSON Schema 详见 `14-dsl-schema.md`。
- 模板编写详见 `15-template-authoring-guide.md`。
- 输入参数详见 `21-input-contracts.md`。
