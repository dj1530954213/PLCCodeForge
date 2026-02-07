import { computed, inject, provide, ref, type ComputedRef, type Ref } from "vue";

import { notifyError, notifySuccess, resolveErrorMessage } from "../services/notify";

export interface WorkspaceSaveHandler {
  id: string;
  label: string;
  isDirty: () => boolean;
  save: () => Promise<boolean>;
}

export interface WorkspaceSaveAllContext {
  handlers: Ref<WorkspaceSaveHandler[]>;
  isSaving: Ref<boolean>;
  hasDirty: ComputedRef<boolean>;
  registerSaveHandler: (handler: WorkspaceSaveHandler) => () => void;
  saveAll: () => Promise<void>;
}

const WORKSPACE_SAVE_ALL_KEY = Symbol("comm-workspace-save-all");

export function provideWorkspaceSaveAll(): WorkspaceSaveAllContext {
  const handlers = ref<WorkspaceSaveHandler[]>([]);
  const isSaving = ref(false);

  const hasDirty = computed(() => handlers.value.some((h) => h.isDirty()));

  function registerSaveHandler(handler: WorkspaceSaveHandler) {
    handlers.value = handlers.value.filter((h) => h.id !== handler.id).concat(handler);
    return () => {
      handlers.value = handlers.value.filter((h) => h.id !== handler.id);
    };
  }

  async function saveAll() {
    if (isSaving.value) return;
    isSaving.value = true;
    try {
      let saved = 0;
      for (const handler of handlers.value) {
        if (!handler.isDirty()) continue;
        const ok = await handler.save();
        if (!ok) return;
        saved += 1;
      }
      if (saved > 0) {
        notifySuccess(`已保存 ${saved} 项配置`);
      } else {
        notifySuccess("暂无需要保存的更改");
      }
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "保存失败"));
    } finally {
      isSaving.value = false;
    }
  }

  const ctx: WorkspaceSaveAllContext = {
    handlers,
    isSaving,
    hasDirty,
    registerSaveHandler,
    saveAll,
  };

  provide(WORKSPACE_SAVE_ALL_KEY, ctx);
  return ctx;
}

export function useWorkspaceSaveAll(): WorkspaceSaveAllContext {
  const ctx = inject<WorkspaceSaveAllContext>(WORKSPACE_SAVE_ALL_KEY);
  if (!ctx) {
    throw new Error("WorkspaceSaveAllContext is missing. Ensure ProjectWorkspace provides it.");
  }
  return ctx;
}
