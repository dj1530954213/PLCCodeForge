# TASK-20-result.md

- **Task 编号与标题**：
  - TASK-20：联合 xlsx 输入规范 v1：代码常量单一真源 + 文档/代码一致性锁定测试

- **完成摘要**：
  - 新增 `union_spec_v1.rs`：把 v1 规范（Sheet/必填列/可选列/允许枚举/默认地址基准）固化为 **const 单一真源**。
  - `import_union_xlsx.rs` 的 strict 校验与 allowedValues 提示全部改为引用该 spec 常量，避免“手改字符串导致文档/实现漂移”。
  - 新增一致性测试（snapshot）：锁定 v1 required columns 与 allowed enums，防止回归。
  - 文档 `联合xlsx输入规范.v1.md` 增补“规范来源”说明，明确代码真源位置（不改冻结清单语义）。

- **改动清单（文件路径 + 关键点）**：
  - `Tauri.CommMapping/src-tauri/src/comm/union_spec_v1.rs`
    - 新增：`SPEC_VERSION_V1`、`DEFAULT_SHEET_V1`、`REQUIRED_COLUMNS_V1`、`OPTIONAL_COLUMNS_V1`
    - 新增：`ALLOWED_PROTOCOLS_V1` / `ALLOWED_DATATYPES_V1` / `ALLOWED_BYTEORDERS_V1`
    - 新增：`DEFAULT_ADDRESS_BASE_V1`（one-based）与 `AddressBase` 枚举
    - 新增测试：
      - `spec_v1_required_columns_snapshot`
      - `spec_v1_allowed_enums_snapshot`
  - `Tauri.CommMapping/src-tauri/src/comm/import_union_xlsx.rs`
    - strict：必填列校验、列名引用、allowedValues 全部来自 `union_spec_v1.rs`
    - 规范化函数（header/token）改为引用 spec 的 helper，减少重复实现
    - `ImportUnionDiagnostics` 新增可选字段：`specVersion/requiredColumns/allowed*`（便于 UI 展示与验收）
  - `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`
    - `comm_import_union_xlsx` 的 fallback diagnostics 补齐 specVersion 与常量清单字段（与 v1 规范一致）
    - 单测改为引用 spec 常量（不再依赖散落常量名）
  - `Tauri.CommMapping/src-tauri/src/comm/error.rs`
    - `AddressBase` 引用改为 `union_spec_v1`（减少耦合）
  - `Tauri.CommMapping/src-tauri/src/comm/mod.rs`
    - 导出：`pub mod union_spec_v1;`
  - `Tauri.CommMapping/Docs/通讯数据采集验证/联合xlsx输入规范.v1.md`
    - 增补：规范的“代码侧单一真源”来源说明（不改变 v1 冻结清单含义）

- **完成证据（build/test）**：
  - `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`：
    ```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.44s
    ```
  - `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`（新增用例名可见）：
    ```text
    test comm::union_spec_v1::tests::spec_v1_required_columns_snapshot ... ok
    test comm::union_spec_v1::tests::spec_v1_allowed_enums_snapshot ... ok
    ```

- **关键 const 列表（单一真源）**：
  - 文件：`Tauri.CommMapping/src-tauri/src/comm/union_spec_v1.rs`
    - `DEFAULT_SHEET_V1 = "联合点表"`
    - `REQUIRED_COLUMNS_V1 = ["变量名称（HMI）","数据类型","字节序","通道名称","协议类型","设备标识"]`
    - `ALLOWED_PROTOCOLS_V1 = ["TCP","485"]`
    - `ALLOWED_DATATYPES_V1 = ["Bool","Int16","UInt16","Int32","UInt32","Float32"]`
    - `ALLOWED_BYTEORDERS_V1 = ["ABCD","BADC","CDAB","DCBA"]`
    - `DEFAULT_ADDRESS_BASE_V1 = AddressBase::One`

- **验收自检**：
  - [x] v1 规范常量化为单一真源（`union_spec_v1.rs`）
  - [x] strict 校验引用 const（required columns + allowed enums + 默认 sheet）
  - [x] 测试锁定 required columns 与 allowed enums（防漂移）
  - [x] 文档补充“来源说明”，不改冻结清单语义
  - [x] 仅新增可选字段（diagnostics 的 specVersion/清单字段），不破坏既有 DTO

- **风险与未决项**：
  - 若未来要调整 Sheet 名/列名/枚举，必须 bump `SPEC_VERSION_V1` → `v2` 并新增 `联合xlsx输入规范.v2.md`，避免在 v1 上“隐式改规范”。
  - 当前 strict/loose 的宽松候选列名（兼容历史）不属于 v1 冻结规范；如需冻结宽松候选集合，应另出规范或在 v2 明确。

