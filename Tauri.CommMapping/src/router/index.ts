import { createRouter, createWebHashHistory } from "vue-router";

const WorkspaceLandingPage = () => import("../comm/pages/WorkspaceLanding.vue");
const ProjectsPage = () => import("../comm/pages/Projects.vue");
const ProjectWorkspacePage = () => import("../comm/pages/ProjectWorkspace.vue");
const PointsPage = () => import("../comm/pages/Points.vue");
const ExportPage = () => import("../comm/pages/Export.vue");
const AdvancedPage = () => import("../comm/pages/Advanced.vue");
const ImportUnionPage = () => import("../comm/pages/ImportUnion.vue");

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
