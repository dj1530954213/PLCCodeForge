// 说明:
// - ClipboardAdapter 封装剪贴板写入的稳定性策略与错误分类。
// - 仅作为内部能力供 flow 调用，不暴露为 RPC 方法。
using System.Runtime.InteropServices;
using System.Security;
using System.Windows.Forms;

namespace Autothink.UiaAgent.Uia;

/// <summary>
/// 剪贴板写入失败的分类，用于诊断与降级决策。
/// </summary>
internal enum ClipboardFailureKind
{
    AccessDenied,
    ClipboardBusy,
    Unexpected,
    Unsupported,
}

/// <summary>
/// 单次剪贴板写入尝试的结果与诊断信息。
/// </summary>
internal sealed class ClipboardAttemptResult
{
    public bool Ok { get; init; }
    public ClipboardFailureKind? FailureKind { get; init; }
    public string? Message { get; init; }
    public string? ExceptionType { get; init; }
}

/// <summary>
/// 剪贴板访问适配器：在 STA 线程内执行写入并返回结构化错误信息。
/// </summary>
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
