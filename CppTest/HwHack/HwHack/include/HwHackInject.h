#pragma once

#include "HwHackContext.h"
#include "HwHackState.h"
#include "HwHackTree.h"

namespace hw {

/// <summary>
/// 注入控制器：定时触发解析与插入逻辑。
/// </summary>
class Injector {
public:
    /// <summary>
    /// 绑定状态、树扫描与上下文解析器。
    /// </summary>
    Injector(AppState& state, TreeScanner& tree, ContextResolver& resolver);

    /// <summary>
    /// 处理注入/日志定时器事件。
    /// </summary>
    void HandleTimer(HWND hwnd, UINT_PTR idEvent);

private:
    /// <summary>
    /// 注入后延迟 Dump。
    /// </summary>
    void HandleDumpTimer(HWND hwnd);
    /// <summary>
    /// 注入主流程入口。
    /// </summary>
    void HandleInjectTimer(HWND hwnd);

    AppState& state_;
    TreeScanner& tree_;
    ContextResolver& resolver_;
};

}  // namespace hw
