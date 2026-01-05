// 说明:
// - 覆盖 RunFlow 分发逻辑：未知 flow、未实现 flow 的错误语义与 StepLog 结构。
using Autothink.UiaAgent.Rpc;
using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Flows;
using Xunit;

namespace Autothink.UiaAgent.Tests;

/// <summary>
/// RunFlow 分发/路由的单元测试。
/// </summary>
public sealed class RunFlowDispatchTests
{
    [Fact]
    public void RunFlow_UnknownFlow_ReturnsInvalidArgument_WithoutSessionResolution()
    {
        var svc = new UiaRpcService();

        RpcResult<RunFlowResponse> result = svc.RunFlow(new RunFlowRequest
        {
            SessionId = "session-1",
            FlowName = "autothink.unknown",
            TimeoutMs = 1,
        });

        Assert.False(result.Ok);
        Assert.NotNull(result.Error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, result.Error!.Kind);
        Assert.Contains("Unknown flow", result.Error.Message, StringComparison.OrdinalIgnoreCase);

        Assert.True(result.StepLog.Steps.Count >= 2);
        Assert.Equal("ValidateRequest", result.StepLog.Steps[0].StepId);
        Assert.Equal(StepOutcomes.Success, result.StepLog.Steps[0].Outcome);

        StepLogEntry dispatch = result.StepLog.Steps[1];
        Assert.Equal("DispatchFlow", dispatch.StepId);
        Assert.Equal(StepOutcomes.Fail, dispatch.Outcome);
        Assert.NotNull(dispatch.Error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, dispatch.Error!.Kind);
    }

    [Fact]
    public void RunFlow_RegisteredButNotImplemented_ReturnsNotImplemented_WithoutSessionResolution()
    {
        var svc = new UiaRpcService();

        using IDisposable _ = FlowRegistry.OverrideForTests("autothink.build", new StubFlow("autothink.build"));

        RpcResult<RunFlowResponse> result = svc.RunFlow(new RunFlowRequest
        {
            SessionId = "session-1",
            FlowName = "autothink.build",
            TimeoutMs = 1,
        });

        Assert.False(result.Ok);
        Assert.NotNull(result.Error);
        Assert.Equal(RpcErrorKinds.NotImplemented, result.Error!.Kind);

        Assert.True(result.StepLog.Steps.Count >= 3);
        Assert.Equal("ValidateRequest", result.StepLog.Steps[0].StepId);
        Assert.Equal(StepOutcomes.Success, result.StepLog.Steps[0].Outcome);

        Assert.Equal("DispatchFlow", result.StepLog.Steps[1].StepId);
        Assert.Equal(StepOutcomes.Success, result.StepLog.Steps[1].Outcome);

        Assert.Equal("NotImplemented", result.StepLog.Steps[2].StepId);
        Assert.Equal(StepOutcomes.Fail, result.StepLog.Steps[2].Outcome);
        Assert.NotNull(result.StepLog.Steps[2].Error);
        Assert.Equal(RpcErrorKinds.NotImplemented, result.StepLog.Steps[2].Error!.Kind);
    }

    private sealed class StubFlow : IFlow
    {
        public StubFlow(string name)
        {
            this.Name = name;
        }

        public string Name { get; }

        public bool IsImplemented => false;

        public RpcResult<RunFlowResponse> Run(FlowContext context, System.Text.Json.JsonElement? args)
        {
            throw new NotSupportedException();
        }
    }
}
