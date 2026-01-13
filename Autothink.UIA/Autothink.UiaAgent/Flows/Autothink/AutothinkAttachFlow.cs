// 说明:
// - autothink.attach：用于附加到 AUTOTHINK 主窗口并验证会话可用。
// - 该 flow 仅做“窗口获取 + 置前”，不包含业务操作。
using System.Text.Json;
using Autothink.UiaAgent.Rpc.Contracts;
using FlaUI.Core.AutomationElements;

namespace Autothink.UiaAgent.Flows.Autothink;

/// <summary>
/// AUTOTHINK 附加流程：确保 main window 可用并返回基本信息。
/// </summary>
internal sealed class AutothinkAttachFlow : IFlow
{
    public string Name => FlowNames.AutothinkAttach;

    public bool IsImplemented => true;

    public RpcResult<RunFlowResponse> Run(FlowContext context, JsonElement? args)
    {
        ArgumentNullException.ThrowIfNull(context);

        var result = new RpcResult<RunFlowResponse> { StepLog = context.StepLog };

        // 业务步骤：获取主窗口 -> 尝试置前 -> 返回进程与窗口信息。
        StepLogEntry mainWindowStep = context.StartStep(stepId: "GetMainWindow", action: "Get main window");
        Window mainWindow;
        try
        {
            mainWindow = context.Session.GetMainWindow(context.Timeout);
            context.MarkSuccess(mainWindowStep);
        }
        catch (Exception ex)
        {
            var error = new RpcError
            {
                Kind = RpcErrorKinds.ConfigError,
                Message = "Failed to get main window",
                Details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["exceptionType"] = ex.GetType().FullName ?? ex.GetType().Name,
                    ["exceptionMessage"] = ex.Message,
                },
            };

            context.MarkFailure(mainWindowStep, error);
            result.Ok = false;
            result.Error = error;
            return result;
        }

        StepLogEntry focusStep = context.StartStep(stepId: "BringToForeground", action: "Bring main window to foreground");
        try
        {
            mainWindow.Focus();
            context.MarkSuccess(focusStep);
        }
        catch (Exception ex)
        {
            // Non-fatal: record as warning.
            var warn = new RpcError
            {
                Kind = RpcErrorKinds.ActionError,
                Message = "Failed to bring window to foreground",
                Details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["exceptionType"] = ex.GetType().FullName ?? ex.GetType().Name,
                    ["exceptionMessage"] = ex.Message,
                },
            };

            context.MarkWarning(focusStep, warn);
        }

        JsonElement data = JsonSerializer.SerializeToElement(
            new
            {
                processId = context.Session.ProcessId,
                mainWindowTitle = mainWindow.Title,
            });

        result.Ok = true;
        result.Value = new RunFlowResponse { Data = data };
        return result;
    }
}
