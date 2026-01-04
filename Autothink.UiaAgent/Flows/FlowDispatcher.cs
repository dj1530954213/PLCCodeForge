using System.Text.Json;
using Autothink.UiaAgent.Rpc.Contracts;

namespace Autothink.UiaAgent.Flows;

/// <summary>
/// Flow 分发器：按 FlowName 路由到具体流程实现。
/// </summary>
internal static class FlowDispatcher
{
    public static RpcResult<RunFlowResponse> Dispatch(FlowContext context, string flowName, JsonElement? args)
    {
        ArgumentNullException.ThrowIfNull(context);

        var result = new RpcResult<RunFlowResponse> { StepLog = context.StepLog };

        StepLogEntry dispatchStep = context.StartStep(
            stepId: "DispatchFlow",
            action: "Dispatch flow",
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["flowName"] = flowName,
            });

        if (string.IsNullOrWhiteSpace(flowName))
        {
            RpcError err = new()
            {
                Kind = RpcErrorKinds.InvalidArgument,
                Message = "FlowName must be provided",
            };
            context.MarkFailure(dispatchStep, err);
            result.Ok = false;
            result.Error = err;
            return result;
        }

        if (FlowRegistry.TryGet(flowName, out IFlow? flow) && flow is not null)
        {
            context.MarkSuccess(dispatchStep);

            if (!flow.IsImplemented)
            {
                return NotImplemented(context, result, flowName);
            }

            try
            {
                return flow.Run(context, args);
            }
            catch (Exception ex)
            {
                var err = new RpcError
                {
                    Kind = RpcErrorKinds.ActionError,
                    Message = "Flow execution failed",
                    Details = new Dictionary<string, string>(StringComparer.Ordinal)
                    {
                        ["exceptionType"] = ex.GetType().FullName ?? ex.GetType().Name,
                        ["exceptionMessage"] = ex.Message,
                    },
                };

                StepLogEntry step = context.StartStep(stepId: "FlowException", action: "FlowException");
                context.MarkFailure(step, err);

                result.Ok = false;
                result.Error = err;
                return result;
            }
        }

        // registry 未包含：
        // - 若属于已知 flow name：返回 NotImplemented（能力还未落地/未注册）。
        // - 否则：InvalidArgument（用户输入错）。
        if (FlowNames.IsKnown(flowName))
        {
            context.MarkSuccess(dispatchStep);
            return NotImplemented(context, result, flowName);
        }

        string available = string.Join(", ", FlowRegistry.KnownFlowNames);
        var error = new RpcError
        {
            Kind = RpcErrorKinds.InvalidArgument,
            Message = "Unknown flow",
            Details = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["flowName"] = flowName,
                ["availableFlows"] = available,
            },
        };

        // 把可用 flow 列表写入 StepLog.Parameters，便于现场定位。
        dispatchStep.Parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
        dispatchStep.Parameters["availableFlows"] = available;

        context.MarkFailure(dispatchStep, error);
        result.Ok = false;
        result.Error = error;
        return result;
    }

    private static RpcResult<RunFlowResponse> NotImplemented(FlowContext context, RpcResult<RunFlowResponse> result, string flowName)
    {
        StepLogEntry step = context.StartStep(
            stepId: "NotImplemented",
            action: "Flow not implemented",
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["flowName"] = flowName,
            });

        var error = new RpcError
        {
            Kind = RpcErrorKinds.NotImplemented,
            Message = "Flow is registered but not implemented yet",
            Details = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["flowName"] = flowName,
            },
        };

        context.MarkFailure(step, error);
        result.Ok = false;
        result.Error = error;
        return result;
    }
}
