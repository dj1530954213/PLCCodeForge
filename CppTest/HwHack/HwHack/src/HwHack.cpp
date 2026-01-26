#include "stdafx.h"

#include "HwHackRuntime.h"

static hw::Runtime g_Runtime;

/**
 * @brief Win32 定时器回调，转发到运行时。
 * @param hwnd 主窗口句柄。
 * @param uMsg 消息类型（未用）。
 * @param idEvent 定时器 ID。
 * @param dwTime 系统时间（未用）。
 */
static void CALLBACK TimerProc(HWND hwnd, UINT uMsg, UINT_PTR idEvent, DWORD dwTime) {
    g_Runtime.OnTimer(hwnd, idEvent);
}

/**
 * @brief 控制台线程入口。
 * @param lpParam DLL 句柄。
 * @return 线程退出码。
 */
static DWORD WINAPI ConsoleThread(LPVOID lpParam) {
    // 控制台线程负责交互与注入流程，避免阻塞主线程。
    g_Runtime.SetTimerProc(TimerProc);
    g_Runtime.RunConsole();
    FreeLibraryAndExitThread(reinterpret_cast<HMODULE>(lpParam), 0);
    return 0;
}

class CHwHackApp : public CWinApp {
public:
    /**
     * @brief DLL 初始化入口，启动控制台线程。
     * @return TRUE 表示初始化成功。
     */
    virtual BOOL InitInstance() {
        CWinApp::InitInstance();
        // DLL 初始化时创建后台线程启动控制台。
        ::CreateThread(NULL, 0, ConsoleThread, m_hInstance, 0, NULL);
        return TRUE;
    }
};

CHwHackApp theApp;
