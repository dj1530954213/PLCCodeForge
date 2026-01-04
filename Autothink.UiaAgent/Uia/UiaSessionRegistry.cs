using FlaUI.Core;
using FlaUI.UIA3;

namespace Autothink.UiaAgent.Uia;

/// <summary>
/// 维护当前进程内的 UIA 会话。
/// </summary>
/// <remarks>
/// 约定：所有 UIA 操作都在同一个 STA 线程内串行执行，因此此处无需额外锁；
/// 但仍保持实现简单，避免在多线程场景下产生隐患。
/// </remarks>
internal static class UiaSessionRegistry
{
    private static readonly Dictionary<string, UiaSession> sessions = new(StringComparer.Ordinal);

    public static UiaSession Create(FlaUI.Core.Application application)
    {
        ArgumentNullException.ThrowIfNull(application);

        string sessionId = Guid.NewGuid().ToString("N");
        var automation = new UIA3Automation();
        var session = new UiaSession(sessionId, application, automation);
        sessions.Add(sessionId, session);
        return session;
    }

    public static bool TryGet(string sessionId, out UiaSession? session)
    {
        if (string.IsNullOrWhiteSpace(sessionId))
        {
            session = null;
            return false;
        }

        return sessions.TryGetValue(sessionId, out session);
    }

    public static bool TryRemove(string sessionId, out UiaSession? session)
    {
        if (!sessions.Remove(sessionId, out session))
        {
            session = null;
            return false;
        }

        return true;
    }
}
