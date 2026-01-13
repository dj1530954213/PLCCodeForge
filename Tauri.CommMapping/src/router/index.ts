import { createRouter, createWebHashHistory } from "vue-router";

import ProjectsPage from "../comm/pages/Projects.vue";
import ProjectWorkspacePage from "../comm/pages/ProjectWorkspace.vue";
import ConnectionPage from "../comm/pages/Connection.vue";
import PointsPage from "../comm/pages/Points.vue";
import RunPage from "../comm/pages/Run.vue";
import ExportPage from "../comm/pages/Export.vue";
import AdvancedPage from "../comm/pages/Advanced.vue";
import ImportUnionPage from "../comm/pages/ImportUnion.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/projects" },
    { path: "/projects", component: ProjectsPage },
    {
      path: "/projects/:projectId",
      redirect: (to) => `/projects/${String(to.params.projectId)}/comm/connection`,
    },
    {
      path: "/projects/:projectId/comm",
      component: ProjectWorkspacePage,
      children: [
        { path: "", redirect: "connection" },
        { path: "connection", component: ConnectionPage },
        { path: "points", component: PointsPage },
        { path: "run", component: RunPage },
        { path: "export", component: ExportPage },
        {
          path: "advanced",
          component: AdvancedPage,
          children: [{ path: "import-union", component: ImportUnionPage }],
        },
      ],
    },
  ],
});
