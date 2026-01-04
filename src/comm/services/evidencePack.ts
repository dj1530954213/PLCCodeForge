import type { CommExportDeliveryXlsxResponse } from "../api";
import { commEvidencePackCreate } from "../api";

export interface EvidencePackInput {
  pipelineLog: unknown;
  exportResponse: CommExportDeliveryXlsxResponse;
  conflictReport?: unknown;
  exportedXlsxPath?: string;
  meta?: unknown;
  irPath?: string;
  plcBridgePath?: string;
  importResultStubPath?: string;
  unifiedImportPath?: string;
  mergeReportPath?: string;
  plcImportStubPath?: string;
  unionXlsxPath?: string;
  parsedColumnsUsed?: string[];
}

export interface EvidencePackOutcome {
  evidenceDir: string;
  zipPath?: string;
  manifest: unknown;
  files: string[];
  warnings?: string[];
}

export async function buildEvidencePack(input: EvidencePackInput): Promise<EvidencePackOutcome> {
  const conflictReport = (() => {
    const v = input.conflictReport as any;
    const conflicts = v?.conflicts;
    return Array.isArray(conflicts) && conflicts.length > 0 ? input.conflictReport : undefined;
  })();

  return commEvidencePackCreate({
    pipelineLog: input.pipelineLog,
    exportResponse: input.exportResponse,
    conflictReport,
    meta: input.meta,
    exportedXlsxPath: input.exportedXlsxPath ?? input.exportResponse.outPath,
    irPath: input.irPath,
    plcBridgePath: input.plcBridgePath,
    importResultStubPath: input.importResultStubPath,
    unifiedImportPath: input.unifiedImportPath,
    mergeReportPath: input.mergeReportPath,
    plcImportStubPath: input.plcImportStubPath,
    unionXlsxPath: input.unionXlsxPath,
    parsedColumnsUsed: input.parsedColumnsUsed,
    copyXlsx: true,
    copyIr: true,
    copyPlcBridge: true,
    copyImportResultStub: true,
    copyUnifiedImport: true,
    copyMergeReport: true,
    copyPlcImportStub: true,
    zip: true,
  });
}
