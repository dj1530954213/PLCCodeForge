import { ElMessage, ElMessageBox } from "element-plus";

type ConfirmOptions = Parameters<typeof ElMessageBox.confirm>[2];
type PromptOptions = Parameters<typeof ElMessageBox.prompt>[2];

export function notifySuccess(message: string): void {
  ElMessage.success(message);
}

export function notifyInfo(message: string): void {
  ElMessage.info(message);
}

export function notifyWarning(message: string): void {
  ElMessage.warning(message);
}

export function notifyError(message: string): void {
  ElMessage.error(message);
}

export function resolveErrorMessage(error: unknown, fallback: string): string {
  if (typeof error === "string" && error.trim()) return error;
  if (error instanceof Error && error.message) return error.message;
  if (error && typeof error === "object" && "message" in error) {
    const msg = (error as { message?: unknown }).message;
    if (typeof msg === "string" && msg.trim()) return msg;
  }
  return fallback;
}

export async function confirmAction(
  message: string,
  title: string,
  options?: ConfirmOptions
): Promise<boolean> {
  try {
    await ElMessageBox.confirm(message, title, options);
    return true;
  } catch {
    return false;
  }
}

export async function promptText(
  message: string,
  title: string,
  options?: PromptOptions
): Promise<string | null> {
  try {
    const { value } = await ElMessageBox.prompt(message, title, options);
    return value;
  } catch {
    return null;
  }
}
