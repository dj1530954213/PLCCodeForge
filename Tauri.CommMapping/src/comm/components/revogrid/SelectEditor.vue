<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import type { EditorType } from "@revolist/vue3-datagrid";

type Option = { label: string; value: string };

const props = defineProps<EditorType>();

const selectEl = ref<HTMLSelectElement | null>(null);
const value = ref<string>("");

function editorOptions(): Option[] {
  const column = (props.column as any)?.column ?? props.column;
  const opts = (column as any)?.editorOptions as unknown;
  return Array.isArray(opts) ? (opts as Option[]) : [];
}

function readInitialValue(): string {
  const v = (props.val ?? props.value) as unknown;
  if (v === null || v === undefined) return "";
  return String(v);
}

function commit(preventFocus?: boolean) {
  props.save(value.value, preventFocus);
}

function close(focusNext?: boolean) {
  props.close(focusNext);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    e.preventDefault();
    commit();
    close(true);
  } else if (e.key === "Escape") {
    e.preventDefault();
    close(false);
  }
}

watch(
  () => props.value,
  () => {
    value.value = readInitialValue();
  }
);

onMounted(() => {
  value.value = readInitialValue();
  void nextTick(() => selectEl.value?.focus());
});
</script>

<template>
  <select
    ref="selectEl"
    v-model="value"
    class="comm-rg-editor comm-rg-editor--select"
    @keydown="onKeydown"
    @change="
      () => {
        commit(true);
        close(true);
      }
    "
    @blur="
      () => {
        commit(true);
        close(false);
      }
    "
  >
    <option v-for="opt in editorOptions()" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
  </select>
</template>
