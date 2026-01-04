namespace Autothink.UiaAgent.Rpc.Contracts;

/// <summary>
/// 不带返回值的 RPC 结果封装（包含 StepLog 与错误）。
/// </summary>
public sealed class RpcResult
{
    /// <summary>
    /// 是否成功。
    /// </summary>
    public bool Ok { get; set; }

    /// <summary>
    /// 失败时的错误（可选）。
    /// </summary>
    public RpcError? Error { get; set; }

    /// <summary>
    /// 步骤日志。
    /// </summary>
    public StepLog StepLog { get; set; } = new();
}

/// <summary>
/// 带返回值的 RPC 结果封装（包含 StepLog 与错误）。
/// </summary>
/// <typeparam name="T">返回值类型。</typeparam>
public sealed class RpcResult<T>
{
    /// <summary>
    /// 是否成功。
    /// </summary>
    public bool Ok { get; set; }

    /// <summary>
    /// 成功时的返回值；失败时通常为 null。
    /// </summary>
    public T? Value { get; set; }

    /// <summary>
    /// 失败时的错误（可选）。
    /// </summary>
    public RpcError? Error { get; set; }

    /// <summary>
    /// 步骤日志。
    /// </summary>
    public StepLog StepLog { get; set; } = new();
}
