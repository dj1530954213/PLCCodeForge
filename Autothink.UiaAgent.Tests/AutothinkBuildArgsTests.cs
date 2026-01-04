using System.Text.Json;
using Autothink.UiaAgent.Flows.Autothink;
using Autothink.UiaAgent.Rpc.Contracts;
using Xunit;

namespace Autothink.UiaAgent.Tests;

public sealed class AutothinkBuildArgsTests
{
    [Fact]
    public void TryParseArgs_MissingBuildButtonSelector_ReturnsInvalidArgument()
    {
        var args = new
        {
            waitCondition = new WaitCondition
            {
                Kind = WaitConditionKinds.ElementEnabled,
                Selector = new ElementSelector
                {
                    Path =
                    {
                        new SelectorStep { ControlType = "Button" },
                    },
                },
            },
        };

        JsonElement element = JsonSerializer.SerializeToElement(args);

        bool ok = AutothinkBuildFlow.TryParseArgs(element, out _, out RpcError? error);

        Assert.False(ok);
        Assert.NotNull(error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, error!.Kind);
        Assert.Contains("BuildButtonSelector", error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void TryParseArgs_MissingWaitConditionSelector_ReturnsInvalidArgument()
    {
        var args = new
        {
            buildButtonSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Button" },
                },
            },
            waitCondition = new WaitCondition
            {
                Kind = WaitConditionKinds.ElementEnabled,
            },
        };

        JsonElement element = JsonSerializer.SerializeToElement(args);

        bool ok = AutothinkBuildFlow.TryParseArgs(element, out _, out RpcError? error);

        Assert.False(ok);
        Assert.NotNull(error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, error!.Kind);
        Assert.Contains("WaitCondition", error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void TryParseArgs_InvalidSearchRoot_ReturnsInvalidArgument()
    {
        var args = new
        {
            buildButtonSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Button" },
                },
            },
            waitCondition = new WaitCondition
            {
                Kind = WaitConditionKinds.ElementEnabled,
                Selector = new ElementSelector
                {
                    Path =
                    {
                        new SelectorStep { ControlType = "Button" },
                    },
                },
            },
            searchRoot = "invalid-root",
        };

        JsonElement element = JsonSerializer.SerializeToElement(args);

        bool ok = AutothinkBuildFlow.TryParseArgs(element, out _, out RpcError? error);

        Assert.False(ok);
        Assert.NotNull(error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, error!.Kind);
        Assert.Contains("SearchRoot", error.Message, StringComparison.OrdinalIgnoreCase);
    }
}
