# TASK-S2-28-result.md

## 完成摘要
- Stage2Runner 默认生成 `evidence_pack_v1`，包含 summary/selector_check/build_outcome/step_logs/resolved_inputs 等证据文件。
- 产出 `evidence_summary.v1.json`（含 digests 与 keyMetrics），并提供 `--verify` 校验命令。
- DemoTarget 全链路跑通并生成证据包；verify 成功与篡改后失败均可复现。

## 改动清单
- `Autothink.UiaAgent.Stage2Runner/Program.cs`：evidence pack 生成、step_logs 汇总、`--verify` 校验。
- `Docs/组态软件自动操作/RunnerConfig/demo.json`：新增 `evidencePack` 配置示例。
- `Docs/组态软件自动操作/Runbook-Autothink-普通型.md`：新增 EvidencePack 生成/验证说明。

## Build/Test 证据
```text
dotnet build PLCCodeForge.sln -c Release
Autothink.UiaAgent.DemoTarget -> ...\Autothink.UiaAgent.DemoTarget.dll
Autothink.UiaAgent -> ...\Autothink.UiaAgent.dll
Autothink.UiaAgent.WinFormsHarness -> ...\Autothink.UiaAgent.WinFormsHarness.dll
Autothink.UiaAgent.Tests -> ...\Autothink.UiaAgent.Tests.dll
Autothink.UiaAgent.Stage2Runner -> ...\Autothink.UiaAgent.Stage2Runner.dll
已成功生成。

dotnet test PLCCodeForge.sln -c Release
已通过! - 失败: 0，通过: 34，已跳过: 1，总计: 35
```

## evidence_pack_v1 目录结构
来自 `logs/20260104-004044/evidence_pack_v1`：
```text
build_outcome.json
selector_check_report.json
step_logs.json
summary.json
evidence_summary.v1.json
resolved_inputs.json
unexpected_ui_state.json
```

## evidence_summary.v1.json 片段（digests + keyMetrics）
来自 `logs/20260104-004044/evidence_pack_v1/evidence_summary.v1.json`：
```json
{
  "packVersion": "v1",
  "digests": {
    "summary.json": "305ba033504718fa33ae9a370f01cd64e8cb92ceeb9b0c3890b5d854dcee1e4f",
    "selector_check_report.json": "4cfdd5d0b28a36a59004376624c149c61c6cceb001b4488868b31e03df44eb74",
    "step_logs.json": "2b5c1a17eb563595acd353d204541057235c4ebfbca254955555f3a7d6a1dd94",
    "build_outcome.json": "9070693560d0d287b1e9bc2c6528a6b7c1534361e2a1c0510245d7d067b1c90a"
  },
  "keyMetrics": {
    "missingKeysCount": 0,
    "buildOutcome": "Success"
  }
}
```

## verify 成功输出
```text
Stage2Runner.exe --verify --evidence logs/20260104-004044/evidence_pack_v1
Evidence verify: OK
```

## 篡改后 verify 失败输出（digest mismatch）
```text
Stage2Runner.exe --verify --evidence logs/20260104-004044/evidence_pack_v1_tampered
Evidence verify: FAIL
  - Digest mismatch: summary.json
  - build_outcome.json exists but summary.build is missing.
```

## Runbook 更新片段
```text
## 3.7 EvidencePack v1（证据包打包/验证）
- 生成位置：logs/<timestamp>/evidence_pack_v1/
- 证据摘要：evidence_summary.v1.json（含 digests 与关键指标）
- 验证命令：Stage2Runner.exe --verify --evidence logs/<timestamp>/evidence_pack_v1
```

## 自检清单
- [x] 未修改 RPC 契约，仅在 Runner 侧落盘与汇总。
- [x] evidence_pack_v1 默认生成，包含 summary/selector_check/step_logs/build_outcome。
- [x] evidence_summary.v1.json 含 digests 与 keyMetrics。
- [x] verify 可成功校验，篡改后能报告 digest mismatch。
