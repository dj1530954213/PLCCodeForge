# TASK-11-result.md

- **Task 编号与标题**：
  - TASK-11：前端框架搭建（Element Plus + Pinia + vue-router + pnpm）

- **完成摘要**：
  - 使用 pnpm 管理依赖并生成真实 `pnpm-lock.yaml`（禁止手写）。
  - 引入 Element Plus、Pinia、vue-router，提供基础 Layout + 左侧菜单导航（连接/点位/运行/导出），路由可切换。
  - 提供 `src/comm/api.ts`：封装 `invoke` 调用并用 TS 类型对齐后端 DTO。

- **改动清单**：
  - `package.json`
    - 新增依赖：`element-plus`、`pinia`、`vue-router`（scripts 使用 `pnpm dev/build`）
  - `pnpm-lock.yaml`
    - 由 `pnpm install` 真实生成/更新
  - `src/main.ts`
    - 注入：`createPinia()`、`router`、`ElementPlus`
  - `src/router/index.ts`
    - 新增路由：`/comm/connection`、`/comm/points`、`/comm/run`、`/comm/export`
  - `src/App.vue`
    - Element Plus Layout（aside 菜单 + router-view）
  - `src/comm/api.ts`
    - comm commands 的 typed invoke 封装
  - `src/comm/pages/Connection.vue`
  - `src/comm/pages/Points.vue`
  - `src/comm/pages/Run.vue`
  - `src/comm/pages/Export.vue`
    - 4 个页面壳/入口（供后续 TASK-12 填充）

- **完成证据**：
  - `pnpm install` 输出片段：
    ```text
    Lockfile is up to date, resolution step is skipped
    Already up to date
    Done in 317ms using pnpm v10.26.2
    ```
  - `pnpm build` 输出片段：
    ```text
    > vue-tsc --noEmit && vite build
    ✓ built in 3.47s
    ```
  - `package.json` 依赖清单片段：
    ```json
    "dependencies": {
      "element-plus": "^2.11.0",
      "pinia": "^3.0.3",
      "vue-router": "^4.5.1"
    }
    ```
  - 路由结构代码片段：`src/router/index.ts`
    ```ts
    export const router = createRouter({
      history: createWebHashHistory(),
      routes: [
        { path: "/", redirect: "/comm/connection" },
        { path: "/comm/connection", component: ConnectionPage },
        { path: "/comm/points", component: PointsPage },
        { path: "/comm/run", component: RunPage },
        { path: "/comm/export", component: ExportPage },
      ],
    });
    ```
  - `pnpm dev` 启动成功日志片段：
    ```text
    VITE v6.4.1  ready in 323 ms
    Local:   http://localhost:1420/
    ```

- **验收自检**：
  - [x] `pnpm-lock.yaml` 存在且由 `pnpm install` 生成/更新。
  - [x] Element Plus + Pinia + vue-router 已引入并可正常路由切换。
  - [x] scripts 使用 pnpm：`pnpm install/dev/build`；Tauri 可用 `pnpm tauri dev`。
  - [x] 已提供 `src/comm/api.ts`（invoke 封装 + 类型对齐）。
  - [x] `Docs/ExecResults/TASK-11-result.md` 已归档（本文件）。

- **风险/未决项**：
  - Element Plus 当前为全量引入；如后续需优化体积，可按需引入或拆分 chunk（不影响 MVP 验收）。

