<script setup lang="ts">
const props = defineProps<{
  deviceName?: string | null;
  deviceId?: string | number | null;
  isExporting?: boolean;
  isDeliveryExporting?: boolean;
}>();

const emit = defineEmits<{
  (e: "default-path"): void;
  (e: "export"): void;
  (e: "delivery"): void;
}>();
</script>

<template>
  <header class="comm-hero comm-animate" style="--delay: 0ms">
    <div class="comm-hero-title">
      <div class="comm-title">导出与证据包</div>
      <div class="comm-subtitle">
        设备：{{ props.deviceName ?? "未选择" }}
        <span v-if="props.deviceId !== null && props.deviceId !== undefined">· deviceId {{ props.deviceId }}</span>
      </div>
    </div>
    <div class="comm-hero-actions">
      <el-button @click="emit('default-path')">默认路径</el-button>
      <el-button type="primary" :loading="props.isExporting" @click="emit('export')">导出 XLSX</el-button>
      <el-button type="success" :loading="props.isDeliveryExporting" @click="emit('delivery')">交付导出</el-button>
    </div>
  </header>
</template>
