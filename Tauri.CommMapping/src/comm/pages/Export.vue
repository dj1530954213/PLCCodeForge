<script setup lang="ts">
import { computed, ref } from "vue";
import { ElMessage } from "element-plus";

import type {
  CommExportDeliveryXlsxResponse,
  CommExportXlsxResponse,
  DeliveryResultsSource,
  PointsV1,
  ProfilesV1,
  SampleResult,
  RunStats,
} from "../api";
import { commExportDeliveryXlsx, commExportXlsx, commPointsLoad, commProfilesLoad, commRunLatest } from "../api";
import { useCommDeviceContext } from "../composables/useDeviceContext";

const { projectId, activeDeviceId, activeDevice } = useCommDeviceContext();

const outPath = ref<string>("");
const last = ref<CommExportXlsxResponse | null>(null);
const lastDelivery = ref<CommExportDeliveryXlsxResponse | null>(null);
const includeResults = ref(false);
const resultsSource = ref<DeliveryResultsSource>("appdata");
const runIdForResults = ref<string>("");
const exportWarnings = computed(() => last.value?.warnings ?? []);
const exportDiagnostics = computed(() => last.value?.diagnostics ?? null);
const deliveryWarnings = computed(() => lastDelivery.value?.warnings ?? []);
const deliveryDiagnostics = computed(() => lastDelivery.value?.diagnostics ?? null);
const deliveryResultsStatus = computed(() => lastDelivery.value?.resultsStatus ?? null);
const deliveryResultsMessage = computed(() => lastDelivery.value?.resultsMessage ?? null);

async function setDefaultPath() {
  // 允许留空：后端会按 outputDir 自动选择默认导出路径（TASK-32）。
  outPath.value = "";
}

async function exportNow() {
  const pid = projectId.value.trim();
  const did = activeDeviceId.value.trim();
  if (!pid || !did) {
    ElMessage.error("未选择设备");
    return;
  }
  const profiles: ProfilesV1 = await commProfilesLoad(pid, did);
  const points: PointsV1 = await commPointsLoad(pid, did);
  if (profiles.profiles.length === 0 || points.points.length === 0) {
    ElMessage.error("profiles/points 为空，请先配置并保存");
    return;
  }

  last.value = await commExportXlsx({ outPath: outPath.value.trim(), profiles, points }, pid, did);
  lastDelivery.value = null;
  ElMessage.success(`已导出：${last.value.outPath}`);
}

async function exportDeliveryNow() {
  const pid = projectId.value.trim();
  const did = activeDeviceId.value.trim();
  if (!pid || !did) {
    ElMessage.error("未选择设备");
    return;
  }
  const profiles: ProfilesV1 = await commProfilesLoad(pid, did);
  const points: PointsV1 = await commPointsLoad(pid, did);
  if (profiles.profiles.length === 0 || points.points.length === 0) {
    ElMessage.error("profiles/points 为空，请先配置并保存");
    return;
  }

  // Results sheet 的来源策略（TASK-24 拍板）：
  // - includeResults=false：不附加 Results sheet
  // - includeResults=true & resultsSource=appdata：由后端读取 AppData/comm/last_results.v1.json
  // - includeResults=true & resultsSource=runLatest：前端先调用 comm_run_latest，把 results/stats 作为参数传入导出
  let results: SampleResult[] | undefined;
  let stats: RunStats | undefined;

  if (includeResults.value && resultsSource.value === "runLatest") {
    const runId = runIdForResults.value.trim();
    if (runId) {
      try {
        const latest = await commRunLatest(runId);
        if (latest.results.length > 0) {
          results = latest.results;
          stats = latest.stats;
        }
      } catch (e: unknown) {
        ElMessage.warning(`runLatest 获取失败，将继续导出但 Results 可能缺失：${String(e ?? "")}`);
      }
    }
  }

  lastDelivery.value = await commExportDeliveryXlsx(
    {
      outPath: outPath.value.trim(),
      includeResults: includeResults.value,
      resultsSource: resultsSource.value,
      results,
      stats,
      profiles,
      points,
    },
    pid,
    did
  );
  last.value = null;
  ElMessage.success(`已交付导出：${lastDelivery.value.outPath}`);
}

setDefaultPath();
</script>

<template>
  <el-card>
    <template #header>
      <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px">
        <div style="font-weight: 600">
          导出 <span v-if="activeDevice">（{{ activeDevice.deviceName }}）</span>
        </div>
        <el-tag v-if="activeDevice" type="info">deviceId={{ activeDevice.deviceId }}</el-tag>
      </div>
    </template>

    <el-form label-width="140px">
      <el-form-item label="导出路径">
        <el-input v-model="outPath" placeholder="可留空：deliveries/{批次}/{deviceName}/通讯地址表.xlsx" />
      </el-form-item>
      <el-form-item label="附加 Results（可选）">
        <el-checkbox v-model="includeResults">写入采集结果 sheet（不影响三表冻结规范）</el-checkbox>
      </el-form-item>
      <el-form-item v-if="includeResults" label="Results 来源">
        <el-select v-model="resultsSource" style="width: 240px">
          <el-option label="appdata（默认：last_results.v1.json）" value="appdata" />
          <el-option label="runLatest（从 runId 的 latest 获取）" value="runLatest" />
        </el-select>
      </el-form-item>
      <el-form-item v-if="includeResults && resultsSource === 'runLatest'" label="runId（用于 latest）">
        <el-input v-model="runIdForResults" placeholder="从 Run 页面复制 runId（UUID）" />
      </el-form-item>
      <el-form-item>
        <el-button @click="setDefaultPath">默认路径</el-button>
        <el-button type="primary" @click="exportNow">导出 XLSX</el-button>
        <el-button type="success" @click="exportDeliveryNow">交付导出（通讯地址表.xlsx）</el-button>
      </el-form-item>
    </el-form>

    <el-divider />

    <el-alert
      v-if="last"
      type="success"
      show-icon
      :closable="false"
      :title="`导出成功：${last.outPath}`"
    />

    <el-alert
      v-if="lastDelivery"
      type="success"
      show-icon
      :closable="false"
      :title="`交付导出成功：${lastDelivery.outPath}`"
    />

    <el-alert
      v-if="lastDelivery && deliveryResultsStatus"
      :type="deliveryResultsStatus === 'written' ? 'success' : deliveryResultsStatus === 'missing' ? 'warning' : 'info'"
      show-icon
      :closable="false"
      :title="`Results: ${deliveryResultsStatus}${deliveryResultsMessage ? ' - ' + deliveryResultsMessage : ''}`"
      style="margin-top: 12px"
    />

    <el-card v-if="last" style="margin-top: 12px">
      <template #header>headers（验收证据）</template>
      <el-row :gutter="12">
        <el-col :span="8">
              <el-card>
                <template #header>TCP通讯地址表</template>
              <pre>{{ JSON.stringify(last.headers.tcp, null, 2) }}</pre>
            </el-card>
          </el-col>
          <el-col :span="8">
            <el-card>
              <template #header>485通讯地址表</template>
              <pre>{{ JSON.stringify(last.headers.rtu, null, 2) }}</pre>
            </el-card>
          </el-col>
          <el-col :span="8">
            <el-card>
              <template #header>通讯参数</template>
              <pre>{{ JSON.stringify(last.headers.params, null, 2) }}</pre>
            </el-card>
          </el-col>
        </el-row>
      </el-card>

    <el-card v-if="lastDelivery" style="margin-top: 12px">
      <template #header>交付版 headers（验收证据）</template>
      <el-row :gutter="12">
        <el-col :span="8">
          <el-card>
            <template #header>TCP通讯地址表</template>
            <pre>{{ JSON.stringify(lastDelivery.headers.tcp, null, 2) }}</pre>
          </el-card>
        </el-col>
        <el-col :span="8">
          <el-card>
            <template #header>485通讯地址表</template>
            <pre>{{ JSON.stringify(lastDelivery.headers.rtu, null, 2) }}</pre>
          </el-card>
        </el-col>
        <el-col :span="8">
          <el-card>
            <template #header>通讯参数</template>
            <pre>{{ JSON.stringify(lastDelivery.headers.params, null, 2) }}</pre>
          </el-card>
        </el-col>
      </el-row>
    </el-card>

    <el-card v-if="last" style="margin-top: 12px">
      <template #header>warnings / diagnostics（可交付诊断）</template>
      <el-alert
        v-if="exportWarnings.length > 0"
        type="warning"
        show-icon
        :closable="false"
        :title="`warnings: ${exportWarnings.length}`"
      />
      <pre v-if="exportWarnings.length > 0">{{ JSON.stringify(exportWarnings, null, 2) }}</pre>
      <el-alert
        v-if="exportDiagnostics"
        type="info"
        show-icon
        :closable="false"
        :title="`diagnostics: durationMs=${exportDiagnostics.durationMs}`"
      />
      <pre v-if="exportDiagnostics">{{ JSON.stringify(exportDiagnostics, null, 2) }}</pre>
    </el-card>

    <el-card v-if="lastDelivery" style="margin-top: 12px">
      <template #header>交付版 warnings / diagnostics</template>
      <el-alert
        v-if="deliveryWarnings.length > 0"
        type="warning"
        show-icon
        :closable="false"
        :title="`warnings: ${deliveryWarnings.length}`"
      />
      <pre v-if="deliveryWarnings.length > 0">{{ JSON.stringify(deliveryWarnings, null, 2) }}</pre>
      <el-alert
        v-if="deliveryDiagnostics"
        type="info"
        show-icon
        :closable="false"
        :title="`diagnostics: durationMs=${deliveryDiagnostics.durationMs}`"
      />
      <pre v-if="deliveryDiagnostics">{{ JSON.stringify(deliveryDiagnostics, null, 2) }}</pre>
    </el-card>
  </el-card>
</template>
