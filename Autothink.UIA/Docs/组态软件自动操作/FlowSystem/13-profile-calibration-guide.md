# Profile 校准指南

## 目标
- 获取稳定锚点。
- 固化 MFC 自绘区域坐标。
- 输出可复现的 Profile 版本。

## 准备工作
- 确认 UI 版本号、语言、分辨率、DPI。
- 关闭可能影响布局的浮动窗口。
- 准备校准模板（坐标清单）。
- 确保 UI 处于“稳定状态”（无弹窗、无刷新）。

## 校准步骤（推荐）
1) 打开目标界面，进入稳定状态。
2) 定位锚点（优先 UIA selector）。
3) 记录锚点偏移（必要时）。
4) 逐一定位坐标点（MFC 自绘区域）。
5) 保存 Profile 并标注版本号。
6) 运行校验步骤：点击/输入验证。

## 坐标采集方法（建议）
- 使用十字准星/截图工具获取坐标。
- 记录点位应取控件“稳定中心”或标识区域。
- 对矩形区域同时记录左上角与宽高。

## 校准流程细化
- Anchor 校准：
  - 先验证 selector 能稳定定位。
  - 若 selector 不稳定，记录 window fallback。
- Position 校准：
  - 以 anchor 左上角为 (0,0)。
  - 记录多点后复查可点击性。

## 坐标采集工具（Stage2Runner 原型）
- 目标：从锚点计算相对坐标并输出 JSON。
- 运行前提：目标应用已启动，Profile 中已有对应 anchors/selector。
- 单点采集示例：
  ```bash
  dotnet run --project Autothink.UiaAgent.Stage2Runner -- ^
    --calibrate --config Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json ^
    --calibrateFlow autothink.importVariables ^
    --anchorKey mfcPanel ^
    --positionKey baudRateField
  ```
- 多点采集：不传 `--positionKey`，按提示逐个输入 key 并回车采集。
- 输出：默认写到 `Autothink.UIA/logs/<run>/calibration_positions.json`。

## 坐标采集注意事项
- 以 MFC 自绘区域左上角为 (0,0)。
- 坐标点避开边缘与可变区域。
- 若控件动态变化，取其稳定中心。

## DPI 与分辨率
- 校准时记录系统缩放比例。
- Profile 中保存 dpiScale，运行时做换算。
- 变更分辨率需新 Profile 或 overlay。

## 校验建议
- 校验至少 3 个关键坐标点。
- 校验后生成 StepLog 与截图留档。

## 校验脚本建议
- 点击关键坐标 → 检查 UI 状态变化。
- 输入关键参数 → 读取结果（若可验证）。

## Profile 静态校验（推荐）
- 使用 Stage2Runner 执行静态校验，检查 anchors/positions/navSequences 的完整性。
- 命令示例：
  ```bash
  dotnet run --project Autothink.UiaAgent.Stage2Runner -- --profileCheck --config Autothink.UIA/Docs/组态软件自动操作/RunnerConfig/demo.json
  ```
- 输出文件：`profile_check_report.json`（位于 logs 目录的本次 run 中）。

## Profile 输出要求
- 必须包含 metadata 版本信息。
- 必须包含 anchors/positions/selectors。
- 记录 dpiScale 与分辨率。

## 失败回滚
- 校准失败时不要覆盖旧 Profile。
- 将失败原因记录到校准日志。

## 常见问题
- 锚点定位失败：使用 window 或 fallback。
- 坐标漂移：确认 DPI，检查分辨率一致。
- 点击无效：检查坐标是否被遮挡。
