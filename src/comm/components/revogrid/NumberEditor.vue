<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import type { EditorType } from "@revolist/vue3-datagrid";

const props = defineProps<EditorType>();

const inputEl = ref<HTMLInputElement | null>(null);
const value = ref<string>("");

function readInitialValue(): string {
  const v = (props.val ?? props.value) as unknown;
  if (v === null || v === undefined) return "";
  if (typeof v === "number") return Number.isFinite(v) ? String(v) : "";
  return String(v);
}

function commit(preventFocus?: boolean) {
  const n = Number(value.value);
  props.save(Number.isFinite(n) ? n : value.value, preventFocus);
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
  void nextTick(() => inputEl.value?.focus());
});
</script>

<template>
  <input
    ref="inputEl"
    v-model="value"
    class="comm-rg-editor comm-rg-editor--number"
    type="number"
    step="0.1"
    @keydown="onKeydown"
    @blur="
      () => {
        commit(true);
        close(false);
      }
    "
  />
</template>

