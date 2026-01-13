# 模板与示例

## 模板编写原则
- 只引用 Profile 中的坐标/选择器，不写硬编码数值。
- 参数从 inputs 或上下文传入，不从外部读取。
- 尽量使用可重试、可回放的动作组合。

## 通用参数约定（建议）
- @inputs.modules：模块清单（type/slot/rack/params）。
- @inputs.racks：机架清单（index/address）。
- @inputs.programs：程序块清单（type/name/text）。
- @inputs.varsXls：变量表路径。
- @inputs.uiProfile：Profile 选择信息（可选）。

## 硬件组态模板（示意）
```yaml
template:
  id: hardware_config
  params:
    deviceType: "LK249"
    protocolType: "DP_MASTER"
    racks: []
    modules: []
  steps:
    - action: ensure_cpu
      args: { types: ["LK220", "LK220S"] }
    - action: for_each
      args:
        items: "@racks"
        do:
          - action: add_device
            args: { type: "@deviceType", address: "@item.addr" }
          - action: ensure_protocol
            args: { name: "@protocolType" }
    - action: for_each
      args:
        items: "@modules"
        do:
          - action: add_module
            args: { type: "@item.type", slot: "@item.slot" }
          - action: configure_module
            args: { profile: "@item.profile" }
```

## 硬件组态细化步骤建议
- 添加设备前确保定位在硬件树。
- 添加协议后等待协议节点出现。
- 添加模块后执行一次 assert（模块节点存在）。

## 模块设置（坐标驱动）
- 适用于 MFC 自绘区，需要先校准 Profile。
- 模板只写语义坐标名，实际坐标在 Profile 中维护。

```yaml
steps:
  - action: click_at
    args: { anchor: "mfcPanel", pos: "param_tab_1" }
  - action: key_nav
    args: { seq: "nav_to_baud" }
  - action: click_at
    args: { anchor: "mfcPanel", pos: "baudRateField" }
  - action: set_text
    args: { selector: "dummy", text: "@params.baudRate", mode: "Replace" }
```

## 变量导入模板参数说明
- varsXls：变量表路径（必填）。
- importMenu：菜单路径选择器（Profile 中配置）。

## 变量导入模板
```yaml
steps:
  - action: menu_import_xls
    args: { path: "@varsXls" }
  - action: wait_until
    args: { kind: "ElementNotExists", selector: "importDialog" }
```

## 程序块创建模板参数说明
- programs[].type：程序类型。
- programs[].name：程序名称。
- programs[].text：程序内容（可来自模板库）。

## 程序块创建模板
```yaml
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

## 通讯程序模板建议
- 组合硬件组态 + 程序块模板。
- 对不同模块类型可拆分子模板（use_template）。

## 通讯程序模板（组合）
- 结合硬件组态模板与程序块模板。
- 允许按模块类型加载不同子模板。

## 全流程示例（任务图）
```yaml
flow:
  id: comm-full
  version: 1.0.0
inputs:
  varsXls: "C:/temp/vars.xlsx"
  programs: []
  modules: []
  racks: []

tasks:
  - id: import_variables
    produces: [vars_imported]
    steps:
      - action: menu_import_xls
        args: { path: "@varsXls" }

  - id: hardware_config
    requires: [vars_imported]
    produces: [hardware_ready]
    steps:
      - action: use_template
        args: { template: "hardware_config" }

  - id: add_program_blocks
    requires: [vars_imported]
    produces: [programs_ready]
    steps:
      - action: use_template
        args: { template: "program_blocks" }

  - id: comm_program
    requires: [hardware_ready, programs_ready]
    produces: [comm_ready]
    steps:
      - action: use_template
        args: { template: "comm_program" }
```

## Profile 示例（坐标）
```json
{
  "positions": {
    "param_tab_1": { "anchor": "mfcPanel", "point": [120, 42] },
    "baudRateField": { "anchor": "mfcPanel", "point": [220, 120] }
  }
}
```

## 常见组合方式
- 硬件先行：import_variables → hardware_config → add_program_blocks → comm_program。
- 程序先行：import_variables → add_program_blocks → hardware_config → comm_program。
- 仅通讯补丁：hardware_config → comm_program（跳过变量导入）。

## 用户可选顺序说明
- 用户可选择从任意 Task 开始执行。
- Orchestrator 自动补齐依赖任务或提示缺失。
- 任务顺序变化不影响模板逻辑。
