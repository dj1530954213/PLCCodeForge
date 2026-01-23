import { computed, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";

import type {
  AddressBase,
  CommImportUnionXlsxResponse,
  CommWarning,
  ImportUnionOptions,
  ImportUnionThrownError,
  PointsV1,
  ProfilesV1,
} from "../api";
import { importUnionXlsx } from "../services/importUnion";
import { loadPoints, savePoints } from "../services/points";
import { loadProfiles, saveProfiles } from "../services/profiles";
import { unionToCommPoints } from "../mappers/unionToCommPoints";

interface UseImportUnionOptions {
  projectId: Ref<string>;
  activeDeviceId: Ref<string>;
}

export function useImportUnion(options: UseImportUnionOptions) {
  const filePath = ref<string>("");
  const strict = ref<boolean>(true);
  const sheetName = ref<string>("");
  const addressBase = ref<AddressBase>("one");

  const importing = ref(false);
  const generating = ref(false);

  const last = ref<CommImportUnionXlsxResponse | null>(null);
  const lastError = ref<ImportUnionThrownError | null>(null);

  const mapperWarnings = ref<CommWarning[]>([]);
  const mapperDecisions = ref<any[]>([]);
  const mapperConflictReport = ref<any | null>(null);

  const savedSummary = ref<{
    points: number;
    profiles: number;
    reusedPointKeys: number;
    createdPointKeys: number;
    skipped: number;
  } | null>(null);

  const warnings = computed(() => last.value?.warnings ?? []);
  const diagnostics = computed(() => last.value?.diagnostics ?? lastError.value?.diagnostics ?? null);
  const allWarnings = computed(() => [...warnings.value, ...mapperWarnings.value]);

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
    if (importing.value) return;
    importing.value = true;

    last.value = null;
    lastError.value = null;
    savedSummary.value = null;
    mapperWarnings.value = [];
    mapperDecisions.value = [];
    mapperConflictReport.value = null;

    if (!filePath.value.trim()) {
      ElMessage.error("请填写联合 xlsx 文件路径");
      importing.value = false;
      return;
    }

    const optionsPayload: ImportUnionOptions = {
      strict: strict.value,
      sheetName: sheetName.value.trim() ? sheetName.value.trim() : undefined,
      addressBase: addressBase.value,
    };

    try {
      last.value = await importUnionXlsx(filePath.value.trim(), optionsPayload);
      ElMessage.success(
        `导入成功：points=${last.value.points.points.length}, profiles=${last.value.profiles.profiles.length}, warnings=${warnings.value.length}`
      );
    } catch (e: unknown) {
      lastError.value = e as ImportUnionThrownError;
      ElMessage.error(`${lastError.value.kind}: ${lastError.value.message}`);
    } finally {
      importing.value = false;
    }
  }

  async function importAndGenerate() {
    if (generating.value) return;
    generating.value = true;
    try {
      await importNow();
      if (!last.value || lastError.value) return;

      const pid = options.projectId.value.trim();
      const did = options.activeDeviceId.value.trim();
      if (!pid || !did) {
        ElMessage.error("未选择设备");
        return;
      }

      const [existingPoints, existingProfiles] = await Promise.all([
        loadPoints(pid, did).catch(() => ({ schemaVersion: 1, points: [] } as PointsV1)),
        loadProfiles(pid, did).catch(() => ({ schemaVersion: 1, profiles: [] } as ProfilesV1)),
      ]);

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

      await savePoints(mapped.points, pid, did);

      const mergedProfiles = mergeProfiles(existingProfiles, last.value.profiles);
      if (mergedProfiles.profiles.length > 0) {
        await saveProfiles(mergedProfiles, pid, did);
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
      generating.value = false;
    }
  }

  return {
    filePath,
    strict,
    sheetName,
    addressBase,
    importing,
    generating,
    last,
    lastError,
    mapperWarnings,
    mapperDecisions,
    mapperConflictReport,
    savedSummary,
    warnings,
    diagnostics,
    allWarnings,
    importNow,
    importAndGenerate,
  };
}
