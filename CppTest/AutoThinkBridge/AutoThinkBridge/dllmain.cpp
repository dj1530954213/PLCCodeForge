#include <Windows.h>
#include <iostream>
// 必须包含 MinHook.h
#include <MinHook.h> 

// =============================================================
// 用户配置区域 (请务必修改！)
// =============================================================

// 1. 添加线圈的偏移量 (之前确认是 0x931A0)
const DWORD OFFSET_ADD_COIL = 0x931A0;

// 2. 鼠标点击的偏移量 
// 计算公式：0x6400AFB0 - LDMDL.dll当前的基址
// 请务必填入计算后的结果！
const DWORD OFFSET_LBUTTON_DOWN = 0x9AFB0; // <--- 这里填你算出的新偏移！

// =============================================================

// 全局变量
void* g_AutoCapturedECX = nullptr;
DWORD_PTR g_ModuleBase = 0;

// 函数指针定义
typedef void(__thiscall* tOnLButtonDown)(void* pThis, UINT nFlags, POINT point);
typedef void(__thiscall* tOnAddLDCoil)(void* pThis);

// 保存原函数的 trampoline 指针
tOnLButtonDown fpOnLButtonDown = NULL;

// -------------------------------------------------------------
// 我们的间谍函数 (Detour)
// -------------------------------------------------------------
void __fastcall Detour_OnLButtonDown(void* pThis, void* _edx, UINT nFlags, POINT point) {

    // 逻辑：只有当 ECX 发生变化（或者是第一次抓到）时，才弹窗
    // 这样不会导致每次点击都弹窗，烦死人
    if (g_AutoCapturedECX != pThis) {
        g_AutoCapturedECX = pThis;

        // 【修改点】这里换成了弹窗
        // 注意：弹窗会暂停程序，直到你点确定
        MessageBoxA(NULL, "成功捕获 ECX 对象！\n现在可以使用 CE 远程调用了。", "Hook 提示", MB_OK | MB_ICONINFORMATION);
    }

    // 放行：调用原函数，保证软件正常运行
    fpOnLButtonDown(pThis, nFlags, point);
}

// 初始化 Hook
void SetupHook() {
    g_ModuleBase = (DWORD_PTR)GetModuleHandle(L"LDMDL.dll");
    if (g_ModuleBase == 0) return;

    if (MH_Initialize() != MH_OK) return;

    DWORD_PTR targetAddress = g_ModuleBase + OFFSET_LBUTTON_DOWN;

    // 创建 Hook
    if (MH_CreateHook((LPVOID)targetAddress, &Detour_OnLButtonDown, (LPVOID*)&fpOnLButtonDown) != MH_OK) {
        MessageBoxA(NULL, "Hook 创建失败！请检查偏移量。", "错误", MB_OK | MB_ICONERROR);
        return;
    }

    // 启用 Hook
    if (MH_EnableHook(MH_ALL_HOOKS) != MH_OK) {
        MessageBoxA(NULL, "Hook 启用失败！", "错误", MB_OK | MB_ICONERROR);
        return;
    }
}

// 导出函数：供 Cheat Engine 调用测试
extern "C" __declspec(dllexport) void RunTest() {

    if (g_AutoCapturedECX == nullptr) {
        MessageBoxA(NULL, "还未捕获 ECX！\n请先回到软件，用鼠标点击一下梯形图区域。", "等待触发", MB_ICONWARNING);
        return;
    }

    DWORD_PTR funcAddress = g_ModuleBase + OFFSET_ADD_COIL;
    tOnAddLDCoil OnAddLDCoil = (tOnAddLDCoil)funcAddress;

    try {
        OnAddLDCoil(g_AutoCapturedECX);
        MessageBoxA(NULL, "添加线圈指令已执行！", "成功", MB_OK);
    }
    catch (...) {
        MessageBoxA(NULL, "调用崩溃！ECX 可能已损坏。", "致命错误", MB_ICONERROR);
    }
}

// DLL 入口
BOOL APIENTRY DllMain(HMODULE hModule, DWORD  ul_reason_for_call, LPVOID lpReserved)
{
    switch (ul_reason_for_call)
    {
    case DLL_PROCESS_ATTACH:
        SetupHook(); // 注入即安装
        break;
    case DLL_PROCESS_DETACH:
        MH_Uninitialize(); // 清理
        break;
    }
    return TRUE;
}