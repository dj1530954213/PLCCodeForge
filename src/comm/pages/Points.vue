<script setup lang="ts">
import { computed, ref } from "vue";
import { ElMessage } from "element-plus";

import type { ByteOrder32, CommPoint, DataType, PointsV1 } from "../api";
import { commPointsLoad, commPointsSave } from "../api";

const DATA_TYPES: DataType[] = [
  "Bool",
  "Int16",
  "UInt16",
  "Int32",
  "UInt32",
  "Float32",
];
const BYTE_ORDERS: ByteOrder32[] = ["ABCD", "BADC", "CDAB", "DCBA"];

const model = ref<PointsV1>({ schemaVersion: 1, points: [] });
const selectedKeys = ref<string[]>([]);

const dialogOpen = ref(false);
const editingIndex = ref<number | null>(null);
const editing = ref<CommPoint>({
  pointKey: crypto.randomUUID(),
  hmiName: "P1",
  dataType: "UInt16",
  byteOrder: "ABCD",
  channelName: "tcp-ok",
  scale: 1.0,
});

function openAdd() {
  editingIndex.value = null;
  editing.value = {
    pointKey: crypto.randomUUID(),
    hmiName: "",
    dataType: "UInt16",
    byteOrder: "ABCD",
    channelName: "",
    scale: 1.0,
  };
  dialogOpen.value = true;
}

function openEdit(index: number) {
  editingIndex.value = index;
  editing.value = JSON.parse(JSON.stringify(model.value.points[index])) as CommPoint;
  dialogOpen.value = true;
}

function removeAt(index: number) {
  model.value.points.splice(index, 1);
}

function saveEditing() {
  if (!editing.value.hmiName.trim()) {
    ElMessage.error("变量名称（HMI）不能为空");
    return;
  }
  if (!editing.value.channelName.trim()) {
    ElMessage.error("通道名称不能为空");
    return;
  }

  if (editingIndex.value === null) {
    model.value.points.push(editing.value);
  } else {
    model.value.points[editingIndex.value] = editing.value;
  }
  dialogOpen.value = false;
}

async function load() {
  model.value = await commPointsLoad();
  ElMessage.success("已加载 points");
}

async function save() {
  await commPointsSave(model.value);
  ElMessage.success("已保存 points");
}

async function loadDemo() {
  model.value = {
    schemaVersion: 1,
    points: [
      {
        pointKey: crypto.randomUUID(),
        hmiName: "OK_U16",
        dataType: "UInt16",
        byteOrder: "ABCD",
        channelName: "tcp-ok",
        scale: 1.0,
      },
      {
        pointKey: crypto.randomUUID(),
        hmiName: "OK_F32_CDAB",
        dataType: "Float32",
        byteOrder: "CDAB",
        channelName: "tcp-ok",
        scale: 0.1,
      },
      {
        pointKey: crypto.randomUUID(),
        hmiName: "OK_I32_DCBA",
        dataType: "Int32",
        byteOrder: "DCBA",
        channelName: "tcp-ok",
        scale: 1.0,
      },
      {
        pointKey: crypto.randomUUID(),
        hmiName: "TIMEOUT_U16",
        dataType: "UInt16",
        byteOrder: "ABCD",
        channelName: "tcp-timeout",
        scale: 1.0,
      },
      {
        pointKey: crypto.randomUUID(),
        hmiName: "DECODE_U32_BADC",
        dataType: "UInt32",
        byteOrder: "BADC",
        channelName: "tcp-decode",
        scale: 1.0,
      },
    ],
  };
  await save();
  ElMessage.success("已加载并保存 demo points（mock）");
}

const importJson = ref("");
function importFromJson() {
  try {
    const parsed = JSON.parse(importJson.value) as PointsV1;
    if (parsed.schemaVersion !== 1) {
      ElMessage.error("schemaVersion 必须为 1");
      return;
    }
    model.value = parsed;
    ElMessage.success("已导入 points JSON");
  } catch (e) {
    ElMessage.error(`JSON 解析失败: ${String(e)}`);
  }
}

const exportJson = computed(() => JSON.stringify(model.value, null, 2));

const batchDataType = ref<DataType>("UInt16");
const batchByteOrder = ref<ByteOrder32>("ABCD");
const batchApplyChannelName = ref(false);
const batchChannelName = ref("");
const batchApplyScale = ref(false);
const batchScale = ref<number>(1.0);
function applyBatch() {
  const selected = new Set(selectedKeys.value);
  let changed = 0;
  for (const p of model.value.points) {
    if (!selected.has(p.pointKey)) continue;
    p.dataType = batchDataType.value;
    p.byteOrder = batchByteOrder.value;
    if (batchApplyChannelName.value) {
      if (!batchChannelName.value.trim()) {
        ElMessage.error("批量通道名称不能为空");
        return;
      }
      p.channelName = batchChannelName.value.trim();
    }
    if (batchApplyScale.value) {
      p.scale = batchScale.value;
    }
    changed += 1;
  }
  ElMessage.success(`已批量修改 ${changed} 个点位`);
}

function onSelectionChange(rows: CommPoint[]) {
  selectedKeys.value = rows.map((r) => r.pointKey);
}
</script>

<template>
  <el-card>
    <template #header>点位列表</template>

    <el-space wrap>
      <el-button type="primary" @click="openAdd">新增点位</el-button>
      <el-button @click="load">加载</el-button>
      <el-button @click="save">保存</el-button>
      <el-button type="success" @click="loadDemo">加载 Demo（mock）</el-button>
    </el-space>

    <el-divider />

      <el-space wrap>
        <el-select v-model="batchDataType" style="width: 180px">
          <el-option v-for="opt in DATA_TYPES" :key="opt" :label="opt" :value="opt" />
        </el-select>
        <el-select v-model="batchByteOrder" style="width: 180px">
          <el-option v-for="opt in BYTE_ORDERS" :key="opt" :label="opt" :value="opt" />
        </el-select>
        <el-checkbox v-model="batchApplyChannelName">批量通道名称</el-checkbox>
        <el-input
          v-model="batchChannelName"
          :disabled="!batchApplyChannelName"
          placeholder="例如：tcp-ok"
          style="width: 180px"
        />
        <el-checkbox v-model="batchApplyScale">批量缩放</el-checkbox>
        <el-input-number v-model="batchScale" :disabled="!batchApplyScale" :step="0.1" />
        <el-button @click="applyBatch">对选中行批量设置</el-button>
      </el-space>

    <el-table
      :data="model.points"
      style="width: 100%; margin-top: 12px"
      @selection-change="onSelectionChange"
    >
      <el-table-column type="selection" width="40" />
      <el-table-column prop="hmiName" label="变量名称（HMI）" min-width="160" />
      <el-table-column prop="dataType" label="数据类型" width="100" />
      <el-table-column prop="byteOrder" label="字节序" width="90" />
      <el-table-column prop="channelName" label="通道名称" min-width="140" />
      <el-table-column prop="scale" label="缩放倍数" width="90" />
      <el-table-column label="操作" width="160">
        <template #default="{ $index }">
          <el-button size="small" @click="openEdit($index)">编辑</el-button>
          <el-button size="small" type="danger" @click="removeAt($index)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-divider />

    <el-row :gutter="12">
      <el-col :span="12">
        <el-card>
          <template #header>导入 JSON</template>
          <el-input v-model="importJson" type="textarea" :rows="10" />
          <div style="margin-top: 10px">
            <el-button type="primary" @click="importFromJson">导入</el-button>
          </div>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card>
          <template #header>导出 JSON</template>
          <el-input :model-value="exportJson" type="textarea" :rows="10" readonly />
        </el-card>
      </el-col>
    </el-row>
  </el-card>

  <el-dialog v-model="dialogOpen" width="720px">
    <template #header>
      <span>{{ editingIndex === null ? "新增" : "编辑" }}点位</span>
    </template>

    <el-form label-width="160px">
      <el-form-item label="pointKey（不可变）">
        <el-input :model-value="editing.pointKey" readonly />
      </el-form-item>
      <el-form-item label="变量名称（HMI）">
        <el-input v-model="editing.hmiName" />
      </el-form-item>
      <el-form-item label="通道名称">
        <el-input v-model="editing.channelName" />
      </el-form-item>
      <el-form-item label="数据类型">
        <el-select v-model="editing.dataType" style="width: 220px">
          <el-option v-for="opt in DATA_TYPES" :key="opt" :label="opt" :value="opt" />
        </el-select>
      </el-form-item>
      <el-form-item label="字节序（32-bit）">
        <el-select v-model="editing.byteOrder" style="width: 220px">
          <el-option v-for="opt in BYTE_ORDERS" :key="opt" :label="opt" :value="opt" />
        </el-select>
      </el-form-item>
      <el-form-item label="缩放倍数">
        <el-input-number v-model="editing.scale" :step="0.1" />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="dialogOpen = false">取消</el-button>
      <el-button type="primary" @click="saveEditing">确定</el-button>
    </template>
  </el-dialog>
</template>
