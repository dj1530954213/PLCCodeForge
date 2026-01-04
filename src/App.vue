<script setup lang="ts">
import { useRoute, useRouter } from "vue-router";

const router = useRouter();
const route = useRoute();

const active = {
  get value() {
    return route.path;
  },
  set value(path: string) {
    router.push(path);
  },
};
</script>

<template>
  <el-container style="height: 100vh">
    <el-aside width="220px" style="border-right: 1px solid var(--el-border-color)">
      <div style="padding: 12px 12px 0; font-weight: 600">通讯采集</div>
      <el-menu v-model="active.value" router>
        <el-menu-item index="/comm/connection">连接配置</el-menu-item>
        <el-menu-item index="/comm/points">点位列表</el-menu-item>
        <el-menu-item index="/comm/run">采集运行</el-menu-item>
        <el-menu-item index="/comm/export">导出</el-menu-item>
        <el-menu-item index="/comm/import-union">联合导入</el-menu-item>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header style="border-bottom: 1px solid var(--el-border-color)">
        <div style="display: flex; align-items: center; height: 100%">
          <div style="font-weight: 600">PLCCodeForge</div>
        </div>
      </el-header>

      <el-main>
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>
