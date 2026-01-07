<template>
  <el-dialog
    v-model="visible"
    title="批量编辑"
    width="600px"
    @close="handleClose"
  >
    <el-alert type="info" :closable="false" style="margin-bottom: 16px">
      将对选中的 {{ selectedCount }} 行进行批量编辑
    </el-alert>

    <el-form label-width="120px">
      <el-form-item label="数据类型">
        <el-select
          v-model="formData.dataType"
          clearable
          placeholder="不修改"
          style="width: 100%"
        >
          <el-option
            v-for="type in DATA_TYPES"
            :key="type"
            :label="type"
            :value="type"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="字节序">
        <el-select
          v-model="formData.byteOrder"
          clearable
          placeholder="不修改"
          style="width: 100%"
        >
          <el-option
            v-for="order in BYTE_ORDERS"
            :key="order"
            :label="order"
            :value="order"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="缩放倍数">
        <el-input
          v-model="formData.scaleExpression"
          placeholder="例如: 10 或 {{x}} * 2 或 {{x}} + 1"
          clearable
        >
          <template #append>
            <span style="padding: 0 8px; color: #909399; cursor: help;" title="支持固定值(如 10)或表达式(如 {{x}} * 2)，{{x}} 代表当前值">?</span>
          </template>
        </el-input>
      </el-form-item>
    </el-form>

    <el-alert
      v-if="preview"
      type="success"
      :closable="false"
      style="margin-top: 16px"
    >
      <template #title>
        预览：将修改 {{ preview.totalRows }} 行，约 {{ preview.estimatedChanges }} 个字段
      </template>
    </el-alert>

    <template #footer>
      <el-button @click="handleCancel">取消 (Esc)</el-button>
      <el-button type="primary" @click="handleConfirm" :disabled="!hasChanges">
        确认 (Enter)
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { ByteOrder32, DataType } from '../api';
import { COMM_BYTE_ORDERS_32, COMM_DATA_TYPES } from "../constants";
import {
  computeBatchEditPreview,
  type BatchEditRequest,
} from '../services/batchEdit';

const DATA_TYPES: DataType[] = COMM_DATA_TYPES;
const BYTE_ORDERS: ByteOrder32[] = COMM_BYTE_ORDERS_32;

interface Props {
  modelValue: boolean;
  selectedCount: number;
  selectedRows: { pointKey: string }[];
}

interface Emits {
  (e: 'update:modelValue', value: boolean): void;
  (e: 'confirm', request: BatchEditRequest): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
});

const formData = ref<{
  dataType: DataType | '';
  byteOrder: ByteOrder32 | '';
  scaleExpression: string;
}>({
  dataType: '',
  byteOrder: '',
  scaleExpression: '',
});

const hasChanges = computed(() => {
  return Boolean(
    formData.value.dataType ||
    formData.value.byteOrder ||
    formData.value.scaleExpression.trim()
  );
});

const preview = computed(() => {
  if (!hasChanges.value) return null;

  const request: BatchEditRequest = {
    pointKeys: props.selectedRows.map((r) => r.pointKey),
    dataType: formData.value.dataType || undefined,
    byteOrder: formData.value.byteOrder || undefined,
    scaleExpression: formData.value.scaleExpression.trim() || undefined,
  };

  return computeBatchEditPreview(request);
});

function handleClose() {
  resetForm();
}

function handleCancel() {
  visible.value = false;
}

function handleConfirm() {
  if (!hasChanges.value) return;

  const request: BatchEditRequest = {
    pointKeys: props.selectedRows.map((r) => r.pointKey),
    dataType: formData.value.dataType || undefined,
    byteOrder: formData.value.byteOrder || undefined,
    scaleExpression: formData.value.scaleExpression.trim() || undefined,
  };

  emit('confirm', request);
  visible.value = false;
}

function resetForm() {
  formData.value = {
    dataType: '',
    byteOrder: '',
    scaleExpression: '',
  };
}

// 键盘快捷键支持
function handleKeydown(e: KeyboardEvent) {
  if (!visible.value) return;

  // Enter 确认
  if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey) {
    const target = e.target as HTMLElement;
    // 如果焦点在输入框中，不拦截
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return;
    
    e.preventDefault();
    handleConfirm();
  }

  // Esc 取消
  if (e.key === 'Escape') {
    e.preventDefault();
    handleCancel();
  }
}

// 监听对话框打开/关闭，注册/注销键盘事件
watch(visible, (isVisible) => {
  if (isVisible) {
    document.addEventListener('keydown', handleKeydown);
  } else {
    document.removeEventListener('keydown', handleKeydown);
    resetForm();
  }
});
</script>

<style scoped>
.el-form {
  margin-top: 16px;
}
</style>
