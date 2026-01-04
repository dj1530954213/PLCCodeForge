import { createRouter, createWebHashHistory } from "vue-router";

import ConnectionPage from "../comm/pages/Connection.vue";
import PointsPage from "../comm/pages/Points.vue";
import RunPage from "../comm/pages/Run.vue";
import ExportPage from "../comm/pages/Export.vue";
import ImportUnionPage from "../comm/pages/ImportUnion.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/comm/connection" },
    { path: "/comm/connection", component: ConnectionPage },
    { path: "/comm/points", component: PointsPage },
    { path: "/comm/run", component: RunPage },
    { path: "/comm/export", component: ExportPage },
    { path: "/comm/import-union", component: ImportUnionPage },
  ],
});
