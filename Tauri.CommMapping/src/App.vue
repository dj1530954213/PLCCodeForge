<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";

const router = useRouter();
const route = useRoute();
</script>

<template>
  <el-container style="height: 100vh">
    <el-header style="border-bottom: 1px solid var(--el-border-color)">
      <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px; height: 100%">
        <div style="font-weight: 600">PLCCodeForge · 通讯采集（工程化）</div>
        <el-button :disabled="route.path === '/projects'" @click="router.push('/projects')">工程列表</el-button>
      </div>
    </el-header>

    <el-main>
      <router-view />
    </el-main>
  </el-container>
</template>
