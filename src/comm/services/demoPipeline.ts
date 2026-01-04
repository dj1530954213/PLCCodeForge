import type {
  CommExportDeliveryXlsxResponse,
  CommExportIrV1Response,
  CommDriverKind,
  CommImportUnionXlsxResponse,
  CommRunLatestResponse,
  CommRunStartResponse,
  ConnectionProfile,
  ImportUnionOptions,
  PointsV1,
  ProfilesV1,
  Quality,
  RegisterArea,
  SampleResult,
} from "../api";
import {
  commExportDeliveryXlsx,
  commExportIrV1,
  commImportUnionXlsx,
  commPointsLoad,
  commPointsSave,
  commProfilesLoad,
  commProfilesSave,
  commRunLatest,
  commRunStart,
  commRunStop,
} from "../api";
import type { UnionToCommPointsOutcome } from "../mappers/unionToCommPoints";
import { unionToCommPoints } from "../mappers/unionToCommPoints";

export type DemoPipelineLogStatus = "info" | "ok" | "error";

export interface DemoPipelineLogEntry {
  tsUtc: string;
  step: string;
  status: DemoPipelineLogStatus;
  message: string;
}

export interface DemoPipelineProgressEvent extends DemoPipelineLogEntry {}

export interface DemoPipelineContext {
  signal?: AbortSignal;
  onProgress?: (event: DemoPipelineProgressEvent) => void;
  confirmExportWithConflicts?: (params: { conflictReport: any }) => Promise<boolean>;
}

export interface DemoPipelineParams {
  filePath: string;
  importOptions?: ImportUnionOptions;
  outPath: string;
  driver?: CommDriverKind;
  yieldEvery?: number;
  maxWaitMs?: number;
}

export interface DemoPipelineResult {
  ok: boolean;
  driver: CommDriverKind;
  logs: DemoPipelineLogEntry[];
  error?: { kind: string; message: string; step?: string };
  importResponse?: CommImportUnionXlsxResponse;
  mapped?: UnionToCommPointsOutcome;
  savedProfiles?: ProfilesV1;
  runProfiles?: ProfilesV1;
  runPoints?: PointsV1;
  run?: CommRunStartResponse;
  latest?: CommRunLatestResponse;
  exportResponse: CommExportDeliveryXlsxResponse;
  irResponse?: CommExportIrV1Response;
}

function nowUtc(): string {
  return new Date().toISOString();
}

function abortError(step: string): { kind: string; message: string } {
  return { kind: "DemoPipelineAborted", message: `aborted at step=${step}` };
}

function assertNotAborted(signal: AbortSignal | undefined, step: string) {
  if (signal?.aborted) throw abortError(step);
}

async function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  if (!signal) {
    await new Promise((resolve) => window.setTimeout(resolve, ms));
    return;
  }
  await new Promise<void>((resolve, reject) => {
    const id = window.setTimeout(() => {
      signal.removeEventListener("abort", onAbort);
      resolve();
    }, ms);
    const onAbort = () => {
      window.clearTimeout(id);
      reject(abortError("sleep"));
    };
    signal.addEventListener("abort", onAbort, { once: true });
  });
}

function mergeProfiles(existing: ProfilesV1, imported: ProfilesV1): ProfilesV1 {
  const out: ProfilesV1 = { schemaVersion: 1, profiles: [] };
  const seen = new Set<string>();

  const keyOf = (p: any) => `${p.protocolType}|${p.channelName}|${p.deviceId}`;

  for (const p of existing.profiles ?? []) {
    const key = keyOf(p);
    if (seen.has(key)) continue;
    seen.add(key);
    out.profiles.push(p);
  }
  for (const p of imported.profiles ?? []) {
    const key = keyOf(p);
    if (seen.has(key)) continue;
    seen.add(key);
    out.profiles.push(p);
  }
  return out;
}

function requiredAreaForPoint(p: any): RegisterArea {
  return p.dataType === "Bool" ? "Coil" : "Holding";
}

function buildDemoProfiles(areas: RegisterArea[]): ProfilesV1 {
  const profiles: ConnectionProfile[] = [];

  const makeTcp = (channelName: string, readArea: RegisterArea): ConnectionProfile => ({
    protocolType: "TCP",
    channelName,
    deviceId: 1,
    readArea,
    startAddress: 0,
    length: 5000,
    ip: "127.0.0.1",
    port: 502,
    timeoutMs: 1000,
    retryCount: 0,
    pollIntervalMs: 500,
  });

  for (const area of areas) {
    const prefix = area === "Coil" || area === "Discrete" ? "demo-coil" : "demo-holding";
    profiles.push(makeTcp(`${prefix}-ok`, area));
    profiles.push(makeTcp(`${prefix}-timeout`, area));
    profiles.push(makeTcp(`${prefix}-decode`, area));
  }

  return { schemaVersion: 1, profiles };
}

function buildDemoRunConfig(points: PointsV1): { points: PointsV1; profiles: ProfilesV1 } {
  const src = points.points ?? [];
  if (src.length < 2) {
    throw { kind: "DemoConfigError", message: "点位数量不足（至少需要 2 个点位才能演示 OK/Timeout/DecodeError）" };
  }

  const areasUsed = new Set<RegisterArea>();
  const outPoints = src.map((p) => {
    const area = requiredAreaForPoint(p);
    areasUsed.add(area);
    return { ...p };
  });

  let special = 0;
  for (const p of outPoints) {
    const area = requiredAreaForPoint(p);
    const prefix = area === "Coil" || area === "Discrete" ? "demo-coil" : "demo-holding";
    if (special === 0) {
      p.channelName = `${prefix}-timeout`;
      special += 1;
      continue;
    }
    if (special === 1) {
      p.channelName = `${prefix}-decode`;
      special += 1;
      continue;
    }
    p.channelName = `${prefix}-ok`;
  }

  const demoProfiles = buildDemoProfiles(Array.from(areasUsed));
  return { points: { schemaVersion: 1, points: outPoints }, profiles: demoProfiles };
}

function qualitiesHit(results: SampleResult[]): Quality[] {
  const observed = new Set(results.map((r) => r.quality));
  const allowed: Quality[] = ["Ok", "Timeout", "DecodeError"];
  return allowed.filter((q) => observed.has(q));
}

function placeholderExportResponse(outPath: string, message: string): CommExportDeliveryXlsxResponse {
  return {
    outPath,
    headers: { tcp: [], rtu: [], params: [] },
    resultsStatus: "missing",
    resultsMessage: message,
  };
}

export async function runDemoPipeline(params: DemoPipelineParams, ctx: DemoPipelineContext = {}): Promise<DemoPipelineResult> {
  const logs: DemoPipelineLogEntry[] = [];
  const emit = (step: string, status: DemoPipelineLogStatus, message: string) => {
    const entry: DemoPipelineLogEntry = { tsUtc: nowUtc(), step, status, message };
    logs.push(entry);
    ctx.onProgress?.(entry);
  };

  const driver: CommDriverKind = params.driver ?? "Mock";
  let runId: string | null = null;
  let runStopped = false;
  let currentStep: string | null = null;

  let importResponse: CommImportUnionXlsxResponse | undefined;
  let mapped: UnionToCommPointsOutcome | undefined;
  let savedProfiles: ProfilesV1 | undefined;
  let runProfiles: ProfilesV1 | undefined;
  let runPoints: PointsV1 | undefined;
  let run: CommRunStartResponse | undefined;
  let latest: CommRunLatestResponse | undefined;
  let exportResponse: CommExportDeliveryXlsxResponse = placeholderExportResponse(params.outPath, "not started");
  let irResponse: CommExportIrV1Response | undefined;
  let error: { kind: string; message: string; step?: string } | undefined;
  let exportAttempted = false;
  let irAttempted = false;

  const runStep = async <T,>(step: string, fn: () => Promise<T>): Promise<T> => {
    currentStep = step;
    assertNotAborted(ctx.signal, step);
    try {
      return await fn();
    } catch (e: unknown) {
      const kind = typeof (e as any)?.kind === "string" ? (e as any).kind : "Error";
      const message = typeof (e as any)?.message === "string" ? (e as any).message : String(e ?? "unknown error");
      emit(step, "error", `${kind}: ${message}`);
      throw e;
    }
  };

  const toError = (e: unknown, step?: string): { kind: string; message: string; step?: string } => {
    const kind = typeof (e as any)?.kind === "string" ? (e as any).kind : "Error";
    const message = typeof (e as any)?.message === "string" ? (e as any).message : String(e ?? "unknown error");
    return { kind, message, step };
  };

  try {
    emit("import", "info", "开始：comm_import_union_xlsx");
    importResponse = await runStep("import", async () => commImportUnionXlsx(params.filePath, params.importOptions));
    const importResp = importResponse;
    if (!importResp) {
      throw { kind: "DemoPipelineInternalError", message: "importResponse is missing after import step" };
    }
    emit(
      "import",
      "ok",
      `导入成功：points=${importResp.points.points.length}, profiles=${importResp.profiles.profiles.length}, warnings=${importResp.warnings.length}`
    );

    emit("load-existing", "info", "读取已有 points/profiles（用于 pointKey 复用与冲突检测）");
    const existingPoints = await runStep("load-existing", async () =>
      commPointsLoad().catch(() => ({ schemaVersion: 1, points: [] }))
    );
    const existingProfiles = await runStep("load-existing", async () =>
      commProfilesLoad().catch(() => ({ schemaVersion: 1, profiles: [] }))
    );
    emit("load-existing", "ok", `existing: points=${existingPoints.points.length}, profiles=${existingProfiles.profiles.length}`);

    emit("map", "info", "映射：ImportUnion -> CommPoint（含 pointKey 复用）");
    mapped = await runStep("map", async () =>
      unionToCommPoints({
        imported: importResp.points,
        importedProfiles: importResp.profiles,
        existing: existingPoints,
        existingProfiles,
        yieldEvery: params.yieldEvery ?? 500,
      })
    );
    const mappedOut = mapped;
    if (!mappedOut) {
      throw { kind: "DemoPipelineInternalError", message: "mapped is missing after map step" };
    }
    emit(
      "map",
      "ok",
      `映射完成：outPoints=${mappedOut.points.points.length}, reused=${mappedOut.reusedPointKeys}, created=${mappedOut.createdPointKeys}, skipped=${mappedOut.skipped}, mapperWarnings=${mappedOut.warnings.length}`
    );

    emit("save", "info", "落盘：comm_points_save + comm_profiles_save");
    await runStep("save", async () => commPointsSave(mappedOut.points));
    const savedProfilesOut = mergeProfiles(existingProfiles, importResp.profiles);
    savedProfiles = savedProfilesOut;
    if (savedProfilesOut.profiles.length > 0) {
      await runStep("save", async () => commProfilesSave(savedProfilesOut));
    }
    emit("save", "ok", `落盘完成：points=${mappedOut.points.points.length}, profiles=${savedProfilesOut.profiles.length}`);

    emit("run_config", "info", `准备 run 配置（driver=${driver}）`);
    const cfg = await runStep("run_config", async () => {
      if (driver === "Mock") {
        const demo = buildDemoRunConfig(mappedOut.points);
        return { profiles: demo.profiles, points: demo.points };
      }
      return { profiles: savedProfilesOut, points: mappedOut.points };
    });
    runProfiles = cfg.profiles;
    runPoints = cfg.points;
    emit(
      "run_config",
      "ok",
      `run config ready: profiles=${runProfiles?.profiles.length ?? 0}, points=${runPoints?.points.length ?? 0}`
    );

    emit("run_start", "info", `启动采集：comm_run_start(driver=${driver}, 后台运行)`);
    run = await runStep("run_start", async () =>
      commRunStart({
        driver,
        profiles: runProfiles,
        points: runPoints,
      })
    );
    runId = run.runId;
    emit("run_start", "ok", `runId=${runId}`);

    emit("latest", "info", "等待采集并轮询 latest（最多 5s）");
    const maxWaitMs = Math.max(1000, params.maxWaitMs ?? 5000);
    const started = Date.now();
    let hit: string[] = [];
    while (Date.now() - started < maxWaitMs) {
      assertNotAborted(ctx.signal, "latest");
      await sleep(1000, ctx.signal);
      const next = await runStep("latest", async () => commRunLatest(runId!));
      latest = next;
      if (driver === "Mock") {
        hit = qualitiesHit(next.results);
        if (hit.length >= 2) break;
      } else {
        hit = Array.from(new Set(next.results.map((r) => r.quality)));
        if (next.results.length > 0) break;
      }
    }
    if (!latest) {
      throw { kind: "DemoRunError", message: "comm_run_latest 未返回结果" };
    }
    emit("latest", "ok", `latest: results=${latest.results.length}, qualities=${hit.join(",") || "(none)"}`);
    if (driver === "Mock" && hit.length < 2) {
      throw {
        kind: "DemoRunError",
        message: `未观察到至少两种 quality（Ok/Timeout/DecodeError）；当前=${hit.join(",") || "(none)"}`,
      };
    }

    emit("run_stop", "info", "停止采集：comm_run_stop（要求 1s 内生效）");
    await runStep("run_stop", async () => commRunStop(runId!));
    runStopped = true;
    emit("run_stop", "ok", "已停止");

    const conflictCount = mappedOut.conflictReport?.conflicts?.length ?? 0;
    if (conflictCount > 0) {
      emit("gate", "info", `检测到 conflicts=${conflictCount}，需要二次确认才能继续导出交付表`);
      if (ctx.confirmExportWithConflicts) {
        const ok = await ctx.confirmExportWithConflicts({ conflictReport: mappedOut.conflictReport });
        if (!ok) {
          throw { kind: "DemoPipelineCancelled", message: "用户取消：未确认冲突风险" };
        }
        emit("gate", "ok", "已确认冲突风险，继续导出");
      }
    }

    emit("export", "info", "交付导出：comm_export_delivery_xlsx(includeResults=true, resultsSource=runLatest)");
    exportAttempted = true;
    exportResponse = await runStep("export", async () =>
      commExportDeliveryXlsx({
        outPath: params.outPath,
        includeResults: true,
        resultsSource: "runLatest",
        results: latest?.results,
        stats: latest?.stats,
        profiles: savedProfilesOut,
        points: mappedOut.points,
      })
    );

    emit(
      "export",
      exportResponse.resultsStatus === "written" ? "ok" : "error",
      `outPath=${exportResponse.outPath}, resultsStatus=${exportResponse.resultsStatus ?? "?"}`
    );
    if (exportResponse.resultsStatus !== "written") {
      throw {
        kind: "DemoExportError",
        message: `Results sheet 未写入：resultsStatus=${exportResponse.resultsStatus ?? "(missing)"}; ${exportResponse.resultsMessage ?? ""}`.trim(),
      };
    }

    emit("ir", "info", "导出 CommIR v1：comm_export_ir_v1（跨模块交接产物）");
    irAttempted = true;
    irResponse = await runStep("ir", async () =>
      commExportIrV1({
        unionXlsxPath: params.filePath,
        resultsSource: "runLatest",
        profiles: savedProfilesOut,
        points: mappedOut.points,
        latestResults: latest?.results,
        stats: latest?.stats,
        decisions: mappedOut.decisions,
        conflictReport: mappedOut.conflictReport,
      })
    );
    emit("ir", "ok", `irPath=${irResponse.irPath}`);

    return {
      ok: true,
      driver,
      logs,
      importResponse: importResp,
      mapped: mappedOut,
      savedProfiles: savedProfilesOut,
      runProfiles,
      runPoints,
      run,
      latest,
      exportResponse,
      irResponse,
    };
  } catch (e: unknown) {
    error = toError(e, currentStep ?? undefined);
  } finally {
    if (runId && !runStopped) {
      emit("cleanup", "info", `尝试停止 runId=${runId}`);
      try {
        await commRunStop(runId);
        emit("cleanup", "ok", "已停止");
      } catch {
        // ignore
      }
    }

    if (!exportAttempted) {
      emit("export", "info", "交付导出（best-effort）：comm_export_delivery_xlsx");
      exportAttempted = true;
      try {
        exportResponse = await commExportDeliveryXlsx({
          outPath: params.outPath,
          includeResults: true,
          resultsSource: "runLatest",
          results: latest?.results,
          stats: latest?.stats,
          profiles: savedProfiles,
          points: mapped?.points,
        });
        emit(
          "export",
          exportResponse.resultsStatus === "written" ? "ok" : "error",
          `outPath=${exportResponse.outPath}, resultsStatus=${exportResponse.resultsStatus ?? "?"}`
        );
      } catch (e: unknown) {
        const err = toError(e, "export");
        if (!error) error = err;
        exportResponse = placeholderExportResponse(params.outPath, `${err.kind}: ${err.message}`);
      }
    }

    if (!irAttempted) {
      emit("ir", "info", "导出 CommIR v1（best-effort）：comm_export_ir_v1");
      irAttempted = true;
      try {
        const fallbackPoints = mapped?.points ?? { schemaVersion: 1, points: [] };
        const fallbackProfiles = savedProfiles ?? importResponse?.profiles ?? { schemaVersion: 1, profiles: [] };
        irResponse = await commExportIrV1({
          unionXlsxPath: params.filePath,
          resultsSource: "runLatest",
          profiles: fallbackProfiles,
          points: fallbackPoints,
          latestResults: latest?.results,
          stats: latest?.stats,
          decisions: mapped?.decisions,
          conflictReport: mapped?.conflictReport,
        });
        emit("ir", "ok", `irPath=${irResponse.irPath}`);
      } catch (e: unknown) {
        const err = toError(e, "ir");
        if (!error) error = err;
        emit("ir", "error", `${err.kind}: ${err.message}`);
      }
    }
  }

  return {
    ok: false,
    driver,
    logs,
    error,
    importResponse,
    mapped,
    savedProfiles,
    runProfiles,
    runPoints,
    run,
    latest,
    exportResponse,
    irResponse,
  };
}
