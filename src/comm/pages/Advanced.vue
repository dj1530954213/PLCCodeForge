<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";

const route = useRoute();
const router = useRouter();

const projectId = String(route.params.projectId ?? "");
</script>

<template>
  <el-alert
    title="高级/集成功能"
    type="info"
    show-icon
    :closable="false"
    description="用于与 PLC 程序生成模块对接（联合导入 / Bridge / Stub / UnifiedImport 等）。不影响通讯采集主流程：工程 -> 连接 -> 点位 -> 运行 -> 导出。"
  />

  <el-divider />

  <el-space wrap>
    <el-button type="primary" @click="router.push(`/projects/${projectId}/comm/advanced/import-union`)">联合导入</el-button>
  </el-space>

  <el-divider />
  <router-view />
</template>

