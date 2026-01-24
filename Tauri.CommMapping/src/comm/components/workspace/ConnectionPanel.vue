<script setup lang="ts">
import { useConnectionPanel } from "../../composables/useConnectionPanel";

const {
  AREA_OPTIONS,
  PARITY_OPTIONS,
  profileDraft,
  profileDirty,
  resetProfileDraft,
  switchProfileProtocol,
  saveProfileDraft,
} = useConnectionPanel();
</script>

<template>
  <section class="comm-panel comm-animate" style="--delay: 80ms">
    <div class="comm-panel-header">
      <div class="comm-section-title">连接配置</div>
      <el-button size="small" :disabled="!profileDraft" @click="resetProfileDraft">重置</el-button>
    </div>

    <div v-if="profileDraft" class="comm-connection-form">
      <el-form label-width="72px" class="comm-form-compact">
        <el-form-item label="协议">
          <el-radio-group v-model="profileDraft.protocolType" @change="switchProfileProtocol">
            <el-radio-button label="TCP">TCP</el-radio-button>
            <el-radio-button label="485">485</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="通道">
          <el-input v-model="profileDraft.channelName" />
        </el-form-item>
        <el-form-item label="站号">
          <el-input-number v-model="profileDraft.deviceId" :min="0" :max="255" />
        </el-form-item>
        <el-form-item label="区域">
          <el-select v-model="profileDraft.readArea">
            <el-option v-for="opt in AREA_OPTIONS" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>

        <template v-if="profileDraft.protocolType === 'TCP'">
          <el-form-item label="IP">
            <el-input v-model="profileDraft.ip" />
          </el-form-item>
          <el-form-item label="端口">
            <el-input-number v-model="profileDraft.port" :min="1" :max="65535" />
          </el-form-item>
        </template>

        <template v-else>
          <el-form-item label="串口">
            <el-input v-model="profileDraft.serialPort" />
          </el-form-item>
          <el-form-item label="波特率">
            <el-input-number v-model="profileDraft.baudRate" :min="300" />
          </el-form-item>
          <el-form-item label="校验">
            <el-select v-model="profileDraft.parity">
              <el-option v-for="opt in PARITY_OPTIONS" :key="opt" :label="opt" :value="opt" />
            </el-select>
          </el-form-item>
          <el-form-item label="数据位">
            <el-input-number v-model="profileDraft.dataBits" :min="5" :max="8" />
          </el-form-item>
          <el-form-item label="停止位">
            <el-input-number v-model="profileDraft.stopBits" :min="1" :max="2" />
          </el-form-item>
        </template>

        <el-form-item label="超时">
          <el-input-number v-model="profileDraft.timeoutMs" :min="1" />
        </el-form-item>
        <el-form-item label="重试">
          <el-input-number v-model="profileDraft.retryCount" :min="0" />
        </el-form-item>
        <el-form-item label="轮询">
          <el-input-number v-model="profileDraft.pollIntervalMs" :min="50" />
        </el-form-item>
      </el-form>

      <div class="comm-panel-actions">
        <el-button size="small" type="primary" :disabled="!profileDirty" @click="saveProfileDraft">保存连接</el-button>
      </div>
    </div>
    <el-empty v-else description="未选择设备，无法编辑连接" />
  </section>
</template>

<style scoped>
.comm-connection-form :deep(.el-input-number),
.comm-connection-form :deep(.el-select) {
  width: 100%;
}

.comm-connection-form :deep(.el-radio-group) {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
</style>
