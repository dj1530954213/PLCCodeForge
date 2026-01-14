<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";

const router = useRouter();
const route = useRoute();
</script>

<template>
  <el-container class="comm-app">
    <el-header class="comm-app-header">
      <div class="comm-header-inner">
        <div class="comm-brand">
          PLCCodeForge
          <span class="comm-brand-badge">通讯采集</span>
        </div>
        <el-button :disabled="route.path === '/projects'" @click="router.push('/projects')">工程列表</el-button>
      </div>
    </el-header>

    <el-main class="comm-app-main">
      <router-view />
    </el-main>
  </el-container>
</template>
