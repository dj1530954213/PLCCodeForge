// 说明:
// - Waiter 提供简单的轮询等待机制，供查找/等待条件复用。
// - 在 UIA 自动化场景中，用可控轮询代替阻塞式等待，便于诊断超时。
namespace Autothink.UiaAgent.Uia;

/// <summary>
/// 轮询等待工具：在指定超时内重复执行 predicate。
/// </summary>
internal static class Waiter
{
    public static bool PollUntil(Func<bool> predicate, TimeSpan timeout, TimeSpan interval)
    {
        ArgumentNullException.ThrowIfNull(predicate);

        if (timeout < TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(timeout));
        }

        if (interval <= TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(interval));
        }

        DateTimeOffset deadline = DateTimeOffset.UtcNow.Add(timeout);
        while (DateTimeOffset.UtcNow <= deadline)
        {
            if (predicate())
            {
                return true;
            }

            Thread.Sleep(interval);
        }

        return predicate();
    }
}
