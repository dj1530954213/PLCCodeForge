using Autothink.UiaAgent.Rpc.Contracts;
using StreamJsonRpc;

namespace Autothink.UiaAgent.Stage2Runner;

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

