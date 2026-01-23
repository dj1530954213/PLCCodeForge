<script setup lang="ts">
import { computed } from "vue";

import type { CommExportDeliveryXlsxResponse, CommExportXlsxResponse, DeliveryResultsStatus } from "../../api";

const props = defineProps<{
  last: CommExportXlsxResponse | null;
  lastDelivery: CommExportDeliveryXlsxResponse | null;
  deliveryResultsStatus: DeliveryResultsStatus | null;
  deliveryResultsMessage: string | null;
  delay?: number;
}>();

const panelStyle = computed(() => ({ "--delay": `${props.delay ?? 120}ms` }));

const resultsAlertType = computed(() => {
  if (props.deliveryResultsStatus === "written") return "success";
  if (props.deliveryResultsStatus === "missing") return "warning";
  return "info";
});
</script>

<template>
  <section class="comm-panel comm-animate" :style="panelStyle">
    <div class="comm-panel-header">
      <div class="comm-section-title">导出状态</div>
      <div class="comm-inline-meta">最近导出路径与结果</div>
    </div>

    <el-alert
      v-if="props.last"
      type="success"
      show-icon
      :closable="false"
      :title="`导出成功：${props.last.outPath}`"
      style="margin-bottom: 10px"
    />

    <el-alert
      v-if="props.lastDelivery"
      type="success"
      show-icon
      :closable="false"
      :title="`交付导出成功：${props.lastDelivery.outPath}`"
      style="margin-bottom: 10px"
    />

    <el-alert
      v-if="props.lastDelivery && props.deliveryResultsStatus"
      :type="resultsAlertType"
      show-icon
      :closable="false"
      :title="`Results: ${props.deliveryResultsStatus}${props.deliveryResultsMessage ? ' - ' + props.deliveryResultsMessage : ''}`"
    />
  </section>
</template>
