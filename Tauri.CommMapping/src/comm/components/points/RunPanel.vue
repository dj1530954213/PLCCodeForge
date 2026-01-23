<script setup lang="ts">
import { computed } from "vue";

import type { ConnectionProfile } from "../../api";

type RunUiState = "idle" | "starting" | "running" | "stopping" | "error";

const props = defineProps<{
  activeProfile: ConnectionProfile | null;
  isStarting: boolean;
  isRunning: boolean;
  isStopping: boolean;
  pollMs: number;
  runUiState: RunUiState;
  configChangedDuringRun: boolean;
  autoRestartPending: boolean;
  runId: string | null;
  resumeAfterFix: boolean;
  latestUpdatedAt?: string | null;
}>();

const emit = defineEmits<{
  (e: "start"): void;
  (e: "stop"): void;
  (e: "update:pollMs", value: number): void;
}>();

const pollValue = computed({
  get() {
    return props.pollMs;
  },
  set(value: number) {
    emit("update:pollMs", value);
  },
});
</script>

<template>
  <section class="comm-panel comm-panel--run comm-animate" style="--delay: 120ms">
    <div class="comm-run-grid">
      <div class="comm-profile-block">
        <div class="comm-label">连接配置</div>
        <div v-if="activeProfile" class="comm-profile-meta">
          <span class="comm-chip">{{ activeProfile.protocolType }}</span>
          <span class="comm-chip">{{ activeProfile.channelName }}</span>
          <span class="comm-chip">区域 {{ activeProfile.readArea }}</span>
          <span class="comm-chip">地址按点位配置</span>
        </div>
        <div v-else class="comm-profile-empty">请先在左侧配置连接</div>
      </div>

      <div class="comm-run-actions">
        <el-button type="primary" :loading="isStarting" :disabled="isRunning || isStopping" @click="emit('start')">
          开始运行
        </el-button>
        <el-button type="danger" :loading="isStopping" :disabled="!isRunning" @click="emit('stop')">
          停止
        </el-button>
        <el-select v-model="pollValue" style="width: 160px">
          <el-option :value="500" label="轮询 500ms" />
          <el-option :value="1000" label="轮询 1s" />
          <el-option :value="2000" label="轮询 2s" />
        </el-select>
      </div>
    </div>

    <div class="comm-run-meta">
      <div class="comm-run-tags">
        <el-tag v-if="isRunning && configChangedDuringRun" type="warning" effect="light">
          {{ autoRestartPending ? "配置已变更：即将自动重启" : "配置已变更：重启中" }}
        </el-tag>
        <el-tag v-if="runUiState === 'running'" type="success">运行中</el-tag>
        <el-tag v-else-if="runUiState === 'starting'" type="warning">启动中</el-tag>
        <el-tag v-else-if="runUiState === 'stopping'" type="warning">停止中</el-tag>
        <el-tag v-else-if="runUiState === 'error'" type="danger">错误</el-tag>
        <el-tag v-else type="info">空闲</el-tag>
      </div>
      <div class="comm-run-tags">
        <el-tag v-if="runId" type="info">运行ID={{ runId }}</el-tag>
        <el-tag v-if="resumeAfterFix && !isRunning" type="warning">配置无效：修复后自动恢复</el-tag>
        <el-tag v-if="latestUpdatedAt" type="info">更新时间={{ latestUpdatedAt }}</el-tag>
      </div>
    </div>
  </section>
</template>

<style scoped>
.comm-profile-empty {
  font-size: 12px;
  color: var(--comm-muted);
  padding: 4px 0;
}
</style>
