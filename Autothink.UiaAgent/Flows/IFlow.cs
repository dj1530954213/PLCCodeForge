// 说明:
// - Flow 层接口：以统一的 Run 入口封装组合动作，产出 StepLog 与结构化错误。
// - 具体实现放在 Flows/Autothink 下（普通型 AUTOTHINK）。
using System.Text.Json;
using Autothink.UiaAgent.Rpc.Contracts;

namespace Autothink.UiaAgent.Flows;

/// <summary>
/// 流程层接口：按 FlowName 执行一段可回放的 UIA 流程。
/// </summary>
internal interface IFlow
{
    /// <summary>
    /// FlowName（大小写敏感）。
    /// </summary>
    string Name { get; }

    /// <summary>
    /// 是否为真实实现（false 表示占位/未实现）。
    /// </summary>
    bool IsImplemented { get; }

    /// <summary>
    /// 执行流程。
    /// </summary>
    RpcResult<RunFlowResponse> Run(FlowContext context, JsonElement? args);
}
