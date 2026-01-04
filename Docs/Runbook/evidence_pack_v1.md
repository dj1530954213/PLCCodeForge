# Evidence Pack v1（证据包）Runbook

本 Runbook 用于现场/回归时生成与校验“证据包”，保证一次运行的产物（XLSX/IR/UnifiedImport/merge_report/plc_import_stub 等）可追溯、可回传、可复现检查。

---

## 1) 前置条件

- 前端依赖可用：`pnpm -v` 可运行
- Rust 工程可构建：`cargo build --manifest-path src-tauri/Cargo.toml`
- Tauri 应用能启动（开发模式或打包版均可）

---

## 2) 生成证据包（UI）

入口：`联合 xlsx 导入` 页面（`/comm/import-union`，或菜单里对应入口）。

步骤：

1. 配置 `outputDir`（可选；默认：`AppData/<app>/comm/deliveries/`）。
2. 点击 `一键演示（Wizard）` 跑通闭环：导入 → 采集 → 导出（Results 必为 written）。
3. 点击 `导出证据包`。
4. 页面会显示：
   - `evidenceDir=...`（目录证据包）
   - `zip=.../evidence.zip`（若开启 zip）
   - `manifest` 摘要（版本、driver、counts、headersDigest 等）

产物位置（默认策略）：

- `outputDir/evidence/<ts>/...`
- `outputDir/evidence/<ts>/evidence.zip`（若启用）

---

## 3) 校验证据包（UI）

在页面的 `Evidence Verify (v1)` 区域：

1. `Path` 输入 `evidenceDir` 或 `evidence.zip` 路径（导出证据包后会自动填充）。
2. 点击 `Verify`。
3. 预期：
   - `ok=true`
   - `checks[]` 显示每项校验通过
   - `errors[]` 为空

> 校验逻辑由后端命令 `comm_evidence_verify_v1` 提供，返回结构化 `checks/errors`，不会抛出“字符串 JSON”。

---

## 4) 证据包目录结构（示例）

`outputDir/evidence/<ts>/`（目录证据包）至少包含：

- `manifest.json`
- `evidence_summary.v1.json`
- `pipeline_log.json`
- `export_response.json`
- `conflict_report.json`（可选：存在冲突时才会生成）
- `evidence.zip`（可选：启用 zip 时生成）

若开启拷贝（UI 默认开启），还会包含（文件名随时间戳变化）：

- `通讯地址表.<ts>.xlsx`（交付版 xlsx）
- `comm_ir.v1.<ts>.json`
- `unified_import.v1.<ts>.json`
- `merge_report.v1.<ts>.json`
- `plc_import.v1.<ts>.json`

---

## 5) 常见问题排查

- `verify: FAILED` 且 `errors.kind=DigestMismatch`
  - 说明 evidence 中某文件被改动/损坏/拷贝不完整；请重新生成证据包，或对比 `errors.details.fileName/expected/actual`。

- `ManifestMissing` / `EvidenceSummaryMissing`
  - 说明路径不是 evidence 目录或 zip，或目录结构不完整；确认输入的是 `outputDir/evidence/<ts>/` 或 `evidence.zip`。

- `FileMissing`
  - 说明某个被记录 digest 的文件没有被打包进 evidence（可能 copy 开关关闭/拷贝失败）；重新导出证据包并确认 UI 的导出过程无错误提示。

