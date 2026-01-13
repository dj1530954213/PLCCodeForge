# TASK-REFACTOR-COMM-STRUCTURE — COMM 模块纯重构（refactor only）

## 1) 重构目标与约束（复述）

- 仅重构目录/模块划分：移动文件、拆分 `mod.rs`、统一命名、增加 re-export；不引入新功能、不改变行为。
- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs` 暴露的 command 名称与 DTO JSON 语义保持不变（仅调整内部实现引用）。
- core 不依赖 IO/驱动/第三方协议库；adapters 不反向依赖 usecase；tauri_api 只做 command/DTO glue。
- fixtures 迁移到更合理位置并更新 tests 引用；`cargo fmt/test/build` 必须通过。

## 2) 依赖关系（Before → After）

### Before（扁平结构）
- `comm/*` 混放 core 逻辑、IO/驱动、用例编排与部分工具实现；`tauri_api.rs` 易膨胀。

### After（分层结构）
- `comm/tauri_api.rs`：仅 DTO/command glue（spawn_blocking + 调用 usecase）。
- `comm/usecase/*`：用例入口（组合 core + adapters）。
- `comm/adapters/*`：驱动/存储/xlsx 解析等 IO 边界（可依赖 core，不依赖 usecase）。
- `comm/core/*`：纯逻辑与常量规格（无 IO、无 modbus/xlsx 依赖）。

> 归属说明：`engine.rs` 放在 `usecase/`，因为其包含 tokio task 管理与 driver 调度（不满足 core 的“无运行时编排/无 IO”约束）。

## 3) Before/After 文件树（depth ≥ 3）

### Before（重构前）
```text
Tauri.CommMapping/src-tauri/src/comm/
  driver/
    mod.rs
    mock.rs
    modbus_tcp.rs
    modbus_rtu.rs
  fixtures/
    comm_ir.sample.v1.json
    plc_import_bridge.expected.v1.json
  codec.rs
  model.rs
  plan.rs
  engine.rs
  union_spec_v1.rs
  union_xlsx_parser.rs
  import_union_xlsx.rs
  merge_unified_import.rs
  export_xlsx.rs
  export_delivery_xlsx.rs
  export_ir.rs
  export_plc_import_stub.rs
  bridge_plc_import.rs
  bridge_importresult_stub.rs
  storage.rs
  path_resolver.rs
  error.rs
  tauri_api.rs
  mod.rs
```

### After（重构后，实际文件树）
```text
tree Tauri.CommMapping/src-tauri/src/comm /A /F
文件夹 PATH 列表
卷序列号为 BA63-A546
C:\\...\\Tauri.CommMapping\\Tauri.CommMapping\src-tauri\\src\\comm
|   error.rs
|   mod.rs
|   tauri_api.rs
|
+---adapters
|   |   mod.rs
|   |   union_xlsx_parser.rs
|   |
|   +---driver
|   |       mock.rs
|   |       mod.rs
|   |       modbus_rtu.rs
|   |       modbus_tcp.rs
|   |
|   \\---storage
|           mod.rs
|           path_resolver.rs
|           storage.rs
|
+---core
|       codec.rs
|       mod.rs
|       model.rs
|       plan.rs
|       union_spec_v1.rs
|
+---testdata
|       comm_ir.sample.v1.json
|       plc_import_bridge.expected.v1.json
|
\\---usecase
    |   engine.rs
    |   evidence_pack.rs
    |   import_union_xlsx.rs
    |   merge_unified_import.rs
    |   mod.rs
    |
    +---bridge
    |       bridge_importresult_stub.rs
    |       bridge_plc_import.rs
    |       mod.rs
    |
    \\---export
            export_delivery_xlsx.rs
            export_ir.rs
            export_plc_import_stub.rs
            export_xlsx.rs
            mod.rs
```

## 4) 移动/重命名清单（旧 → 新 → 理由）

| 旧路径 | 新路径 | 理由 |
|---|---|---|
| `Tauri.CommMapping/src-tauri/src/comm/codec.rs` | `Tauri.CommMapping/src-tauri/src/comm/core/codec.rs` | 纯逻辑（core） |
| `Tauri.CommMapping/src-tauri/src/comm/model.rs` | `Tauri.CommMapping/src-tauri/src/comm/core/model.rs` | DTO/模型（core，无 IO） |
| `Tauri.CommMapping/src-tauri/src/comm/plan.rs` | `Tauri.CommMapping/src-tauri/src/comm/core/plan.rs` | 读取计划纯逻辑（core） |
| `Tauri.CommMapping/src-tauri/src/comm/union_spec_v1.rs` | `Tauri.CommMapping/src-tauri/src/comm/core/union_spec_v1.rs` | v1 规范常量（core 单一真源） |
| `Tauri.CommMapping/src-tauri/src/comm/driver/*` | `Tauri.CommMapping/src-tauri/src/comm/adapters/driver/*` | 驱动是 adapter/infra |
| `Tauri.CommMapping/src-tauri/src/comm/storage.rs` | `Tauri.CommMapping/src-tauri/src/comm/adapters/storage/storage.rs` | AppData IO（adapters） |
| `Tauri.CommMapping/src-tauri/src/comm/path_resolver.rs` | `Tauri.CommMapping/src-tauri/src/comm/adapters/storage/path_resolver.rs` | 路径策略（adapters） |
| `Tauri.CommMapping/src-tauri/src/comm/union_xlsx_parser.rs` | `Tauri.CommMapping/src-tauri/src/comm/adapters/union_xlsx_parser.rs` | xlsx 解析是 IO 边界（adapters） |
| `Tauri.CommMapping/src-tauri/src/comm/engine.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/engine.rs` | tokio task + driver 调度（usecase） |
| `Tauri.CommMapping/src-tauri/src/comm/import_union_xlsx.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/import_union_xlsx.rs` | 用例编排（usecase） |
| `Tauri.CommMapping/src-tauri/src/comm/merge_unified_import.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/merge_unified_import.rs` | 用例编排（usecase） |
| `Tauri.CommMapping/src-tauri/src/comm/export_xlsx.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/export/export_xlsx.rs` | 导出用例（usecase/export） |
| `Tauri.CommMapping/src-tauri/src/comm/export_delivery_xlsx.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/export/export_delivery_xlsx.rs` | 导出用例（usecase/export） |
| `Tauri.CommMapping/src-tauri/src/comm/export_ir.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/export/export_ir.rs` | 导出用例（usecase/export） |
| `Tauri.CommMapping/src-tauri/src/comm/export_plc_import_stub.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/export/export_plc_import_stub.rs` | 导出用例（usecase/export） |
| `Tauri.CommMapping/src-tauri/src/comm/bridge_plc_import.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/bridge/bridge_plc_import.rs` | 桥接用例（usecase/bridge） |
| `Tauri.CommMapping/src-tauri/src/comm/bridge_importresult_stub.rs` | `Tauri.CommMapping/src-tauri/src/comm/usecase/bridge/bridge_importresult_stub.rs` | 桥接用例（usecase/bridge） |
| `Tauri.CommMapping/src-tauri/src/comm/fixtures/*` | `Tauri.CommMapping/src-tauri/src/comm/testdata/*` | 测试/回归数据归档（避免运行时裸放） |
| （新增） | `Tauri.CommMapping/src-tauri/src/comm/*/mod.rs` | 明确模块边界与导出面 |
| （新增） | `Tauri.CommMapping/src-tauri/src/comm/usecase/evidence_pack.rs` | 将 evidence pack 业务实现从 tauri_api 抽离到 usecase |

## 5) 模块分层说明（core / usecase / adapters / tauri_api）

- `Tauri.CommMapping/src-tauri/src/comm/core/`：`model/codec/plan/union_spec_v1`（纯逻辑、可单测、无 IO）。
- `Tauri.CommMapping/src-tauri/src/comm/adapters/`：
  - `driver/`：mock/modbus_tcp/modbus_rtu（协议与外设实现）。
  - `storage/`：AppData 读写与路径策略（文件 IO）。
  - `union_xlsx_parser.rs`：读取/解析 xlsx（第三方库边界）。
- `Tauri.CommMapping/src-tauri/src/comm/usecase/`：
  - `import_union_xlsx/merge_unified_import/export/*/bridge/*/engine/evidence_pack`（编排层）。
- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`：冻结契约入口（DTO + command glue），不包含用例业务实现。

## 6) 兼容性说明（对外 API/DTO 不变）

- 冻结入口仍在 `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`：command 名称/请求响应结构未改，仅替换为调用 `comm/usecase/*` 实现。
- `Tauri.CommMapping/src-tauri/src/comm/mod.rs` 增加 re-export 兼容层，保持内部既有引用路径可用（例如 `crate::comm::model/plan/driver/...`）。
- fixtures 迁移后，golden test 改为从 `Tauri.CommMapping/src-tauri/src/comm/testdata/*` 读取（`include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), ...))`），不改变断言内容。

## 7) 验收证据（fmt/test/build）

```text
cargo fmt --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
  (ok, no output)
```

```text
cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
  running 41 tests
  ...
  test result: ok. 41 passed; 0 failed
  running 2 tests
  test result: ok. 2 passed; 0 failed
```

```text
cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.41s
```

## 8) 风险与后续建议

- `comm/mod.rs` 的 re-export 兼容层会导致“同一类型多路径可见”；建议后续逐步收敛内部调用只走新分层路径（但本次为兼容未做破坏性调整）。
- `tauri_api.rs` 仍包含少量“payload/缓存/落盘解析”的 helper（属于 glue/infra）；如后续要进一步瘦身，可迁移到 `adapters/storage` 或独立 `usecase` helper（需评估改动面）。
- 若未来 v2 规范变更（sheet/列名/枚举扩展），建议新增 `union_spec_v2.rs` 并在用例层显式分支，避免破坏 v1 的确定性与 golden tests。

