import type { ByteOrder32, CommPoint, DataType } from "../api";

/**
 * 批量编辑请求接口
 * 遵循ISP：只包含批量编辑所需的字段
 */
export interface BatchEditRequest {
  pointKeys: string[];              // 要编辑的点位键列表
  dataType?: DataType;              // 可选：批量设置数据类型
  byteOrder?: ByteOrder32;          // 可选：批量设置字节序
  scaleExpression?: string;         // 可选：缩放倍数表达式（如 "2" 或 "{{x}}*10"）
}

/**
 * 单个编辑操作
 */
export interface EditOperation {
  pointKey: string;
  field: keyof CommPoint;
  oldValue: unknown;
  newValue: unknown;
}

/**
 * 批量编辑结果接口
 */
export interface BatchEditResult {
  operations: EditOperation[];      // 所有编辑操作
  affectedPoints: number;           // 受影响的点位数量
  totalChanges: number;             // 总变更数量
}

/**
 * 批量编辑预览接口
 */
export interface BatchEditPreview {
  totalRows: number;                // 总行数
  fieldsToUpdate: string[];         // 将要更新的字段列表
  estimatedChanges: number;         // 预计变更数量
}

/**
 * 缩放表达式编译结果
 */
export interface ScaleExpressionResult {
  ok: boolean;
  value?: number;
  error?: string;
}


/**
 * 计算批量编辑预览
 * 遵循SRP：只负责预览计算
 * 
 * @param request 批量编辑请求
 * @returns 预览信息
 */
export function computeBatchEditPreview(request: BatchEditRequest): BatchEditPreview {
  const fieldsToUpdate: string[] = [];
  let estimatedChanges = 0;

  if (request.dataType) {
    fieldsToUpdate.push('数据类型');
    estimatedChanges += request.pointKeys.length;
  }

  if (request.byteOrder) {
    fieldsToUpdate.push('字节序');
    estimatedChanges += request.pointKeys.length;
  }

  if (request.scaleExpression) {
    fieldsToUpdate.push('缩放倍数');
    estimatedChanges += request.pointKeys.length;
  }

  return {
    totalRows: request.pointKeys.length,
    fieldsToUpdate,
    estimatedChanges,
  };
}

/**
 * 计算批量编辑操作
 * 遵循SRP：只负责计算需要执行的编辑操作
 * 
 * @param points 所有点位数据
 * @param request 批量编辑请求
 * @returns 批量编辑结果
 */
export function computeBatchEdits(
  points: CommPoint[],
  request: BatchEditRequest
): BatchEditResult {
  const operations: EditOperation[] = [];
  const pointKeySet = new Set(request.pointKeys);

  // 遍历所有点位，找到需要编辑的点位
  for (const point of points) {
    if (!pointKeySet.has(point.pointKey)) {
      continue;
    }

    // 数据类型编辑
    if (request.dataType && point.dataType !== request.dataType) {
      operations.push({
        pointKey: point.pointKey,
        field: 'dataType',
        oldValue: point.dataType,
        newValue: request.dataType,
      });
    }

    // 字节序编辑
    if (request.byteOrder && point.byteOrder !== request.byteOrder) {
      operations.push({
        pointKey: point.pointKey,
        field: 'byteOrder',
        oldValue: point.byteOrder,
        newValue: request.byteOrder,
      });
    }

    // 缩放倍数编辑
    if (request.scaleExpression) {
      const scaleResult = evaluateScaleExpression(request.scaleExpression, point.scale);
      if (scaleResult.ok && scaleResult.value !== undefined && scaleResult.value !== point.scale) {
        operations.push({
          pointKey: point.pointKey,
          field: 'scale',
          oldValue: point.scale,
          newValue: scaleResult.value,
        });
      }
    }
  }

  return {
    operations,
    affectedPoints: pointKeySet.size,
    totalChanges: operations.length,
  };
}

/**
 * 评估缩放表达式
 * 支持固定值（如 "2"）或表达式（如 "{{x}}*10"）
 * 
 * @param expression 缩放表达式
 * @param currentValue 当前值
 * @returns 计算结果
 */
function evaluateScaleExpression(expression: string, currentValue: number): ScaleExpressionResult {
  const trimmed = expression.trim();
  
  if (!trimmed) {
    return { ok: false, error: '表达式不能为空' };
  }

  // 如果表达式不包含占位符，尝试作为固定值解析
  if (!trimmed.includes('{{x}}')) {
    const value = Number(trimmed);
    if (Number.isFinite(value)) {
      return { ok: true, value };
    }
    return { ok: false, error: '无效的数字格式' };
  }

  // 替换占位符并计算
  const replaced = trimmed.replace(/\{\{x\}\}/g, String(currentValue));
  
  try {
    // 使用 Function 构造器安全地评估表达式
    // 只允许数学运算，不允许访问其他对象
    const func = new Function('return (' + replaced + ')');
    const result = func();
    
    if (Number.isFinite(result)) {
      return { ok: true, value: result };
    }
    return { ok: false, error: '表达式结果不是有效数字' };
  } catch (e) {
    return { ok: false, error: `表达式错误: ${String(e)}` };
  }
}


/**
 * 应用批量编辑操作
 * 遵循原子性原则：要么全部成功，要么全部失败
 * 遵循SRP：只负责应用编辑操作
 * 
 * @param points 所有点位数据（会被修改）
 * @param result 批量编辑结果
 * @returns 修改后的点位数组
 */
export function applyBatchEdits(
  points: CommPoint[],
  result: BatchEditResult
): CommPoint[] {
  // 创建点位键到点位的映射，提高查找效率
  const pointMap = new Map<string, CommPoint>();
  for (const point of points) {
    pointMap.set(point.pointKey, point);
  }

  // 应用所有编辑操作
  for (const operation of result.operations) {
    const point = pointMap.get(operation.pointKey);
    if (!point) {
      continue;
    }

    // 根据字段类型应用修改
    switch (operation.field) {
      case 'dataType':
        point.dataType = operation.newValue as DataType;
        break;
      case 'byteOrder':
        point.byteOrder = operation.newValue as ByteOrder32;
        break;
      case 'scale':
        point.scale = operation.newValue as number;
        break;
    }
  }

  return points;
}

/**
 * 创建批量编辑的撤销操作
 * 用于撤销管理器
 * 
 * @param points 所有点位数据
 * @param result 批量编辑结果
 * @returns 撤销函数
 */
export function createBatchEditUndoOperation(
  points: CommPoint[],
  result: BatchEditResult
): () => void {
  // 保存原始值
  const originalValues = new Map<string, Map<string, unknown>>();
  
  for (const operation of result.operations) {
    if (!originalValues.has(operation.pointKey)) {
      originalValues.set(operation.pointKey, new Map());
    }
    originalValues.get(operation.pointKey)!.set(operation.field, operation.oldValue);
  }

  // 返回撤销函数
  return () => {
    const pointMap = new Map<string, CommPoint>();
    for (const point of points) {
      pointMap.set(point.pointKey, point);
    }

    for (const [pointKey, fieldValues] of originalValues) {
      const point = pointMap.get(pointKey);
      if (!point) continue;

      for (const [field, value] of fieldValues) {
        switch (field) {
          case 'dataType':
            point.dataType = value as DataType;
            break;
          case 'byteOrder':
            point.byteOrder = value as ByteOrder32;
            break;
          case 'scale':
            point.scale = value as number;
            break;
        }
      }
    }
  };
}
