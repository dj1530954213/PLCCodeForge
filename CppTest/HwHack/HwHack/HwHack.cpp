// HwHack.cpp : 定义 DLL 的初始化例程。
#include "stdafx.h"
#include <afxwin.h>
#include <iostream>
#include <iomanip>

// -----------------------------------------------------------------------------
// [1] 定义目标函数原型 (上帝函数)
// -----------------------------------------------------------------------------
typedef char (__thiscall *FnMakeNewLogicData_Slave)(
    void* pThis,                // ECX
    CString name,               // Arg1: Name
    unsigned int typeID,        // Arg2: Type
    char flag,                  // Arg3: Flag
    unsigned int* pOutID,       // Arg4: Output ID
    void* pParent,              // Arg5: Parent
    void* pLink,                // Arg6: Link
    CString desc,               // Arg7: Desc
    unsigned int count,         // Arg8: Count
    void* pContext              // Arg9: Context
);

// -----------------------------------------------------------------------------
// [2] 核心逻辑线程：弹窗控制台 + 循环输入执行
// -----------------------------------------------------------------------------
DWORD WINAPI ConsoleThread(LPVOID lpParam)
{
    // 必须加上这句，确保在新线程中使用 MFC 不会崩
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // A. 分配一个控制台窗口 (黑框框)
    AllocConsole();
    FILE* fDummy;
    freopen_s(&fDummy, "CONIN$", "r", stdin);
    freopen_s(&fDummy, "CONOUT$", "w", stdout);
    freopen_s(&fDummy, "CONOUT$", "w", stderr);

    std::cout << "========================================================\n";
    std::cout << "          ICS Auto-Config Internal Console              \n";
    std::cout << "========================================================\n";
    std::cout << "[+] Console Allocated inside Target Process.\n";
    std::cout << "[+] Waiting for user input...\n\n";

    // B. 获取函数地址 (只获取一次)
    HMODULE hDll = GetModuleHandleA("dllDPLogic.dll");
    if (!hDll) {
        std::cout << "[-] FATAL: dllDPLogic.dll not found in memory!\n";
        return 0;
    }
    // 偏移量 0x59F10 (基于之前的分析)
    DWORD funcAddr = (DWORD)hDll + 0x59F10;
    std::cout << "[+] dllDPLogic Base: " << std::hex << hDll << "\n";
    std::cout << "[+] Target Function: " << std::hex << funcAddr << "\n\n";

    FnMakeNewLogicData_Slave MakeSlave = (FnMakeNewLogicData_Slave)funcAddr;

    // C. 循环交互
    while (true) {
        DWORD addrInstance = 0;
        DWORD addrParent = 0;
        DWORD addrLink = 0;

        std::cout << "--------------------------------------------------------\n";
        std::cout << "Please enter Hex Addresses from x32dbg (Input 0 to exit)\n";
        
        std::cout << "1. ECX (CHWDataContainer Instance): ";
        std::cin >> std::hex >> addrInstance;
        if (addrInstance == 0) break;

        std::cout << "2. Parent Node (Arg5): ";
        std::cin >> std::hex >> addrParent;

        std::cout << "3. Link Object (Arg6): ";
        std::cin >> std::hex >> addrLink;

        std::cout << "\n[Run] Executing injection with:\n";
        std::cout << "   Instance: " << (void*)addrInstance << "\n";
        std::cout << "   Parent:   " << (void*)addrParent << "\n";
        std::cout << "   Link:     " << (void*)addrLink << "\n";

        // 构造参数
        CString strName;
        strName.Format("AutoSlave_%04X", rand() % 0xFFFF); // 随机名字防止冲突
        CString strDesc = "192.168.1.100";
        unsigned int newID = 0;

        try {
            // D. 调用上帝函数
            char result = MakeSlave(
                (void*)addrInstance,
                strName,
                1,              // TypeID = 1 (Slave)
                0,              // Flag
                &newID,         // Out ID
                (void*)addrParent,
                (void*)addrLink,
                strDesc,
                1,              // Count
                (void*)addrParent // Context
            );

            if (result) {
                std::cout << "\n[+] SUCCESS! Device Created.\n";
                std::cout << "    New ID: " << std::dec << newID << " (Hex: " << std::hex << newID << ")\n";
                std::cout << "    Name:   " << strName << "\n";
            } else {
                std::cout << "\n[-] FAILED. Function returned 0.\n";
            }
        }
        catch (...) {
            std::cout << "\n[!] EXCEPTION CRASH CAUGHT! check your addresses.\n";
        }
        
        std::cout << "\n"; // 空行
    }

    // 清理并退出
    FreeConsole();
    FreeLibraryAndExitThread((HMODULE)lpParam, 0);
    return 0;
}

// -----------------------------------------------------------------------------
// [3] MFC DLL 入口 (在 InitInstance 中启动线程)
// -----------------------------------------------------------------------------
class CHwHackApp : public CWinApp
{
public:
    BOOL InitInstance() override
    {
        CWinApp::InitInstance();
        // 注入成功后，立刻启动一个新线程运行控制台
        ::CreateThread(NULL, 0, ConsoleThread, AfxGetInstanceHandle(), 0, NULL);
        return TRUE;
    }
};

CHwHackApp theApp;
