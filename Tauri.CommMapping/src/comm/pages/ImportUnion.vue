<script setup lang="ts">
import { computed, ref } from "vue";
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
import { commImportUnionXlsx, commPointsLoad, commPointsSave, commProfilesLoad, commProfilesSave } from "../api";
import { unionToCommPoints } from "../mappers/unionToCommPoints";
import { useCommDeviceContext } from "../composables/useDeviceContext";

const { projectId, activeDeviceId } = useCommDeviceContext();

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

    const pid = projectId.value.trim();
    const did = activeDeviceId.value.trim();
    if (!pid || !did) {
      ElMessage.error("未选择设备");
      return;
    }

    const [existingPoints, existingProfiles] = await Promise.all([
      commPointsLoad(pid, did).catch(() => ({ schemaVersion: 1, points: [] } as PointsV1)),
      commProfilesLoad(pid, did).catch(() => ({ schemaVersion: 1, profiles: [] } as ProfilesV1)),
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

    await commPointsSave(mapped.points, pid, did);

    const mergedProfiles = mergeProfiles(existingProfiles, last.value.profiles);
    if (mergedProfiles.profiles.length > 0) {
      await commProfilesSave(mergedProfiles, pid, did);
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
</script>

<template>
  <el-card>
    <template #header>联合 xlsx 导入（高级）</template>

    <el-alert
      type="info"
      show-icon
      :closable="false"
      title="用于从“联合 xlsx”快速生成连接参数与点位；不影响通讯采集主流程。"
      style="margin-bottom: 12px"
    />

    <el-form label-width="140px">
      <el-form-item label="文件路径">
        <el-input v-model="filePath" placeholder="*.xlsx" />
      </el-form-item>

      <el-form-item label="Strict">
        <el-switch v-model="strict" />
      </el-form-item>

      <el-form-item label="Sheet 名称（可选）">
        <el-input v-model="sheetName" placeholder="留空则自动匹配" />
      </el-form-item>

      <el-form-item label="地址基准">
        <el-select v-model="addressBase" style="width: 220px">
          <el-option label="one（UI 1-based）" value="one" />
          <el-option label="zero（0-based）" value="zero" />
        </el-select>
      </el-form-item>

      <el-form-item>
        <el-button :loading="importing" @click="importNow">导入</el-button>
        <el-button type="primary" :loading="generating" @click="importAndGenerate">导入并生成连接/点位并保存到工程</el-button>
      </el-form-item>
    </el-form>

    <el-alert
      v-if="lastError"
      type="error"
      show-icon
      :closable="false"
      :title="`${lastError.kind}: ${lastError.message}`"
      style="margin-top: 12px"
    />

    <el-card v-if="diagnostics" shadow="never" style="margin-top: 12px">
      <template #header>Diagnostics</template>
      <pre style="margin: 0">{{ JSON.stringify(diagnostics, null, 2) }}</pre>
    </el-card>

    <el-card v-if="allWarnings.length > 0" shadow="never" style="margin-top: 12px">
      <template #header>Warnings（Import + Mapper）</template>
      <pre style="margin: 0">{{ JSON.stringify(allWarnings, null, 2) }}</pre>
    </el-card>

    <el-card v-if="savedSummary" shadow="never" style="margin-top: 12px">
      <template #header>已保存到工程</template>
      <el-descriptions :column="3" border size="small">
        <el-descriptions-item label="points">{{ savedSummary.points }}</el-descriptions-item>
        <el-descriptions-item label="profiles">{{ savedSummary.profiles }}</el-descriptions-item>
        <el-descriptions-item label="skipped">{{ savedSummary.skipped }}</el-descriptions-item>
        <el-descriptions-item label="pointKey reused">{{ savedSummary.reusedPointKeys }}</el-descriptions-item>
        <el-descriptions-item label="pointKey created">{{ savedSummary.createdPointKeys }}</el-descriptions-item>
      </el-descriptions>
    </el-card>

    <el-card v-if="mapperConflictReport" shadow="never" style="margin-top: 12px">
      <template #header>冲突报告</template>
      <el-space wrap>
        <el-button size="small" @click="downloadJson('conflict_report.json', mapperConflictReport)"
          >导出 conflict_report.json</el-button
        >
      </el-space>
      <pre style="margin-top: 8px">{{ JSON.stringify(mapperConflictReport, null, 2) }}</pre>
    </el-card>
  </el-card>
</template>
