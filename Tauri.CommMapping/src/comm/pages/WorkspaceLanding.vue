<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ElMessage } from "element-plus";

import { createProject as createProjectService, listProjects } from "../services/projects";

const router = useRouter();
const loading = ref(true);
const createForm = ref({ name: "", device: "", notes: "" });

function goProject(projectId: string) {
  router.replace(`/projects/${projectId}/comm/points`);
}

async function tryRedirect() {
  const last = localStorage.getItem("comm.lastProjectId")?.trim();
  if (last) {
    goProject(last);
    return;
  }
  const resp = await listProjects({ includeDeleted: false });
  const first = resp.projects.find((p) => !p.deletedAtUtc);
  if (first) {
    goProject(first.projectId);
    return;
  }
  loading.value = false;
}

async function createProject() {
  const name = createForm.value.name.trim();
  if (!name) {
    ElMessage.error("工程名称不能为空");
    return;
  }
  const device = createForm.value.device.trim();
  const notes = createForm.value.notes.trim();
  const project = await createProjectService({
    name,
    device: device ? device : undefined,
    notes: notes ? notes : undefined,
  });
  goProject(project.projectId);
}

onMounted(() => {
  tryRedirect().catch(() => {
    loading.value = false;
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

        <div v-if="loading" class="comm-inline-meta">正在进入最近工程...</div>

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
            <el-button type="primary" @click="createProject">创建并进入</el-button>
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
