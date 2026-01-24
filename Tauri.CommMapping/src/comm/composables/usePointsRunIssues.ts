import { computed, type ComputedRef, type Ref } from "vue";

import type { CommRunError } from "../api";
import { formatBackendReason, formatFieldLabel, type ValidationIssue } from "./usePointsValidation";

export interface BackendFieldIssue {
  pointKey?: string;
  hmiName?: string;
  field: string;
  reason?: string;
}

export interface UsePointsRunIssuesOptions {
  runError: Ref<CommRunError | null>;
  validationIssues: ComputedRef<ValidationIssue[]>;
  hasValidationIssues: ComputedRef<boolean>;
}

export function usePointsRunIssues(options: UsePointsRunIssuesOptions) {
  const backendFieldIssues = computed<BackendFieldIssue[]>(
    () => options.runError.value?.details?.missingFields ?? []
  );

  const backendFieldIssuesView = computed(() =>
    backendFieldIssues.value.map((issue) => ({
      ...issue,
      fieldLabel: formatFieldLabel(issue.field),
      reasonLabel: formatBackendReason(issue.reason),
    }))
  );

  const hasBackendFieldIssues = computed(() => backendFieldIssues.value.length > 0);

  const validationSummary = computed(() => {
    if (!options.hasValidationIssues.value && !hasBackendFieldIssues.value) {
      return "当前无阻断错误";
    }
    const parts: string[] = [];
    if (options.hasValidationIssues.value) {
      parts.push(`前端校验 ${options.validationIssues.value.length} 条`);
    }
    if (hasBackendFieldIssues.value) {
      parts.push(`后端校验 ${backendFieldIssues.value.length} 条`);
    }
    return `运行已阻止 · ${parts.join(" / ")}`;
  });

  return {
    backendFieldIssuesView,
    hasBackendFieldIssues,
    validationSummary,
  };
}
