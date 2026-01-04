using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Uia;
using Xunit;

namespace Autothink.UiaAgent.Tests;

public sealed class ElementFinderTextMatchTests
{
    [Fact]
    public void MatchesText_Exact_DefaultIsCaseSensitive()
    {
        Assert.True(ElementFinder.MatchesText("OK", expectedExact: "OK", expectedContains: null, ignoreCase: false));
        Assert.False(ElementFinder.MatchesText("OK", expectedExact: "ok", expectedContains: null, ignoreCase: false));
    }

    [Fact]
    public void MatchesText_Exact_IgnoreCaseTrue_AllowsDifferentCase()
    {
        Assert.True(ElementFinder.MatchesText("OK", expectedExact: "ok", expectedContains: null, ignoreCase: true));
    }

    [Fact]
    public void MatchesText_Contains_DefaultIsCaseSensitive()
    {
        Assert.True(ElementFinder.MatchesText("Hello World", expectedExact: null, expectedContains: "World", ignoreCase: false));
        Assert.False(ElementFinder.MatchesText("Hello World", expectedExact: null, expectedContains: "world", ignoreCase: false));
    }

    [Fact]
    public void MatchesText_Contains_IgnoreCaseTrue_AllowsDifferentCase()
    {
        Assert.True(ElementFinder.MatchesText("Hello World", expectedExact: null, expectedContains: "world", ignoreCase: true));
    }

    [Fact]
    public void MatchesText_Exact_PrefersExactOverContains()
    {
        Assert.True(ElementFinder.MatchesText("ABC", expectedExact: "ABC", expectedContains: "ZZ", ignoreCase: false));
    }

    [Fact]
    public void MatchesText_NormalizeWhitespace_CollapsesAndTrims()
    {
        Assert.False(ElementFinder.MatchesText("A  B\r\nC", expectedExact: "A B C", expectedContains: null, ignoreCase: false));
        Assert.True(ElementFinder.MatchesText("A  B\r\nC", expectedExact: "A B C", expectedContains: null, ignoreCase: false, normalizeWhitespace: true));
    }

    [Fact]
    public void DescribeMatchRules_ReportsExactPreferenceForAutomationIdAndClassName()
    {
        var selector = new ElementSelector
        {
            Path =
            {
                new SelectorStep
                {
                    AutomationId = "AUTO_ID",
                    AutomationIdContains = "AUTO",
                    ClassName = "Window",
                    ClassNameContains = "Win",
                },
            },
        };

        string? rule = ElementFinder.DescribeMatchRules(selector);

        Assert.Equal("step0.AutomationId=exact;step0.ClassName=exact", rule);
    }
}
