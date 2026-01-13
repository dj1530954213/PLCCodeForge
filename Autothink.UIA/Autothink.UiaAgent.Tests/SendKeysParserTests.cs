using Autothink.UiaAgent.Uia;
using FlaUI.Core.WindowsAPI;
using Xunit;

// 说明:
// - 覆盖 SendKeys 解析器对组合键/单键/文本的解析规则。
namespace Autothink.UiaAgent.Tests;

/// <summary>
/// SendKeysParser 的解析规则测试。
/// </summary>
public sealed class SendKeysParserTests
{
    [Fact]
    public void TryParse_Null_ReturnsFalse()
    {
        bool ok = SendKeysParser.TryParse(null, out ParsedSendKeys? parsed, out string? error);
        Assert.False(ok);
        Assert.Null(parsed);
        Assert.False(string.IsNullOrWhiteSpace(error));
    }

    [Fact]
    public void TryParse_Whitespace_ReturnsFalse()
    {
        bool ok = SendKeysParser.TryParse("   ", out ParsedSendKeys? parsed, out string? error);
        Assert.False(ok);
        Assert.Null(parsed);
        Assert.False(string.IsNullOrWhiteSpace(error));
    }

    [Fact]
    public void TryParse_Text_ReturnsTextKind()
    {
        bool ok = SendKeysParser.TryParse("hello", out ParsedSendKeys? parsed, out string? error);
        Assert.True(ok);
        Assert.NotNull(parsed);
        Assert.Equal(ParsedSendKeysKinds.Text, parsed.Kind);
        Assert.Equal("hello", parsed.Text);
        Assert.Null(error);
    }

    [Theory]
    [InlineData("ENTER", 0x0D)]
    [InlineData("TAB", 0x09)]
    [InlineData("ESC", 0x1B)]
    [InlineData("DELETE", 0x2E)]
    [InlineData("F1", 0x70)]
    [InlineData("F12", 0x7B)]
    public void TryParse_SpecialKey_ReturnsKeyKind(string input, int expectedVk)
    {
        bool ok = SendKeysParser.TryParse(input, out ParsedSendKeys? parsed, out string? error);
        Assert.True(ok);
        Assert.NotNull(parsed);
        Assert.Equal(ParsedSendKeysKinds.Key, parsed.Kind);
        Assert.Equal((VirtualKeyShort)expectedVk, parsed.Key);
        Assert.Null(error);
    }

    [Fact]
    public void TryParse_Chord_ReturnsChordKind()
    {
        bool ok = SendKeysParser.TryParse("CTRL+V", out ParsedSendKeys? parsed, out string? error);
        Assert.True(ok);
        Assert.NotNull(parsed);
        Assert.Equal(ParsedSendKeysKinds.Chord, parsed.Kind);
        Assert.NotNull(parsed.Key);
        Assert.Equal((VirtualKeyShort)'V', parsed.Key);
        Assert.Contains((VirtualKeyShort)0x11, parsed.Modifiers); // VK_CONTROL
        Assert.Null(error);
    }

    [Fact]
    public void TryParse_InvalidChord_ReturnsFalse()
    {
        bool ok = SendKeysParser.TryParse("CTRL+", out ParsedSendKeys? parsed, out string? error);
        Assert.False(ok);
        Assert.Null(parsed);
        Assert.False(string.IsNullOrWhiteSpace(error));
    }

    [Fact]
    public void TryParse_TooManyKeysInChord_ReturnsFalse()
    {
        bool ok = SendKeysParser.TryParse("CTRL+A+B", out ParsedSendKeys? parsed, out string? error);
        Assert.False(ok);
        Assert.Null(parsed);
        Assert.False(string.IsNullOrWhiteSpace(error));
    }
}
