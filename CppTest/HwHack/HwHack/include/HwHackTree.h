#pragma once

#include <string>
#include <vector>

#include "HwHackState.h"
#include "HwHackTypes.h"
#include "HwHackUtils.h"

namespace hw {

/// <summary>
/// TreeView 扫描与调试输出工具（保留为后续验证基石）。
/// </summary>
class TreeScanner {
public:
    /// <summary>
    /// 绑定运行时状态。
    /// </summary>
    explicit TreeScanner(AppState& state);

    /// <summary>
    /// 获取当前 TreeView 句柄。
    /// </summary>
    HWND tree() const;
    /// <summary>
    /// 设置 TreeView 句柄。
    /// </summary>
    void SetTree(HWND hwnd);

    /// <summary>
    /// 获取 TreeItem 文本（UTF-8）。
    /// </summary>
    std::string GetTreeItemTextUtf8(HTREEITEM item) const;
    /// <summary>
    /// 获取指定树中的 TreeItem 文本（UTF-8）。
    /// </summary>
    std::string GetTreeItemTextUtf8(HWND tree, HTREEITEM item) const;
    /// <summary>
    /// 获取 TreeItem 文本（本地编码）。
    /// </summary>
    std::string GetTreeItemTextMbc(HTREEITEM item) const;
    /// <summary>
    /// 获取指定树中的 TreeItem 文本（本地编码）。
    /// </summary>
    std::string GetTreeItemTextMbc(HWND tree, HTREEITEM item) const;

    /// <summary>
    /// 打印 TreeItem 的路径。
    /// </summary>
    void DumpTreePath(HTREEITEM item, const char* label) const;
    /// <summary>
    /// 打印子节点列表（限制数量）。
    /// </summary>
    void DumpTreeChildren(HTREEITEM parent, const char* label, int maxCount) const;
    /// <summary>
    /// 统计子节点数量（带超时保护）。
    /// </summary>
    int CountTreeChildren(HTREEITEM parent) const;
    /// <summary>
    /// 安全获取整树节点数量。
    /// </summary>
    int GetTreeCountSafe() const;
    /// <summary>
    /// 输出目标节点前后的子节点统计信息。
    /// </summary>
    void DumpTargetChildren(HTREEITEM target, const char* label) const;
    /// <summary>
    /// 打印整棵树（可限制节点数/深度）。
    /// </summary>
    void DumpTreeAll(int maxNodes, int maxDepth) const;
    /// <summary>
    /// 打印树控件的基础信息。
    /// </summary>
    void DumpTreeInfo(HWND hwnd, const char* tag) const;

    /// <summary>
    /// 按文本查找节点（深度优先）。
    /// </summary>
    HTREEITEM FindNodeByText(HTREEITEM start, const char* targetText) const;
    /// <summary>
    /// 按映射 ID 查找节点（深度优先）。
    /// </summary>
    HTREEITEM FindNodeById(HTREEITEM start, void* mapTree, FnMapTreeToId mapTreeToId,
                           int targetId) const;
    /// <summary>
    /// 收集指定父节点的子节点句柄。
    /// </summary>
    bool CollectChildren(HTREEITEM parent, std::vector<HTREEITEM>* out) const;
    /// <summary>
    /// 通过前后对比定位新增子节点。
    /// </summary>
    HTREEITEM FindNewChildByDiff(const std::vector<HTREEITEM>& before,
                                 const std::vector<HTREEITEM>& after,
                                 int* outNewCount) const;
    /// <summary>
    /// 获取同级节点图标索引。
    /// </summary>
    int GetSiblingImageIndex(HTREEITEM parent) const;
    /// <summary>
    /// 在 UI 树中插入节点（SmartInsert 路径）。
    /// </summary>
    HTREEITEM SmartInsertNode(HTREEITEM parent, const CString& name, const CString& desc,
                              int image, LPARAM lParam);

private:
    /// <summary>
    /// 打印 TreeItem 简要信息。
    /// </summary>
    void DumpTreeItemSummary(HWND hTree, const char* label, HTREEITEM hItem) const;
    /// <summary>
    /// 递归打印树结构。
    /// </summary>
    void DumpTreeRecursive(HTREEITEM item, int depth, int* count, int maxNodes,
                           int maxDepth) const;

    AppState& state_;
};

}  // namespace hw
