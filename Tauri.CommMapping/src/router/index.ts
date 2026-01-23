import { createRouter, createWebHashHistory } from "vue-router";

import ProjectsPage from "../comm/pages/Projects.vue";
import WorkspaceLandingPage from "../comm/pages/WorkspaceLanding.vue";
import ProjectWorkspacePage from "../comm/pages/ProjectWorkspace.vue";
import PointsPage from "../comm/pages/Points.vue";
import ExportPage from "../comm/pages/Export.vue";
import AdvancedPage from "../comm/pages/Advanced.vue";
import ImportUnionPage from "../comm/pages/ImportUnion.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", component: WorkspaceLandingPage },
    { path: "/projects", component: ProjectsPage },
    {
      path: "/projects/:projectId",
      redirect: (to) => `/projects/${String(to.params.projectId)}/comm/points`,
    },
    {
      path: "/projects/:projectId/comm",
      component: ProjectWorkspacePage,
      children: [
        { path: "", redirect: "points" },
        { path: "connection", redirect: "points" },
        { path: "points", component: PointsPage },
        { path: "run", redirect: "points" },
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
