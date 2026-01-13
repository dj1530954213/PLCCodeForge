# TASK-ADV-01-result：联合导入/桥接等能力降级为“高级/集成”入口

## 1) 完成摘要
- 主流程信息架构固定为：工程 → 连接参数 → 点位配置 → 运行采集 → 导出与证据包。
- “联合导入/bridge/stub/unifiedImport”等集成能力移入工程工作区的 `高级/集成` 页签（子菜单/弱入口），并在页面顶部增加用途说明：不影响通讯采集主流程，仅用于与 PLC 程序生成模块对接。
- 主侧边栏不再直接把用户带进“联合导入”，避免现场用户困惑。

## 2) 改动清单（文件路径 + 关键点）
- `src/router/index.ts`
  - 路由结构：`/projects/:projectId/comm/*`
  - 集成功能仅在 `advanced` 子路由下暴露：`/projects/:projectId/comm/advanced/import-union`
- `src/App.vue`
  - 左侧菜单仅保留“工程列表”入口，避免与工作区 Tabs 重复
- `src/comm/pages/Advanced.vue`
  - 顶部 `el-alert` 说明：高级功能用途与主流程不冲突
  - 提供弱入口按钮进入联合导入页

## 3) 验收证据
### 3.1 路由结构（文字版）
```text
/projects
/projects/:projectId/comm/connection
/projects/:projectId/comm/points
/projects/:projectId/comm/run
/projects/:projectId/comm/export
/projects/:projectId/comm/advanced
/projects/:projectId/comm/advanced/import-union
```

### 3.2 构建输出片段
#### `pnpm build`
```text
vite v6.4.1 building for production...
✓ built in 4.56s
```

#### `cargo build --manifest-path Tauri.CommMapping/src-tauri/Cargo.toml`
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s
```

### 3.3 UI 验收点（截图/录屏说明）
1. 进入任意工程工作区后：
   - 左侧仅保留“工程列表”入口（避免与工作区导航重复）
   - 工作区顶部使用 Tabs 切换：连接参数 / 点位配置 / 运行采集 / 导出与证据包 / 高级/集成
2. 点击 `高级/集成` Tab → 进入 `概览`：
   - 页面顶部出现说明文字：“用于与 PLC 程序生成模块对接…不影响通讯采集主流程…”
3. 从 `高级/集成` 进入 `联合导入` 页面（弱入口），确认不会干扰主流程操作路径。

## 4) 风险与未决项
- 当前已不在侧边栏展示“联合导入”，未来如需进一步弱化，可将“高级/集成”Tab 默认折叠/隐藏，仅在需要集成时显式打开。

## 5) 回滚点说明（如何恢复到旧结构）
- 回滚 `src/router/index.ts` 与 `src/App.vue`：把联合导入路由与菜单恢复为顶层入口即可。
- 不影响后端能力：仅为前端信息架构调整。
