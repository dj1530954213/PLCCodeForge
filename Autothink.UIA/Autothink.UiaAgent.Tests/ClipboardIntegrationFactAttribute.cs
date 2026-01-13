// 说明:
// - 用环境变量控制剪贴板集成测试的启用/跳过，避免 CI 环境误触系统剪贴板。
using Xunit;

namespace Autothink.UiaAgent.Tests;

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
/// <summary>
/// 剪贴板集成测试标记：只有显式启用时才执行。
/// </summary>
public sealed class ClipboardIntegrationFactAttribute : FactAttribute
{
    public ClipboardIntegrationFactAttribute()
    {
        if (!string.Equals(Environment.GetEnvironmentVariable("UIA_CLIPBOARD_IT"), "1", StringComparison.Ordinal))
        {
            this.Skip = "Set UIA_CLIPBOARD_IT=1 to enable clipboard integration test.";
        }
    }
}
