<script setup lang="ts">
import ExportHeader from "../components/export/ExportHeader.vue";
import ExportSettingsPanel from "../components/export/ExportSettingsPanel.vue";
import ExportStatusPanel from "../components/export/ExportStatusPanel.vue";
import ExportHeadersPanel from "../components/export/ExportHeadersPanel.vue";
import ExportDiagnosticsPanel from "../components/export/ExportDiagnosticsPanel.vue";

import { useCommDeviceContext } from "../composables/useDeviceContext";
import { useExportXlsx } from "../composables/useExportXlsx";

const { projectId, activeDeviceId, activeDevice } = useCommDeviceContext();

const {
  outPath,
  last,
  lastDelivery,
  includeResults,
  resultsSource,
  runIdForResults,
  exportWarnings,
  exportDiagnostics,
  deliveryWarnings,
  deliveryDiagnostics,
  deliveryResultsStatus,
  deliveryResultsMessage,
  isExporting,
  isDeliveryExporting,
  setDefaultPath,
  exportNow,
  exportDeliveryNow,
} = useExportXlsx({ projectId, activeDeviceId });
</script>

<template>
  <div class="comm-subpage comm-subpage--export">
    <ExportHeader
      :device-name="activeDevice?.deviceName ?? '未选择'"
      :device-id="activeDevice?.deviceId"
      :is-exporting="isExporting"
      :is-delivery-exporting="isDeliveryExporting"
      @default-path="setDefaultPath"
      @export="exportNow"
      @delivery="exportDeliveryNow"
    />

    <ExportSettingsPanel
      v-model:out-path="outPath"
      v-model:include-results="includeResults"
      v-model:results-source="resultsSource"
      v-model:run-id-for-results="runIdForResults"
      :delay="80"
    />

    <ExportStatusPanel
      :last="last"
      :last-delivery="lastDelivery"
      :delivery-results-status="deliveryResultsStatus"
      :delivery-results-message="deliveryResultsMessage"
      :delay="120"
    />

    <ExportHeadersPanel
      v-if="last"
      title="headers（验收证据）"
      subtitle="导出 XLSX 真实表头"
      :headers="last.headers"
      :delay="160"
    />

    <ExportHeadersPanel
      v-if="lastDelivery"
      title="交付版 headers（验收证据）"
      subtitle="交付版表头快照"
      :headers="lastDelivery.headers"
      :delay="200"
    />

    <ExportDiagnosticsPanel
      v-if="last"
      title="warnings / diagnostics（可交付诊断）"
      :warnings="exportWarnings"
      :diagnostics="exportDiagnostics"
      :delay="240"
    />

    <ExportDiagnosticsPanel
      v-if="lastDelivery"
      title="交付版 warnings / diagnostics"
      :warnings="deliveryWarnings"
      :diagnostics="deliveryDiagnostics"
      :delay="280"
    />
  </div>
</template>
