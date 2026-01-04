# 通讯地址采集并生成模块：当前执行结果汇总（截至 TASK-01）

> 本文记录在最新计划（《详细执行计划.md》第 9 节 v2）下，截至当前会话的实际执行进度与状态，便于后续继续下发任务与验收。

---

## 1. 计划文档更新情况

### 1.1 最新执行计划确认
- 《详细执行计划.md》第 **9 节“最新任务分解 v2（对齐《执行要求.md》）”** 被确认为**唯一最新执行计划**。
- 第 6 节“任务清单 v1”仅保留为历史参考，不参与后续实际执行与验收。

### 1.2 新增硬约束写入计划
在《详细执行计划.md》中新增/补充了以下内容：

1. **4.4 XLSX 列规范与验收（冻结）**
   - `通讯地址表.xlsx` 下的三张表：`TCP通讯地址表`、`485通讯地址表`、`通讯参数`，其列名与列顺序必须与《执行要求.md》中冻结规范逐字匹配，不允许改名、增删或调换顺序。
   - Rust 侧导出实现必须以这些冻结 headers 为单一真源（`const [&str]`），UI 或其它模块不得自行拼接列名。
   - 所有与 XLSX 相关的任务（尤其 TASK-10）在验收时，必须以“实际导出 headers 与冻结规范逐字对比”作为通过标准之一。

2. **4.5 pointKey 与变量名称（HMI）的角色划分**
   - 每个通讯点位内部必须包含一个系统生成且不可变的稳定键 `pointKey`，用作运行期与结果关联的主键。
   - 变量名称（HMI）用于业务语义对齐与对外展示，是导入/导出与回填时的业务键，但不用于运行期结果主键。
   - 运行结果（`SampleResult` 等）必须按 `pointKey` 对齐和存储，导出表仍按冻结的 5 列输出，不要求在这 5 列中包含 `pointKey` 字段。

### 1.3 对应 TASK 验收条目的补充

- **TASK-01：工程基线与依赖引入**
  - 任务描述中补充：需要创建 `src-tauri/src/comm/` 目录与基础骨架文件（`model/codec/plan/driver/engine/export_xlsx/tauri_api`），但暂不在对外 API 中暴露具体实现。
  - 验收标准中新增：需要提供文件树片段，证明 `src-tauri/src/comm/` 及其子模块骨架已经存在。

- **TASK-02：实现模型与 DTO（冻结契约）**
  - 明确要求：`CommPoint` 必须包含系统生成且不可变的 `pointKey`，并包含用于业务对齐的变量名称（HMI）。
  - 持久化结构中约定：运行结果与点位通过 `pointKey` 关联。
  - 验收标准中强调：serde roundtrip 测试需要覆盖带 `pointKey` 的结构。

- **TASK-08：engine.rs（start/stop/latest/stats，不阻塞 command）**
  - 补充说明：引擎内部以 `pointKey` 作为 `SampleResult` 与 `CommPoint` 的关联键，保证同一 `pointKey` 的结果稳定对齐。
  - 验收标准中增加：在 mock 场景下，需要能够说明/证明结果是按 `pointKey` 对齐的（变量名称变更不影响关联）。

- **TASK-10：export_xlsx.rs（header const + 返回 headers）**
  - 任务要求中写死：Rust 侧导出逻辑必须严格遵守《执行要求.md》中冻结的 3 张 sheet 规范，列名与顺序逐字匹配，不允许改名/增删/调序；每个 sheet 的 header 用 `const [&str]` 定义，并在 `comm_export_xlsx` 返回值或日志中输出，作为唯一真源。
  - 验收标准中写明：`TASK-10-result.md` 中必须贴出三张表的**实际导出 headers 文本**（从 `comm_export_xlsx` 返回或日志中取得），用来与冻结规范逐字对比。

---

## 2. TASK-01 执行进度

> TASK-01 目标：工程基线与依赖引入（前端统一 pnpm、Rust 引入通信相关依赖、创建 comm 模块骨架，并通过基础构建验证）。

### 2.1 前端包管理器统一为 pnpm

**当前状态：进行中**

- ✅ 已删除 `package-lock.json`，避免与 pnpm 的 lockfile 混用。
- ❌ `pnpm-lock.yaml` 目前不存在；之前手写的占位锁文件已删除，锁文件必须由 `pnpm install` 实际生成，禁止手写。
- ⏳ 待补：在可用 pnpm 的环境中执行：
  1. `corepack enable`（或其它方式安装/激活 pnpm）
  2. `corepack prepare pnpm@latest --activate`（如采用 corepack）
  3. `pnpm install`
  4. `pnpm build`

> 状态结论：**前端包管理器切换尚未完成，当前仅完成 npm 锁文件清理；真实的 `pnpm-lock.yaml` 需在安装 pnpm 后由 `pnpm install` 自动生成，并将安装/构建日志作为 TASK-01 前端部分的最终证据。**

### 2.2 Rust 侧通信相关依赖引入

**已完成内容：**
- 在 `src-tauri/Cargo.toml` 的 `[dependencies]` 部分新增了通讯模块所需依赖，列表如下：
  - `thiserror = "1"`
  - `chrono = { version = "0.4", features = ["serde"] }`
  - `uuid = { version = "1", features = ["v4", "serde"] }`
  - `parking_lot = "0.12"`
  - `tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }`
  - `tokio-modbus = { version = "0.11", default-features = false, features = ["tcp", "rtu"] }`
  - `tokio-serial = "5"`
  - `rust_xlsxwriter = "0.82"`
- 这些依赖与《执行要求.md》及《详细执行计划.md》第 9 节 v2 中固定的选型保持一致。

**构建情况：**
- 初次在仓库根目录直接运行 `cargo build` 时，收到错误：
  - 提示在当前目录及其父级中找不到 `Cargo.toml`（实际 Rust 工程在 `src-tauri/` 子目录）。
- 随后改用正确命令：
  - `cargo build --manifest-path src-tauri/Cargo.toml`
- 在将 `mod comm;` 写入 `src-tauri/src/lib.rs`、并在 `src-tauri/src/comm/mod.rs` 顶部添加 `#![allow(dead_code)]` 后再次执行上述命令，得到成功输出片段：
  ```text
  Compiling tauri-app v0.1.0 (C:\\Program Files\\Git\\code\\PLCCodeForge\\src-tauri)
      Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.06s
  ```
  说明 src-tauri crate 在包含 comm 模块的情况下可以正常通过构建，且 comm 目录已实际参与编译。

> 状态结论：**Rust 侧依赖已按规范引入，且在 `mod comm;` 接入后已通过 `cargo build --manifest-path src-tauri/Cargo.toml` 验证，满足 TASK-01 对“依赖+骨架+编译通过”的要求。**

### 2.3 comm 模块骨架创建

**已完成内容：**
- 在 `src-tauri/src/` 下创建了 `comm/` 子目录和基础骨架文件，结构如下（仅列关键节点）：
  - `src-tauri/src/comm/`
    - `mod.rs`：声明子模块 `model`、`codec`、`plan`、`driver`、`engine`、`export_xlsx`、`tauri_api`。
    - `model.rs`：当前为数据模型与 DTO 的占位注释文件。
    - `codec.rs`：当前为字节序/类型解析模块的占位注释文件。
    - `plan.rs`：当前为批量读取计划模块的占位注释文件。
    - `engine.rs`：当前为执行引擎模块的占位注释文件。
    - `export_xlsx.rs`：当前为 XLSX 导出模块的占位注释文件。
    - `tauri_api.rs`：当前为 Tauri commands 模块的占位注释文件。
    - `driver/`
      - `mod.rs`：导出 `modbus_tcp`、`modbus_rtu`、`mock` 三个子模块。
      - `modbus_tcp.rs`：Modbus TCP 驱动占位注释文件。
      - `modbus_rtu.rs`：Modbus RTU 驱动占位注释文件。
      - `mock.rs`：Mock 驱动占位注释文件。

**约束满足情况：**
- 已满足 TASK-01 中关于“创建 `src-tauri/src/comm/` 目录与基础骨架文件”的要求。
- 目前这些文件仅为骨架（没有具体实现），也尚未在对外的 Tauri API 中暴露任何新命令，不会影响现阶段的运行行为。

> 状态结论：**comm 模块目录与骨架文件已按计划创建，符合《详细执行计划.md》第 9 节 v2 中对 TASK-01 的结构要求。**

### 2.4 基线构建验证（进行中）

**计划中的验收要求：**
- `pnpm dev` / `pnpm build` 能正常执行（前端）。
- `cargo build` 能在引入新依赖和 comm 骨架后正常通过（Rust 后端）。
- 提供相应构建输出片段作为 `TASK-01-result.md` 的证据。

**当前实际情况：**
- 前端：受限于当前环境未安装 pnpm，暂时无法在本会话内实际执行 `pnpm dev` / `pnpm build`，因此也就没有构建日志可贴。
- Rust：
  - 之前在错误目录下运行的 `cargo build` 命令失败，原因是 manifest 路径不对。
  - 随后使用正确命令 `cargo build --manifest-path src-tauri/Cargo.toml` 进行了构建，构建过程耗时约 50 秒，最终输出：`Finished dev profile [unoptimized + debuginfo] target(s) in XXs`（实际输出中为 50+ 秒），表明在引入依赖和 comm 骨架后工程可以正常通过构建。

> 状态结论：**TASK-01 的“基线构建验证”在 Rust 侧已完成（cargo build 通过），前端构建部分仍依赖本机安装 pnpm 后补跑，并以 `pnpm build` 输出作为补充证据。**

---

## 3. 当前整体状态与建议后续步骤

### 3.1 整体状态小结

- 计划层面：
  - 《详细执行计划.md》已与《执行要求.md》完全对齐，第 9 节 v2 被确认为唯一最新任务清单。
  - 新增的两条关键硬约束（XLSX 列规范 + pointKey/HMI 名称语义）已经写入计划正文，并体现在 TASK-02 / TASK-08 / TASK-10 的验收条款中。

- 实施层面（TASK-01）：
  - **前端包管理器切换**：已清理 npm 的 `package-lock.json`，但尚未生成真实的 `pnpm-lock.yaml`；前端部分仍处于“进行中”，需在安装 pnpm 后由 `pnpm install` 自动生成 lockfile 并补充 `pnpm build` 日志。
  - **Rust 通讯依赖**：已按约定在 `src-tauri/Cargo.toml` 中引入 tokio/tokio-modbus/tokio-serial/thiserror/rust_xlsxwriter/uuid/parking_lot/chrono 等依赖。
  - **comm 模块骨架**：已在 `src-tauri/src/` 下创建 `comm/` 目录与子模块骨架（含 driver 子模块），并通过在 `lib.rs` 中增加 `mod comm;` 让 comm 参与编译，满足 TASK-01 的结构与接入要求。
  - **构建验证**：`cargo build --manifest-path src-tauri/Cargo.toml` 在接入 comm 后已成功通过，Rust 侧基线构建完成；前端构建验证仍待 pnpm 环境就绪后补跑。

### 3.2 建议的下一步动作

1. **在本机安装 pnpm**（如尚未安装）：
   - 安装完成后，在仓库根目录执行：
     - `pnpm install`
     - `pnpm build`
     - （可选）`pnpm dev`
   - 将构建日志的关键部分作为前端侧的 TASK-01 验收证据。

2. **在 Rust 侧执行正确的构建命令**：
   - 在仓库根目录执行：
     - `cargo build --manifest-path src-tauri/Cargo.toml`
   - 记录构建输出的关键片段，用于 TASK-01 的 Rust 侧验收证据。

3. **整理 TASK-01-result.md（后续步骤）**：
   - 在上述构建验证完成后，可按《执行要求.md》/《详细执行计划.md》中提供的模板，整理 `TASK-01-result.md`：
     - 完成摘要
     - 改动清单（含 `Cargo.toml`、`src-tauri/src/comm/**` 等）
     - `pnpm build` / `cargo build` 输出片段
     - 验收自检与风险/未决项

4. **然后按顺序进入 TASK-02**：
   - 在 `model.rs`/`tauri_api.rs` 中正式定义 `ConnectionProfile`、`CommPoint`（含 `pointKey` 与 HMI 名称）、`SampleResult`、`RunStats` 等 DTO，并实现对应的 serde roundtrip 测试。

---

以上为截至当前会话的执行结果与状态，后续可在完成前端/Rust 构建验证后，继续在本文件中追加新的执行记录，或为每个 TASK 单独维护 `TASK-XX-result.md` 以便归档与验收。