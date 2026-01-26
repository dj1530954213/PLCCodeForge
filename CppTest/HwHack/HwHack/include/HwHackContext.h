#pragma once

#include "HwHackState.h"
#include "HwHackTree.h"
#include "HwHackTypes.h"

namespace hw {

/// <summary>
/// 解析注入所需上下文（容器/父节点/Link/索引）。
/// </summary>
class ContextResolver {
public:
    /// <summary>
    /// 绑定运行时状态。
    /// </summary>
    explicit ContextResolver(AppState& state);

    /// <summary>
    /// 解析上下文（可能抛异常，由调用方保证安全）。
    /// </summary>
    bool Resolve(DWORD rawParentData, const char* targetName, ResolvedContext* out);
    /// <summary>
    /// 安全解析：带 SEH 保护。
    /// </summary>
    bool SafeResolve(DWORD rawParentData, const char* targetName, ResolvedContext* out);

private:
    /// <summary>
    /// 将阶段标识转换为中文描述。
    /// </summary>
    const char* StageToZh(const char* stage) const;
    /// <summary>
    /// 更新当前阶段。
    /// </summary>
    void SetStage(const char* stage);
    /// <summary>
    /// 输出线程与进程信息。
    /// </summary>
    void LogThreadInfo() const;

    AppState& state_;
};

}  // namespace hw
