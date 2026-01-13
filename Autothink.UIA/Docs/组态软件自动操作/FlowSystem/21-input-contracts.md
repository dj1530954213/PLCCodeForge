# 输入契约与参数规范

## 设计目标
- 明确 Flow 输入的最小稳定字段。
- 让“规则计算”与“UIA 操作”完全解耦。
- 支持按模块类型扩展参数而不改引擎。

## 输入顶层结构（建议）
```json
{
  "varsXls": "C:/data/vars.xlsx",
  "cpuTypes": ["LK220", "LK220S"],
  "deviceType": "LK249",
  "protocolType": "DP_MASTER",
  "racks": [
    { "rackId": "R1", "addr": 1 }
  ],
  "modules": [
    {
      "moduleId": "M1",
      "type": "LK249",
      "slot": 3,
      "rackId": "R1",
      "address": 120,
      "profileId": "modbus_dp_master",
      "params": {
        "baudRate": 9600,
        "stationId": 1
      }
    }
  ],
  "programs": [
    { "type": "FB", "name": "COMM_MAIN", "text": "..." }
  ],
  "commTemplates": {
    "LK249": "comm_lk249"
  }
}
```

## 顶层字段说明
- varsXls：变量表路径（必填）。
- cpuTypes：允许的 CPU 型号列表（必填）。
- deviceType：新增设备类型（默认 LK249）。
- protocolType：设备协议类型（默认 DP_MASTER）。
- racks：机架清单（可选）。
- modules：模块清单（必填）。
- programs：程序块清单（必填）。
- commTemplates：模块类型到通讯模板映射（必填）。

## racks 字段规范
- rackId：机架唯一标识（字符串）。
- addr：机架地址（整数，规则计算提供）。
- 可扩展字段：desc、order、group。

## modules 字段规范
- moduleId：模块唯一标识。
- type：模块类型（用于模板选择）。
- slot：模块槽号（规则计算提供）。
- rackId：所属机架。
- address：模块地址（规则计算提供）。
- profileId：模块参数模板标识（与模板库匹配）。
- params：参数字典（由模板定义具体字段）。

## programs 字段规范
- type：程序类型（如 FB/FC/OB）。
- name：程序名称（字符串）。
- text：程序内容（字符串）。
- 可扩展：templateId、version。

## commTemplates 字段规范
- key：模块类型。
- value：通讯模板 id（对应 templates 库）。

## 规则与约束
- 地址/槽位/机架映射必须由外部规则系统计算完成后传入。
- UIA 自动化不负责解析点表原始数据。
- params 字段为开放结构，具体字段由模板定义并在模板文档中声明。
- 所有输入字段必须可追溯来源（点表/规则计算/人工配置）。

## 参数校验（建议）
- varsXls 必须为有效路径。
- modules 数量必须 > 0。
- 每个 module 必须有 type/slot/rackId/profileId。
- commTemplates 必须覆盖所有 module.type。

## 典型输入来源映射
- 点表解析结果 → modules/programs。
- 规则计算结果 → rack.addr/module.slot/module.address。
- 固定流程规则 → commTemplates/profileId。
