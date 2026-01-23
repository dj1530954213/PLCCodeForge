<script setup lang="ts">
import { computed } from "vue";

import type { CommExportDeliveryXlsxHeaders, CommExportXlsxHeaders } from "../../api";

type ExportHeaders = CommExportXlsxHeaders | CommExportDeliveryXlsxHeaders;

const props = defineProps<{
  title: string;
  subtitle?: string;
  headers: ExportHeaders;
  delay?: number;
}>();

const panelStyle = computed(() => ({ "--delay": `${props.delay ?? 160}ms` }));
</script>

<template>
  <section class="comm-panel comm-animate" :style="panelStyle">
    <div class="comm-panel-header">
      <div class="comm-section-title">{{ props.title }}</div>
      <div v-if="props.subtitle" class="comm-inline-meta">{{ props.subtitle }}</div>
    </div>
    <el-row :gutter="12">
      <el-col :span="8">
        <el-card>
          <template #header>TCP通讯地址表</template>
          <pre class="comm-code-block">{{ JSON.stringify(props.headers.tcp, null, 2) }}</pre>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card>
          <template #header>485通讯地址表</template>
          <pre class="comm-code-block">{{ JSON.stringify(props.headers.rtu, null, 2) }}</pre>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card>
          <template #header>通讯参数</template>
          <pre class="comm-code-block">{{ JSON.stringify(props.headers.params, null, 2) }}</pre>
        </el-card>
      </el-col>
    </el-row>
  </section>
</template>
