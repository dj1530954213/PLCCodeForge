<script setup lang="ts">
import { computed } from "vue";

import type { CommExportDiagnostics, CommWarning } from "../../api";

const props = defineProps<{
  title: string;
  warnings: CommWarning[];
  diagnostics: CommExportDiagnostics | null;
  delay?: number;
}>();

const panelStyle = computed(() => ({ "--delay": `${props.delay ?? 200}ms` }));
</script>

<template>
  <section class="comm-panel comm-animate" :style="panelStyle">
    <div class="comm-panel-header">
      <div class="comm-section-title">{{ props.title }}</div>
    </div>
    <el-alert
      v-if="props.warnings.length > 0"
      type="warning"
      show-icon
      :closable="false"
      :title="`warnings: ${props.warnings.length}`"
    />
    <pre v-if="props.warnings.length > 0" class="comm-code-block">{{ JSON.stringify(props.warnings, null, 2) }}</pre>
    <el-alert
      v-if="props.diagnostics"
      type="info"
      show-icon
      :closable="false"
      :title="`diagnostics: durationMs=${props.diagnostics.durationMs}`"
    />
    <pre v-if="props.diagnostics" class="comm-code-block">{{ JSON.stringify(props.diagnostics, null, 2) }}</pre>
  </section>
</template>
