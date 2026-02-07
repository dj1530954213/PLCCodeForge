<script setup lang="ts">
import { onBeforeUnmount } from "vue";
import { useDevicePanel } from "../../composables/useDevicePanel";
import { useWorkspaceSaveAll } from "../../composables/useWorkspaceSaveAll";

const {
  devices,
  activeDeviceId,
  activeDevice,
  deviceEdit,
  deviceDirty,
  addDialogOpen,
  addDeviceName,
  addUseActiveProfile,
  copyDialogOpen,
  copySourceDeviceId,
  copyDeviceName,
  copyRules,
  copyTemplateId,
  copyTemplateName,
  copyTemplates,
  sanitizeWorkbookName,
  selectDevice,
  openAddDialog,
  openCopyDialog,
  confirmDeleteDevice,
  confirmAddDevice,
  confirmCopyDevice,
  saveDeviceMeta,
  saveCopyTemplate,
  deleteCopyTemplate,
} = useDevicePanel();

const workspaceSaveAll = useWorkspaceSaveAll();
const unregisterSave = workspaceSaveAll.registerSaveHandler({
  id: "device-meta",
  label: "设备信息",
  isDirty: () => deviceDirty.value,
  save: () => saveDeviceMeta({ silent: true }),
});

onBeforeUnmount(() => {
  unregisterSave();
});
</script>

<template>
  <section class="comm-panel comm-animate" style="--delay: 60ms">
    <div class="comm-panel-header">
      <div class="comm-section-title">设备</div>
      <el-space wrap>
        <el-button size="small" type="primary" @click="openAddDialog">新增</el-button>
        <el-button size="small" :disabled="!activeDevice" @click="openCopyDialog">复制</el-button>
        <el-button size="small" type="danger" :disabled="!activeDevice" @click="confirmDeleteDevice">删除</el-button>
      </el-space>
    </div>

    <el-menu class="comm-device-menu" :default-active="activeDeviceId" @select="selectDevice">
      <el-menu-item v-for="d in devices" :key="d.deviceId" :index="d.deviceId">
        <div class="comm-device-item">
          <span class="comm-device-name">{{ d.deviceName }}</span>
          <span class="comm-device-meta">{{ d.points.points.length }} 点位</span>
        </div>
      </el-menu-item>
    </el-menu>

    <el-form label-width="72px" class="comm-form-compact" style="margin-top: 10px">
      <el-form-item label="名称">
        <el-input v-model="deviceEdit.name" :disabled="!activeDevice" />
      </el-form-item>
      <el-form-item label="Workbook">
        <el-input v-model="deviceEdit.workbookName" :disabled="!activeDevice" />
      </el-form-item>
    </el-form>

    <el-alert
      v-if="devices.length === 0"
      type="warning"
      show-icon
      :closable="false"
      title="当前工程没有设备，请先新增设备"
      style="margin-top: 12px"
    />
  </section>

  <el-dialog v-model="addDialogOpen" width="520px">
    <template #header>新增设备</template>
    <el-form label-width="140px">
      <el-form-item label="设备名称">
        <el-input v-model="addDeviceName" placeholder="例如 Pump-A" />
      </el-form-item>
      <el-form-item label="workbook 名称">
        <el-input :model-value="sanitizeWorkbookName(addDeviceName)" disabled />
      </el-form-item>
      <el-form-item label="连接参数">
        <el-switch v-model="addUseActiveProfile" active-text="复制当前设备连接" inactive-text="使用默认连接" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="addDialogOpen = false">取消</el-button>
      <el-button type="primary" @click="confirmAddDevice">确定</el-button>
    </template>
  </el-dialog>

  <el-dialog v-model="copyDialogOpen" width="780px">
    <template #header>复制设备（替换变量名称）</template>
    <el-form label-width="140px">
      <el-form-item label="源设备">
        <el-select v-model="copySourceDeviceId" style="width: 320px">
          <el-option v-for="d in devices" :key="d.deviceId" :label="d.deviceName" :value="d.deviceId" />
        </el-select>
      </el-form-item>
      <el-form-item label="新设备名称">
        <el-input v-model="copyDeviceName" />
      </el-form-item>
      <el-form-item label="workbook 名称">
        <el-input :model-value="sanitizeWorkbookName(copyDeviceName)" disabled />
      </el-form-item>

      <el-form-item label="模板">
        <el-space wrap>
          <el-select v-model="copyTemplateId" placeholder="选择模板" style="width: 240px">
            <el-option v-for="t in copyTemplates" :key="t.templateId" :label="t.name" :value="t.templateId" />
          </el-select>
          <el-button :disabled="!copyTemplateId" @click="deleteCopyTemplate">删除模板</el-button>
        </el-space>
      </el-form-item>

      <el-form-item label="替换规则">
        <el-table :data="copyRules" size="small" style="width: 100%">
          <el-table-column label="查找">
            <template #default="{ row }">
              <el-input v-model="row.find" />
            </template>
          </el-table-column>
          <el-table-column label="替换">
            <template #default="{ row }">
              <el-input v-model="row.replace" />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="120">
            <template #default="{ $index }">
              <el-button size="small" type="danger" @click="copyRules.splice($index, 1)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button style="margin-top: 8px" @click="copyRules.push({ find: '', replace: '' })">新增规则</el-button>
      </el-form-item>

      <el-form-item label="保存模板">
        <el-space wrap>
          <el-input v-model="copyTemplateName" placeholder="模板名称" style="width: 240px" />
          <el-button @click="saveCopyTemplate">保存为模板</el-button>
        </el-space>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="copyDialogOpen = false">取消</el-button>
      <el-button type="primary" @click="confirmCopyDevice">确定</el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.comm-device-menu {
  border-right: none;
  background: transparent;
  max-height: 36vh;
  overflow: auto;
  padding-right: 4px;
}

:deep(.comm-device-menu) {
  background-color: transparent;
}

:deep(.comm-device-menu .el-menu-item) {
  height: auto;
  line-height: 1.2;
  padding: 10px 12px;
  border-radius: 12px;
  margin-bottom: 6px;
}

:deep(.comm-device-menu .el-menu-item.is-active) {
  background: rgba(31, 94, 107, 0.16);
  color: var(--comm-text);
}

:deep(.comm-device-menu .el-menu-item:hover) {
  background: rgba(31, 94, 107, 0.1);
}

.comm-device-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.comm-device-name {
  font-weight: 600;
}

.comm-device-meta {
  font-size: 12px;
  color: var(--comm-muted);
}
</style>
