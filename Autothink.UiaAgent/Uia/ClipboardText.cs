// 说明:
// - ClipboardText 是早期剪贴板写入工具，提供简单重试机制。
// - 新增的 ClipboardAdapter 会提供更细的失败分类；该类保留用于兼容与测试。
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace Autothink.UiaAgent.Uia;

/// <summary>
/// 旧版剪贴板写入辅助：基于重试与超时的最小实现。
/// </summary>
internal static class ClipboardText
{
    private static readonly TimeSpan DefaultTimeout = TimeSpan.FromSeconds(2);
    private static readonly TimeSpan DefaultRetryInterval = TimeSpan.FromMilliseconds(50);

    public static void SetTextWithRetry(string? text)
    {
        SetTextWithRetry(text, DefaultTimeout, DefaultRetryInterval);
    }

    public static void SetTextWithRetry(string? text, TimeSpan timeout, TimeSpan retryInterval)
    {
        if (timeout < TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(timeout));
        }

        if (retryInterval <= TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(retryInterval));
        }

        if (Thread.CurrentThread.GetApartmentState() != ApartmentState.STA)
        {
            throw new InvalidOperationException("Clipboard access requires STA thread.");
        }

        string value = text ?? string.Empty;

        Exception? lastException = null;
        DateTimeOffset deadline = DateTimeOffset.UtcNow.Add(timeout);

        while (DateTimeOffset.UtcNow <= deadline)
        {
            try
            {
                Clipboard.SetDataObject(value, true);

                if (Clipboard.ContainsText() && Clipboard.GetText() == value)
                {
                    return;
                }
            }
            catch (ExternalException ex)
            {
                // Most common: OpenClipboard failed (clipboard is busy).
                lastException = ex;
            }

            Thread.Sleep(retryInterval);
        }

        if (lastException is not null)
        {
            throw new InvalidOperationException("Failed to set clipboard text.", lastException);
        }

        throw new InvalidOperationException("Failed to set clipboard text.");
    }
}
