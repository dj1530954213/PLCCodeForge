<script setup lang="ts">
import { computed } from "vue";

import { useCommWorkspaceRuntime } from "../../composables/useWorkspaceRuntime";

const workspaceRuntime = useCommWorkspaceRuntime();

const runtimeStats = computed(() => {
  return (
    workspaceRuntime.stats.value ?? {
      total: 0,
      ok: 0,
      timeout: 0,
      commError: 0,
      decodeError: 0,
      configError: 0,
    }
  );
});
const runtimeUpdatedAt = computed(() => workspaceRuntime.updatedAtUtc.value || "--");
</script>

<template>
  <section class="comm-panel comm-panel--stats comm-animate" style="--delay: 120ms">
    <div class="comm-panel-header">
      <div class="comm-section-title">运行统计</div>
      <div class="comm-inline-meta">更新时间：{{ runtimeUpdatedAt }}</div>
    </div>
    <div class="comm-stat-grid">
      <div class="comm-stat"><el-statistic title="总数" :value="runtimeStats.total" /></div>
      <div class="comm-stat"><el-statistic title="正常" :value="runtimeStats.ok" /></div>
      <div class="comm-stat"><el-statistic title="超时" :value="runtimeStats.timeout" /></div>
      <div class="comm-stat"><el-statistic title="通讯错误" :value="runtimeStats.commError" /></div>
      <div class="comm-stat"><el-statistic title="解析错误" :value="runtimeStats.decodeError" /></div>
      <div class="comm-stat"><el-statistic title="配置错误" :value="runtimeStats.configError" /></div>
    </div>
  </section>
</template>
