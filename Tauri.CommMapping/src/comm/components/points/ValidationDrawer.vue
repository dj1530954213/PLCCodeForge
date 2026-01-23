<script setup lang="ts">
interface ValidationIssueView {
  pointKey: string;
  hmiName: string;
  modbusAddress: string;
  message: string;
  field?: string;
  fieldLabel: string;
}

interface BackendIssueView {
  pointKey?: string;
  hmiName?: string;
  fieldLabel: string;
  reasonLabel: string;
}

defineProps<{
  modelValue: boolean;
  validationIssues: ValidationIssueView[];
  hasValidationIssues: boolean;
  backendIssues: BackendIssueView[];
  hasBackendIssues: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
  (e: "jump", issue: ValidationIssueView): void;
}>();

function close() {
  emit("update:modelValue", false);
}
</script>

<template>
  <el-drawer
    :model-value="modelValue"
    title="配置校验"
    size="min(92vw, 960px)"
    :append-to-body="true"
    class="comm-validation-panel"
    @close="close"
  >
    <div class="comm-validation-drawer">
      <el-empty v-if="!hasValidationIssues && !hasBackendIssues" description="暂无校验错误" />

      <template v-else>
        <el-alert
          v-if="hasValidationIssues"
          type="error"
          show-icon
          :closable="false"
          :title="`前端校验阻断错误 ${validationIssues.length} 条`"
          style="margin-bottom: 12px"
        />
        <div v-if="hasValidationIssues" class="comm-validation-table">
          <el-table :data="validationIssues" size="small" style="width: 100%">
            <el-table-column prop="hmiName" label="变量名称（HMI）" min-width="160" />
            <el-table-column prop="modbusAddress" label="点位地址" width="120" />
            <el-table-column prop="fieldLabel" label="字段" min-width="140" />
            <el-table-column prop="message" label="原因" min-width="240" class-name="comm-validation-reason" />
            <el-table-column label="定位" width="96" align="center" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="emit('jump', row)">定位</el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>

        <el-divider v-if="hasBackendIssues" style="margin: 16px 0" />

        <el-alert
          v-if="hasBackendIssues"
          type="warning"
          show-icon
          :closable="false"
          :title="`后端校验字段问题 ${backendIssues.length} 条`"
          style="margin-bottom: 12px"
        />
        <div v-if="hasBackendIssues" class="comm-validation-table">
          <el-table :data="backendIssues" size="small" style="width: 100%">
            <el-table-column prop="hmiName" label="变量名称（HMI）" min-width="160" />
            <el-table-column prop="pointKey" label="pointKey（稳定键）" min-width="200" show-overflow-tooltip />
            <el-table-column prop="fieldLabel" label="字段" min-width="140" />
            <el-table-column prop="reasonLabel" label="原因" min-width="240" class-name="comm-validation-reason" />
          </el-table>
        </div>
      </template>
    </div>
  </el-drawer>
</template>

<style scoped>
.comm-validation-drawer {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
}

.comm-validation-table {
  border: 1px solid var(--comm-border);
  border-radius: 12px;
  overflow: auto;
  max-height: min(46vh, 420px);
}

:deep(.comm-validation-panel .el-drawer__body) {
  padding: 16px 18px 20px;
  overflow: auto;
}

:deep(.comm-validation-reason .cell) {
  white-space: normal;
  line-height: 1.4;
}
</style>
