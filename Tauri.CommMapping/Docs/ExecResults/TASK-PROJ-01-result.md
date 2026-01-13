# TASK-PROJ-01-result：工程(Project)管理与工程化目录（comm）

## 1) 完成摘要
- 新增“工程(Project)”概念：工程列表页支持新建/打开/复制/软删。
- 打开工程后进入“工程工作区”信息架构：连接参数 / 点位配置 / 运行采集 / 导出与证据包 / 高级集成。
- 所有 comm 配置与数据按 `projectId` 归档到工程目录：`AppData/<app>/projects/<projectId>/comm/**`；未传 `projectId` 仍兼容旧 `AppData/<app>/comm/**`（legacy）。

## 2) 改动清单（文件路径 + 关键点）
### Rust
- `Tauri.CommMapping/src-tauri/src/comm/core/model.rs`：新增 `CommProjectV1`（`schemaVersion: 1`，含 `projectId/name/device?/createdAtUtc/notes?/deletedAtUtc?`）。
- `Tauri.CommMapping/src-tauri/src/comm/adapters/storage/projects.rs`：工程目录规则与 CRUD（create/list/get/copy/soft-delete），并做 `projectId` 路径穿越校验。
- `Tauri.CommMapping/src-tauri/src/comm/tauri_api.rs`：
  - 新增 commands：`comm_project_create/comm_projects_list/comm_project_get/comm_project_copy/comm_project_delete`（均 `spawn_blocking`）。
  - 绝大多数 comm commands 增加 `project_id: Option<String>` 参数；缺省走 legacy。
  - `comm_evidence_pack_create`：project 模式下 evidence 输出落在工程 `comm/evidence/**`（避免落到 legacy outputDir）。
- `Tauri.CommMapping/src-tauri/src/comm/adapters/storage/storage.rs`：新增 `runs/<runId>/last_results.v1.json` 读写辅助（仍兼容 legacy `comm/last_results.v1.json`）。
- `Tauri.CommMapping/src-tauri/src/lib.rs`：注册新增 commands。

### Frontend
- `src/comm/pages/Projects.vue`：工程列表页（新建/打开/复制/软删）。
- `src/comm/pages/ProjectWorkspace.vue`：工程工作区容器 + Tabs。
- `src/comm/pages/Advanced.vue`：高级/集成入口（不阻塞主流程）。
- `src/router/index.ts`：路由改为 `/projects/:projectId/comm/*` 作为主工作区。
- `src/App.vue`：侧边栏导航适配工程工作区。
- `src/comm/api.ts`：新增 project API 封装；原 comm API 增加可选 `projectId` 参数。

## 3) 验收证据
### 3.1 AppData 目录树（创建工程后）
Windows 默认 AppData 根目录：`%APPDATA%\\com.plccodeforge.app\\`

创建一个工程后，目录结构应为（示例）：
```text
%APPDATA%\\com.plccodeforge.app\\projects\\<projectId>\\comm\\
  project.v1.json
  profiles.v1.json
  points.v1.json
  config.v1.json                 (首次保存连接配置后生成)
  plan.v1.json                   (首次 build plan/run 后生成)
  runs\\
    <runId>\\
      last_results.v1.json       (stop 时落盘，可选)
  exports\\
    通讯地址表.<ts>.xlsx         (未指定 outPath 时默认落盘)
  evidence\\
    <ts>\\
      manifest.json
      pipeline_log.json
      export_response.json
      evidence_summary.v1.json
      evidence.zip               (若启用 zip)
```

### 3.2 构建/测试输出片段
#### `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.68s
```

#### `cargo test --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`
```text
running 42 tests
test result: ok. 42 passed; 0 failed; ...
running 2 tests
test result: ok. 2 passed; 0 failed; ...
```

#### `pnpm build`
```text
vite v6.4.1 building for production...
✓ 1767 modules transformed.
✓ built in 6.89s
```

### 3.3 UI 演示步骤（截图/录屏要点）
1. 启动应用后进入“工程列表”（`/projects`）。
2. 点击“新建工程”，填写工程名称（可选：设备/备注），点击“创建并进入”。
3. 进入工程工作区后，顶部可看到工程名称与 `projectId`；Tabs 显示：连接参数/点位配置/运行采集/导出与证据包/高级集成。
4. 进入“连接参数/点位配置”保存后，检查 `AppData/<app>/projects/<projectId>/comm/` 下对应文件生成/更新。

## 4) 风险与未决项
- 旧数据迁移：当前保留 legacy `AppData/<app>/comm/**` 兼容（未自动迁移到 projects）。后续如需“从 legacy 一键迁入工程”，应新增显式迁移入口并产出迁移报告。
- outputDir 旧概念：工程模式下 evidence 已强制落到工程目录；其它高级导出仍可能受 `config.v1.json.outputDir` 影响（后续可统一为“工程内固定出口”）。
- 删除策略：当前为软删（写 `deletedAtUtc`）；如需物理删除需额外确认与回收站机制。

## 5) 回滚点说明（如何恢复到旧结构）
- 前端：将路由入口从 `/projects` 改回原 `/comm/*`（或增加 redirect）即可恢复“非工程化”的入口体验。
- 后端：所有 comm commands 仍兼容不传 `projectId` 的 legacy 调用，数据仍写入 `AppData/<app>/comm/**`；可作为回滚运行模式。

