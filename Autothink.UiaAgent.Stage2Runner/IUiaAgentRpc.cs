// 说明:
// - Runner 侧的 RPC 代理接口，只声明 Stage2Runner 需要调用的最小方法集。
// - 通过 StreamJsonRpc 的 JsonRpcMethod 特性用于映射到 Agent 的 JSON-RPC 方法名。
using Autothink.UiaAgent.Rpc.Contracts;
using StreamJsonRpc;

namespace Autothink.UiaAgent.Stage2Runner;

/// <summary>
/// Stage2Runner 与 UiaAgent 之间的轻量 RPC 代理（最小可用集合）。
/// </summary>
internal interface IUiaAgentRpc
{
    [JsonRpcMethod("Ping")]
    Task<string> PingAsync();

    [JsonRpcMethod("OpenSession")]
    Task<RpcResult<OpenSessionResponse>> OpenSessionAsync(OpenSessionRequest request);

    [JsonRpcMethod("CloseSession")]
    Task<RpcResult> CloseSessionAsync(CloseSessionRequest request);

    [JsonRpcMethod("RunFlow")]
    Task<RpcResult<RunFlowResponse>> RunFlowAsync(RunFlowRequest request);
}
