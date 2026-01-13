# 资产目录结构与命名规范

## 推荐目录结构
```
Autothink.UIA/Docs/组态软件自动操作/FlowSystem/
  assets/
    profiles/
      autothink/zh-CN/1920x1080/v1.0.0/profile.json
    overlays/
      autothink/zh-CN/1920x1080/v1.0.0/overlay.json
    templates/
      hardware_config.yaml
      program_blocks.yaml
      comm_program.yaml
    flows/
      comm-full.yaml
    schemas/
      flow.schema.json
      profile.schema.json
  evidence/
    2026-01-08/comm-full/
      summary.json
      step_logs.json
      screenshots/
```

## 命名规范
- Profile 文件名：profile.json
- Template 文件名：{template-id}.yaml
- Flow 文件名：{flow-id}.yaml
- Evidence 文件夹：{date}/{flow-id}

## 版本策略
- Profile 路径中必须包含版本号。
- Template/Flow 版本在文件内容中声明。

## 资产归档建议
- Profile/Template/Flow 以版本号归档。
- Evidence 保留最近 N 次执行（可配置）。

## 存储策略
- Profile 与 Template 可以随版本归档。
- Flow 为实例化配置，可按项目或现场区分。
