using System.Text.Json;
using Autothink.UiaAgent.Flows.Autothink;
using Autothink.UiaAgent.Rpc.Contracts;
using Xunit;

namespace Autothink.UiaAgent.Tests;

public sealed class AutothinkImportProgramTextPasteArgsTests
{
    [Fact]
    public void TryParseArgs_EmptyProgramText_ReturnsInvalidArgument()
    {
        var args = new
        {
            programText = "",
            editorSelector = new ElementSelector
            {
                Path =
                {
                    new SelectorStep { ControlType = "Edit" },
                },
            },
        };

        JsonElement element = JsonSerializer.SerializeToElement(args);

        bool ok = AutothinkImportProgramTextPasteFlow.TryParseArgs(element, out _, out RpcError? error);

        Assert.False(ok);
        Assert.NotNull(error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, error!.Kind);
        Assert.Contains("ProgramText", error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void TryParseArgs_MissingEditorSelector_ReturnsInvalidArgument()
    {
        var args = new
        {
            programText = "demo",
        };

        JsonElement element = JsonSerializer.SerializeToElement(args);

        bool ok = AutothinkImportProgramTextPasteFlow.TryParseArgs(element, out _, out RpcError? error);

        Assert.False(ok);
        Assert.NotNull(error);
        Assert.Equal(RpcErrorKinds.InvalidArgument, error!.Kind);
        Assert.Contains("EditorSelector", error.Message, StringComparison.OrdinalIgnoreCase);
    }
}
