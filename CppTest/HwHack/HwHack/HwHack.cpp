#include "stdafx.h"

#include <afxwin.h>

#include <commctrl.h>

#include <iostream>

#include <cstdio>

#include <cstdlib>

#include <string>

  

// ============================================================================

// 1. 配置区域

// ============================================================================

const DWORD OFFSET_MAKE_NEW = 0x59F10;

const DWORD OFFSET_GET_DEVICE = 0x50770; // 务必确认此偏移

  

// ============================================================================

// 2. 类型定义

// ============================================================================

typedef char (__thiscall *FnMakeNewLogicData_Slave)(

    void* pThis, CString name, unsigned int typeID, char flag, unsigned int* pOutID,

    void* pParent, void* pLink, CString desc, unsigned int count, void* pContext

);

typedef void* (__thiscall *FnGetDeviceByLogicID)(void* pThis, unsigned int id);

  

struct InjectionParams {

    DWORD addrInstance;

    DWORD valParentData; // 存 ID 或 指针

    DWORD addrLink;

} g_Params;

  

HWND g_hMainWnd = NULL;

HWND g_hTreeView = NULL;

  

// ============================================================================

// 3. 基础查找工具

// ============================================================================

BOOL IsConsoleWindow(HWND hwnd) {

    char className[256];

    if (GetClassNameA(hwnd, className, sizeof(className))) {

        if (strstr(className, "Console") != NULL) return TRUE;

    }

    return FALSE;

}

  

BOOL CALLBACK FindRealMainWindowCallback(HWND hwnd, LPARAM lParam) {

    DWORD lpdwProcessId;

    GetWindowThreadProcessId(hwnd, &lpdwProcessId);

    if (lpdwProcessId == GetCurrentProcessId()) {

        if (IsWindowVisible(hwnd) && GetParent(hwnd) == NULL) {

            if (IsConsoleWindow(hwnd)) return TRUE;

            char title[256];

            GetWindowTextA(hwnd, title, sizeof(title));

            if (strstr(title, "AutoThink") != NULL) {

                if (strstr(title, "-") != NULL || strlen(title) > 10) {

                    g_hMainWnd = hwnd;

                    return FALSE;

                }

            }

        }

    }

    return TRUE;

}

  

BOOL CALLBACK FindTreeViewCallback(HWND hwnd, LPARAM lParam) {

    char className[256];

    GetClassNameA(hwnd, className, sizeof(className));

    if (strstr(className, "SysTreeView32") != NULL || strstr(className, "Tree") != NULL) {

        if (IsWindowVisible(hwnd)) {

            g_hTreeView = hwnd;

            return FALSE;

        }

    }

    return TRUE;

}

  

// ============================================================================

// 4. [新增] 树节点搜索算法 (递归)

// ============================================================================

HTREEITEM FindNodeByText(HTREEITEM hStart, const char* targetText) {

    if (!hStart) return NULL;

  

    HTREEITEM hCurrent = hStart;

    while (hCurrent) {

        // 1. 获取当前节点的文字

        char buf[256] = {0};

        TVITEMA tvi;

        tvi.mask = TVIF_TEXT | TVIF_HANDLE;

        tvi.hItem = hCurrent;

        tvi.pszText = buf;

        tvi.cchTextMax = 255;

        ::SendMessage(g_hTreeView, TVM_GETITEMA, 0, (LPARAM)&tvi);

  

        // 2. 匹配检查 (模糊匹配)

        // 例如输入 "LK220"，如果节点叫 "LK220 (LK220)" 也能匹配

        if (strstr(buf, targetText) != NULL) {

            return hCurrent; // 找到了！

        }

  

        // 3. 递归查找子节点

        HTREEITEM hChild = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_CHILD, (LPARAM)hCurrent);

        if (hChild) {

            HTREEITEM hResult = FindNodeByText(hChild, targetText);

            if (hResult) return hResult; // 在子树里找到了

        }

  

        // 4. 查找下一个兄弟节点

        hCurrent = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_NEXT, (LPARAM)hCurrent);

    }

    return NULL;

}

  

// ============================================================================

// 5. 智能插入节点

// ============================================================================

void SmartInsertNode(HTREEITEM hParent, CString name, CString desc, void* pRealDeviceObject) {

    if (hParent) {

        TVINSERTSTRUCT tvi;

        tvi.hParent = hParent;

        tvi.hInsertAfter = TVI_LAST;

        tvi.item.mask = TVIF_TEXT | TVIF_PARAM | TVIF_IMAGE | TVIF_SELECTEDIMAGE;

        CString displayText;

        displayText.Format("%s (%s:%s)", name, desc, name);

        tvi.item.pszText = (LPSTR)(LPCTSTR)displayText;

        tvi.item.iImage = 4;          

        tvi.item.iSelectedImage = 4;  

        tvi.item.lParam = (LPARAM)pRealDeviceObject;

  

        HTREEITEM hNewItem = (HTREEITEM)::SendMessage(g_hTreeView, TVM_INSERTITEM, 0, (LPARAM)&tvi);

        if (hNewItem) {

            TreeView_Expand(g_hTreeView, hParent, TVE_EXPAND);

            TreeView_EnsureVisible(g_hTreeView, hNewItem);

        }

    }

}

  

// ============================================================================

// 6. 执行注入 (Timer Callback)

// ============================================================================

void CALLBACK MyTimerProc(HWND hwnd, UINT uMsg, UINT_PTR idEvent, DWORD dwTime)

{

    if (idEvent == 7777)

    {

        KillTimer(hwnd, idEvent);

        AFX_MANAGE_STATE(AfxGetStaticModuleState());

  

        HMODULE hDll = GetModuleHandleA("dllDPLogic.dll");

        if (hDll) {

            FnMakeNewLogicData_Slave MakeSlave = (FnMakeNewLogicData_Slave)((DWORD)hDll + OFFSET_MAKE_NEW);

            FnGetDeviceByLogicID GetDevice = (FnGetDeviceByLogicID)((DWORD)hDll + OFFSET_GET_DEVICE);

  

            // 1. 处理 Parent (ID 转 指针)

            void* pRealParent = NULL;

            DWORD rawData = g_Params.valParentData;

  

            if (rawData < 0x100000) {

                // 是 ID，转换

                pRealParent = GetDevice((void*)g_Params.addrInstance, rawData);

                if (!pRealParent) {

                    std::cout << "[-] Failed to convert ID " << rawData << " to pointer.\n";

                    return;

                }

            } else {

                pRealParent = (void*)rawData;

            }

  

            // 2. 注入

            CString strName;

            strName.Format("AUTO_SLAVE_%d", rand() % 1000);

            CString strDesc = "192.168.2.39";

            unsigned int newID = 0;

  

            try {

                char result = MakeSlave(

                    (void*)g_Params.addrInstance, strName, 1, 0, &newID,

                    pRealParent, (void*)g_Params.addrLink, strDesc, 1, pRealParent

                );

  

                if (result) {

                    void* pDeviceObj = GetDevice((void*)g_Params.addrInstance, newID);

                    if (pDeviceObj) {

                        // 重新获取一下选中的节点，或者我们需要把 hTargetItem 传进来

                        // 这里简化处理：我们刚刚选中的那个节点应该还是选中的

                        HTREEITEM hTarget = TreeView_GetSelection(g_hTreeView);

                        SmartInsertNode(hTarget, strName, strDesc, pDeviceObj);

                        std::cout << "[SUCCESS] Injected " << strName << " (ID: " << newID << ")\n";

                        Beep(1500, 100);

                    }

                } else {

                    std::cout << "[FAIL] Injection returned 0.\n";

                }

            }

            catch (...) { std::cout << "[CRASH]\n"; }

        }

    }

}

  

// ============================================================================

// 7. 工程化控制台 (Search Mode)

// ============================================================================

DWORD WINAPI ConsoleThread(LPVOID lpParam)

{

    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    AllocConsole();

    FILE* f; freopen_s(&f, "CONIN$", "r", stdin); freopen_s(&f, "CONOUT$", "w", stdout);

    std::cout << "=== ICS Auto-Config V10.0 (Engineering Mode) ===\n";

  

    // 1. 找窗口

    while (!g_hMainWnd) {

        EnumWindows(FindRealMainWindowCallback, 0);

        if (!g_hMainWnd) Sleep(1000);

    }

    std::cout << "[OK] Main Window Locked.\n";

    // 2. 找 TreeView

    while (!g_hTreeView) {

        EnumChildWindows(g_hMainWnd, FindTreeViewCallback, 0);

        if (!g_hTreeView) Sleep(1000);

    }

    std::cout << "[OK] TreeView Found.\n";

    std::cout << "----------------------------------------\n";

    // 3. 配置一次 (后续可以写死在配置文件里)

    DWORD cachedECX = 0;

    DWORD cachedLink = 0;

  

    std::cout << ">> Setup ECX: "; std::cin >> std::hex >> cachedECX;

    std::cout << ">> Setup Link: "; std::cin >> std::hex >> cachedLink;

    std::cin.ignore(); // 清除换行符

  

    std::cout << "----------------------------------------\n";

    std::cout << "SYSTEM READY. Type parent node name to inject.\n";

    std::cout << "Example: LK220, ETHERNET, GROUP1\n";

    std::cout << "----------------------------------------\n";

  

    char targetName[256];

    while (true) {

        std::cout << "\nTarget Parent Name > ";

        std::cin.getline(targetName, 256);

  

        if (strlen(targetName) == 0) continue;

        if (strcmp(targetName, "exit") == 0) break;

  

        std::cout << "[*] Searching for node: '" << targetName << "'...\n";

  

        // 4. 自动遍历查找

        // ✅ 正确写法

        HTREEITEM hRoot = (HTREEITEM)::SendMessage(g_hTreeView, TVM_GETNEXTITEM, TVGN_ROOT, 0);

        HTREEITEM hFound = FindNodeByText(hRoot, targetName);

  

        if (hFound) {

            // 选中它 (为了视觉反馈，也为了 SmartInsertNode 能用)

            TreeView_SelectItem(g_hTreeView, hFound);

            TreeView_EnsureVisible(g_hTreeView, hFound);

  

            // 获取 ID

            TVITEM tvi;

            tvi.mask = TVIF_PARAM | TVIF_HANDLE;

            tvi.hItem = hFound;

            ::SendMessage(g_hTreeView, TVM_GETITEM, 0, (LPARAM)&tvi);

            DWORD parentData = (DWORD)tvi.lParam;

            std::cout << "[+] Node Found! Data: " << std::hex << parentData << "\n";

  

            // 触发注入

            g_Params.addrInstance = cachedECX;

            g_Params.valParentData = parentData;

            g_Params.addrLink = cachedLink;

  

            ::SetTimer(g_hMainWnd, 7777, 10, (TIMERPROC)MyTimerProc);

        } else {

            std::cout << "[-] Node NOT found. Check spelling.\n";

        }

    }

  

    FreeConsole(); FreeLibraryAndExitThread((HMODULE)lpParam, 0);

    return 0;

}

  

class CHwHackApp : public CWinApp {

public: CHwHackApp() {}

    virtual BOOL InitInstance() { CWinApp::InitInstance(); ::CreateThread(NULL, 0, ConsoleThread, m_hInstance, 0, NULL); return TRUE; }

};

CHwHackApp theApp;