using System.Runtime.InteropServices;
using System.Security;
using System.Windows.Forms;

namespace Autothink.UiaAgent.Uia;

internal enum ClipboardFailureKind
{
    AccessDenied,
    ClipboardBusy,
    Unexpected,
    Unsupported,
}

internal sealed class ClipboardAttemptResult
{
    public bool Ok { get; init; }
    public ClipboardFailureKind? FailureKind { get; init; }
    public string? Message { get; init; }
    public string? ExceptionType { get; init; }
}

internal static class ClipboardAdapter
{
    private const int ClipbrdCantOpen = unchecked((int)0x800401D0);

    public static ClipboardAttemptResult TrySetText(string? text)
    {
        if (Thread.CurrentThread.GetApartmentState() != ApartmentState.STA)
        {
            return new ClipboardAttemptResult
            {
                Ok = false,
                FailureKind = ClipboardFailureKind.Unsupported,
                Message = "Clipboard access requires STA thread.",
            };
        }

        string value = text ?? string.Empty;

        try
        {
            Clipboard.SetDataObject(value, true);

            if (Clipboard.ContainsText() && Clipboard.GetText() == value)
            {
                return new ClipboardAttemptResult { Ok = true };
            }

            return new ClipboardAttemptResult
            {
                Ok = false,
                FailureKind = ClipboardFailureKind.Unexpected,
                Message = "Clipboard verification failed.",
            };
        }
        catch (COMException ex) when (ex.HResult == ClipbrdCantOpen)
        {
            return new ClipboardAttemptResult
            {
                Ok = false,
                FailureKind = ClipboardFailureKind.ClipboardBusy,
                Message = ex.Message,
                ExceptionType = ex.GetType().FullName ?? ex.GetType().Name,
            };
        }
        catch (ExternalException ex)
        {
            return new ClipboardAttemptResult
            {
                Ok = false,
                FailureKind = ClipboardFailureKind.ClipboardBusy,
                Message = ex.Message,
                ExceptionType = ex.GetType().FullName ?? ex.GetType().Name,
            };
        }
        catch (UnauthorizedAccessException ex)
        {
            return new ClipboardAttemptResult
            {
                Ok = false,
                FailureKind = ClipboardFailureKind.AccessDenied,
                Message = ex.Message,
                ExceptionType = ex.GetType().FullName ?? ex.GetType().Name,
            };
        }
        catch (SecurityException ex)
        {
            return new ClipboardAttemptResult
            {
                Ok = false,
                FailureKind = ClipboardFailureKind.AccessDenied,
                Message = ex.Message,
                ExceptionType = ex.GetType().FullName ?? ex.GetType().Name,
            };
        }
        catch (Exception ex)
        {
            return new ClipboardAttemptResult
            {
                Ok = false,
                FailureKind = ClipboardFailureKind.Unexpected,
                Message = ex.Message,
                ExceptionType = ex.GetType().FullName ?? ex.GetType().Name,
            };
        }
    }
}
