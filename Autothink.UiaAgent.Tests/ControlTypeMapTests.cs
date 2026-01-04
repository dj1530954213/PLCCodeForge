using Autothink.UiaAgent.Uia;
using Xunit;

namespace Autothink.UiaAgent.Tests;

public sealed class ControlTypeMapTests
{
    [Fact]
    public void TryGet_Unknown_ReturnsFalse()
    {
        bool ok = ControlTypeMap.TryGet("__not_a_control_type__", out _);
        Assert.False(ok);
    }

    [Fact]
    public void TryGet_Button_ReturnsTrue()
    {
        bool ok = ControlTypeMap.TryGet("Button", out _);
        Assert.True(ok);
    }
}
