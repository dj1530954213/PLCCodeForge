using System.Text.Json;

namespace Autothink.UiaAgent.Rpc.Contracts;

/// <summary>
/// 统一流程入口请求。
/// </summary>
public sealed class RunFlowRequest
{
    /// <summary>
    /// 会话标识（复用 OpenSession 返回的 session）。
    /// </summary>
    public string SessionId { get; set; } = string.Empty;

    /// <summary>
    /// 流程名（大小写敏感；例如："autothink.attach"）。
    /// </summary>
    public string FlowName { get; set; } = string.Empty;

    /// <summary>
    /// 通用参数载体（可选）。
    /// </summary>
    public JsonElement Args { get; set; }

    /// <summary>
    /// 可选：参数原始 JSON 字符串（用于跨端传递 JsonElement 失败时的回退）。
    /// </summary>
    public string? ArgsJson { get; set; }

    /// <summary>
    /// 超时（毫秒）。
    /// </summary>
    public int TimeoutMs { get; set; } = 30_000;
}

/// <summary>
/// 统一流程入口返回值。
/// </summary>
public sealed class RunFlowResponse
{
    /// <summary>
    /// 流程输出数据（MVP 可为 null）。
    /// </summary>
    public JsonElement? Data { get; set; }
}
