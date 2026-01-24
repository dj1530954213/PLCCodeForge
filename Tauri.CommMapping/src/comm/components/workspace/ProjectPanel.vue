<script setup lang="ts">
import { useProjectPanel } from "../../composables/useProjectPanel";

const {
  project,
  devices,
  activeDevice,
  projectId,
  projectList,
  projectListLoading,
  projectCreateOpen,
  projectCreateForm,
  selectedProjectId,
  projectEdit,
  projectDirty,
  loadProjectList,
  openCreateProject,
  confirmCreateProject,
  copyCurrentProject,
  deleteCurrentProject,
  reloadProject,
  saveProjectMeta,
} = useProjectPanel();
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
