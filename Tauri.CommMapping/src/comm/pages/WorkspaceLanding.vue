<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";

import { useProjectCatalog } from "../composables/useProjectCatalog";

const router = useRouter();
const landingLoading = ref(true);

const { projects, createForm, refresh, createProject } = useProjectCatalog();

function goProject(projectId: string) {
  router.replace(`/projects/${projectId}/comm/points`);
}

async function tryRedirect() {
  await refresh(false);
  const last = localStorage.getItem("comm.lastProjectId")?.trim();
  if (last) {
    const target = projects.value.find((p) => !p.deletedAtUtc && p.projectId === last);
    if (target) {
      goProject(target.projectId);
      return;
    }
    localStorage.removeItem("comm.lastProjectId");
  }
  const first = projects.value.find((p) => !p.deletedAtUtc);
  if (first) {
    goProject(first.projectId);
    return;
  }
  landingLoading.value = false;
}

async function handleCreateProject() {
  const project = await createProject();
  if (!project) return;
  goProject(project.projectId);
}

onMounted(() => {
  tryRedirect().catch(() => {
    landingLoading.value = false;
  });
});
</script>

<template>
  <div class="comm-page comm-page--landing">
    <div class="comm-shell">
      <section class="comm-panel comm-animate" style="--delay: 0ms">
        <div class="comm-panel-header">
          <div class="comm-section-title">进入主界面</div>
        </div>

        <div v-if="landingLoading" class="comm-inline-meta">正在进入最近工程...</div>

        <div v-else>
          <el-alert
            type="info"
            show-icon
            :closable="false"
            title="当前没有工程，请先创建一个工程开始配置"
            style="margin-bottom: 12px"
          />

          <el-form label-width="110px">
            <el-form-item label="工程名称">
          <el-input v-model="createForm.name" placeholder="例如 生产线-A" />
        </el-form-item>
        <el-form-item label="设备（可选）">
          <el-input v-model="createForm.device" />
        </el-form-item>
        <el-form-item label="备注（可选）">
          <el-input v-model="createForm.notes" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>

      <div class="comm-panel-actions">
        <el-button type="primary" @click="handleCreateProject">创建并进入</el-button>
      </div>
    </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.comm-panel-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
