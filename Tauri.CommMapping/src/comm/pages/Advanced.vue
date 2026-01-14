<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";

const route = useRoute();
const router = useRouter();

const projectId = String(route.params.projectId ?? "");
</script>

<template>
  <div class="comm-subpage comm-subpage--advanced">
    <header class="comm-hero comm-animate" style="--delay: 0ms">
      <div class="comm-hero-title">
        <div class="comm-title">高级/集成功能</div>
        <div class="comm-subtitle">对接 PLC 程序生成模块与联合导入流程</div>
      </div>
      <div class="comm-hero-actions">
        <el-button type="primary" @click="router.push(`/projects/${projectId}/comm/advanced/import-union`)">联合导入</el-button>
      </div>
    </header>

    <section class="comm-panel comm-animate" style="--delay: 80ms">
      <el-alert
        title="高级/集成功能"
        type="info"
        show-icon
        :closable="false"
        description="用于与 PLC 程序生成模块对接（联合导入 / Bridge / Stub / UnifiedImport 等）。不影响通讯采集主流程：工程 -> 连接 -> 点位 -> 运行 -> 导出。"
      />
    </section>

    <router-view />
  </div>
</template>
