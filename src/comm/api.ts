import { invoke } from "@tauri-apps/api/core";

export type DataType =
  | "Bool"
  | "Int16"
  | "UInt16"
  | "Int32"
  | "UInt32"
  | "Float32"
  | "Unknown";

export type ByteOrder32 = "ABCD" | "BADC" | "CDAB" | "DCBA" | "Unknown";

export type RegisterArea = "Holding" | "Input" | "Coil" | "Discrete";

export type SerialParity = "None" | "Even" | "Odd";

export type Quality = "Ok" | "Timeout" | "CommError" | "DecodeError" | "ConfigError";

export type ConnectionProfile =
  | {
      protocolType: "TCP";
      channelName: string;
      deviceId: number;
      readArea: RegisterArea;
      startAddress: number; // internal 0-based
      length: number;
      ip: string;
      port: number;
      timeoutMs: number;
      retryCount: number;
      pollIntervalMs: number;
    }
  | {
      protocolType: "485";
      channelName: string;
      deviceId: number;
      readArea: RegisterArea;
      startAddress: number; // internal 0-based
      length: number;
      serialPort: string;
      baudRate: number;
      parity: SerialParity;
      dataBits: number;
      stopBits: number;
      timeoutMs: number;
      retryCount: number;
      pollIntervalMs: number;
    };

export interface ProfilesV1 {
  schemaVersion: number;
  profiles: ConnectionProfile[];
}

export interface CommPoint {
  pointKey: string;
  hmiName: string;
  dataType: DataType;
  byteOrder: ByteOrder32;
  channelName: string;
  addressOffset?: number;
  scale: number;
}

export interface PointsV1 {
  schemaVersion: number;
  points: CommPoint[];
}

export interface CommConfigV1 {
  schemaVersion: number;
  outputDir: string;
}

export interface PlannedPointRead {
  pointKey: string;
  dataType: DataType;
  byteOrder: ByteOrder32;
  scale: number;
  offset: number;
  length: number;
}

export interface ReadJob {
  channelName: string;
  readArea: RegisterArea;
  startAddress: number;
  length: number;
  points: PlannedPointRead[];
}

export interface ReadPlan {
  jobs: ReadJob[];
}

export interface PlanV1 extends ReadPlan {
  schemaVersion: number;
}

export interface PlanOptions {
  maxRegistersPerJob: number;
  maxCoilsPerJob: number;
}

export type CommDriverKind = "Mock" | "Tcp" | "Rtu485";

export interface SampleResult {
  pointKey: string;
  valueDisplay: string;
  quality: Quality;
  timestamp: string;
  durationMs: number;
  errorMessage: string;
}

export interface RunStats {
  total: number;
  ok: number;
  timeout: number;
  commError: number;
  decodeError: number;
  configError: number;
}

export interface CommWarning {
  code: string;
  message: string;
  pointKey?: string;
  hmiName?: string;
  channelName?: string;
  deviceId?: number;
}

export interface CommRunStartResponse {
  runId: string;
}

export interface CommRunLatestResponse {
  results: SampleResult[];
  stats: RunStats;
  updatedAtUtc: string;
  runWarnings?: CommWarning[];
}

export interface CommExportXlsxHeaders {
  tcp: string[];
  rtu: string[];
  params: string[];
  tcpSheet: string[];
  rtu485Sheet: string[];
  paramsSheet: string[];
}

export interface ExportedRows {
  tcp: number;
  rtu: number;
  params: number;
}

export interface CommExportDiagnostics {
  exportedRows: ExportedRows;
  durationMs: number;
}

export interface CommExportXlsxResponse {
  outPath: string;
  headers: CommExportXlsxHeaders;
  warnings?: CommWarning[];
  diagnostics?: CommExportDiagnostics;
}

export interface CommExportDeliveryXlsxHeaders {
  tcp: string[];
  rtu: string[];
  params: string[];
}

export type DeliveryResultsSource = "appdata" | "runLatest";

export type DeliveryResultsStatus = "written" | "missing" | "skipped";

export interface CommExportDeliveryXlsxResponse {
  outPath: string;
  headers: CommExportDeliveryXlsxHeaders;
  resultsStatus?: DeliveryResultsStatus;
  resultsMessage?: string;
  warnings?: CommWarning[];
  diagnostics?: CommExportDiagnostics;
}

export type CommIrResultsSource = "appdata" | "runLatest";

export interface CommIrExportSummary {
  points: number;
  profiles: number;
  results: number;
  conflicts: number;
  irDigest: string;
}

export interface CommExportIrV1Request {
  unionXlsxPath?: string;
  resultsSource?: CommIrResultsSource;
  profiles?: ProfilesV1;
  points?: PointsV1;
  latestResults?: SampleResult[];
  stats?: RunStats;
  decisions?: unknown;
  conflictReport?: unknown;
}

export interface CommExportIrV1Response {
  irPath: string;
  summary: CommIrExportSummary;
}

export type PlcBridgeErrorKind =
  | "CommIrReadError"
  | "CommIrDeserializeError"
  | "CommIrUnsupportedSchemaVersion"
  | "CommIrUnsupportedSpecVersion"
  | "CommIrValidationError"
  | "PlcBridgeWriteError";

export interface PlcBridgeErrorDetails {
  irPath?: string;
  schemaVersion?: number;
  specVersion?: string;
  pointKey?: string;
  hmiName?: string;
  channelName?: string;
  field?: string;
  rawValue?: string;
  allowedValues?: string[];
}

export interface PlcBridgeError {
  kind: PlcBridgeErrorKind;
  message: string;
  details?: PlcBridgeErrorDetails;
}

export interface PlcImportBridgeExportSummary {
  points: number;
  stats: RunStats;
  sourceIrDigest: string;
  plcBridgeDigest: string;
}

export interface CommBridgeToPlcImportV1Request {
  irPath: string;
  outPath?: string;
}

export interface CommBridgeToPlcImportV1Response {
  outPath: string;
  summary?: PlcImportBridgeExportSummary;
  ok?: boolean;
  error?: PlcBridgeError;
}

export type BridgeCheckErrorKind =
  | "PlcBridgeReadError"
  | "PlcBridgeDeserializeError"
  | "PlcBridgeUnsupportedSchemaVersion"
  | "PlcBridgeUnsupportedSpecVersion"
  | "PlcBridgeValidationError"
  | "BridgeSummaryWriteError";

export interface BridgeCheckErrorDetails {
  bridgePath?: string;
  schemaVersion?: number;
  specVersion?: string;
  message?: string;
}

export interface BridgeCheckError {
  kind: BridgeCheckErrorKind;
  message: string;
  details?: BridgeCheckErrorDetails;
}

export interface BridgeConsumerSummaryPoint {
  name: string;
  channelName: string;
  readArea?: string;
  absoluteAddress?: number;
}

export interface BridgeConsumerSummary {
  schemaVersion: number;
  specVersion: string;
  generatedAtUtc: string;
  bridgePath: string;
  totalPoints: number;
  byChannel: Record<string, number>;
  byQuality: Record<string, number>;
  first10: BridgeConsumerSummaryPoint[];
}

export interface CommBridgeConsumeCheckRequest {
  bridgePath: string;
}

export interface CommBridgeConsumeCheckResponse {
  outPath: string;
  summary?: BridgeConsumerSummary;
  ok?: boolean;
  error?: BridgeCheckError;
}

export type ImportResultStubErrorKind =
  | "PlcBridgeReadError"
  | "PlcBridgeDeserializeError"
  | "PlcBridgeUnsupportedSchemaVersion"
  | "PlcBridgeUnsupportedSpecVersion"
  | "ImportResultStubValidationError"
  | "ImportResultStubWriteError";

export interface ImportResultStubErrorDetails {
  bridgePath?: string;
  schemaVersion?: number;
  specVersion?: string;
  name?: string;
  field?: string;
}

export interface ImportResultStubError {
  kind: ImportResultStubErrorKind;
  message: string;
  details?: ImportResultStubErrorDetails;
}

export interface ImportResultStubExportSummary {
  points: number;
  stats: RunStats;
  sourceBridgeDigest: string;
  importResultStubDigest: string;
}

export interface CommBridgeExportImportResultStubV1Request {
  bridgePath: string;
  outPath?: string;
}

export interface CommBridgeExportImportResultStubV1Response {
  outPath: string;
  summary?: ImportResultStubExportSummary;
  ok?: boolean;
  error?: ImportResultStubError;
}

export type MergeImportSourcesErrorKind =
  | "UnionXlsxReadError"
  | "UnionXlsxParseError"
  | "UnionXlsxValidationError"
  | "ImportResultStubReadError"
  | "ImportResultStubDeserializeError"
  | "ImportResultStubUnsupportedSchemaVersion"
  | "ImportResultStubUnsupportedSpecVersion"
  | "ImportResultStubValidationError"
  | "MergeWriteError";

export interface MergeImportSourcesErrorDetails {
  unionXlsxPath?: string;
  importResultStubPath?: string;
  outPath?: string;
  reportPath?: string;
  name?: string;
  field?: string;
}

export interface MergeImportSourcesError {
  kind: MergeImportSourcesErrorKind;
  message: string;
  details?: MergeImportSourcesErrorDetails;
}

export interface MergeImportSourcesSummary {
  unionPoints: number;
  stubPoints: number;
  matched: number;
  unmatchedStub: number;
  overridden: number;
  conflicts: number;
  unionXlsxDigest: string;
  importResultStubDigest: string;
  unifiedImportDigest: string;
  mergeReportDigest: string;
  parsedColumnsUsed?: string[];
}

export interface CommMergeImportSourcesV1Request {
  unionXlsxPath: string;
  importResultStubPath: string;
  outPath?: string;
}

export interface CommMergeImportSourcesV1Response {
  outPath: string;
  reportPath?: string;
  summary?: MergeImportSourcesSummary;
  warnings: CommWarning[];
  ok?: boolean;
  error?: MergeImportSourcesError;
}

export type UnifiedPlcImportStubErrorKind =
  | "UnifiedImportReadError"
  | "UnifiedImportDeserializeError"
  | "UnifiedImportUnsupportedSchemaVersion"
  | "UnifiedImportUnsupportedSpecVersion"
  | "UnifiedImportValidationError"
  | "PlcImportStubWriteError";

export interface UnifiedPlcImportStubErrorDetails {
  unifiedImportPath?: string;
  schemaVersion?: number;
  specVersion?: string;
  name?: string;
  field?: string;
  rawValue?: string;
  allowedValues?: string[];
}

export interface UnifiedPlcImportStubError {
  kind: UnifiedPlcImportStubErrorKind;
  message: string;
  details?: UnifiedPlcImportStubErrorDetails;
}

export interface PlcImportStubExportSummary {
  points: number;
  commCovered: number;
  verified: number;
  sourceUnifiedImportDigest: string;
  plcImportStubDigest: string;
}

export interface CommUnifiedExportPlcImportStubV1Request {
  unifiedImportPath: string;
  outPath?: string;
}

export interface CommUnifiedExportPlcImportStubV1Response {
  outPath: string;
  summary?: PlcImportStubExportSummary;
  ok?: boolean;
  error?: UnifiedPlcImportStubError;
}

export interface CommEvidencePackRequest {
  pipelineLog: unknown;
  exportResponse: CommExportDeliveryXlsxResponse;
  conflictReport?: unknown;
  meta?: unknown;
  exportedXlsxPath?: string;
  irPath?: string;
  plcBridgePath?: string;
  importResultStubPath?: string;
  unifiedImportPath?: string;
  mergeReportPath?: string;
  plcImportStubPath?: string;
  unionXlsxPath?: string;
  parsedColumnsUsed?: string[];
  copyXlsx?: boolean;
  copyIr?: boolean;
  copyPlcBridge?: boolean;
  copyImportResultStub?: boolean;
  copyUnifiedImport?: boolean;
  copyMergeReport?: boolean;
  copyPlcImportStub?: boolean;
  zip?: boolean;
}

export interface CommEvidencePackResponse {
  evidenceDir: string;
  zipPath?: string;
  manifest: unknown;
  files: string[];
  warnings?: string[];
}

export type EvidenceVerifyErrorKind =
  | "PathNotFound"
  | "ZipReadError"
  | "ManifestMissing"
  | "ManifestParseError"
  | "EvidenceSummaryMissing"
  | "EvidenceSummaryParseError"
  | "FileMissing"
  | "DigestMismatch"
  | "SchemaMismatch"
  | "PointsOrderMismatch";

export interface EvidenceVerifyErrorDetails {
  fileName?: string;
  expected?: string;
  actual?: string;
  message?: string;
}

export interface EvidenceVerifyError {
  kind: EvidenceVerifyErrorKind;
  message: string;
  details?: EvidenceVerifyErrorDetails;
}

export interface EvidenceVerifyCheck {
  name: string;
  ok: boolean;
  message: string;
}

export interface CommEvidenceVerifyV1Response {
  ok: boolean;
  checks: EvidenceVerifyCheck[];
  errors: EvidenceVerifyError[];
}

export type AddressBase = "zero" | "one";

export interface ImportUnionOptions {
  strict?: boolean;
  sheetName?: string;
  addressBase?: AddressBase;
}

export interface ImportUnionDiagnostics {
  detectedSheets: string[];
  detectedColumns: string[];
  usedSheet: string;
  strict: boolean;
  addressBaseUsed: AddressBase;
  rowsScanned: number;
}

export type ImportUnionErrorKind =
  | "UnionXlsxReadError"
  | "UnionXlsxInvalidSheet"
  | "UnionXlsxMissingColumns"
  | "UnionXlsxInvalidEnum"
  | "UnionXlsxInvalidRow";

export interface ImportUnionErrorDetails {
  sheetName?: string;
  detectedSheets?: string[];
  detectedColumns?: string[];
  missingColumns?: string[];
  rowIndex?: number;
  columnName?: string;
  rawValue?: string;
  allowedValues?: string[];
  addressBaseUsed?: AddressBase;
}

export interface ImportUnionError {
  kind: ImportUnionErrorKind;
  message: string;
  details?: ImportUnionErrorDetails;
}

export type ImportUnionThrownError = ImportUnionError & { diagnostics?: ImportUnionDiagnostics };

export interface CommImportUnionXlsxResponse {
  points: PointsV1;
  profiles: ProfilesV1;
  warnings: CommWarning[];
  diagnostics?: ImportUnionDiagnostics;
  ok?: boolean;
  error?: ImportUnionError;
}

export async function commPing(): Promise<{ ok: boolean }> {
  return invoke("comm_ping");
}

export async function commConfigLoad(): Promise<CommConfigV1> {
  return invoke("comm_config_load");
}

export async function commConfigSave(payload: CommConfigV1): Promise<void> {
  await invoke("comm_config_save", { payload });
}

export async function commProfilesSave(payload: ProfilesV1): Promise<void> {
  await invoke("comm_profiles_save", { payload });
}

export async function commProfilesLoad(): Promise<ProfilesV1> {
  return invoke("comm_profiles_load");
}

export async function commPointsSave(payload: PointsV1): Promise<void> {
  await invoke("comm_points_save", { payload });
}

export async function commPointsLoad(): Promise<PointsV1> {
  return invoke("comm_points_load");
}

export async function commPlanBuild(request: {
  profiles?: ProfilesV1;
  points?: PointsV1;
  options?: PlanOptions;
}): Promise<PlanV1> {
  return invoke("comm_plan_build", { request });
}

export async function commRunStart(request: {
  driver?: CommDriverKind;
  profiles?: ProfilesV1;
  points?: PointsV1;
  plan?: ReadPlan;
}): Promise<CommRunStartResponse> {
  return invoke("comm_run_start", { request });
}

export async function commRunLatest(runId: string): Promise<CommRunLatestResponse> {
  return invoke("comm_run_latest", { runId });
}

export async function commRunStop(runId: string): Promise<void> {
  await invoke("comm_run_stop", { runId });
}

export async function commExportXlsx(request: {
  outPath: string;
  profiles?: ProfilesV1;
  points?: PointsV1;
}): Promise<CommExportXlsxResponse> {
  return invoke("comm_export_xlsx", { request });
}

export async function commExportDeliveryXlsx(request: {
  outPath: string;
  includeResults?: boolean;
  resultsSource?: DeliveryResultsSource;
  results?: SampleResult[];
  stats?: RunStats;
  profiles?: ProfilesV1;
  points?: PointsV1;
}): Promise<CommExportDeliveryXlsxResponse> {
  return invoke("comm_export_delivery_xlsx", { request });
}

export async function commExportIrV1(request: CommExportIrV1Request): Promise<CommExportIrV1Response> {
  return invoke("comm_export_ir_v1", { request });
}

export async function commBridgeToPlcImportV1(
  request: CommBridgeToPlcImportV1Request
): Promise<CommBridgeToPlcImportV1Response> {
  const resp = await invoke<CommBridgeToPlcImportV1Response>("comm_bridge_to_plc_import_v1", { request });
  if (resp.ok === false || resp.error) {
    throw (
      resp.error ?? {
        kind: "PlcBridgeWriteError",
        message: "comm_bridge_to_plc_import_v1 failed (ok=false) but error is missing",
      }
    ) as PlcBridgeError;
  }
  return resp;
}

export async function commBridgeConsumeCheck(
  request: CommBridgeConsumeCheckRequest
): Promise<CommBridgeConsumeCheckResponse> {
  const resp = await invoke<CommBridgeConsumeCheckResponse>("comm_bridge_consume_check", { request });
  if (resp.ok === false || resp.error) {
    throw (
      resp.error ?? {
        kind: "BridgeSummaryWriteError",
        message: "comm_bridge_consume_check failed (ok=false) but error is missing",
      }
    ) as BridgeCheckError;
  }
  return resp;
}

export async function commBridgeExportImportResultStubV1(
  request: CommBridgeExportImportResultStubV1Request
): Promise<CommBridgeExportImportResultStubV1Response> {
  const resp = await invoke<CommBridgeExportImportResultStubV1Response>("comm_bridge_export_importresult_stub_v1", { request });
  if (resp.ok === false || resp.error) {
    throw (
      resp.error ?? {
        kind: "ImportResultStubWriteError",
        message: "comm_bridge_export_importresult_stub_v1 failed (ok=false) but error is missing",
      }
    ) as ImportResultStubError;
  }
  return resp;
}

export async function commMergeImportSourcesV1(
  request: CommMergeImportSourcesV1Request
): Promise<CommMergeImportSourcesV1Response> {
  const resp = await invoke<CommMergeImportSourcesV1Response>("comm_merge_import_sources_v1", { request });
  if (resp.ok === false || resp.error) {
    throw (
      resp.error ?? {
        kind: "MergeWriteError",
        message: "comm_merge_import_sources_v1 failed (ok=false) but error is missing",
      }
    ) as MergeImportSourcesError;
  }
  return resp;
}

export async function commUnifiedExportPlcImportStubV1(
  request: CommUnifiedExportPlcImportStubV1Request
): Promise<CommUnifiedExportPlcImportStubV1Response> {
  const resp = await invoke<CommUnifiedExportPlcImportStubV1Response>("comm_unified_export_plc_import_stub_v1", { request });
  if (resp.ok === false || resp.error) {
    throw (
      resp.error ?? {
        kind: "PlcImportStubWriteError",
        message: "comm_unified_export_plc_import_stub_v1 failed (ok=false) but error is missing",
      }
    ) as UnifiedPlcImportStubError;
  }
  return resp;
}

export async function commEvidencePackCreate(request: CommEvidencePackRequest): Promise<CommEvidencePackResponse> {
  return invoke("comm_evidence_pack_create", { request });
}

export async function commEvidenceVerifyV1(path: string): Promise<CommEvidenceVerifyV1Response> {
  return invoke("comm_evidence_verify_v1", { path });
}

export async function commImportUnionXlsx(
  path: string,
  options?: ImportUnionOptions
): Promise<CommImportUnionXlsxResponse> {
  try {
    const resp = await invoke<CommImportUnionXlsxResponse>("comm_import_union_xlsx", {
      path,
      options,
    });
    if (resp.ok === false) {
      throw {
        ...(resp.error ?? {
          kind: "UnionXlsxReadError",
          message: "comm_import_union_xlsx failed (ok=false) but error is missing",
        }),
        diagnostics: resp.diagnostics,
      } as ImportUnionThrownError;
    }
    if (resp.error) {
      throw { ...resp.error, diagnostics: resp.diagnostics } as ImportUnionThrownError;
    }
    return resp;
  } catch (e: unknown) {
    if (typeof e === "object" && e !== null && "kind" in e && "message" in e) {
      throw e as ImportUnionThrownError;
    }
    throw {
      kind: "UnionXlsxReadError",
      message: String(e ?? "unknown error"),
    } as ImportUnionThrownError;
  }
}
