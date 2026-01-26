#include "stdafx.h"

#include "HwHackRuntime.h"

#include <commctrl.h>
#include <cstring>
#include <iostream>
#include <string>

#include "HwHackUtils.h"

namespace hw {
namespace {

struct TreeEnumContext {
    AppState* state = nullptr;
    TreeScanner* tree = nullptr;
};

/**
 * @brief 判断窗口是否为控制台窗口。
 * @param hwnd 窗口句柄。
 * @return 是控制台窗口返回 TRUE。
 */
BOOL IsConsoleWindow(HWND hwnd) {
    char className[256];
    if (GetClassNameA(hwnd, className, sizeof(className))) {
        if (strstr(className, "Console") != nullptr) return TRUE;
    }
    return FALSE;
}

/**
 * @brief 枚举主窗口回调：定位 AutoThink 主窗口。
 * @param hwnd 当前窗口句柄。
 * @param lParam AppState 指针。
 * @return TRUE 继续枚举；FALSE 终止枚举。
 */
BOOL CALLBACK FindRealMainWindowCallback(HWND hwnd, LPARAM lParam) {
    AppState* state = reinterpret_cast<AppState*>(lParam);
    if (!state) return TRUE;
    DWORD pid = 0;
    GetWindowThreadProcessId(hwnd, &pid);
    if (pid == GetCurrentProcessId()) {
        if (IsWindowVisible(hwnd) && GetParent(hwnd) == NULL) {
            if (IsConsoleWindow(hwnd)) return TRUE;
            // 通过标题特征识别 AutoThink 主窗口。
            char title[256];
            GetWindowTextA(hwnd, title, sizeof(title));
            if (strstr(title, "AutoThink") != nullptr) {
                if (strstr(title, "-") != nullptr || strlen(title) > 10) {
                    state->mainWnd = hwnd;
                    return FALSE;
                }
            }
        }
    }
    return TRUE;
}

/**
 * @brief 枚举子窗口回调：定位 TreeView 控件。
 * @param hwnd 子窗口句柄。
 * @param lParam AppState 指针。
 * @return TRUE 继续枚举；FALSE 终止枚举。
 */
BOOL CALLBACK FindTreeViewCallback(HWND hwnd, LPARAM lParam) {
    AppState* state = reinterpret_cast<AppState*>(lParam);
    if (!state) return TRUE;
    char className[256];
    GetClassNameA(hwnd, className, sizeof(className));
    if (strstr(className, "SysTreeView32") != nullptr) {
        if (IsWindowVisible(hwnd)) {
            int ctrlId = GetDlgCtrlID(hwnd);
            // 优先匹配指定控件 ID，找不到则保留第一个可见树控件作为回退。
            if (ctrlId == state->settings.treeCtrlIdWanted) {
                state->treeView = hwnd;
                return FALSE;
            }
            if (!state->treeViewFallback) {
                state->treeViewFallback = hwnd;
            }
        }
    }
    return TRUE;
}

/**
 * @brief 枚举子窗口回调：输出 TreeView 候选信息。
 * @param hwnd 子窗口句柄。
 * @param lParam TreeEnumContext 指针。
 * @return TRUE 继续枚举。
 */
BOOL CALLBACK DumpTreeViewCallback(HWND hwnd, LPARAM lParam) {
    TreeEnumContext* ctx = reinterpret_cast<TreeEnumContext*>(lParam);
    if (!ctx || !ctx->tree) return TRUE;
    char className[256] = {0};
    GetClassNameA(hwnd, className, sizeof(className));
    if (strstr(className, "SysTreeView32") != nullptr) {
        // 仅用于调试输出候选树控件信息。
        ctx->tree->DumpTreeInfo(hwnd, "candidate");
    }
    return TRUE;
}

}  // namespace

/**
 * @brief 构造运行时对象并初始化各组件。
 */
Runtime::Runtime()
    : state_(),
      tree_(state_),
      resolver_(state_),
      injector_(state_, tree_, resolver_) {}

/**
 * @brief 设置 Win32 定时器回调。
 * @param proc 定时器回调函数。
 */
void Runtime::SetTimerProc(TimerProcFn proc) { state_.timerProc = proc; }

/**
 * @brief 获取全局运行时状态。
 * @return AppState 引用。
 */
AppState& Runtime::state() { return state_; }

/**
 * @brief 转发定时器事件到注入器。
 * @param hwnd 主窗口句柄。
 * @param idEvent 定时器 ID。
 */
void Runtime::OnTimer(HWND hwnd, UINT_PTR idEvent) { injector_.HandleTimer(hwnd, idEvent); }

/**
 * @brief 轮询定位主窗口。
 * @return 成功返回 true。
 */
bool Runtime::FindMainWindow() {
    // 轮询直到锁定主窗口，避免注入过早。
    while (!state_.mainWnd) {
        EnumWindows(FindRealMainWindowCallback, reinterpret_cast<LPARAM>(&state_));
        if (!state_.mainWnd) Sleep(1000);
    }
    return state_.mainWnd != nullptr;
}

/**
 * @brief 定位 TreeView 控件并绑定。
 * @return 成功返回 true。
 */
bool Runtime::FindTreeView() {
    TreeEnumContext ctx{&state_, &tree_};
    EnumChildWindows(state_.mainWnd, DumpTreeViewCallback, reinterpret_cast<LPARAM>(&ctx));

    while (!state_.treeView) {
        state_.treeViewFallback = nullptr;
        EnumChildWindows(state_.mainWnd, FindTreeViewCallback, reinterpret_cast<LPARAM>(&state_));
        if (!state_.treeView && state_.treeViewFallback) {
            state_.treeView = state_.treeViewFallback;
        }
        if (!state_.treeView) Sleep(1000);
    }
    // 绑定 TreeView 句柄，后续扫描/注入都依赖该句柄。
    tree_.SetTree(state_.treeView);
    return state_.treeView != nullptr;
}

/**
 * @brief 打印启动提示。
 */
void Runtime::PrintIntro() {
    std::cout << "=== ICS 自动组态 V11.0（工程模式） ===\n";
}

/**
 * @brief 控制台交互主循环：查找树节点并触发注入。
 */
void Runtime::RunConsole() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());
    // 初始化控制台并设置 UTF-8，确保中文日志可读。
    AllocConsole();
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
    FILE* f;
    freopen_s(&f, "CONIN$", "r", stdin);
    freopen_s(&f, "CONOUT$", "w", stdout);

    PrintIntro();

    if (!FindMainWindow()) {
        FreeConsole();
        return;
    }
    std::cout << "[OK] 主窗口已锁定。\n";
    if (!FindTreeView()) {
        FreeConsole();
        return;
    }

    std::cout << "[OK] 已找到树控件。hwnd=0x" << std::hex
              << reinterpret_cast<uintptr_t>(state_.treeView) << " id=" << std::dec
              << GetDlgCtrlID(state_.treeView) << "\n";
    tree_.DumpTreeInfo(state_.treeView, "selected");

    HTREEITEM root =
        reinterpret_cast<HTREEITEM>(::SendMessage(state_.treeView, TVM_GETNEXTITEM, TVGN_ROOT, 0));
    if (root) {
        tree_.DumpTreeChildren(root, "root", state_.settings.dumpTreeChildrenLimit);
        std::string hwText = ToUtf8FromAnsi("硬件配置");
        HTREEITEM hwNode = tree_.FindNodeByText(root, hwText.c_str());
        if (hwNode) {
            tree_.DumpTreePath(hwNode, "硬件配置");
            tree_.DumpTreeChildren(hwNode, "硬件配置", state_.settings.dumpTreeChildrenLimit);
        }
    }
    if (state_.settings.dumpTreeOnStart) {
        tree_.DumpTreeAll(state_.settings.dumpTreeMaxNodes, state_.settings.dumpTreeMaxDepth);
    }

    std::cout << "----------------------------------------\n";
    std::cout << "[AUTO] 已启用上下文解析器。\n";
    std::cout << "----------------------------------------\n";
    std::cout << "系统就绪，请输入父节点名称以注入。\n";
    std::cout << "示例：LK220、ETHERNET、GROUP1\n";
    std::cout << "----------------------------------------\n";

    char targetName[256];
    while (true) {
        std::cout << "\n目标父节点名称 > ";
        std::cin.getline(targetName, sizeof(targetName));

        if (strlen(targetName) == 0) continue;
        if (strcmp(targetName, "exit") == 0) break;

        std::cout << "[*] 正在查找节点：'" << targetName << "'...\n";

        HTREEITEM hRoot =
            reinterpret_cast<HTREEITEM>(::SendMessage(state_.treeView, TVM_GETNEXTITEM, TVGN_ROOT,
                                                      0));
        HTREEITEM hFound = tree_.FindNodeByText(hRoot, targetName);

        if (hFound) {
            TreeView_SelectItem(state_.treeView, hFound);
            TreeView_EnsureVisible(state_.treeView, hFound);

            TVITEM tvi;
            tvi.mask = TVIF_PARAM | TVIF_HANDLE;
            tvi.hItem = hFound;
            ::SendMessage(state_.treeView, TVM_GETITEM, 0, reinterpret_cast<LPARAM>(&tvi));
            DWORD parentData = static_cast<DWORD>(tvi.lParam);
            std::cout << "[+] 已找到节点！Data: " << std::hex << parentData << "\n";

            // 解析 TreeItem 文本：full/short/type 三种名称供上下文匹配。
            strncpy_s(state_.targetName, sizeof(state_.targetName), targetName, _TRUNCATE);
            std::string fullName = tree_.GetTreeItemTextMbc(hFound);
            strncpy_s(state_.targetNameFull, sizeof(state_.targetNameFull), fullName.c_str(),
                      _TRUNCATE);
            std::string shortName = fullName;
            std::string typeName;
            size_t lp = fullName.find('(');
            if (lp != std::string::npos) {
                shortName = fullName.substr(0, lp);
                size_t rp = fullName.find(')', lp + 1);
                if (rp != std::string::npos && rp > lp + 1) {
                    typeName = fullName.substr(lp + 1, rp - lp - 1);
                }
            }
            strncpy_s(state_.targetNameShort, sizeof(state_.targetNameShort), shortName.c_str(),
                      _TRUNCATE);
            strncpy_s(state_.targetNameType, sizeof(state_.targetNameType), typeName.c_str(),
                      _TRUNCATE);
            if (state_.settings.verbose) {
                std::cout << "[DBG] TreeItem文本(full)=" << ToUtf8FromAnsi(state_.targetNameFull)
                          << " short=" << ToUtf8FromAnsi(state_.targetNameShort)
                          << " type=" << ToUtf8FromAnsi(state_.targetNameType) << "\n";
            }

            state_.targetItem = hFound;
            state_.params.addrContainer = 0;
            state_.params.addrInstance = 0;
            state_.params.valParentData = parentData;
            state_.params.addrLink = 0;

            if (state_.settings.dumpTreeAfterInject && state_.treeView && state_.targetItem) {
                tree_.DumpTargetChildren(state_.targetItem, "target_before");
            }

            // 使用定时器触发注入，避免阻塞 UI 线程。
            if (state_.timerProc) {
                ::SetTimer(state_.mainWnd, state_.settings.injectTimerId, 10, state_.timerProc);
            } else {
                std::cout << "[-] TimerProc 未初始化，无法触发注入。\n";
            }
        } else {
            std::cout << "[-] 未找到节点，请检查名称拼写。\n";
        }
    }

    FreeConsole();
}

}  // namespace hw
