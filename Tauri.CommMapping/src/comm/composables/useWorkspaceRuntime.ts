import { inject, provide, ref, type Ref } from "vue";

import type { RunStats } from "../api";

export interface CommWorkspaceRuntime {
  stats: Ref<RunStats | null>;
  updatedAtUtc: Ref<string>;
}

const COMM_WORKSPACE_RUNTIME_KEY = Symbol("comm-workspace-runtime");

export function provideCommWorkspaceRuntime(): CommWorkspaceRuntime {
  const ctx: CommWorkspaceRuntime = {
    stats: ref<RunStats | null>(null),
    updatedAtUtc: ref<string>(""),
  };
  provide(COMM_WORKSPACE_RUNTIME_KEY, ctx);
  return ctx;
}

export function useCommWorkspaceRuntime(): CommWorkspaceRuntime {
  const ctx = inject<CommWorkspaceRuntime>(COMM_WORKSPACE_RUNTIME_KEY);
  if (!ctx) {
    throw new Error("CommWorkspaceRuntime is missing. Ensure ProjectWorkspace provides it.");
  }
  return ctx;
}
