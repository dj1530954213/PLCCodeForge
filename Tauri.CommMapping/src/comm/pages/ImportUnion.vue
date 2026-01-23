<script setup lang="ts">
import { useCommDeviceContext } from "../composables/useDeviceContext";
import { useImportUnion } from "../composables/useImportUnion";

const { projectId, activeDeviceId } = useCommDeviceContext();
const {
  filePath,
  strict,
  sheetName,
  addressBase,
  importing,
  generating,
  lastError,
  mapperConflictReport,
  savedSummary,
  diagnostics,
  allWarnings,
  importNow,
  importAndGenerate,
} = useImportUnion({ projectId, activeDeviceId });

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
</script>

<template>
  <div class="comm-subpage comm-subpage--import-union">
    <header class="comm-hero comm-animate" style="--delay: 0ms">
      <div class="comm-hero-title">
        <div class="comm-title">联合 xlsx 导入</div>
        <div class="comm-subtitle">快速生成连接参数与点位（高级流程）</div>
      </div>
      <div class="comm-hero-actions">
        <el-button :loading="importing" @click="importNow">导入</el-button>
        <el-button type="primary" :loading="generating" @click="importAndGenerate">导入并生成保存</el-button>
      </div>
    </header>

    <section class="comm-panel comm-animate" style="--delay: 80ms">
      <div class="comm-panel-header">
        <div class="comm-section-title">导入参数</div>
        <div class="comm-inline-meta">支持 strict 校验与地址基准切换</div>
      </div>
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
            <el-option label="从 1 开始（UI）" value="one" />
            <el-option label="zero（0-based）" value="zero" />
          </el-select>
        </el-form-item>
      </el-form>
    </section>

    <section v-if="lastError" class="comm-panel comm-animate" style="--delay: 120ms">
      <el-alert
        type="error"
        show-icon
        :closable="false"
        :title="`${lastError.kind}: ${lastError.message}`"
      />
    </section>

    <section v-if="diagnostics" class="comm-panel comm-animate" style="--delay: 140ms">
      <div class="comm-panel-header">
        <div class="comm-section-title">Diagnostics</div>
      </div>
      <pre class="comm-code-block">{{ JSON.stringify(diagnostics, null, 2) }}</pre>
    </section>

    <section v-if="allWarnings.length > 0" class="comm-panel comm-animate" style="--delay: 180ms">
      <div class="comm-panel-header">
        <div class="comm-section-title">Warnings（Import + Mapper）</div>
      </div>
      <pre class="comm-code-block">{{ JSON.stringify(allWarnings, null, 2) }}</pre>
    </section>

    <section v-if="savedSummary" class="comm-panel comm-animate" style="--delay: 220ms">
      <div class="comm-panel-header">
        <div class="comm-section-title">已保存到工程</div>
      </div>
      <el-descriptions :column="3" border size="small">
        <el-descriptions-item label="points">{{ savedSummary.points }}</el-descriptions-item>
        <el-descriptions-item label="profiles">{{ savedSummary.profiles }}</el-descriptions-item>
        <el-descriptions-item label="skipped">{{ savedSummary.skipped }}</el-descriptions-item>
        <el-descriptions-item label="pointKey reused">{{ savedSummary.reusedPointKeys }}</el-descriptions-item>
        <el-descriptions-item label="pointKey created">{{ savedSummary.createdPointKeys }}</el-descriptions-item>
      </el-descriptions>
    </section>

    <section v-if="mapperConflictReport" class="comm-panel comm-animate" style="--delay: 260ms">
      <div class="comm-panel-header">
        <div class="comm-section-title">冲突报告</div>
        <el-space wrap>
          <el-button size="small" @click="downloadJson('conflict_report.json', mapperConflictReport)">
            导出 conflict_report.json
          </el-button>
        </el-space>
      </div>
      <pre class="comm-code-block">{{ JSON.stringify(mapperConflictReport, null, 2) }}</pre>
    </section>
  </div>
</template>
