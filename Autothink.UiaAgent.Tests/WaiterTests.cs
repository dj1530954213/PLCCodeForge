using Autothink.UiaAgent.Uia;
using Xunit;

// 说明:
// - 覆盖 Waiter 的轮询等待语义与超时行为。
namespace Autothink.UiaAgent.Tests;

/// <summary>
/// Waiter 轮询等待测试。
/// </summary>
public sealed class WaiterTests
{
    [Fact]
    public void PollUntil_EventuallyTrue_ReturnsTrue()
    {
        int calls = 0;
        bool ok = Waiter.PollUntil(
            predicate: () =>
            {
                calls++;
                return calls >= 3;
            },
            timeout: TimeSpan.FromMilliseconds(200),
            interval: TimeSpan.FromMilliseconds(10));

        Assert.True(ok);
        Assert.True(calls >= 3);
    }

    [Fact]
    public void PollUntil_AlwaysFalse_ReturnsFalse()
    {
        bool ok = Waiter.PollUntil(
            predicate: () => false,
            timeout: TimeSpan.FromMilliseconds(50),
            interval: TimeSpan.FromMilliseconds(10));

        Assert.False(ok);
    }
}
