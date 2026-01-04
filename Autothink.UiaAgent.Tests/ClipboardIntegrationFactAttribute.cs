using Xunit;

namespace Autothink.UiaAgent.Tests;

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
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
