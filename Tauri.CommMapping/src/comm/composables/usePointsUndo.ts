import { ElMessage } from "element-plus";

import { UndoManager } from "../services/undoRedo";

interface UsePointsUndoOptions {
  limit?: number;
  onAfterChange?: () => void | Promise<void>;
}

export function usePointsUndo(options: UsePointsUndoOptions = {}) {
  const undoManager = new UndoManager(options.limit ?? 20);

  function handleUndo() {
    if (!undoManager.canUndo()) {
      ElMessage.warning("没有可撤销的操作");
      return;
    }
    undoManager.undo();
    void options.onAfterChange?.();
    ElMessage.success("已撤销");
  }

  function handleRedo() {
    if (!undoManager.canRedo()) {
      ElMessage.warning("没有可重做的操作");
      return;
    }
    undoManager.redo();
    void options.onAfterChange?.();
    ElMessage.success("已重做");
  }

  return {
    undoManager,
    handleUndo,
    handleRedo,
  };
}
