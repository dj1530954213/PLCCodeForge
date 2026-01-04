using System.Text.Json;
using Autothink.UiaAgent.Flows.Autothink;
using Autothink.UiaAgent.Rpc.Contracts;
using Xunit;

namespace Autothink.UiaAgent.Tests;

public sealed class AutothinkImportVariablesArgsTests
{
    [Fact]
    public void TryParseArgs_EmptyFilePath_ReturnsInvalidArgument()
    {
        var args = new
        {
            filePath = "",
            filePathEditorSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Edit" },
                },
            },
            confirmButtonSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Button" },
                },
            },
            dialogSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Window" },
                },
            },
        };

        JsonElement element = JsonSerializer.SerializeToElement(args);

        bool ok = AutothinkImportVariablesFlow.TryParseArgs(element, out _, out RpcError? error);

        Assert.False(ok);
        Assert.NotNull(error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, error!.Kind);
        Assert.Contains("FilePath", error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void TryParseArgs_MissingFilePathEditorSelector_ReturnsInvalidArgument()
    {
        var args = new
        {
            filePath = "C:\\\\temp\\\\vars.xlsx",
            confirmButtonSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Button" },
                },
            },
            dialogSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Window" },
                },
            },
        };

        JsonElement element = JsonSerializer.SerializeToElement(args);

        bool ok = AutothinkImportVariablesFlow.TryParseArgs(element, out _, out RpcError? error);

        Assert.False(ok);
        Assert.NotNull(error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, error!.Kind);
        Assert.Contains("FilePathEditorSelector", error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void TryParseArgs_InvalidSearchRoot_ReturnsInvalidArgument()
    {
        var args = new
        {
            filePath = "C:\\\\temp\\\\vars.xlsx",
            searchRoot = "invalid-root",
            filePathEditorSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Edit" },
                },
            },
            confirmButtonSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Button" },
                },
            },
            dialogSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Window" },
                },
            },
        };

        JsonElement element = JsonSerializer.SerializeToElement(args);

        bool ok = AutothinkImportVariablesFlow.TryParseArgs(element, out _, out RpcError? error);

        Assert.False(ok);
        Assert.NotNull(error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, error!.Kind);
        Assert.Contains("SearchRoot", error.Message, StringComparison.OrdinalIgnoreCase);
    }
}
