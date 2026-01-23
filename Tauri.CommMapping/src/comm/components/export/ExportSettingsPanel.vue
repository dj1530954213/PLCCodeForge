<script setup lang="ts">
import { computed } from "vue";

import type { DeliveryResultsSource } from "../../api";

const props = defineProps<{
  outPath: string;
  includeResults: boolean;
  resultsSource: DeliveryResultsSource;
  runIdForResults: string;
  delay?: number;
}>();

const emit = defineEmits<{
  (e: "update:outPath", value: string): void;
  (e: "update:includeResults", value: boolean): void;
  (e: "update:resultsSource", value: DeliveryResultsSource): void;
  (e: "update:runIdForResults", value: string): void;
}>();

const panelStyle = computed(() => ({ "--delay": `${props.delay ?? 80}ms` }));

const outPathModel = computed({
  get: () => props.outPath,
  set: (value: string) => emit("update:outPath", value),
});

const includeResultsModel = computed({
  get: () => props.includeResults,
  set: (value: boolean) => emit("update:includeResults", value),
});

const resultsSourceModel = computed({
  get: () => props.resultsSource,
  set: (value: DeliveryResultsSource) => emit("update:resultsSource", value),
});

const runIdModel = computed({
  get: () => props.runIdForResults,
  set: (value: string) => emit("update:runIdForResults", value),
});
</script>

<template>
  <section class="comm-panel comm-animate" :style="panelStyle">
    <div class="comm-panel-header">
      <div class="comm-section-title">导出设置</div>
      <div class="comm-inline-meta">交付版会输出通讯地址表.xlsx</div>
    </div>
    <el-form label-width="140px">
      <el-form-item label="导出路径">
        <el-input v-model="outPathModel" placeholder="可留空：deliveries/{批次}/{deviceName}/通讯地址表.xlsx" />
      </el-form-item>
      <el-form-item label="附加 Results（可选）">
        <el-checkbox v-model="includeResultsModel">写入采集结果 sheet（不影响三表冻结规范）</el-checkbox>
      </el-form-item>
      <el-form-item v-if="includeResultsModel" label="Results 来源">
        <el-select v-model="resultsSourceModel" style="width: 240px">
          <el-option label="appdata（默认：last_results.v1.json）" value="appdata" />
          <el-option label="runLatest（从 runId 的 latest 获取）" value="runLatest" />
        </el-select>
      </el-form-item>
      <el-form-item v-if="includeResultsModel && resultsSourceModel === 'runLatest'" label="runId（用于 latest）">
        <el-input v-model="runIdModel" placeholder="从 Run 页面复制 runId（UUID）" />
      </el-form-item>
    </el-form>
  </section>
</template>
