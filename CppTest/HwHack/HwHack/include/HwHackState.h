#pragma once

#include <windows.h>

#include "HwHackConfig.h"
#include "HwHackTypes.h"

namespace hw {

/// <summary>
/// 定时器回调签名（Win32 计时器）。
/// </summary>
typedef void (CALLBACK *TimerProcFn)(HWND, UINT, UINT_PTR, DWORD);

/// <summary>
/// 全局运行时状态：窗口句柄、目标节点与注入参数。
/// </summary>
struct AppState {
    /// 配置开关。
    Settings settings{};

    /// 主窗口句柄。
    HWND mainWnd = nullptr;
    /// 树控件句柄（主）。
    HWND treeView = nullptr;
    /// 树控件句柄（备用）。
    HWND treeViewFallback = nullptr;

    /// 目标 TreeItem。
    HTREEITEM targetItem = nullptr;
    /// 注入后等待 Dump 的目标节点。
    HTREEITEM pendingDumpTarget = nullptr;

    /// 当前解析阶段标识。
    const char* lastStage = "init";

    /// 用户输入的目标名称（原始）。
    char targetName[256] = {};
    /// TreeItem 文本（full）。
    char targetNameFull[256] = {};
    /// TreeItem 文本（short）。
    char targetNameShort[256] = {};
    /// TreeItem 文本（type）。
    char targetNameType[256] = {};

    /// 注入参数缓存。
    InjectionParams params{};

    /// 注入定时器回调。
    TimerProcFn timerProc = nullptr;
};

}  // namespace hw
