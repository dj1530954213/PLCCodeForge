<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";

import type { CommProjectDataV1, CommProjectV1 } from "../../api";
import { useCommDeviceContext } from "../../composables/useDeviceContext";
import { copyProject, createProject, deleteProject, listProjects } from "../../services/projects";

const route = useRoute();
const router = useRouter();

const { project, devices, activeDevice, reloadProject, saveProject } = useCommDeviceContext();

const projectId = computed(() => String(route.params.projectId ?? ""));

const projectList = ref<CommProjectV1[]>([]);
const projectListLoading = ref(false);
const projectCreateOpen = ref(false);
const projectCreateForm = ref({ name: "", device: "", notes: "" });

const selectedProjectId = computed<string>({
  get() {
    return projectId.value;
  },
  set(value) {
    if (!value || value === projectId.value) return;
    router.push(`/projects/${value}/comm/points`);
  },
});

const projectEdit = ref({ name: "", device: "", notes: "" });
const projectDirty = computed(() => {
  const current = project.value;
  if (!current) return false;
  return (
    projectEdit.value.name.trim() !== current.name ||
    projectEdit.value.device.trim() !== (current.device ?? "") ||
    projectEdit.value.notes.trim() !== (current.notes ?? "")
  );
});

async function loadProjectList() {
  projectListLoading.value = true;
  try {
    const resp = await listProjects({ includeDeleted: false });
    projectList.value = resp.projects.filter((p) => !p.deletedAtUtc);
  } finally {
    projectListLoading.value = false;
  }
}

function openCreateProject() {
  projectCreateForm.value = { name: "", device: "", notes: "" };
  projectCreateOpen.value = true;
}

async function confirmCreateProject() {
  const name = projectCreateForm.value.name.trim();
  if (!name) {
    ElMessage.error("工程名称不能为空");
    return;
  }
  const device = projectCreateForm.value.device.trim();
  const notes = projectCreateForm.value.notes.trim();
  const created = await createProject({
    name,
    device: device ? device : undefined,
    notes: notes ? notes : undefined,
  });
  projectCreateOpen.value = false;
  await loadProjectList();
  router.push(`/projects/${created.projectId}/comm/points`);
}

async function copyCurrentProject() {
  const current = project.value;
  if (!current) {
    ElMessage.error("未选择工程");
    return;
  }
  const suggested = `${current.name} (copy)`;
  const name = await ElMessageBox.prompt("输入复制后的工程名称", "复制工程", {
    inputValue: suggested,
    confirmButtonText: "复制",
    cancelButtonText: "取消",
  })
    .then((r) => r.value)
    .catch(() => "");
  if (!name.trim()) return;
  const created = await copyProject({ projectId: current.projectId, name: name.trim() });
  ElMessage.success("已复制工程");
  await loadProjectList();
  router.push(`/projects/${created.projectId}/comm/points`);
}

async function deleteCurrentProject() {
  const current = project.value;
  if (!current) {
    ElMessage.error("未选择工程");
    return;
  }
  await ElMessageBox.confirm(`确认删除工程「${current.name}」？（软删）`, "删除工程", {
    confirmButtonText: "删除",
    cancelButtonText: "取消",
    type: "warning",
  });
  await deleteProject(current.projectId);
  ElMessage.success("已删除（软删）");
  await loadProjectList();
  const next = projectList.value.find((p) => p.projectId !== current.projectId);
  if (next) {
    router.push(`/projects/${next.projectId}/comm/points`);
  } else {
    router.push("/");
  }
}

async function saveProjectMeta() {
  const current = project.value;
  if (!current) {
    ElMessage.error("未选择工程");
    return;
  }
  const name = projectEdit.value.name.trim();
  if (!name) {
    ElMessage.error("工程名称不能为空");
    return;
  }
  const next: CommProjectDataV1 = {
    ...current,
    name,
    device: projectEdit.value.device.trim() || undefined,
    notes: projectEdit.value.notes.trim() || undefined,
  };
  await saveProject(next);
  await loadProjectList();
  ElMessage.success("工程信息已保存");
}

watch(projectId, () => {
  void loadProjectList();
}, { immediate: true });

watch(project, (next) => {
  if (!next) {
    projectEdit.value = { name: "", device: "", notes: "" };
    return;
  }
  projectEdit.value = {
    name: next.name ?? "",
    device: next.device ?? "",
    notes: next.notes ?? "",
  };
});

watch(project, (next) => {
  if (next) {
    localStorage.setItem("comm.lastProjectId", next.projectId);
  }
});
</script>

<template>
  <section class="comm-panel comm-animate" style="--delay: 40ms">
    <div class="comm-panel-header">
      <div class="comm-section-title">项目</div>
      <el-space wrap>
        <el-button size="small" type="primary" @click="openCreateProject">新建</el-button>
        <el-button size="small" :disabled="!project" @click="copyCurrentProject">复制</el-button>
        <el-button size="small" type="danger" :disabled="!project" @click="deleteCurrentProject">删除</el-button>
      </el-space>
    </div>

    <div class="comm-project-overview">
      <div class="comm-project-title">{{ project?.name ?? "未选择工程" }}</div>
      <div class="comm-project-sub">
        设备：{{ activeDevice?.deviceName ?? "未选择" }}
      </div>
      <div class="comm-inline-meta">
        <span>projectId: {{ projectId || "--" }}</span>
        <span>设备数: {{ devices.length }}</span>
      </div>
    </div>

    <el-form label-width="72px" class="comm-form-compact">
      <el-form-item label="选择">
        <el-select v-model="selectedProjectId" filterable placeholder="选择工程" style="width: 100%">
          <el-option v-for="p in projectList" :key="p.projectId" :label="p.name" :value="p.projectId" />
        </el-select>
      </el-form-item>
      <el-form-item label="名称">
        <el-input v-model="projectEdit.name" />
      </el-form-item>
      <el-form-item label="设备">
        <el-input v-model="projectEdit.device" />
      </el-form-item>
      <el-form-item label="备注">
        <el-input v-model="projectEdit.notes" type="textarea" :rows="2" />
      </el-form-item>
    </el-form>

    <div class="comm-panel-actions">
      <el-button size="small" :loading="projectListLoading" @click="loadProjectList">刷新</el-button>
      <el-button size="small" @click="reloadProject">同步当前</el-button>
      <el-button size="small" type="primary" :disabled="!projectDirty" @click="saveProjectMeta">保存项目</el-button>
    </div>
  </section>

  <el-dialog v-model="projectCreateOpen" title="新建工程" width="560px">
    <el-form label-width="110px">
      <el-form-item label="工程名称">
        <el-input v-model="projectCreateForm.name" />
      </el-form-item>
      <el-form-item label="设备（可选）">
        <el-input v-model="projectCreateForm.device" />
      </el-form-item>
      <el-form-item label="备注（可选）">
        <el-input v-model="projectCreateForm.notes" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="projectCreateOpen = false">取消</el-button>
      <el-button type="primary" @click="confirmCreateProject">创建并进入</el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.comm-project-overview {
  padding: 8px 10px 4px;
  border-radius: 12px;
  border: 1px solid var(--comm-border);
  background: #ffffff;
  margin-bottom: 12px;
}

.comm-project-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--comm-text);
}

.comm-project-sub {
  font-size: 12px;
  color: var(--comm-muted);
  margin-top: 4px;
}
</style>
