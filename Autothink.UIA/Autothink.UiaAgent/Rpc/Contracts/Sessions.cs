// 说明:
// - 会话相关 RPC 契约：用于附加目标进程并获取 SessionId。
// - SessionId 是后续 UIA 操作的稳定上下文标识。
namespace Autothink.UiaAgent.Rpc.Contracts;

/// <summary>
/// 打开/附加到目标组态软件进程的请求。
/// </summary>
public sealed class OpenSessionRequest
{
    /// <summary>
    /// 目标进程 ID（可选）。
    /// </summary>
    public int? ProcessId { get; set; }

    /// <summary>
    /// 目标进程名（不含 .exe，可选）。
    /// </summary>
    public string? ProcessName { get; set; }

    /// <summary>
    /// 主窗口标题应包含的片段（可选）。
    /// </summary>
    public string? MainWindowTitleContains { get; set; }

    /// <summary>
    /// 等待超时（毫秒）。
    /// </summary>
    public int TimeoutMs { get; set; } = 10_000;

    /// <summary>
    /// 是否尝试将目标窗口置前（可选）。
    /// </summary>
    public bool BringToForeground { get; set; } = true;
}

/// <summary>
/// 打开会话的返回值。
/// </summary>
public sealed class OpenSessionResponse
{
    /// <summary>
    /// 会话标识（后续调用需携带）。
    /// </summary>
    public string SessionId { get; set; } = string.Empty;

    /// <summary>
    /// 进程 ID。
    /// </summary>
    public int ProcessId { get; set; }

    /// <summary>
    /// 主窗口标题（可选）。
    /// </summary>
    public string? MainWindowTitle { get; set; }
}

/// <summary>
/// 关闭会话请求。
/// </summary>
public sealed class CloseSessionRequest
{
    /// <summary>
    /// 会话标识。
    /// </summary>
    public string SessionId { get; set; } = string.Empty;
}
