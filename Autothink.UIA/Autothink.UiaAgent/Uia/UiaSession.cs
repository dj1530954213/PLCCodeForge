// 说明:
// - UiaSession 表示一次与目标进程绑定的会话，承载 UIA Automation 资源与窗口访问能力。
// - 由 UiaSessionRegistry 管理生命周期，供 RPC/Flow 层复用。
using FlaUI.Core;
using FlaUI.Core.AutomationElements;
using FlaUI.UIA3;

namespace Autothink.UiaAgent.Uia;

/// <summary>
/// 一个 UIA 会话：绑定到一个目标进程，并在同一个 <see cref="UIA3Automation"/> 上执行查找/动作。
/// </summary>
internal sealed class UiaSession : IDisposable
{
    internal UiaSession(string sessionId, FlaUI.Core.Application application, UIA3Automation automation)
    {
        this.SessionId = sessionId;
        this.Application = application;
        this.Automation = automation;
        this.CreatedAtUtc = DateTimeOffset.UtcNow;
    }

    public string SessionId { get; }

    public FlaUI.Core.Application Application { get; }

    public UIA3Automation Automation { get; }

    public DateTimeOffset CreatedAtUtc { get; }

    public int ProcessId => this.Application.ProcessId;

    /// <summary>
    /// 获取当前主窗口（每次调用都会重新查询，避免 UI 刷新后引用失效）。
    /// </summary>
    public Window GetMainWindow(TimeSpan timeout)
    {
        Window? window = this.Application.GetMainWindow(this.Automation, timeout);
        if (window is null)
        {
            throw new InvalidOperationException("Main window not found.");
        }

        return window;
    }

    public void Dispose()
    {
        this.Automation.Dispose();
    }
}
