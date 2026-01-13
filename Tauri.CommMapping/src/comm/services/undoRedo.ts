/**
 * 撤销/重做管理器
 * 遵循SRP：只负责历史记录管理
 * 遵循LSP：所有操作实现统一的UndoableAction接口
 */

/**
 * 可撤销操作接口
 * 遵循LSP：所有可撤销操作都必须实现此接口
 */
export interface UndoableAction {
  type: 'batch-add' | 'batch-edit' | 'delete-rows';
  timestamp: number;
  description: string;
  undo: () => void;
  redo: () => void;
  /**
   * Optional hook to capture "after" state at push time.
   * Callers should create the action BEFORE mutating state, then push AFTER mutation.
   */
  finalize?: () => void;
}

/**
 * 历史记录条目
 */
export interface UndoHistoryEntry {
  action: UndoableAction;
  timestamp: number;
  canUndo: boolean;
  canRedo: boolean;
}


/**
 * 撤销/重做管理器类
 * 遵循SRP：只负责管理历史记录栈
 */
export class UndoManager {
  private history: UndoableAction[] = [];
  private currentIndex: number = -1;
  private readonly maxHistorySize: number;

  constructor(maxHistorySize: number = 20) {
    this.maxHistorySize = maxHistorySize;
  }

  /**
   * 判断是否可以撤销
   */
  canUndo(): boolean {
    return this.currentIndex >= 0;
  }

  /**
   * 判断是否可以重做
   */
  canRedo(): boolean {
    return this.currentIndex < this.history.length - 1;
  }

  /**
   * 执行撤销操作
   */
  undo(): void {
    if (!this.canUndo()) {
      throw new Error('没有可撤销的操作');
    }

    const action = this.history[this.currentIndex];
    action.undo();
    this.currentIndex--;
  }

  /**
   * 执行重做操作
   */
  redo(): void {
    if (!this.canRedo()) {
      throw new Error('没有可重做的操作');
    }

    this.currentIndex++;
    const action = this.history[this.currentIndex];
    action.redo();
  }

  /**
   * 推入新的操作到历史记录
   * 如果当前不在历史记录末尾，会清除后面的记录
   */
  push(action: UndoableAction): void {
    // Capture after-state snapshot (if supported) before storing into history.
    action.finalize?.();

    // 如果当前不在末尾，清除后面的历史
    if (this.currentIndex < this.history.length - 1) {
      this.history = this.history.slice(0, this.currentIndex + 1);
    }

    // 添加新操作
    this.history.push(action);
    this.currentIndex++;

    // 限制历史记录大小
    if (this.history.length > this.maxHistorySize) {
      const removeCount = this.history.length - this.maxHistorySize;
      this.history = this.history.slice(removeCount);
      this.currentIndex -= removeCount;
    }
  }

  /**
   * 清空历史记录
   */
  clear(): void {
    this.history = [];
    this.currentIndex = -1;
  }

  /**
   * 获取历史记录列表
   */
  getHistory(): UndoHistoryEntry[] {
    return this.history.map((action, index) => ({
      action,
      timestamp: action.timestamp,
      canUndo: index <= this.currentIndex,
      canRedo: index > this.currentIndex,
    }));
  }

  /**
   * 获取当前操作的描述
   */
  getCurrentActionDescription(): string | null {
    if (this.currentIndex < 0 || this.currentIndex >= this.history.length) {
      return null;
    }
    return this.history[this.currentIndex].description;
  }

  /**
   * 获取下一个可重做操作的描述
   */
  getNextRedoActionDescription(): string | null {
    if (!this.canRedo()) {
      return null;
    }
    return this.history[this.currentIndex + 1].description;
  }

  /**
   * 获取历史记录大小
   */
  getHistorySize(): number {
    return this.history.length;
  }

  /**
   * 获取当前索引
   */
  getCurrentIndex(): number {
    return this.currentIndex;
  }
}


/**
 * 创建状态快照（深拷贝）
 * 使用JSON序列化实现深拷贝
 * 
 * @param data 要快照的数据
 * @returns 深拷贝的数据
 */
export function createSnapshot<T>(data: T): T {
  return JSON.parse(JSON.stringify(data));
}

/**
 * 创建批量添加的撤销操作
 * 
 * @param pointsGetter 获取points数组的函数
 * @param pointsSetter 设置points数组的函数
 * @param addedPointKeys 新增的点位键列表
 * @param description 操作描述
 * @returns 可撤销操作
 */
export function createBatchAddUndoAction<T>(
  pointsGetter: () => T[],
  pointsSetter: (points: T[]) => void,
  _addedPointKeys: string[],
  description: string
): UndoableAction {
  // 保存操作前的状态
  const beforeSnapshot = createSnapshot(pointsGetter());
  
  // 保存操作后的状态（在push时捕获）
  let afterSnapshot: T[] | null = null;
  const finalize = () => {
    if (afterSnapshot === null) afterSnapshot = createSnapshot(pointsGetter());
  };

  return {
    type: 'batch-add',
    timestamp: Date.now(),
    description,
    finalize,
    undo: () => {
      pointsSetter(createSnapshot(beforeSnapshot));
    },
    redo: () => {
      if (afterSnapshot === null) {
        console.warn(`[UndoManager] redo skipped (missing afterSnapshot): ${description}`);
        return;
      }
      pointsSetter(createSnapshot(afterSnapshot));
    },
  };
}

/**
 * 创建批量编辑的撤销操作
 * 
 * @param pointsGetter 获取points数组的函数
 * @param pointsSetter 设置points数组的函数
 * @param description 操作描述
 * @returns 可撤销操作
 */
export function createBatchEditUndoAction<T>(
  pointsGetter: () => T[],
  pointsSetter: (points: T[]) => void,
  description: string
): UndoableAction {
  // 保存操作前的状态
  const beforeSnapshot = createSnapshot(pointsGetter());
  
  // 保存操作后的状态（在push时捕获）
  let afterSnapshot: T[] | null = null;
  const finalize = () => {
    if (afterSnapshot === null) afterSnapshot = createSnapshot(pointsGetter());
  };

  return {
    type: 'batch-edit',
    timestamp: Date.now(),
    description,
    finalize,
    undo: () => {
      pointsSetter(createSnapshot(beforeSnapshot));
    },
    redo: () => {
      if (afterSnapshot === null) {
        console.warn(`[UndoManager] redo skipped (missing afterSnapshot): ${description}`);
        return;
      }
      pointsSetter(createSnapshot(afterSnapshot));
    },
  };
}

/**
 * 创建删除行的撤销操作
 * 
 * @param pointsGetter 获取points数组的函数
 * @param pointsSetter 设置points数组的函数
 * @param deletedPointKeys 删除的点位键列表
 * @param description 操作描述
 * @returns 可撤销操作
 */
export function createDeleteRowsUndoAction<T>(
  pointsGetter: () => T[],
  pointsSetter: (points: T[]) => void,
  _deletedPointKeys: string[],
  description: string
): UndoableAction {
  // 保存操作前的状态
  const beforeSnapshot = createSnapshot(pointsGetter());
  
  // 保存操作后的状态（在push时捕获）
  let afterSnapshot: T[] | null = null;
  const finalize = () => {
    if (afterSnapshot === null) afterSnapshot = createSnapshot(pointsGetter());
  };

  return {
    type: 'delete-rows',
    timestamp: Date.now(),
    description,
    finalize,
    undo: () => {
      pointsSetter(createSnapshot(beforeSnapshot));
    },
    redo: () => {
      if (afterSnapshot === null) {
        console.warn(`[UndoManager] redo skipped (missing afterSnapshot): ${description}`);
        return;
      }
      pointsSetter(createSnapshot(afterSnapshot));
    },
  };
}
