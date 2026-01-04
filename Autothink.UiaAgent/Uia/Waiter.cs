namespace Autothink.UiaAgent.Uia;

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
