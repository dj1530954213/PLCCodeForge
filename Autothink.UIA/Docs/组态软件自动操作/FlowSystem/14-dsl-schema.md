# DSL 数据模型与 Schema

## 数据模型概览
- FlowDefinition
  - id, name, version
  - inputs
  - tasks
  - templates

- TaskDefinition
  - id, requires, produces, steps, policies

- StepDefinition
  - action, args, policy

## 扩展字段概览（建议）
- FlowDefinition.description/tags
- TaskDefinition.precheck/postcheck/outputs
- StepDefinition.stepId/saveAs

## 字段类型
- string, number, bool
- list<T>
- map<string, T>

## 简化 JSON Schema（示意）
```json
{
  "type": "object",
  "required": ["flow", "tasks"],
  "properties": {
    "flow": {
      "type": "object",
      "required": ["id", "version"],
      "properties": {
        "id": { "type": "string" },
        "name": { "type": "string" },
        "version": { "type": "string" }
      }
    },
    "inputs": {
      "type": "object",
      "additionalProperties": {
        "type": "object",
        "properties": {
          "type": { "type": "string" },
          "required": { "type": "boolean" },
          "default": {},
          "desc": { "type": "string" }
        }
      }
    },
    "tasks": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "steps"],
        "properties": {
          "id": { "type": "string" },
          "requires": { "type": "array", "items": { "type": "string" } },
          "produces": { "type": "array", "items": { "type": "string" } },
          "precheck": { "$ref": "#/definitions/steps" },
          "postcheck": { "$ref": "#/definitions/steps" },
          "steps": {
            "type": "array",
            "items": {
              "type": "object",
              "required": ["action"],
              "properties": {
                "stepId": { "type": "string" },
                "action": { "type": "string" },
                "args": { "type": "object" },
                "policy": { "type": "object" },
                "saveAs": { "type": "string" }
              }
            }
          }
        }
      }
    },
    "templates": { "type": "array" }
  },
  "definitions": {
    "steps": {
      "type": "array",
      "items": { "$ref": "#/properties/tasks/items/properties/steps/items" }
    }
  }
}
```

## 语义约束
- Task.id 必须唯一。
- requires 必须引用已有任务。
- action 必须是内置动作或已注册扩展动作。
- selector/pos/anchor 引用必须在 Profile 中解析。

## 表达式约束
- 禁止任意脚本，仅允许内置表达式与函数。
- 表达式错误必须在校验阶段发现。

## 运行时扩展
- 支持自定义 action，但必须注册 Schema。
- 支持模板扩展，但展开后必须通过校验。
