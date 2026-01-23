import { computed, ref, type Ref } from "vue";
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
import { exportDeliveryXlsx, exportXlsx } from "../services/export";
import { loadPoints } from "../services/points";
import { loadProfiles } from "../services/profiles";
import { runLatest } from "../services/run";

interface UseExportXlsxOptions {
  projectId: Ref<string>;
  activeDeviceId: Ref<string>;
}

export function useExportXlsx(options: UseExportXlsxOptions) {
  const outPath = ref<string>("");
  const last = ref<CommExportXlsxResponse | null>(null);
  const lastDelivery = ref<CommExportDeliveryXlsxResponse | null>(null);
  const includeResults = ref(false);
  const resultsSource = ref<DeliveryResultsSource>("appdata");
  const runIdForResults = ref<string>("");
  const isExporting = ref(false);
  const isDeliveryExporting = ref(false);

  const exportWarnings = computed(() => last.value?.warnings ?? []);
  const exportDiagnostics = computed(() => last.value?.diagnostics ?? null);
  const deliveryWarnings = computed(() => lastDelivery.value?.warnings ?? []);
  const deliveryDiagnostics = computed(() => lastDelivery.value?.diagnostics ?? null);
  const deliveryResultsStatus = computed(() => lastDelivery.value?.resultsStatus ?? null);
  const deliveryResultsMessage = computed(() => lastDelivery.value?.resultsMessage ?? null);

  function setDefaultPath() {
    // 允许留空：后端会按 outputDir 自动选择默认导出路径（TASK-32）。
    outPath.value = "";
  }

  async function loadProfilesAndPoints(): Promise<{ pid: string; did: string; profiles: ProfilesV1; points: PointsV1 } | null> {
    const pid = options.projectId.value.trim();
    const did = options.activeDeviceId.value.trim();
    if (!pid || !did) {
      ElMessage.error("未选择设备");
      return null;
    }

    const profiles: ProfilesV1 = await loadProfiles(pid, did);
    const points: PointsV1 = await loadPoints(pid, did);
    if (profiles.profiles.length === 0 || points.points.length === 0) {
      ElMessage.error("profiles/points 为空，请先配置并保存");
      return null;
    }
    return { pid, did, profiles, points };
  }

  async function exportNow() {
    if (isExporting.value) return;
    try {
      const payload = await loadProfilesAndPoints();
      if (!payload) return;
      isExporting.value = true;
      last.value = await exportXlsx(
        { outPath: outPath.value.trim(), profiles: payload.profiles, points: payload.points },
        payload.pid,
        payload.did
      );
      lastDelivery.value = null;
      ElMessage.success(`已导出：${last.value.outPath}`);
    } finally {
      isExporting.value = false;
    }
  }

  async function exportDeliveryNow() {
    if (isDeliveryExporting.value) return;
    try {
      const payload = await loadProfilesAndPoints();
      if (!payload) return;
      isDeliveryExporting.value = true;

      // Results sheet 的来源策略（TASK-24 拍板）。
      let results: SampleResult[] | undefined;
      let stats: RunStats | undefined;

      if (includeResults.value && resultsSource.value === "runLatest") {
        const runId = runIdForResults.value.trim();
        if (runId) {
          try {
            const latest = await runLatest(runId);
            if (latest.results.length > 0) {
              results = latest.results;
              stats = latest.stats;
            }
          } catch (e: unknown) {
            ElMessage.warning(`runLatest 获取失败，将继续导出但 Results 可能缺失：${String(e ?? "")}`);
          }
        }
      }

      lastDelivery.value = await exportDeliveryXlsx(
        {
          outPath: outPath.value.trim(),
          includeResults: includeResults.value,
          resultsSource: resultsSource.value,
          results,
          stats,
          profiles: payload.profiles,
          points: payload.points,
        },
        payload.pid,
        payload.did
      );
      last.value = null;
      ElMessage.success(`已交付导出：${lastDelivery.value.outPath}`);
    } finally {
      isDeliveryExporting.value = false;
    }
  }

  setDefaultPath();

  return {
    outPath,
    last,
    lastDelivery,
    includeResults,
    resultsSource,
    runIdForResults,
    exportWarnings,
    exportDiagnostics,
    deliveryWarnings,
    deliveryDiagnostics,
    deliveryResultsStatus,
    deliveryResultsMessage,
    isExporting,
    isDeliveryExporting,
    setDefaultPath,
    exportNow,
    exportDeliveryNow,
  };
}
