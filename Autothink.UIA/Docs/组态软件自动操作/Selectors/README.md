# Selector 存放约定

- 位置：本目录 `Autothink.UIA/Docs/组态软件自动操作/Selectors/`
- 规则：每个 flow 一份 JSON；仅维护 selectors/anchors/positions/navSequences，不改 flow 代码
- 命名：`<profile>.<flow-suffix>.json`，例如 `autothink.importProgram.textPaste.json`
- 本地覆盖：`<profile>.<flow-suffix>.local.json`，优先级高于 baseline（仅现场机器使用）
- 共享基线（v1 pack）：`<profile>.v1.base.json`，用于通用/跨 flow 的 selector key（支持 `<profile>.v1.local.json` 覆盖）
- pack 版本：在 `RunnerConfig` 中设置 `selectorPackVersion: "v1"` 以启用冻结 pack
- 全局弹窗处理 keys：`global.popupRoot/global.popupOkButton/global.popupNoButton/...`（用于 UIStateRecovery 处理异常弹窗）
- 结构：
  ```json
  { "schemaVersion": 1, "selectors": { "key": { "...": "ElementSelector" } }, "anchors": {}, "positions": {}, "navSequences": {} }
  ```
- 更新：现场录制后替换对应 JSON，并在 TASK-S2-XX-result.md 中记录变更

文件示例：
- `autothink.demo.json`：与 DemoTarget/Stage2Runner 对齐的示例 selector
- `autothink.attach.json` / `autothink.importProgram.textPaste.json` / `autothink.importVariables.json` / `autothink.build.json`：现场 AUTOTHINK 普通型 profile 模板
- `autothink.v1.base.json`：v1 selector pack（DemoTarget 基线）
- `autothink.v1.local.sample.json`：真实 AUTOTHINK local 覆盖模板（复制为 `autothink.v1.local.json` 后再改）
- `autothink.popups.json`：弹窗收敛（popupHandling）基线 selector
