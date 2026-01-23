<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";

import ProjectPanel from "../components/workspace/ProjectPanel.vue";
import DevicePanel from "../components/workspace/DevicePanel.vue";
import ConnectionPanel from "../components/workspace/ConnectionPanel.vue";
import PointStatsPanel from "../components/workspace/PointStatsPanel.vue";
import RuntimeStatsPanel from "../components/workspace/RuntimeStatsPanel.vue";
import DiagnosticsPanel from "../components/workspace/DiagnosticsPanel.vue";

import { provideCommDeviceContext } from "../composables/useDeviceContext";
import { provideCommWorkspaceRuntime } from "../composables/useWorkspaceRuntime";

const route = useRoute();
const router = useRouter();

const projectId = computed(() => String(route.params.projectId ?? ""));
provideCommDeviceContext(projectId);
provideCommWorkspaceRuntime();

const tabs = computed(() => {
  const pid = projectId.value;
  return [
    { name: "points", label: "点位配置", path: `/projects/${pid}/comm/points` },
    { name: "export", label: "导出与证据包", path: `/projects/${pid}/comm/export` },
    { name: "advanced", label: "高级/集成", path: `/projects/${pid}/comm/advanced` },
  ];
});

const activeTab = computed<string>({
  get() {
    const p = route.path;
    if (p.includes("/comm/points")) return "points";
    if (p.includes("/comm/export")) return "export";
    if (p.includes("/comm/advanced")) return "advanced";
    return "points";
  },
  set(name) {
    const tab = tabs.value.find((t) => t.name === name);
    if (tab) router.push(tab.path);
  },
});
</script>

<template>
  <div class="comm-page comm-page--workspace">
    <div class="comm-shell comm-shell--wide">
      <div class="comm-workspace-grid">
        <aside class="comm-workspace-side">
          <ProjectPanel />
          <DevicePanel />
          <ConnectionPanel />
          <PointStatsPanel />
        </aside>

        <main class="comm-workspace-main">
          <section class="comm-panel comm-panel--flat comm-animate" style="--delay: 100ms">
            <div class="comm-panel-header">
            <div class="comm-section-title">功能导航</div>
              <div class="comm-inline-meta">点位 → 导出 → 集成</div>
            </div>
            <el-tabs v-model="activeTab" type="card" class="comm-workspace-tabs">
              <el-tab-pane v-for="t in tabs" :key="t.name" :name="t.name" :label="t.label" />
            </el-tabs>
          </section>

          <router-view />
        </main>

        <aside class="comm-workspace-context">
          <RuntimeStatsPanel />
          <DiagnosticsPanel />
        </aside>
      </div>
    </div>
  </div>
</template>

<style scoped>
.comm-workspace-grid {
  display: grid;
  grid-template-columns: minmax(260px, 340px) minmax(0, 1fr) minmax(260px, 340px);
  gap: 16px;
  align-items: start;
}

.comm-workspace-side,
.comm-workspace-main,
.comm-workspace-context {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

@media (max-width: 1400px) {
  .comm-workspace-grid {
    grid-template-columns: minmax(260px, 340px) minmax(0, 1fr);
  }

  .comm-workspace-context {
    display: none;
  }
}

@media (max-width: 1200px) {
  .comm-workspace-grid {
    grid-template-columns: 1fr;
  }
}
</style>
