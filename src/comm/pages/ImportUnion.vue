<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { useRouter } from "vue-router";

import type {
  AddressBase,
  CommBridgeConsumeCheckResponse,
  CommBridgeToPlcImportV1Response,
  CommBridgeExportImportResultStubV1Response,
  CommEvidenceVerifyV1Response,
  CommMergeImportSourcesV1Response,
  CommUnifiedExportPlcImportStubV1Response,
  CommDriverKind,
  CommImportUnionXlsxResponse,
  CommWarning,
  ImportUnionOptions,
  ImportUnionThrownError,
  ProfilesV1,
} from "../api";
import {
  commBridgeConsumeCheck,
  commBridgeToPlcImportV1,
  commBridgeExportImportResultStubV1,
  commEvidenceVerifyV1,
  commMergeImportSourcesV1,
  commUnifiedExportPlcImportStubV1,
  commConfigLoad,
  commConfigSave,
  commImportUnionXlsx,
  commPointsLoad,
  commPointsSave,
  commProfilesLoad,
  commProfilesSave,
} from "../api";
import { unionToCommPoints } from "../mappers/unionToCommPoints";
import type { DemoPipelineLogEntry, DemoPipelineResult } from "../services/demoPipeline";
import { runDemoPipeline } from "../services/demoPipeline";
import type { EvidencePackOutcome } from "../services/evidencePack";
import { buildEvidencePack } from "../services/evidencePack";

const router = useRouter();

const filePath = ref<string>("");
const strict = ref<boolean>(true);
const sheetName = ref<string>("联合点表");
const addressBase = ref<AddressBase>("one");
const demoOutPath = ref<string>("");
const wizardDriver = ref<CommDriverKind>("Mock");

const outputDir = ref<string>("");
const configSaving = ref(false);

const last = ref<CommImportUnionXlsxResponse | null>(null);
const lastError = ref<ImportUnionThrownError | null>(null);
const saving = ref(false);

const wizardRunning = ref(false);
const wizardController = ref<AbortController | null>(null);
const wizardLogs = ref<DemoPipelineLogEntry[]>([]);
const wizardResult = ref<DemoPipelineResult | null>(null);
const evidenceOutcome = ref<EvidencePackOutcome | null>(null);
const evidenceVerifyPath = ref<string>("");
const evidenceVerifyResponse = ref<CommEvidenceVerifyV1Response | null>(null);
const evidenceVerifying = ref(false);
const plcBridgeResponse = ref<CommBridgeToPlcImportV1Response | null>(null);
const bridgeCheckResponse = ref<CommBridgeConsumeCheckResponse | null>(null);
const importResultStubResponse = ref<CommBridgeExportImportResultStubV1Response | null>(null);
const unifiedImportResponse = ref<CommMergeImportSourcesV1Response | null>(null);
const plcImportStubResponse = ref<CommUnifiedExportPlcImportStubV1Response | null>(null);

const savedSummary = ref<{
  points: number;
  profiles: number;
  reusedPointKeys: number;
  createdPointKeys: number;
  skipped: number;
} | null>(null);
const mapperWarnings = ref<CommWarning[]>([]);
const mapperDecisions = ref<any[]>([]);
const mapperConflictReport = ref<any | null>(null);

const decisionsOnlyCreated = ref(false);
const decisionsOnlyWarn = ref(false);

const conflictConfirmVisible = ref(false);
const conflictConfirmChecked = ref(false);
const conflictConfirmReport = ref<any | null>(null);
let conflictConfirmResolve: ((value: boolean) => void) | null = null;

const warnings = computed(() => last.value?.warnings ?? []);
const diagnostics = computed(() => last.value?.diagnostics ?? lastError.value?.diagnostics ?? null);
const allWarnings = computed(() => [...warnings.value, ...mapperWarnings.value]);
const allDecisions = computed(() => mapperDecisions.value ?? []);
const evidenceManifest = computed(() => evidenceOutcome.value?.manifest ?? null);

const evidenceManifestSummary = computed(() => {
  const m = evidenceManifest.value as any;
  if (!m || typeof m !== "object") return null;
  return {
    appName: m.app?.appName ?? "",
    appVersion: m.app?.appVersion ?? "",
    gitCommit: m.app?.gitCommit ?? "",
    driver: m.run?.driver ?? "",
    durationMs: m.run?.durationMs ?? 0,
    points: m.counts?.points ?? 0,
    conflicts: m.counts?.conflicts ?? 0,
    results: m.counts?.results ?? 0,
    headersDigest: m.outputs?.headersDigest ?? "",
  };
});

const decisionWarnKeySet = computed(() => {
  const set = new Set<string>();
  for (const w of allWarnings.value) {
    if (w.pointKey) set.add(`pk:${w.pointKey}`);
    if (w.hmiName && w.channelName) set.add(`hc:${w.hmiName}|${w.channelName}`);
  }
  return set;
});

const filteredDecisions = computed(() => {
  let out = allDecisions.value;
  if (decisionsOnlyCreated.value) {
    out = out.filter((d) => d.reuseDecision === "created:new");
  }
  if (decisionsOnlyWarn.value) {
    out = out.filter((d) => {
      if (d.pointKey && decisionWarnKeySet.value.has(`pk:${d.pointKey}`)) return true;
      if (d.hmiName && d.channelName && decisionWarnKeySet.value.has(`hc:${d.hmiName}|${d.channelName}`)) return true;
      return false;
    });
  }
  return out;
});

const diagnosticSummary = computed(() => {
  const counts = {
    reusedKeyV2: 0,
    reusedKeyV2NoDevice: 0,
    reusedKeyV1: 0,
    createdNew: 0,
  };
  for (const d of mapperDecisions.value ?? []) {
    switch (d.reuseDecision) {
      case "reused:keyV2":
        counts.reusedKeyV2 += 1;
        break;
      case "reused:keyV2NoDevice":
        counts.reusedKeyV2NoDevice += 1;
        break;
      case "reused:keyV1":
        counts.reusedKeyV1 += 1;
        break;
      case "created:new":
        counts.createdNew += 1;
        break;
    }
  }
  const conflicts = mapperConflictReport.value?.conflicts ?? [];
  const keyV1Conflicts = conflicts.filter((c: any) => c.keyType === "keyV1").length;
  return { ...counts, conflicts: conflicts.length, keyV1Conflicts };
});

function formatError(e: unknown): { kind: string; message: string } {
  if (typeof e === "object" && e !== null) {
    const any = e as any;
    if (typeof any.kind === "string" && typeof any.message === "string") {
      return { kind: any.kind, message: any.message };
    }
    if (typeof any.message === "string") {
      return { kind: "Error", message: any.message };
    }
  }
  if (typeof e === "string") return { kind: "Error", message: e };
  return { kind: "Error", message: String(e ?? "unknown error") };
}

async function setDefaultDemoOutPath() {
  demoOutPath.value = "";
}

function downloadJson(filename: string, value: unknown) {
  const text = JSON.stringify(value, null, 2);
  const blob = new Blob([text], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

async function openPath(path: string) {
  try {
    const mod = await import("@tauri-apps/plugin-opener");
    const open = (mod as any).open;
    if (typeof open === "function") {
      await open(path);
      return;
    }
  } catch {
    // ignore
  }
  ElMessage.info(`无法打开：${path}`);
}

function durationMsFromLogs(logs: DemoPipelineLogEntry[]): number {
  if (!Array.isArray(logs) || logs.length < 2) return 0;
  const start = Date.parse(logs[0].tsUtc);
  const end = Date.parse(logs[logs.length - 1].tsUtc);
  if (!Number.isFinite(start) || !Number.isFinite(end)) return 0;
  return Math.max(0, end - start);
}

function decisionsCountFromOutcome(mapped: any): {
  reusedKeyV2: number;
  reusedKeyV2NoDevice: number;
  reusedKeyV1: number;
  createdNew: number;
} {
  const out = { reusedKeyV2: 0, reusedKeyV2NoDevice: 0, reusedKeyV1: 0, createdNew: 0 };
  const decisions = mapped?.decisions;
  if (!Array.isArray(decisions)) return out;
  for (const d of decisions) {
    switch (d?.reuseDecision) {
      case "reused:keyV2":
        out.reusedKeyV2 += 1;
        break;
      case "reused:keyV2NoDevice":
        out.reusedKeyV2NoDevice += 1;
        break;
      case "reused:keyV1":
        out.reusedKeyV1 += 1;
        break;
      case "created:new":
        out.createdNew += 1;
        break;
    }
  }
  return out;
}

function manifestDriver(driver: CommDriverKind): string {
  switch (driver) {
    case "Mock":
      return "mock";
    case "Tcp":
      return "modbus_tcp";
    case "Rtu485":
      return "modbus_rtu";
  }
}

function connectionSnapshotFromProfiles(driver: CommDriverKind, profiles: ProfilesV1 | undefined): unknown {
  const list = profiles?.profiles ?? [];
  if (driver === "Tcp") {
    return {
      protocol: "TCP",
      channels: list
        .filter((p: any) => p.protocolType === "TCP")
        .map((p: any) => ({
          channelName: p.channelName,
          deviceId: p.deviceId,
          readArea: p.readArea,
          startAddress: p.startAddress,
          length: p.length,
          ip: p.ip,
          port: p.port,
        })),
    };
  }
  if (driver === "Rtu485") {
    return {
      protocol: "485",
      channels: list
        .filter((p: any) => p.protocolType === "485")
        .map((p: any) => ({
          channelName: p.channelName,
          deviceId: p.deviceId,
          readArea: p.readArea,
          startAddress: p.startAddress,
          length: p.length,
          serialPort: p.serialPort,
          baudRate: p.baudRate,
          parity: p.parity,
          dataBits: p.dataBits,
          stopBits: p.stopBits,
        })),
    };
  }
  return null;
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

async function importNow() {
  last.value = null;
  lastError.value = null;
  savedSummary.value = null;
  mapperWarnings.value = [];
  mapperDecisions.value = [];
  mapperConflictReport.value = null;
  plcBridgeResponse.value = null;
  bridgeCheckResponse.value = null;
  importResultStubResponse.value = null;
  unifiedImportResponse.value = null;
  plcImportStubResponse.value = null;

  wizardLogs.value = [];
  wizardResult.value = null;
  evidenceOutcome.value = null;

  if (!filePath.value.trim()) {
    ElMessage.error("请填写联合 xlsx 文件路径");
    return;
  }

  const options: ImportUnionOptions = {
    strict: strict.value,
    sheetName: sheetName.value.trim() ? sheetName.value.trim() : undefined,
    addressBase: addressBase.value,
  };

  try {
    last.value = await commImportUnionXlsx(filePath.value.trim(), options);
    ElMessage.success(
      `导入成功：points=${last.value.points.points.length}, profiles=${last.value.profiles.profiles.length}, warnings=${warnings.value.length}`
    );
  } catch (e: unknown) {
    lastError.value = e as ImportUnionThrownError;
    ElMessage.error(`${lastError.value.kind}: ${lastError.value.message}`);
  }
}

async function loadConfig() {
  try {
    const cfg = await commConfigLoad();
    outputDir.value = cfg.outputDir ?? "";
  } catch {
    // ignore
  }
}

async function saveConfig() {
  if (configSaving.value) return;
  configSaving.value = true;
  try {
    await commConfigSave({ schemaVersion: 1, outputDir: outputDir.value.trim() });
    ElMessage.success("已保存 outputDir");
    await loadConfig();
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    ElMessage.error(`${kind}: ${message}`);
  } finally {
    configSaving.value = false;
  }
}

async function importAndGenerate() {
  if (saving.value) return;
  saving.value = true;
  try {
    await importNow();
    if (!last.value) return;
    if (lastError.value) return;

    const existingPoints = await commPointsLoad().catch(() => ({ schemaVersion: 1, points: [] }));
    const existingProfiles = await commProfilesLoad().catch(() => ({ schemaVersion: 1, profiles: [] }));
    const mapped = await unionToCommPoints({
      imported: last.value.points,
      importedProfiles: last.value.profiles,
      existing: existingPoints,
      existingProfiles,
      yieldEvery: 500,
    });

    mapperWarnings.value = mapped.warnings;
    mapperDecisions.value = mapped.decisions ?? [];
    mapperConflictReport.value = mapped.conflictReport ?? null;
    await commPointsSave(mapped.points);

    const mergedProfiles = mergeProfiles(existingProfiles, last.value.profiles);
    if (mergedProfiles.profiles.length > 0) {
      await commProfilesSave(mergedProfiles);
    }

    savedSummary.value = {
      points: mapped.points.points.length,
      profiles: mergedProfiles.profiles.length,
      reusedPointKeys: mapped.reusedPointKeys,
      createdPointKeys: mapped.createdPointKeys,
      skipped: mapped.skipped,
    };

    ElMessage.success(`已生成并保存：points=${savedSummary.value.points}, profiles=${savedSummary.value.profiles}`);
  } finally {
    saving.value = false;
  }
}

function confirmExportWithConflicts(params: { conflictReport: any }): Promise<boolean> {
  conflictConfirmChecked.value = false;
  conflictConfirmReport.value = params.conflictReport;
  conflictConfirmVisible.value = true;
  return new Promise((resolve) => {
    conflictConfirmResolve = resolve;
  });
}

function onConflictConfirmCancel() {
  conflictConfirmVisible.value = false;
  conflictConfirmResolve?.(false);
  conflictConfirmResolve = null;
}

function onConflictConfirmOk() {
  if (!conflictConfirmChecked.value) return;
  conflictConfirmVisible.value = false;
  conflictConfirmResolve?.(true);
  conflictConfirmResolve = null;
}

async function runWizard() {
  if (wizardRunning.value) return;
  if (!filePath.value.trim()) {
    ElMessage.error("请填写联合 xlsx 文件路径");
    return;
  }
  // outPath 允许留空：TASK-32 将按 outputDir 自动选择默认导出路径。

  wizardRunning.value = true;
  wizardLogs.value = [];
  wizardResult.value = null;
  evidenceOutcome.value = null;
  plcBridgeResponse.value = null;
  bridgeCheckResponse.value = null;
  importResultStubResponse.value = null;
  unifiedImportResponse.value = null;
  plcImportStubResponse.value = null;

  const controller = new AbortController();
  wizardController.value = controller;

  const options: ImportUnionOptions = {
    strict: strict.value,
    sheetName: sheetName.value.trim() ? sheetName.value.trim() : undefined,
    addressBase: addressBase.value,
  };

  try {
    const result = await runDemoPipeline(
      {
        filePath: filePath.value.trim(),
        importOptions: options,
        outPath: demoOutPath.value.trim(),
        driver: wizardDriver.value,
        yieldEvery: 500,
        maxWaitMs: 5000,
      },
      {
        signal: controller.signal,
        onProgress: (e) => wizardLogs.value.push(e),
        confirmExportWithConflicts,
      }
    );

    wizardResult.value = result;
    last.value = result.importResponse ?? null;
    lastError.value = null;
    mapperWarnings.value = result.mapped?.warnings ?? [];
    mapperDecisions.value = result.mapped?.decisions ?? [];
    mapperConflictReport.value = result.mapped?.conflictReport ?? null;

    if (result.mapped && result.savedProfiles) {
      savedSummary.value = {
        points: result.mapped.points.points.length,
        profiles: result.savedProfiles.profiles.length,
        reusedPointKeys: result.mapped.reusedPointKeys,
        createdPointKeys: result.mapped.createdPointKeys,
        skipped: result.mapped.skipped,
      };
    } else {
      savedSummary.value = null;
    }

    if (result.ok) {
      ElMessage.success("一键演示（Wizard）完成");
    } else {
      ElMessage.error(`${result.error?.kind ?? "Error"}: ${result.error?.message ?? "unknown error"}`);
    }
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    wizardLogs.value.push({ tsUtc: new Date().toISOString(), step: "error", status: "error", message: `${kind}: ${message}` });
    ElMessage.error(`${kind}: ${message}`);
  } finally {
    wizardRunning.value = false;
    wizardController.value = null;
  }
}

function cancelWizard() {
  wizardController.value?.abort();
}

async function exportPlcBridge() {
  const irPath = wizardResult.value?.irResponse?.irPath ?? "";
  if (!irPath) {
    ElMessage.error("请先运行 Wizard 导出 IR，然后再导出 PLC Bridge");
    return;
  }
  try {
    const resp = await commBridgeToPlcImportV1({ irPath, outPath: "" });
    plcBridgeResponse.value = resp;
    ElMessage.success(`PLC Bridge 已导出：${resp.outPath}`);
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    ElMessage.error(`${kind}: ${message}`);
  }
}

async function runBridgeCheck() {
  const bridgePath = plcBridgeResponse.value?.outPath ?? "";
  if (!bridgePath) {
    ElMessage.error("请先导出 PLC Bridge，再运行 bridge consumer check");
    return;
  }
  try {
    const resp = await commBridgeConsumeCheck({ bridgePath });
    bridgeCheckResponse.value = resp;
    ElMessage.success(`bridge_check 已生成：${resp.outPath}`);
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    ElMessage.error(`${kind}: ${message}`);
  }
}

async function exportImportResultStub() {
  const bridgePath = plcBridgeResponse.value?.outPath ?? "";
  if (!bridgePath) {
    ElMessage.error("请先导出 PLC Bridge，再导出 ImportResultStub");
    return;
  }
  try {
    const resp = await commBridgeExportImportResultStubV1({ bridgePath, outPath: "" });
    importResultStubResponse.value = resp;
    ElMessage.success(`ImportResultStub 已导出：${resp.outPath}`);
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    ElMessage.error(`${kind}: ${message}`);
  }
}

async function mergeUnifiedImport() {
  const stubPath = importResultStubResponse.value?.outPath ?? "";
  const unionPath = filePath.value.trim();
  if (!unionPath) {
    ElMessage.error("请填写联合 xlsx 文件路径");
    return;
  }
  if (!stubPath) {
    ElMessage.error("请先导出 ImportResultStub，再合并生成 UnifiedImport");
    return;
  }

  try {
    const resp = await commMergeImportSourcesV1({
      unionXlsxPath: unionPath,
      importResultStubPath: stubPath,
      outPath: "",
    });
    unifiedImportResponse.value = resp;
    plcImportStubResponse.value = null;
    ElMessage.success(`UnifiedImport 已生成：${resp.outPath}`);
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    ElMessage.error(`${kind}: ${message}`);
  }
}

async function exportPlcImportStub() {
  const unifiedPath = unifiedImportResponse.value?.outPath ?? "";
  if (!unifiedPath) {
    ElMessage.error("请先合并生成 UnifiedImport，再导出 PLC Import stub");
    return;
  }
  try {
    const resp = await commUnifiedExportPlcImportStubV1({ unifiedImportPath: unifiedPath, outPath: "" });
    plcImportStubResponse.value = resp;
    ElMessage.success(`PLC Import stub 已导出：${resp.outPath}`);
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    ElMessage.error(`${kind}: ${message}`);
  }
}

async function exportEvidence() {
  if (!wizardResult.value) {
    ElMessage.error("请先运行一键演示（Wizard），再导出证据包");
    return;
  }

  try {
    const profilesCount =
      wizardResult.value.savedProfiles?.profiles.length ?? wizardResult.value.importResponse?.profiles.profiles.length ?? 0;
    const pointsCount =
      wizardResult.value.mapped?.points.points.length ?? wizardResult.value.importResponse?.points.points.length ?? 0;
    const resultsCount = wizardResult.value.latest?.results.length ?? 0;
    const driver = wizardResult.value.driver;
    const connectionSnapshot = connectionSnapshotFromProfiles(driver, wizardResult.value.savedProfiles);

    const meta = {
      run: {
        driver: manifestDriver(driver),
        includeResults: true,
        resultsSource: "runLatest",
        durationMs: durationMsFromLogs(wizardResult.value.logs),
      },
      counts: {
        profiles: profilesCount,
        points: pointsCount,
        results: resultsCount,
        decisions: decisionsCountFromOutcome(wizardResult.value.mapped),
        conflicts: wizardResult.value.mapped?.conflictReport?.conflicts?.length ?? 0,
      },
      connectionSnapshot,
    };
    const outcome = await buildEvidencePack({
      pipelineLog: wizardResult.value.logs,
      exportResponse: wizardResult.value.exportResponse,
      conflictReport: wizardResult.value.mapped?.conflictReport,
      exportedXlsxPath: wizardResult.value.exportResponse.outPath,
      irPath: wizardResult.value.irResponse?.irPath,
      plcBridgePath: plcBridgeResponse.value?.outPath,
      importResultStubPath: importResultStubResponse.value?.outPath,
      unifiedImportPath: unifiedImportResponse.value?.outPath,
      mergeReportPath: unifiedImportResponse.value?.reportPath,
      plcImportStubPath: plcImportStubResponse.value?.outPath,
      unionXlsxPath: filePath.value.trim() ? filePath.value : undefined,
      parsedColumnsUsed:
        unifiedImportResponse.value?.summary?.parsedColumnsUsed ??
        wizardResult.value.importResponse?.diagnostics?.detectedColumns ??
        undefined,
      meta,
    });
    evidenceOutcome.value = outcome;
    evidenceVerifyPath.value = outcome.zipPath ?? outcome.evidenceDir;
    evidenceVerifyResponse.value = null;
    ElMessage.success(`证据包已生成：${outcome.evidenceDir}`);
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    ElMessage.error(`${kind}: ${message}`);
  }
}

async function verifyEvidence() {
  const p = evidenceVerifyPath.value.trim();
  if (!p) {
    ElMessage.error("Please input evidenceDir or evidence.zip path");
    return;
  }
  evidenceVerifying.value = true;
  try {
    const resp = await commEvidenceVerifyV1(p);
    evidenceVerifyResponse.value = resp;
    if (resp.ok) {
      ElMessage.success("evidence verify: OK");
    } else {
      ElMessage.error("evidence verify: FAILED (see checks/errors)");
    }
  } catch (e: unknown) {
    const { kind, message } = formatError(e);
    ElMessage.error(`${kind}: ${message}`);
  } finally {
    evidenceVerifying.value = false;
  }
}

function goPoints() {
  router.push("/comm/points");
}

function goRun() {
  router.push("/comm/run");
}

setDefaultDemoOutPath();
onMounted(loadConfig);
</script>

<template>
  <el-card>
    <template #header>联合 xlsx 导入（strict 错误结构化展示）</template>

      <el-card style="margin-bottom: 12px">
      <template #header>一键演示（Wizard）+ 证据包</template>
      <el-form label-width="140px">
        <el-form-item label="交付目录 outputDir">
          <el-input v-model="outputDir" placeholder="默认：AppData/<app>/comm/deliveries/" />
          <el-button style="margin-left: 8px" :loading="configSaving" @click="saveConfig">保存</el-button>
        </el-form-item>
        <el-form-item label="驱动">
          <el-select v-model="wizardDriver" style="width: 240px">
            <el-option label="mock（默认）" value="Mock" />
            <el-option label="modbus_tcp（真实联调）" value="Tcp" />
            <el-option label="modbus_rtu（485，真实联调）" value="Rtu485" />
          </el-select>
        </el-form-item>
        <el-form-item label="导出路径">
          <el-input v-model="demoOutPath" placeholder="可留空：自动导出到 outputDir/xlsx/通讯地址表.<ts>.xlsx" />
        </el-form-item>
        <el-form-item>
          <el-button type="danger" :loading="wizardRunning" @click="runWizard">一键演示（Wizard）</el-button>
          <el-button :disabled="!wizardRunning" @click="cancelWizard">取消</el-button>
          <el-button
            type="success"
            :disabled="!wizardResult || wizardRunning"
            @click="exportEvidence"
          >
            导出证据包
          </el-button>
        </el-form-item>
      </el-form>

      <el-alert
        v-if="wizardResult"
        :type="wizardResult.ok ? 'success' : 'error'"
        show-icon
        :closable="false"
        :title="`Wizard 结束：ok=${wizardResult.ok}, driver=${wizardResult.driver}, outPath=${wizardResult.exportResponse.outPath}, resultsStatus=${wizardResult.exportResponse.resultsStatus}`"
      />

      <el-alert
        v-if="wizardResult?.irResponse?.irPath"
        type="info"
        show-icon
        :closable="false"
        :title="`CommIR 已导出：${wizardResult.irResponse.irPath}`"
        style="margin-top: 12px"
      />

      <el-space v-if="wizardResult?.irResponse?.irPath" wrap style="margin-top: 8px">
        <el-button size="small" @click="openPath(wizardResult.irResponse.irPath)">打开 IR</el-button>
        <el-button size="small" @click="downloadJson('comm_ir_summary.json', wizardResult.irResponse)">导出 IR 摘要(JSON)</el-button>
        <el-button size="small" type="primary" @click="exportPlcBridge">导出 PLC Bridge（v1）</el-button>
      </el-space>

      <el-alert
        v-if="plcBridgeResponse?.outPath"
        type="info"
        show-icon
        :closable="false"
        :title="`PLC Bridge 已导出：${plcBridgeResponse.outPath}`"
        style="margin-top: 12px"
      />

      <el-space v-if="plcBridgeResponse?.outPath" wrap style="margin-top: 8px">
        <el-button size="small" @click="openPath(plcBridgeResponse.outPath)">打开 Bridge</el-button>
        <el-button size="small" type="success" @click="runBridgeCheck">运行 bridge consumer check</el-button>
        <el-button size="small" type="primary" @click="exportImportResultStub">导出 ImportResultStub（v1）</el-button>
        <el-button size="small" @click="downloadJson('plc_bridge_export.json', plcBridgeResponse)">导出 Bridge 摘要(JSON)</el-button>
      </el-space>

      <el-alert
        v-if="bridgeCheckResponse?.outPath"
        type="info"
        show-icon
        :closable="false"
        :title="`bridge_check 已生成：${bridgeCheckResponse.outPath}`"
        style="margin-top: 12px"
      />

      <el-space v-if="bridgeCheckResponse?.outPath" wrap style="margin-top: 8px">
        <el-button size="small" @click="openPath(bridgeCheckResponse.outPath)">打开 summary.json</el-button>
        <el-button size="small" @click="downloadJson('bridge_check.json', bridgeCheckResponse)">导出 check 摘要(JSON)</el-button>
      </el-space>

      <el-alert
        v-if="importResultStubResponse?.outPath"
        type="info"
        show-icon
        :closable="false"
        :title="`ImportResultStub 已导出：${importResultStubResponse.outPath}`"
        style="margin-top: 12px"
      />

      <el-space v-if="importResultStubResponse?.outPath" wrap style="margin-top: 8px">
        <el-button size="small" @click="openPath(importResultStubResponse.outPath)">打开 ImportResultStub</el-button>
        <el-button size="small" @click="downloadJson('import_result_stub_export.json', importResultStubResponse)"
          >导出 Stub 摘要(JSON)</el-button
        >
        <el-button size="small" type="primary" @click="mergeUnifiedImport">合并生成 UnifiedImport（v1）</el-button>
      </el-space>

      <el-alert
        v-if="unifiedImportResponse?.outPath"
        type="info"
        show-icon
        :closable="false"
        :title="`UnifiedImport 已生成：${unifiedImportResponse.outPath}${unifiedImportResponse.reportPath ? ' | report=' + unifiedImportResponse.reportPath : ''}`"
        style="margin-top: 12px"
      />

      <el-space v-if="unifiedImportResponse?.outPath" wrap style="margin-top: 8px">
        <el-button size="small" @click="openPath(unifiedImportResponse.outPath)">打开 UnifiedImport</el-button>
        <el-button
          v-if="unifiedImportResponse.reportPath"
          size="small"
          @click="openPath(unifiedImportResponse.reportPath)"
          >打开 merge_report</el-button
        >
        <el-button size="small" type="primary" @click="exportPlcImportStub">导出 PLC Import stub（v1）</el-button>
        <el-button size="small" @click="downloadJson('unified_import_merge.json', unifiedImportResponse)"
          >导出 Merge 摘要(JSON)</el-button
        >
      </el-space>

      <el-card v-if="unifiedImportResponse?.warnings?.length" style="margin-top: 12px">
        <template #header>Merge warnings</template>
        <pre>{{ JSON.stringify(unifiedImportResponse.warnings, null, 2) }}</pre>
      </el-card>

      <el-alert
        v-if="plcImportStubResponse?.outPath"
        type="info"
        show-icon
        :closable="false"
        :title="`PLC Import stub 已导出：${plcImportStubResponse.outPath}`"
        style="margin-top: 12px"
      />

      <el-space v-if="plcImportStubResponse?.outPath" wrap style="margin-top: 8px">
        <el-button size="small" @click="openPath(plcImportStubResponse.outPath)">打开 plc_import_stub</el-button>
        <el-button size="small" @click="downloadJson('plc_import_stub_export.json', plcImportStubResponse)"
          >导出 plc_import_stub 摘要(JSON)</el-button
        >
      </el-space>

      <el-alert
        v-if="evidenceOutcome"
        type="info"
        show-icon
        :closable="false"
        :title="`evidenceDir=${evidenceOutcome.evidenceDir}${evidenceOutcome.zipPath ? ' | zip=' + evidenceOutcome.zipPath : ''}`"
        style="margin-top: 12px"
      />

      <el-card v-if="evidenceManifestSummary" style="margin-top: 12px">
        <template #header>manifest 摘要（验收用）</template>
        <el-descriptions :column="2" border>
          <el-descriptions-item label="appName">{{ evidenceManifestSummary.appName }}</el-descriptions-item>
          <el-descriptions-item label="appVersion">{{ evidenceManifestSummary.appVersion }}</el-descriptions-item>
          <el-descriptions-item label="gitCommit">{{ evidenceManifestSummary.gitCommit }}</el-descriptions-item>
          <el-descriptions-item label="driver">{{ evidenceManifestSummary.driver }}</el-descriptions-item>
          <el-descriptions-item label="durationMs">{{ evidenceManifestSummary.durationMs }}</el-descriptions-item>
          <el-descriptions-item label="points">{{ evidenceManifestSummary.points }}</el-descriptions-item>
          <el-descriptions-item label="results">{{ evidenceManifestSummary.results }}</el-descriptions-item>
          <el-descriptions-item label="conflicts">{{ evidenceManifestSummary.conflicts }}</el-descriptions-item>
          <el-descriptions-item label="headersDigest">{{ evidenceManifestSummary.headersDigest }}</el-descriptions-item>
        </el-descriptions>
        <el-divider />
        <pre>{{ JSON.stringify(evidenceManifest, null, 2) }}</pre>
      </el-card>

      <el-card v-if="evidenceOutcome" style="margin-top: 12px">
        <template #header>Evidence Verify (v1)</template>
        <el-form label-width="140px">
          <el-form-item label="Path">
            <el-input v-model="evidenceVerifyPath" placeholder="evidenceDir or evidence.zip" />
            <el-button style="margin-left: 8px" :loading="evidenceVerifying" @click="verifyEvidence">Verify</el-button>
          </el-form-item>
        </el-form>
        <el-alert
          v-if="evidenceVerifyResponse"
          :type="evidenceVerifyResponse.ok ? 'success' : 'error'"
          show-icon
          :closable="false"
          :title="evidenceVerifyResponse.ok ? 'verify: OK' : 'verify: FAILED'"
        />
        <pre v-if="evidenceVerifyResponse" style="margin-top: 8px">{{ JSON.stringify(evidenceVerifyResponse, null, 2) }}</pre>
      </el-card>

      <el-card v-if="wizardLogs.length > 0" style="margin-top: 12px">
        <template #header>流水线日志（验收用）</template>
        <pre>{{ JSON.stringify(wizardLogs, null, 2) }}</pre>
      </el-card>
    </el-card>

    <el-form label-width="140px">
      <el-form-item label="文件路径">
        <el-input v-model="filePath" placeholder="例如：C:\\temp\\联合点表.xlsx" />
      </el-form-item>
      <el-form-item label="strict 校验">
        <el-switch v-model="strict" />
      </el-form-item>
      <el-form-item label="Sheet 名">
        <el-input v-model="sheetName" placeholder="联合点表" />
      </el-form-item>
      <el-form-item label="地址基准">
        <el-select v-model="addressBase" style="width: 220px">
          <el-option label="one（v1 默认，Excel 1 -> 内部 0）" value="one" />
          <el-option label="zero（Excel 0 -> 内部 0）" value="zero" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" :loading="saving" @click="importNow">导入（仅解析）</el-button>
        <el-button type="success" :loading="saving" @click="importAndGenerate">导入并生成通讯点位</el-button>
      </el-form-item>
    </el-form>

    <el-divider />

    <el-alert
      v-if="last"
      type="success"
      show-icon
      :closable="false"
      :title="`导入成功：points=${last.points.points.length}, profiles=${last.profiles.profiles.length}, warnings=${warnings.length}`"
    />

    <el-card v-if="savedSummary" style="margin-top: 12px">
      <template #header>落盘结果（AppData/comm/*.v1.json）</template>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="points 保存数">{{ savedSummary.points }}</el-descriptions-item>
        <el-descriptions-item label="profiles 保存数">{{ savedSummary.profiles }}</el-descriptions-item>
        <el-descriptions-item label="pointKey 复用">{{ savedSummary.reusedPointKeys }}</el-descriptions-item>
        <el-descriptions-item label="pointKey 新建">{{ savedSummary.createdPointKeys }}</el-descriptions-item>
        <el-descriptions-item label="跳过行">{{ savedSummary.skipped }}</el-descriptions-item>
      </el-descriptions>
      <el-space wrap style="margin-top: 12px">
        <el-button @click="goPoints">打开 Points</el-button>
        <el-button type="primary" @click="goRun">打开 Run（Mock 可直接跑）</el-button>
      </el-space>
    </el-card>

    <el-card v-if="mapperDecisions.length > 0 || (mapperConflictReport && mapperConflictReport.conflicts)" style="margin-top: 12px">
      <template #header>诊断摘要（质量门禁提示）</template>
      <el-descriptions :column="2" border>
        <el-descriptions-item label="reused:keyV2">{{ diagnosticSummary.reusedKeyV2 }}</el-descriptions-item>
        <el-descriptions-item label="reused:keyV2NoDevice">{{ diagnosticSummary.reusedKeyV2NoDevice }}</el-descriptions-item>
        <el-descriptions-item label="reused:keyV1">{{ diagnosticSummary.reusedKeyV1 }}</el-descriptions-item>
        <el-descriptions-item label="created:new">{{ diagnosticSummary.createdNew }}</el-descriptions-item>
        <el-descriptions-item label="conflicts">{{ diagnosticSummary.conflicts }}</el-descriptions-item>
        <el-descriptions-item label="keyV1Conflicts">{{ diagnosticSummary.keyV1Conflicts }}</el-descriptions-item>
      </el-descriptions>
      <el-alert
        v-if="diagnosticSummary.keyV1Conflicts > 0"
        type="warning"
        show-icon
        :closable="false"
        title="检测到 keyV1(hmiName-only) 冲突：导出交付表将触发二次确认，建议先修复 channelName/deviceId"
        style="margin-top: 12px"
      />
    </el-card>

    <el-alert
      v-if="lastError"
      type="error"
      show-icon
      :closable="false"
      :title="`导入失败：${lastError.kind}`"
      style="margin-top: 12px"
    />

    <el-card v-if="lastError" style="margin-top: 12px">
      <template #header>错误详情（结构化）</template>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="kind">{{ lastError.kind }}</el-descriptions-item>
        <el-descriptions-item label="message">{{ lastError.message }}</el-descriptions-item>
      </el-descriptions>
      <el-divider />
      <pre>{{ JSON.stringify(lastError, null, 2) }}</pre>
    </el-card>

    <el-card v-if="diagnostics" style="margin-top: 12px">
      <template #header>diagnostics（可用于快速定位）</template>
      <pre>{{ JSON.stringify(diagnostics, null, 2) }}</pre>
    </el-card>

    <el-card v-if="last" style="margin-top: 12px">
      <template #header>导入结果摘要</template>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="points">{{ last.points.points.length }}</el-descriptions-item>
        <el-descriptions-item label="profiles">{{ last.profiles.profiles.length }}</el-descriptions-item>
        <el-descriptions-item label="warnings">{{ allWarnings.length }}</el-descriptions-item>
      </el-descriptions>
    </el-card>

    <el-card v-if="allWarnings.length > 0" style="margin-top: 12px">
      <template #header>warnings（import + mapper）</template>
      <pre>{{ JSON.stringify(allWarnings, null, 2) }}</pre>
    </el-card>

    <el-card v-if="filteredDecisions.length > 0" style="margin-top: 12px">
      <template #header>复用诊断（pointKey 决策可解释）</template>
      <el-space wrap style="margin-bottom: 12px">
        <el-checkbox v-model="decisionsOnlyCreated">只看 created</el-checkbox>
        <el-checkbox v-model="decisionsOnlyWarn">只看 warnings 相关</el-checkbox>
        <el-button
          :disabled="!(mapperConflictReport && mapperConflictReport.conflicts && mapperConflictReport.conflicts.length > 0)"
          @click="downloadJson('conflict_report.json', mapperConflictReport)"
        >
          导出 conflict_report.json
        </el-button>
      </el-space>
      <el-table :data="filteredDecisions" size="small" border height="360">
        <el-table-column prop="hmiName" label="hmiName" min-width="160" />
        <el-table-column prop="channelName" label="channelName" min-width="180" />
        <el-table-column prop="deviceId" label="deviceId" width="100" />
        <el-table-column prop="pointKey" label="pointKey" min-width="260" />
        <el-table-column prop="reuseDecision" label="reuseDecision" width="160" />
      </el-table>
    </el-card>

    <el-dialog v-model="conflictConfirmVisible" title="冲突风险确认" width="720px" @close="onConflictConfirmCancel">
      <el-alert
        type="warning"
        show-icon
        :closable="false"
        title="检测到 pointKey 复用冲突：继续导出交付表可能导致同名变量映射不确定。"
      />
      <el-divider />
      <pre style="max-height: 260px; overflow: auto">{{ JSON.stringify(conflictConfirmReport, null, 2) }}</pre>
      <el-divider />
      <el-checkbox v-model="conflictConfirmChecked">我已知晓冲突风险，仍要继续导出交付表</el-checkbox>
      <template #footer>
        <el-button @click="onConflictConfirmCancel">取消</el-button>
        <el-button type="primary" :disabled="!conflictConfirmChecked" @click="onConflictConfirmOk">继续</el-button>
      </template>
    </el-dialog>
  </el-card>
</template>
