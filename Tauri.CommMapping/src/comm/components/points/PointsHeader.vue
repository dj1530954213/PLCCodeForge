<script setup lang="ts">
defineProps<{
  activeDeviceName?: string;
}>();

const emit = defineEmits<{
  (e: "load"): void;
  (e: "save"): void;
}>();
</script>

<template>
  <header class="comm-hero comm-animate" style="--delay: 0ms">
    <div class="comm-hero-title">
      <div class="comm-title">点位配置</div>
      <div class="comm-subtitle">
        实时采集 <span v-if="activeDeviceName">· {{ activeDeviceName }}</span>
      </div>
    </div>
    <div class="comm-hero-actions">
      <el-button @click="emit('load')">加载</el-button>
      <el-button type="primary" @click="emit('save')">保存</el-button>
    </div>
  </header>
</template>
