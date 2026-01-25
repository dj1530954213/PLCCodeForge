import { UndoManager } from "../services/undoRedo";
import { notifySuccess, notifyWarning } from "../services/notify";

interface UsePointsUndoOptions {
  limit?: number;
  onAfterChange?: () => void | Promise<void>;
}

export function usePointsUndo(options: UsePointsUndoOptions = {}) {
  const undoManager = new UndoManager(options.limit ?? 20);

  function handleUndo() {
    if (!undoManager.canUndo()) {
      notifyWarning("没有可撤销的操作");
      return;
    }
    undoManager.undo();
    void options.onAfterChange?.();
    notifySuccess("已撤销");
  }

  function handleRedo() {
    if (!undoManager.canRedo()) {
      notifyWarning("没有可重做的操作");
      return;
    }
    undoManager.redo();
    void options.onAfterChange?.();
    notifySuccess("已重做");
  }

  return {
    undoManager,
    handleUndo,
    handleRedo,
  };
}
