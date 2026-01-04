<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { ElMessage } from "element-plus";

import type { CommDriverKind, CommRunLatestResponse, PointsV1, ProfilesV1, ReadPlan } from "../api";
import {
  commPlanBuild,
  commRunLatest,
  commRunStart,
  commRunStop,
  commPointsLoad,
  commProfilesLoad,
} from "../api";

const driver = ref<CommDriverKind>("Mock");
const running = ref(false);
const runId = ref<string | null>(null);
const plan = ref<ReadPlan | null>(null);
const latest = ref<CommRunLatestResponse | null>(null);

let timer: number | null = null;
function clearTimer() {
  if (timer !== null) {
    window.clearInterval(timer);
    timer = null;
  }
}

async function buildPlan() {
  const profiles: ProfilesV1 = await commProfilesLoad();
  const points: PointsV1 = await commPointsLoad();
  plan.value = await commPlanBuild({ profiles, points });
  ElMessage.success(`已生成 plan：jobs=${plan.value.jobs.length}`);
}

async function start() {
  const profiles: ProfilesV1 = await commProfilesLoad();
  const points: PointsV1 = await commPointsLoad();
  if (profiles.profiles.length === 0) {
    ElMessage.error("profiles 为空，请先在连接配置页保存或加载 demo");
    return;
  }
  if (points.points.length === 0) {
    ElMessage.error("points 为空，请先在点位列表页保存或加载 demo");
    return;
  }

  const planToUse = plan.value ?? (await commPlanBuild({ profiles, points }));
  plan.value = planToUse;

  const resp = await commRunStart({
    driver: driver.value,
    profiles,
    points,
    plan: planToUse,
  });
  runId.value = resp.runId;
  running.value = true;

  clearTimer();
  timer = window.setInterval(async () => {
    if (!runId.value) return;
    latest.value = await commRunLatest(runId.value);
  }, 1000);

  ElMessage.success(`已启动 run：${runId.value}`);
}

async function stop() {
  if (!runId.value) return;
  clearTimer();

  try {
    await commRunStop(runId.value);
    ElMessage.success("已停止");
  } catch (e: any) {
    ElMessage.error(String(e?.message ?? e ?? "停止失败"));
  } finally {
    running.value = false;
  }
}

onBeforeUnmount(() => {
  clearTimer();
});
</script>

<template>
  <el-card>
    <template #header>采集运行</template>

    <el-space wrap>
      <el-select v-model="driver" style="width: 160px">
        <el-option label="Mock（默认）" value="Mock" />
        <el-option label="TCP（真实）" value="Tcp" />
        <el-option label="RTU 485（真实）" value="Rtu485" />
      </el-select>
      <el-button @click="buildPlan">生成 Plan</el-button>
      <el-button type="primary" :disabled="running" @click="start">Start</el-button>
      <el-button type="danger" :disabled="!running" @click="stop">Stop</el-button>
      <el-tag v-if="runId" type="info">runId={{ runId }}</el-tag>
    </el-space>

    <el-divider />

    <el-row :gutter="12">
      <el-col :span="4">
        <el-statistic title="Total" :value="latest?.stats.total ?? 0" />
      </el-col>
      <el-col :span="4">
        <el-statistic title="OK" :value="latest?.stats.ok ?? 0" />
      </el-col>
      <el-col :span="4">
        <el-statistic title="Timeout" :value="latest?.stats.timeout ?? 0" />
      </el-col>
      <el-col :span="4">
        <el-statistic title="CommError" :value="latest?.stats.commError ?? 0" />
      </el-col>
      <el-col :span="4">
        <el-statistic title="DecodeError" :value="latest?.stats.decodeError ?? 0" />
      </el-col>
      <el-col :span="4">
        <el-statistic title="ConfigError" :value="latest?.stats.configError ?? 0" />
      </el-col>
    </el-row>

    <el-divider />

    <el-table :data="latest?.results ?? []" style="width: 100%">
      <el-table-column prop="valueDisplay" label="valueDisplay" min-width="120" />
      <el-table-column prop="quality" label="quality" width="120" />
      <el-table-column prop="errorMessage" label="errorMessage" min-width="200" />
      <el-table-column prop="timestamp" label="timestamp" min-width="200" />
      <el-table-column prop="durationMs" label="durationMs" width="120" />
      <el-table-column prop="pointKey" label="pointKey" min-width="240" />
    </el-table>
  </el-card>
</template>
