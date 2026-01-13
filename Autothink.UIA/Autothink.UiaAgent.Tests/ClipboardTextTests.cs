using System.Windows.Forms;
// 说明:
// - 覆盖 ClipboardText 基础行为与异常路径（非真实剪贴板联通）。
using Autothink.UiaAgent.Uia;
using Xunit;

namespace Autothink.UiaAgent.Tests;

/// <summary>
/// ClipboardText 的基础逻辑测试。
/// </summary>
public sealed class ClipboardTextTests
{
    [Fact]
    public void SetTextWithRetry_NotSta_Throws()
    {
        var ex = Assert.Throws<InvalidOperationException>(
            () => ClipboardText.SetTextWithRetry("hello", TimeSpan.FromMilliseconds(10), TimeSpan.FromMilliseconds(1)));

        Assert.Contains("STA", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [ClipboardIntegrationFact]
    public void SetTextWithRetry_WritesClipboardText()
    {
        string text = $"uia-agent-{Guid.NewGuid():N}";
        Exception? failure = null;

        var t = new Thread(() =>
        {
            try
            {
                ClipboardText.SetTextWithRetry(text, timeout: TimeSpan.FromSeconds(2), retryInterval: TimeSpan.FromMilliseconds(50));
                Assert.Equal(text, Clipboard.GetText());
            }
            catch (Exception ex)
            {
                failure = ex;
            }
        });

        t.SetApartmentState(ApartmentState.STA);
        t.Start();
        t.Join();

        if (failure is not null)
        {
            throw new Xunit.Sdk.XunitException(failure.ToString());
        }
    }
}
