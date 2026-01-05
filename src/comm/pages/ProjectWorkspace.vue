<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage } from "element-plus";

import type { CommProjectV1 } from "../api";
import { commProjectGet } from "../api";

const route = useRoute();
const router = useRouter();

const projectId = computed(() => String(route.params.projectId ?? ""));
const project = ref<CommProjectV1 | null>(null);
const loading = ref(false);

const tabs = computed(() => {
  const pid = projectId.value;
  return [
    { name: "connection", label: "连接参数", path: `/projects/${pid}/comm/connection` },
    { name: "points", label: "点位与运行", path: `/projects/${pid}/comm/points` },
    { name: "export", label: "导出与证据包", path: `/projects/${pid}/comm/export` },
    { name: "advanced", label: "高级/集成", path: `/projects/${pid}/comm/advanced` },
  ];
});

const activeTab = computed<string>({
  get() {
    const p = route.path;
    if (p.includes("/comm/points") || p.includes("/comm/run")) return "points";
    if (p.includes("/comm/export")) return "export";
    if (p.includes("/comm/advanced")) return "advanced";
    return "connection";
  },
  set(name) {
    const tab = tabs.value.find((t) => t.name === name);
    if (tab) router.push(tab.path);
  },
});

async function loadProject() {
  const pid = projectId.value.trim();
  if (!pid) {
    project.value = null;
    return;
  }

  loading.value = true;
  try {
    project.value = await commProjectGet(pid);
    if (!project.value) {
      ElMessage.error("工程不存在或已损坏");
    } else {
      localStorage.setItem("comm.lastProjectId", pid);
    }
  } finally {
    loading.value = false;
  }
}

watch(projectId, loadProject);
onMounted(loadProject);
</script>

<template>
  <el-card style="margin-bottom: 12px" shadow="never">
    <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px">
      <div>
        <div style="font-weight: 600">
          工程：<span v-if="project">{{ project.name }}</span
          ><span v-else>（未找到）</span>
        </div>
        <div v-if="project?.device" style="color: var(--el-text-color-secondary); font-size: 12px">
          设备：{{ project.device }}
        </div>
        <div style="color: var(--el-text-color-secondary); font-size: 12px">projectId：{{ projectId }}</div>
      </div>

      <el-button @click="router.push('/projects')">返回工程列表</el-button>
    </div>
  </el-card>

  <el-tabs v-model="activeTab" type="card">
    <el-tab-pane v-for="t in tabs" :key="t.name" :name="t.name" :label="t.label" />
  </el-tabs>

  <div style="margin-top: 12px">
    <router-view />
  </div>
</template>
