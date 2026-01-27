<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import type { CommProjectV1 } from "../api";
import { useProjectCatalog } from "../composables/useProjectCatalog";
import { notifySuccess } from "../services/notify";

const router = useRouter();

const createDialogOpen = ref(false);

const {
  loading,
  showDeleted,
  projects,
  createForm,
  activeCount,
  deletedCount,
  refresh,
  resetCreateForm,
  createProject,
  copyProject,
  deleteProject,
} = useProjectCatalog();

function openProject(projectId: string) {
  router.push(`/projects/${projectId}/comm/points`);
}

function onRowDblClick(row: CommProjectV1) {
  openProject(row.projectId);
}

async function handleCreateProject() {
  const project = await createProject();
  if (!project) return;
  createDialogOpen.value = false;
  resetCreateForm();
  notifySuccess("已创建工程");
  openProject(project.projectId);
}

async function handleCopyProject(project: CommProjectV1) {
  const created = await copyProject(project);
  if (!created) return;
  notifySuccess("已复制工程");
  await refresh();
  openProject(created.projectId);
}

async function handleDeleteProject(project: CommProjectV1) {
  const deleted = await deleteProject(project);
  if (!deleted) return;
  notifySuccess("已删除（软删）");
  await refresh();
}

onMounted(refresh);
</script>

<template>
  <div class="comm-page comm-page--projects">
    <div class="comm-shell">
      <header class="comm-hero comm-animate" style="--delay: 0ms">
        <div class="comm-hero-title">
          <div class="comm-title">工程列表</div>
          <div class="comm-subtitle">快速进入工程，管理通讯采集流程</div>
        </div>
        <div class="comm-hero-actions">
          <el-button type="primary" @click="createDialogOpen = true">新建工程</el-button>
          <el-button :loading="loading" @click="() => refresh()">刷新</el-button>
          <el-checkbox v-model="showDeleted" @change="() => refresh()">显示已删除</el-checkbox>
        </div>
      </header>

      <section class="comm-panel comm-panel--table comm-animate" style="--delay: 80ms">
        <div class="comm-panel-header">
          <div class="comm-section-title">工程清单</div>
          <div class="comm-inline-meta">
            <span>可用 {{ activeCount }}</span>
            <span>已删除 {{ deletedCount }}</span>
          </div>
        </div>

        <el-table
          :data="projects"
          style="width: 100%"
          row-key="projectId"
          @row-dblclick="onRowDblClick"
        >
          <el-table-column prop="name" label="名称" min-width="220" />
          <el-table-column prop="device" label="设备" min-width="180" />
          <el-table-column label="创建时间(UTC)" min-width="220">
            <template #default="{ row }"><span class="comm-mono">{{ row.createdAtUtc }}</span></template>
          </el-table-column>
          <el-table-column label="projectId" min-width="260">
            <template #default="{ row }"><span class="comm-mono">{{ row.projectId }}</span></template>
          </el-table-column>
          <el-table-column label="状态" width="120">
            <template #default="{ row }">
              <el-tag v-if="row.deletedAtUtc" type="danger">已删除</el-tag>
              <el-tag v-else type="success">可用</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="240">
            <template #default="{ row }">
              <el-button size="small" @click="openProject(row.projectId)">打开</el-button>
              <el-button size="small" @click="handleCopyProject(row)">复制</el-button>
              <el-button size="small" type="danger" :disabled="!!row.deletedAtUtc" @click="handleDeleteProject(row)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </section>
    </div>

    <el-dialog v-model="createDialogOpen" title="新建工程" width="560px">
      <el-form label-width="110px">
        <el-form-item label="工程名称">
          <el-input v-model="createForm.name" />
        </el-form-item>
        <el-form-item label="设备（可选）">
          <el-input v-model="createForm.device" />
        </el-form-item>
        <el-form-item label="备注（可选）">
          <el-input v-model="createForm.notes" type="textarea" :rows="4" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="createDialogOpen = false">取消</el-button>
        <el-button type="primary" @click="handleCreateProject">创建并进入</el-button>
      </template>
    </el-dialog>
  </div>
</template>
