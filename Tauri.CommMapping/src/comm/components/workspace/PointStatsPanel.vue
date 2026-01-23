<script setup lang="ts">
import { computed } from "vue";

import { useCommDeviceContext } from "../../composables/useDeviceContext";

const { project, activeDevice } = useCommDeviceContext();

const totalPoints = computed(() => (project.value?.devices ?? []).reduce((sum, d) => sum + d.points.points.length, 0));
const activePointCount = computed(() => activeDevice.value?.points.points.length ?? 0);
</script>

<template>
  <section class="comm-panel comm-animate" style="--delay: 100ms">
    <div class="comm-panel-header">
      <div class="comm-section-title">点位统计</div>
    </div>
    <div class="comm-kpi-grid">
      <div class="comm-kpi-item">
        <div class="comm-kpi-label">当前设备点位</div>
        <div class="comm-kpi-value">{{ activePointCount }}</div>
      </div>
      <div class="comm-kpi-item">
        <div class="comm-kpi-label">项目点位总数</div>
        <div class="comm-kpi-value">{{ totalPoints }}</div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.comm-kpi-grid {
  display: grid;
  gap: 12px;
}

.comm-kpi-item {
  padding: 12px;
  border-radius: 12px;
  border: 1px solid var(--comm-border);
  background: #ffffff;
}

.comm-kpi-label {
  font-size: 12px;
  color: var(--comm-muted);
  letter-spacing: 0.04em;
}

.comm-kpi-value {
  font-size: 18px;
  font-weight: 600;
  margin-top: 4px;
}
</style>
