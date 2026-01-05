// 说明:
// - 覆盖 ControlTypeMap 的字符串解析正确性。
using Autothink.UiaAgent.Uia;
using Xunit;

namespace Autothink.UiaAgent.Tests;

/// <summary>
/// ControlTypeMap 的解析测试。
/// </summary>
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
