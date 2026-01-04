using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace Autothink.UiaAgent.Uia;

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
