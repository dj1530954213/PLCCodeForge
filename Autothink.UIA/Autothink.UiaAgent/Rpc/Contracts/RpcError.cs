// 说明:
// - RPC 错误契约：用于在不抛异常的情况下返回稳定的错误分类与可诊断信息。
// - Runner 与上层会据此做 summary 分类、重试与现场定位。
namespace Autothink.UiaAgent.Rpc.Contracts;

/// <summary>
/// 表示一次 RPC 调用失败时返回的结构化错误。
/// </summary>
public sealed class RpcError
{
    /// <summary>
    /// 错误分类（稳定口径）。
    /// </summary>
    /// <remarks>
    /// 约定值：ConfigError / FindError / TimeoutError / ActionError / UnexpectedUIState / StaleElement / InvalidArgument / NotImplemented。
    /// </remarks>
    public string Kind { get; set; } = string.Empty;

    /// <summary>
    /// 面向人类的错误摘要。
    /// </summary>
    public string Message { get; set; } = string.Empty;

    /// <summary>
    /// 结构化的错误上下文（可选）。
    /// </summary>
    public Dictionary<string, string>? Details { get; set; }
}

/// <summary>
/// <see cref="RpcError.Kind"/> 的约定取值。
/// </summary>
public static class RpcErrorKinds
{
    /// <summary>配置错误（路径/进程/版本/环境不匹配等）。</summary>
    public const string ConfigError = "ConfigError";

    /// <summary>控件找不到（元素定位失败）。</summary>
    public const string FindError = "FindError";

    /// <summary>等待超时（可观测条件未在期限内达成）。</summary>
    public const string TimeoutError = "TimeoutError";

    /// <summary>动作执行失败（点击/输入/粘贴等）。</summary>
    public const string ActionError = "ActionError";

    /// <summary>出现未知弹窗/模式不一致等 UI 状态异常。</summary>
    public const string UnexpectedUIState = "UnexpectedUIState";

    /// <summary>元素已失效（UI 刷新/重建导致引用不再有效）。</summary>
    public const string StaleElement = "StaleElement";

    /// <summary>参数不合法（契约层面可判断的请求错误）。</summary>
    public const string InvalidArgument = "InvalidArgument";

    /// <summary>能力未实现（FlowName 已注册但仍是占位/未做）。</summary>
    public const string NotImplemented = "NotImplemented";
}
