// 说明:
// - StepLog 是现场诊断证据链：记录每个关键动作/等待的时间线与结果。
// - Runner 与验收环节依赖 StepLog 做证据提交与问题定位。
namespace Autothink.UiaAgent.Rpc.Contracts;

/// <summary>
/// 一次 RPC 调用的步骤日志（可用于现场定位与回放）。
/// </summary>
public sealed class StepLog
{
    /// <summary>
    /// 调用链路的追踪标识（可选）。
    /// </summary>
    public string? TraceId { get; set; }

    /// <summary>
    /// 步骤列表（按执行顺序）。
    /// </summary>
    public List<StepLogEntry> Steps { get; set; } = new();
}

/// <summary>
/// 单个可观测步骤的结构化记录。
/// </summary>
public sealed class StepLogEntry
{
    /// <summary>
    /// 步骤标识（例如：FindElement/Click/WaitUntil 等）。
    /// </summary>
    public string StepId { get; set; } = string.Empty;

    /// <summary>
    /// 动作名称（面向人类）。
    /// </summary>
    public string Action { get; set; } = string.Empty;

    /// <summary>
    /// 关键参数摘要（避免写入敏感完整路径，可在上层脱敏）。
    /// </summary>
    public Dictionary<string, string>? Parameters { get; set; }

    /// <summary>
    /// 目标元素选择器（可选）。
    /// </summary>
    public ElementSelector? Selector { get; set; }

    /// <summary>
    /// 开始时间（UTC）。
    /// </summary>
    public DateTimeOffset StartedAtUtc { get; set; }

    /// <summary>
    /// 结束时间（UTC）。
    /// </summary>
    public DateTimeOffset FinishedAtUtc { get; set; }

    /// <summary>
    /// 耗时（毫秒）。
    /// </summary>
    public long DurationMs { get; set; }

    /// <summary>
    /// 结果：Success / Warning / Fail。
    /// </summary>
    public string Outcome { get; set; } = StepOutcomes.Fail;

    /// <summary>
    /// 失败时的结构化错误（可选）。
    /// </summary>
    public RpcError? Error { get; set; }
}

/// <summary>
/// <see cref="StepLogEntry.Outcome"/> 的约定取值。
/// </summary>
public static class StepOutcomes
{
    /// <summary>动作成功。</summary>
    public const string Success = "Success";

    /// <summary>动作成功但伴随警告（预留）。</summary>
    public const string Warning = "Warning";

    /// <summary>动作失败。</summary>
    public const string Fail = "Fail";
}
