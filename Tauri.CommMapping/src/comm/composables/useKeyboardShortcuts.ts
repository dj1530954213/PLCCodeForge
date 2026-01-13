import { onMounted, onUnmounted } from 'vue';

/**
 * 键盘快捷键配置接口
 * 遵循ISP：只包含快捷键所需的字段
 */
export interface KeyboardShortcut {
  key: string;                    // 按键（如 'b', 'e', 'z', 'Delete'）
  ctrl?: boolean;                 // 是否需要Ctrl键
  shift?: boolean;                // 是否需要Shift键
  alt?: boolean;                  // 是否需要Alt键
  handler: () => void;            // 处理函数
  description: string;            // 快捷键描述
  preventDefault?: boolean;       // 是否阻止默认行为（默认true）
}

/**
 * 键盘快捷键上下文接口
 */
export interface KeyboardShortcutsContext {
  onBatchAdd?: () => void;
  onBatchEdit?: () => void;
  onDelete?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  onSave?: () => void;
}


/**
 * 检查键盘事件是否匹配快捷键配置
 * 
 * @param event 键盘事件
 * @param shortcut 快捷键配置
 * @returns 是否匹配
 */
function matchesShortcut(event: KeyboardEvent, shortcut: KeyboardShortcut): boolean {
  // 检查按键
  if (event.key.toLowerCase() !== shortcut.key.toLowerCase()) {
    return false;
  }

  // 检查修饰键
  if (Boolean(shortcut.ctrl) !== event.ctrlKey) return false;
  if (Boolean(shortcut.shift) !== event.shiftKey) return false;
  if (Boolean(shortcut.alt) !== event.altKey) return false;

  return true;
}

/**
 * 键盘快捷键 Composable
 * 遵循SRP：只负责键盘事件处理和快捷键管理
 * 
 * @param shortcuts 快捷键配置数组
 * @returns 注册和注销函数
 */
export function useKeyboardShortcuts(shortcuts: KeyboardShortcut[]) {
  const handleKeyDown = (event: KeyboardEvent) => {
    // 如果焦点在输入框、文本域或可编辑元素上，不处理快捷键
    const target = event.target as HTMLElement;
    if (
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable
    ) {
      // 但允许Esc键在输入框中工作
      if (event.key !== 'Escape') {
        return;
      }
    }

    // 查找匹配的快捷键
    for (const shortcut of shortcuts) {
      if (matchesShortcut(event, shortcut)) {
        // 阻止默认行为
        if (shortcut.preventDefault !== false) {
          event.preventDefault();
        }
        
        // 执行处理函数
        shortcut.handler();
        break;
      }
    }
  };

  const register = () => {
    window.addEventListener('keydown', handleKeyDown);
  };

  const unregister = () => {
    window.removeEventListener('keydown', handleKeyDown);
  };

  // 自动注册和注销
  onMounted(register);
  onUnmounted(unregister);

  return {
    register,
    unregister,
  };
}

/**
 * 创建标准的键盘快捷键配置
 * 提供常用快捷键的预设配置
 * 
 * @param context 快捷键上下文
 * @returns 快捷键配置数组
 */
export function createStandardShortcuts(context: KeyboardShortcutsContext): KeyboardShortcut[] {
  const shortcuts: KeyboardShortcut[] = [];

  if (context.onBatchAdd) {
    shortcuts.push({
      key: 'b',
      ctrl: true,
      handler: context.onBatchAdd,
      description: '批量添加点位',
    });
  }

  if (context.onBatchEdit) {
    shortcuts.push({
      key: 'e',
      ctrl: true,
      handler: context.onBatchEdit,
      description: '批量编辑选中行',
    });
  }

  if (context.onDelete) {
    shortcuts.push({
      key: 'Delete',
      handler: context.onDelete,
      description: '删除选中行',
    });
  }

  if (context.onUndo) {
    shortcuts.push({
      key: 'z',
      ctrl: true,
      handler: context.onUndo,
      description: '撤销',
    });
  }

  if (context.onRedo) {
    // Ctrl+Shift+Z
    shortcuts.push({
      key: 'z',
      ctrl: true,
      shift: true,
      handler: context.onRedo,
      description: '重做',
    });
    
    // Ctrl+Y (alternative)
    shortcuts.push({
      key: 'y',
      ctrl: true,
      handler: context.onRedo,
      description: '重做',
    });
  }

  if (context.onSave) {
    shortcuts.push({
      key: 's',
      ctrl: true,
      handler: context.onSave,
      description: '保存',
    });
  }

  return shortcuts;
}
