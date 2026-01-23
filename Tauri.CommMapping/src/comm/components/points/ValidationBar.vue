<script setup lang="ts">
defineProps<{
  summary: string;
  hasIssues: boolean;
}>();

const emit = defineEmits<{
  (e: "open"): void;
}>();
</script>

<template>
  <div class="comm-status-bar comm-animate" style="--delay: 160ms" :class="{ 'is-error': hasIssues }">
    <div class="comm-status-left">
      <div class="comm-status-title">配置校验</div>
      <div class="comm-status-desc">{{ summary }}</div>
    </div>
    <div class="comm-status-actions">
      <el-button size="small" :disabled="!hasIssues" @click="emit('open')">查看详情</el-button>
    </div>
  </div>
</template>
