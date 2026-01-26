#pragma once

#include "HwHackContext.h"
#include "HwHackInject.h"
#include "HwHackState.h"
#include "HwHackTree.h"

namespace hw {

/// <summary>
/// 运行时入口：窗口定位、树扫描与控制台交互。
/// </summary>
class Runtime {
public:
    /// <summary>
    /// 构造并初始化核心组件。
    /// </summary>
    Runtime();

    /// <summary>
    /// 设置定时器回调（从 DLL 入口传入）。
    /// </summary>
    void SetTimerProc(TimerProcFn proc);
    /// <summary>
    /// 启动控制台交互主循环。
    /// </summary>
    void RunConsole();
    /// <summary>
    /// 响应 Win32 定时器事件。
    /// </summary>
    void OnTimer(HWND hwnd, UINT_PTR idEvent);

    /// <summary>
    /// 访问全局状态。
    /// </summary>
    AppState& state();

private:
    /// <summary>
    /// 定位主窗口句柄。
    /// </summary>
    bool FindMainWindow();
    /// <summary>
    /// 定位 TreeView 句柄。
    /// </summary>
    bool FindTreeView();
    /// <summary>
    /// 打印启动提示。
    /// </summary>
    void PrintIntro();

    AppState state_;
    TreeScanner tree_;
    ContextResolver resolver_;
    Injector injector_;
};

}  // namespace hw
