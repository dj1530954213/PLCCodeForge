#include "stdafx.h"

#include "HwHackTree.h"

#include <algorithm>
#include <commctrl.h>
#include <iostream>
#include <vector>

namespace hw {

/**
 * @brief 绑定运行时状态对象。
 * @param state 全局运行时状态引用。
 */
TreeScanner::TreeScanner(AppState& state) : state_(state) {}

/**
 * @brief 获取当前 TreeView 句柄。
 * @return TreeView 句柄。
 */
HWND TreeScanner::tree() const { return state_.treeView; }

/**
 * @brief 设置 TreeView 句柄。
 * @param hwnd TreeView 窗口句柄。
 */
void TreeScanner::SetTree(HWND hwnd) { state_.treeView = hwnd; }

/**
 * @brief 获取 TreeItem 文本（UTF-8）。
 * @param item TreeItem 句柄。
 * @return UTF-8 文本。
 */
std::string TreeScanner::GetTreeItemTextUtf8(HTREEITEM item) const {
    return GetTreeItemTextUtf8(state_.treeView, item);
}

/**
 * @brief 获取指定树控件中的 TreeItem 文本（UTF-8）。
 * @param hTree TreeView 句柄。
 * @param item TreeItem 句柄。
 * @return UTF-8 文本。
 */
std::string TreeScanner::GetTreeItemTextUtf8(HWND hTree, HTREEITEM item) const {
    if (!hTree || !item) return std::string();
    LRESULT ok = 0;
    // TreeView 可能是 Unicode 或 ANSI，按窗口类型选择消息与缓冲区。
    if (IsWindowUnicode(hTree)) {
        wchar_t wbuf[256] = {0};
        TVITEMW tvi = {};
        tvi.mask = TVIF_TEXT | TVIF_HANDLE;
        tvi.hItem = item;
        tvi.pszText = wbuf;
        tvi.cchTextMax = static_cast<int>(sizeof(wbuf) / sizeof(wbuf[0]) - 1);
        if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETITEMW, 0, reinterpret_cast<LPARAM>(&tvi),
                            &ok) ||
            !ok) {
            return std::string();
        }
        return ToUtf8FromWide(wbuf);
    }
    char buf[256] = {0};
    TVITEMA tvi = {};
    tvi.mask = TVIF_TEXT | TVIF_HANDLE;
    tvi.hItem = item;
    tvi.pszText = buf;
    tvi.cchTextMax = static_cast<int>(sizeof(buf) / sizeof(buf[0]) - 1);
    if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETITEMA, 0, reinterpret_cast<LPARAM>(&tvi),
                        &ok) ||
        !ok) {
        return std::string();
    }
    return ToUtf8FromAnsi(buf);
}

/**
 * @brief 获取 TreeItem 文本（本地编码）。
 * @param item TreeItem 句柄。
 * @return 本地编码文本。
 */
std::string TreeScanner::GetTreeItemTextMbc(HTREEITEM item) const {
    return GetTreeItemTextMbc(state_.treeView, item);
}

/**
 * @brief 获取指定树控件中的 TreeItem 文本（本地编码）。
 * @param hTree TreeView 句柄。
 * @param item TreeItem 句柄。
 * @return 本地编码文本。
 */
std::string TreeScanner::GetTreeItemTextMbc(HWND hTree, HTREEITEM item) const {
    if (!hTree || !item) return std::string();
    LRESULT ok = 0;
    char buf[256] = {0};
    TVITEMA tvi = {};
    tvi.mask = TVIF_TEXT | TVIF_HANDLE;
    tvi.hItem = item;
    tvi.pszText = buf;
    tvi.cchTextMax = static_cast<int>(sizeof(buf) / sizeof(buf[0]) - 1);
    if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETITEMA, 0, reinterpret_cast<LPARAM>(&tvi),
                        &ok) ||
        !ok) {
        return std::string();
    }
    return std::string(buf);
}

/**
 * @brief 输出 TreeItem 的路径。
 * @param item TreeItem 句柄。
 * @param label 日志标签。
 */
void TreeScanner::DumpTreePath(HTREEITEM item, const char* label) const {
    HWND hTree = state_.treeView;
    if (!hTree || !item) return;
    std::vector<std::string> parts;
    HTREEITEM cur = item;
    while (cur) {
        std::string text = GetTreeItemTextUtf8(cur);
        if (!text.empty()) parts.push_back(text);
        cur = reinterpret_cast<HTREEITEM>(
            ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_PARENT, reinterpret_cast<LPARAM>(cur)));
    }
    std::reverse(parts.begin(), parts.end());
    std::cout << "[DBG] TreePath(" << label << ")=";
    for (size_t i = 0; i < parts.size(); ++i) {
        if (i) std::cout << " / ";
        std::cout << parts[i];
    }
    std::cout << "\n";
}

/**
 * @brief 输出指定父节点下的子节点列表。
 * @param parent 父节点 TreeItem。
 * @param label 日志标签。
 * @param maxCount 最大输出数量。
 */
void TreeScanner::DumpTreeChildren(HTREEITEM parent, const char* label, int maxCount) const {
    HWND hTree = state_.treeView;
    if (!hTree || !parent) return;
    int printed = 0;
    std::cout << "[DBG] TreeChildren(" << label << ")\n";
    LRESULT res = 0;
    // 通过带超时的消息获取子节点，避免 UI 线程被阻塞。
    if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETNEXTITEM, TVGN_CHILD,
                        reinterpret_cast<LPARAM>(parent), &res)) {
        std::cout << "[DBG] TreeChildren(" << label << ") timeout\n";
        return;
    }
    HTREEITEM child = reinterpret_cast<HTREEITEM>(res);
    while (child && printed < maxCount) {
        std::string text = GetTreeItemTextUtf8(child);
        std::cout << "[DBG]  - child[" << printed << "] handle=0x" << std::hex
                  << reinterpret_cast<uintptr_t>(child) << std::dec << " text=" << text << "\n";
        ++printed;
        if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETNEXTITEM, TVGN_NEXT,
                            reinterpret_cast<LPARAM>(child), &res)) {
            std::cout << "[DBG] TreeChildren(" << label << ") timeout\n";
            return;
        }
        child = reinterpret_cast<HTREEITEM>(res);
    }
    if (child) {
        std::cout << "[DBG]  - ... more\n";
    }
}

/**
 * @brief 统计指定父节点的子节点数量。
 * @param parent 父节点 TreeItem。
 * @return 子节点数量；超时返回 -1。
 */
int TreeScanner::CountTreeChildren(HTREEITEM parent) const {
    HWND hTree = state_.treeView;
    if (!hTree || !parent) return 0;
    int count = 0;
    LRESULT res = 0;
    if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETNEXTITEM, TVGN_CHILD,
                        reinterpret_cast<LPARAM>(parent), &res)) {
        return -1;
    }
    HTREEITEM child = reinterpret_cast<HTREEITEM>(res);
    while (child) {
        ++count;
        if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETNEXTITEM, TVGN_NEXT,
                            reinterpret_cast<LPARAM>(child), &res)) {
            return -1;
        }
        child = reinterpret_cast<HTREEITEM>(res);
    }
    return count;
}

/**
 * @brief 安全获取整棵树节点数量。
 * @return 节点数量；超时返回 -1。
 */
int TreeScanner::GetTreeCountSafe() const {
    HWND hTree = state_.treeView;
    if (!hTree) return 0;
    LRESULT res = 0;
    if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETCOUNT, 0, 0, &res)) {
        return -1;
    }
    return static_cast<int>(res);
}

/**
 * @brief 输出目标节点前后子节点统计信息。
 * @param target 目标 TreeItem。
 * @param label 日志标签。
 */
void TreeScanner::DumpTargetChildren(HTREEITEM target, const char* label) const {
    if (!target) return;
    int treeCount = GetTreeCountSafe();
    int childCount = CountTreeChildren(target);
    if (childCount >= 0) {
        std::cout << "[DBG] Target 子节点(" << label << ") count=" << childCount << "\n";
    } else {
        std::cout << "[DBG] Target 子节点(" << label << ") count=timeout\n";
    }
    DumpTreeChildren(target, label, state_.settings.dumpTreeChildrenLimit);
    if (treeCount >= 0) {
        std::cout << "[DBG] TreeCount(" << label << ")=" << treeCount << "\n";
    } else {
        std::cout << "[DBG] TreeCount(" << label << ")=timeout\n";
    }
}

/**
 * @brief 递归输出树结构。
 * @param item 当前 TreeItem。
 * @param depth 当前深度。
 * @param count 已输出计数（输入/输出）。
 * @param maxNodes 最大节点数（0 表示不限制）。
 * @param maxDepth 最大深度（0 表示不限制）。
 */
void TreeScanner::DumpTreeRecursive(HTREEITEM item, int depth, int* count, int maxNodes,
                                    int maxDepth) const {
    HWND hTree = state_.treeView;
    if (!hTree || !item || !count) return;
    // 递归输出树结构，同时限制节点数与深度，避免过深遍历。
    if (maxNodes > 0 && *count >= maxNodes) return;
    if (maxDepth > 0 && depth > maxDepth) return;
    std::string text = GetTreeItemTextUtf8(item);
    std::string indent(static_cast<size_t>(depth * 2), ' ');
    std::cout << "[DBG] TreeNode " << indent << "handle=0x" << std::hex
              << reinterpret_cast<uintptr_t>(item) << std::dec << " text=" << text << "\n";
    ++(*count);
    HTREEITEM child = reinterpret_cast<HTREEITEM>(
        ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_CHILD, reinterpret_cast<LPARAM>(item)));
    while (child) {
        DumpTreeRecursive(child, depth + 1, count, maxNodes, maxDepth);
        if (maxNodes > 0 && *count >= maxNodes) return;
        child = reinterpret_cast<HTREEITEM>(
            ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_NEXT, reinterpret_cast<LPARAM>(child)));
    }
}

/**
 * @brief 输出整棵树内容。
 * @param maxNodes 最大节点数（0 表示不限制）。
 * @param maxDepth 最大深度（0 表示不限制）。
 */
void TreeScanner::DumpTreeAll(int maxNodes, int maxDepth) const {
    HWND hTree = state_.treeView;
    if (!hTree) return;
    int count = 0;
    std::cout << "[DBG] TreeDump start\n";
    HTREEITEM root = reinterpret_cast<HTREEITEM>(
        ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_ROOT, 0));
    while (root) {
        DumpTreeRecursive(root, 0, &count, maxNodes, maxDepth);
        if (maxNodes > 0 && count >= maxNodes) break;
        root = reinterpret_cast<HTREEITEM>(
            ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_NEXT, reinterpret_cast<LPARAM>(root)));
    }
    std::cout << "[DBG] TreeDump end count=" << count << "\n";
    if (maxNodes > 0 && count >= maxNodes) {
        std::cout << "[DBG] TreeDump reached maxNodes=" << maxNodes << "\n";
    }
}

/**
 * @brief 输出 TreeItem 简要信息。
 * @param hTree TreeView 句柄。
 * @param label 日志标签。
 * @param hItem TreeItem 句柄。
 */
void TreeScanner::DumpTreeItemSummary(HWND hTree, const char* label, HTREEITEM hItem) const {
    if (!hTree || !hItem) return;
    std::string text = GetTreeItemTextUtf8(hTree, hItem);
    std::cout << "[DBG] TreeItem " << label << " handle=0x" << std::hex
              << reinterpret_cast<uintptr_t>(hItem) << std::dec << " text=" << text << "\n";
}

/**
 * @brief 输出树控件的基础信息。
 * @param hwnd TreeView 句柄。
 * @param tag 日志标签。
 */
void TreeScanner::DumpTreeInfo(HWND hwnd, const char* tag) const {
    if (!hwnd) return;
    std::string className = GetClassNameUtf8(hwnd);
    std::string title = GetWindowTextUtf8(hwnd);
    std::string parentTitle;
    RECT rc = {};
    RECT crc = {};
    GetWindowRect(hwnd, &rc);
    GetClientRect(hwnd, &crc);
    HWND hParent = GetParent(hwnd);
    if (hParent) parentTitle = GetWindowTextUtf8(hParent);
    DWORD pid = 0;
    DWORD tid = GetWindowThreadProcessId(hwnd, &pid);
    LONG_PTR style = GetWindowLongPtr(hwnd, GWL_STYLE);
    LONG_PTR exStyle = GetWindowLongPtr(hwnd, GWL_EXSTYLE);
    int count = static_cast<int>(::SendMessage(hwnd, TVM_GETCOUNT, 0, 0));
    HTREEITEM hRoot = reinterpret_cast<HTREEITEM>(
        ::SendMessage(hwnd, TVM_GETNEXTITEM, TVGN_ROOT, 0));
    HTREEITEM hSel = reinterpret_cast<HTREEITEM>(
        ::SendMessage(hwnd, TVM_GETNEXTITEM, TVGN_CARET, 0));
    std::cout << "[DBG] TreeInfo(" << tag << ") hwnd=0x" << std::hex
              << reinterpret_cast<uintptr_t>(hwnd) << " id=" << std::dec << GetDlgCtrlID(hwnd)
              << " class=" << className << " title=" << title << " parent=0x" << std::hex
              << reinterpret_cast<uintptr_t>(hParent) << " parentTitle=" << parentTitle
              << " pid=" << std::dec << pid << " tid=" << tid << " style=0x" << std::hex
              << static_cast<uintptr_t>(style) << " exStyle=0x" << static_cast<uintptr_t>(exStyle)
              << std::dec << " rect=(" << rc.left << "," << rc.top << "," << rc.right << ","
              << rc.bottom << ")" << " client=(" << crc.left << "," << crc.top << "," << crc.right
              << "," << crc.bottom << ")" << " count=" << count << "\n";
    DumpTreeItemSummary(hwnd, "root", hRoot);
    DumpTreeItemSummary(hwnd, "sel", hSel);
}

/**
 * @brief 按文本查找节点（深度优先）。
 * @param start 起始 TreeItem。
 * @param targetText 目标文本。
 * @return 命中节点；未命中返回 nullptr。
 */
HTREEITEM TreeScanner::FindNodeByText(HTREEITEM start, const char* targetText) const {
    HWND hTree = state_.treeView;
    if (!hTree || !start) return nullptr;
    // 深度优先遍历，命中目标文本后立即返回。
    HTREEITEM current = start;
    while (current) {
        std::string text = GetTreeItemTextUtf8(current);
        if (!text.empty() && targetText && *targetText) {
            if (text.find(targetText) != std::string::npos) {
                return current;
            }
        }
        HTREEITEM child = reinterpret_cast<HTREEITEM>(
            ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_CHILD, reinterpret_cast<LPARAM>(current)));
        if (child) {
            HTREEITEM result = FindNodeByText(child, targetText);
            if (result) return result;
        }
        current = reinterpret_cast<HTREEITEM>(
            ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_NEXT, reinterpret_cast<LPARAM>(current)));
    }
    return nullptr;
}

/**
 * @brief 按映射 ID 查找节点（深度优先）。
 * @param start 起始 TreeItem。
 * @param mapTree Tree->ID 映射对象。
 * @param mapTreeToId 映射查询函数。
 * @param targetId 目标 ID。
 * @return 命中节点；未命中返回 nullptr。
 */
HTREEITEM TreeScanner::FindNodeById(HTREEITEM start, void* mapTree, FnMapTreeToId mapTreeToId,
                                    int targetId) const {
    HWND hTree = state_.treeView;
    if (!hTree || !start || !mapTree || !mapTreeToId) return nullptr;
    // 通过 TreeItem->ID 映射进行匹配，仍使用深度优先遍历。
    HTREEITEM current = start;
    while (current) {
        int* slot = mapTreeToId(mapTree, static_cast<int>(reinterpret_cast<uintptr_t>(current)));
        if (slot && *slot == targetId) {
            return current;
        }
        HTREEITEM child = reinterpret_cast<HTREEITEM>(
            ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_CHILD, reinterpret_cast<LPARAM>(current)));
        if (child) {
            HTREEITEM result = FindNodeById(child, mapTree, mapTreeToId, targetId);
            if (result) return result;
        }
        current = reinterpret_cast<HTREEITEM>(
            ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_NEXT, reinterpret_cast<LPARAM>(current)));
    }
    return nullptr;
}

/**
 * @brief 收集指定父节点的子节点句柄。
 * @param parent 父节点 TreeItem。
 * @param out 输出子节点列表。
 * @return 成功返回 true；超时返回 false。
 */
bool TreeScanner::CollectChildren(HTREEITEM parent, std::vector<HTREEITEM>* out) const {
    if (!out) return false;
    out->clear();
    HWND hTree = state_.treeView;
    if (!hTree || !parent) return false;
    LRESULT res = 0;
    if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETNEXTITEM, TVGN_CHILD,
                        reinterpret_cast<LPARAM>(parent), &res)) {
        return false;
    }
    HTREEITEM child = reinterpret_cast<HTREEITEM>(res);
    while (child) {
        out->push_back(child);
        if (!TrySendTreeMsg(state_.settings, hTree, TVM_GETNEXTITEM, TVGN_NEXT,
                            reinterpret_cast<LPARAM>(child), &res)) {
            return false;
        }
        child = reinterpret_cast<HTREEITEM>(res);
    }
    return true;
}

/**
 * @brief 通过前后对比定位新增子节点。
 * @param before 修改前子节点列表。
 * @param after 修改后子节点列表。
 * @param outNewCount 新增数量输出（可选）。
 * @return 唯一新增节点句柄；不唯一返回 nullptr。
 */
HTREEITEM TreeScanner::FindNewChildByDiff(const std::vector<HTREEITEM>& before,
                                          const std::vector<HTREEITEM>& after,
                                          int* outNewCount) const {
    int newCount = 0;
    HTREEITEM candidate = nullptr;
    for (HTREEITEM item : after) {
        if (std::find(before.begin(), before.end(), item) == before.end()) {
            ++newCount;
            if (!candidate) {
                candidate = item;
            }
        }
    }
    if (outNewCount) *outNewCount = newCount;
    return (newCount == 1) ? candidate : nullptr;
}

/**
 * @brief 获取同级节点图标索引。
 * @param parent 父节点 TreeItem。
 * @return 图标索引；失败返回 -1。
 */
int TreeScanner::GetSiblingImageIndex(HTREEITEM parent) const {
    HWND hTree = state_.treeView;
    if (!hTree || !parent) return -1;
    HTREEITEM child = reinterpret_cast<HTREEITEM>(
        ::SendMessage(hTree, TVM_GETNEXTITEM, TVGN_CHILD, reinterpret_cast<LPARAM>(parent)));
    if (!child) return -1;
    TVITEMA tvi;
    ZeroMemory(&tvi, sizeof(tvi));
    tvi.mask = TVIF_IMAGE | TVIF_SELECTEDIMAGE | TVIF_HANDLE;
    tvi.hItem = child;
    if (!::SendMessage(hTree, TVM_GETITEMA, 0, reinterpret_cast<LPARAM>(&tvi))) return -1;
    return tvi.iImage;
}

/**
 * @brief 在树中插入节点并确保可见。
 * @param parent 父节点 TreeItem。
 * @param name 名称。
 * @param desc 描述。
 * @param image 图标索引。
 * @param lParam 附加参数。
 * @return 新插入节点；失败返回 nullptr。
 */
HTREEITEM TreeScanner::SmartInsertNode(HTREEITEM parent, const CString& name, const CString& desc,
                                       int image, LPARAM lParam) {
    HWND hTree = state_.treeView;
    if (!hTree || !parent) return nullptr;
    TVINSERTSTRUCT tvi;
    tvi.hParent = parent;
    tvi.hInsertAfter = TVI_LAST;
    tvi.item.mask = TVIF_TEXT | TVIF_PARAM | TVIF_IMAGE | TVIF_SELECTEDIMAGE;
    CString displayText;
    displayText.Format("%s(%s:%s)", name, desc, name);
    tvi.item.pszText = (LPSTR)(LPCTSTR)displayText;
    tvi.item.iImage = image;
    tvi.item.iSelectedImage = image;
    tvi.item.lParam = lParam;
    HTREEITEM newItem = reinterpret_cast<HTREEITEM>(
        ::SendMessage(hTree, TVM_INSERTITEM, 0, reinterpret_cast<LPARAM>(&tvi)));
    if (newItem) {
        // 确保父节点展开且新节点可见，便于后续映射。
        TreeView_Expand(hTree, parent, TVE_EXPAND);
        TreeView_EnsureVisible(hTree, newItem);
    }
    return newItem;
}

}  // namespace hw
