import { computed, ref, watch, type ComputedRef, type Ref } from "vue";
import type {
  CommRunError,
  CommRunLatestResponse,
  ConnectionProfile,
  PointsV1,
  SampleResult,
} from "../api";
import { buildPlan, runLatestObs, runStartObs, runStopObs } from "../services/run";
import { notifyError, notifySuccess } from "../services/notify";
import type { CommWorkspaceRuntime } from "./useWorkspaceRuntime";

type RunUiState = "idle" | "starting" | "running" | "stopping" | "error";
type LogLevel = "info" | "success" | "warning" | "error";

interface LogEntry {
  ts: string;
  step: string;
  level: LogLevel;
  message: string;
}

interface UsePointsRunOptions {
  projectId: Ref<string>;
  activeDeviceId: Ref<string>;
  activeProfile: ComputedRef<ConnectionProfile | null>;
  points: Ref<PointsV1>;
  pointsRevision: Ref<number>;
  showAllValidation: Ref<boolean>;
  getValidationError: () => string | null;
  syncFromGridAndMapAddresses: () => Promise<void>;
  onLatestResults: (results: SampleResult[]) => void;
  workspaceRuntime: CommWorkspaceRuntime;
}

export function usePointsRun(options: UsePointsRunOptions) {
  const runUiState = ref<RunUiState>("idle");
  const runId = ref<string | null>(null);
  const latest = ref<CommRunLatestResponse | null>(null);
  const runError = ref<CommRunError | null>(null);
  const logs = ref<LogEntry[]>([]);
  const pollMs = ref<number>(1000);
  const autoRestartPending = ref(false);
  const resumeAfterFix = ref(false);
  const runPointsRevision = ref<number | null>(null);

  const configChangedDuringRun = computed(() => {
    return isRunning.value && runPointsRevision.value !== null && options.pointsRevision.value !== runPointsRevision.value;
  });

  const isStarting = computed(() => runUiState.value === "starting");
  const isRunning = computed(() => runUiState.value === "running");
  const isStopping = computed(() => runUiState.value === "stopping");

  let timer: number | null = null;
  function clearTimer() {
    if (timer !== null) {
      window.clearInterval(timer);
      timer = null;
    }
  }

  function pushLog(step: string, level: LogLevel, message: string) {
    logs.value.unshift({ ts: new Date().toISOString(), step, level, message });
    if (logs.value.length > 20) logs.value.length = 20;
  }

  let autoRestartTimer: number | null = null;
  function clearAutoRestartTimer() {
    if (autoRestartTimer !== null) {
      window.clearTimeout(autoRestartTimer);
      autoRestartTimer = null;
    }
    autoRestartPending.value = false;
  }

  function formatRunErrorKind(kind: CommRunError["kind"]): string {
    switch (kind) {
      case "ConfigError":
        return "配置错误";
      case "RunNotFound":
        return "运行不存在";
      case "InternalError":
        return "内部错误";
      default:
        return "未知错误";
    }
  }

  function formatRunErrorMessage(message?: string): string {
    const raw = String(message ?? "").trim();
    if (!raw) return "未知错误";
    if (/[\u4e00-\u9fa5]/.test(raw)) return raw;
    if (raw === "profiles is empty") return "连接配置为空";
    if (raw === "points is empty") return "点位列表为空";
    if (raw === "invalid points/profiles configuration") return "点位或连接配置无效";
    return raw;
  }

  function formatRunErrorTitle(err: CommRunError): string {
    return `${formatRunErrorKind(err.kind)}：${formatRunErrorMessage(err.message)}`;
  }

  const runErrorTitle = computed(() => (runError.value ? formatRunErrorTitle(runError.value) : ""));

  function makeUiConfigError(message: string): CommRunError {
    return {
      kind: "ConfigError",
      message,
      details: {
        projectId: options.projectId.value,
        deviceId: options.activeDeviceId.value || undefined,
      },
    };
  }

  type AutoRestartMode = "restart" | "start";

  function scheduleAutoRestart(reason: string, mode: AutoRestartMode) {
    if (isStarting.value || isStopping.value) return;
    if (mode === "restart" && !isRunning.value) return;
    if (mode === "start" && isRunning.value) return;
    clearAutoRestartTimer();
    autoRestartPending.value = true;
    autoRestartTimer = window.setTimeout(() => {
      autoRestartTimer = null;
      autoRestartPending.value = false;
      pushLog("run_restart", "info", `自动重启：${reason}`);
      if (mode === "restart") {
        if (!isRunning.value) return;
        void restartRun();
      } else {
        if (isRunning.value) return;
        void startRun();
      }
    }, 600);
  }

  async function startPolling() {
    clearTimer();
    timer = window.setInterval(pollLatest, pollMs.value);
    await pollLatest();
    pushLog("poll", "info", `轮询间隔 ${pollMs.value}ms`);
  }

  async function startRun() {
    if (runUiState.value === "starting" || runUiState.value === "running") return;
    clearAutoRestartTimer();
    runUiState.value = "starting";
    runError.value = null;
    latest.value = null;
    runPointsRevision.value = null;
    pushLog("run_start", "info", "点击启动");

    options.showAllValidation.value = true;
    await options.syncFromGridAndMapAddresses();
    const invalid = options.getValidationError();
    if (invalid) {
      runError.value = makeUiConfigError(invalid);
      pushLog("run_start", "error", formatRunErrorTitle(runError.value));
      notifyError(invalid);
      runUiState.value = "error";
      return;
    }

    try {
      const profile = options.activeProfile.value;
      if (!profile) {
        const err = makeUiConfigError("未选择连接");
        runError.value = err;
        pushLog("run_start", "error", formatRunErrorTitle(err));
        notifyError(formatRunErrorMessage(err.message));
        runUiState.value = "error";
        return;
      }

      const channelPoints = options.points.value.points.filter((p) => p.channelName === profile.channelName);
      if (channelPoints.length === 0) {
        const err = makeUiConfigError("点位为空：请先新增点位并保存");
        runError.value = err;
        pushLog("run_start", "error", formatRunErrorTitle(err));
        notifyError(formatRunErrorMessage(err.message));
        runUiState.value = "error";
        return;
      }

      const planToUse = await buildPlan(
        { profiles: { schemaVersion: 1, profiles: [profile] }, points: { schemaVersion: 1, points: channelPoints } },
        options.projectId.value,
        options.activeDeviceId.value
      );
      pushLog("run_start", "info", `读取计划生成完成：${planToUse.jobs.length} 个任务`);

      pushLog("run_start", "info", "调用 comm_run_start_obs（后端 spawn，不阻塞 UI）");
      const resp = await runStartObs(
        {
          profiles: { schemaVersion: 1, profiles: [profile] },
          points: { schemaVersion: 1, points: channelPoints },
          plan: planToUse,
        },
        options.projectId.value,
        options.activeDeviceId.value
      );

      if (!resp.ok || !resp.runId) {
        const err =
          resp.error ??
          ({
            kind: "InternalError",
            message: "comm_run_start_obs 失败（ok=false 且 error 为空）",
          } as CommRunError);
        runError.value = err;
        pushLog("run_start", "error", formatRunErrorTitle(err));
        notifyError(formatRunErrorTitle(err));
        runUiState.value = "error";
        return;
      }

      runId.value = resp.runId;
      runUiState.value = "running";
      runPointsRevision.value = options.pointsRevision.value;
      pushLog("run_start", "success", `采集已启动：运行ID=${resp.runId}`);
      notifySuccess(`采集已启动：运行ID=${resp.runId}`);

      await startPolling();
    } catch (e: unknown) {
      const err = makeUiConfigError(String((e as any)?.message ?? e ?? "未知错误"));
      runError.value = err;
      pushLog("run_start", "error", formatRunErrorTitle(err));
      notifyError(formatRunErrorTitle(err));
      runUiState.value = "error";
    }
  }

  async function pollLatest() {
    const id = runId.value;
    if (!id) return;
    const resp = await runLatestObs(id);
    if (!resp.ok || !resp.value) {
      const err =
        resp.error ??
        ({
          kind: "InternalError",
          message: "comm_run_latest_obs 失败（ok=false 且 error 为空）",
        } as CommRunError);
      runError.value = err;
      pushLog("run_latest", "error", formatRunErrorTitle(err));
      runUiState.value = "error";
      clearTimer();
      if (runId.value) {
        void runStopObs(id, options.projectId.value).catch(() => {
          // ignore stop errors after latest failure
        });
      }
      return;
    }

    latest.value = resp.value;
    options.onLatestResults(resp.value.results);
    pushLog("run_latest", "success", `采集成功：总数 ${resp.value.stats.total} / 正常 ${resp.value.stats.ok}`);
  }

  async function stopRun(reason: "manual" | "restart" | "validation" = "manual") {
    if (!runId.value || runUiState.value !== "running") return;
    clearAutoRestartTimer();
    if (reason !== "validation") {
      resumeAfterFix.value = false;
    }
    runUiState.value = "stopping";
    const id = runId.value;
    const reasonLabel =
      reason === "validation" ? "配置无效自动停止" : reason === "restart" ? "重启前停止" : "点击停止";
    pushLog("run_stop", "info", reasonLabel);

    try {
      const resp = await runStopObs(id, options.projectId.value);
      if (!resp.ok) {
        const err =
          resp.error ??
          ({
            kind: "InternalError",
            message: "comm_run_stop_obs 失败（ok=false 且 error 为空）",
          } as CommRunError);
        runError.value = err;
        pushLog("run_stop", "error", formatRunErrorTitle(err));
        notifyError(formatRunErrorTitle(err));
        runUiState.value = "error";
        return;
      }
      pushLog("run_stop", "success", "已停止");
      notifySuccess("采集已停止");
      runUiState.value = "idle";
      clearTimer();
    } catch (e: unknown) {
      const err = makeUiConfigError(String((e as any)?.message ?? e ?? "未知错误"));
      runError.value = err;
      pushLog("run_stop", "error", formatRunErrorTitle(err));
      notifyError(formatRunErrorTitle(err));
      runUiState.value = "error";
    }
  }

  async function restartRun() {
    if (!isRunning.value || !runId.value) return;
    clearAutoRestartTimer();
    pushLog("run_restart", "info", "点击重启");
    await stopRun("restart");
    await startRun();
  }

  function markPointsChanged() {
    options.pointsRevision.value += 1;

    const invalid = options.getValidationError();
    if (invalid) {
      if (isRunning.value) {
        resumeAfterFix.value = true;
        runError.value = makeUiConfigError(invalid);
        void stopRun("validation");
      }
      return;
    }

    if (resumeAfterFix.value && !isRunning.value) {
      resumeAfterFix.value = false;
      runError.value = null;
      scheduleAutoRestart("配置已修复", "start");
      return;
    }

    scheduleAutoRestart("配置变更", "restart");
  }

  watch(pollMs, (v) => {
    if (!isRunning.value) return;
    clearTimer();
    timer = window.setInterval(pollLatest, v);
    pushLog("poll", "info", `轮询间隔变更：${v}ms`);
  });

  watch(latest, (value) => {
    options.workspaceRuntime.stats.value = value?.stats ?? null;
    options.workspaceRuntime.updatedAtUtc.value = value?.updatedAtUtc ?? "";
  });

  function dispose() {
    clearTimer();
    clearAutoRestartTimer();
  }

  return {
    runUiState,
    runId,
    latest,
    runError,
    runErrorTitle,
    logs,
    pollMs,
    autoRestartPending,
    resumeAfterFix,
    configChangedDuringRun,
    isStarting,
    isRunning,
    isStopping,
    pushLog,
    startRun,
    stopRun,
    restartRun,
    markPointsChanged,
    dispose,
  };
}
