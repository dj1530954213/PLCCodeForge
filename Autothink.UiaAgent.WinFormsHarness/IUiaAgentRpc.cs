using Autothink.UiaAgent.Rpc.Contracts;
using StreamJsonRpc;

namespace Autothink.UiaAgent.WinFormsHarness;

internal interface IUiaAgentRpc
{
    [JsonRpcMethod("Ping")]
    Task<string> PingAsync();

    [JsonRpcMethod("OpenSession")]
    Task<RpcResult<OpenSessionResponse>> OpenSessionAsync(OpenSessionRequest request);

    [JsonRpcMethod("CloseSession")]
    Task<RpcResult> CloseSessionAsync(CloseSessionRequest request);

    [JsonRpcMethod("FindElement")]
    Task<RpcResult<FindElementResponse>> FindElementAsync(FindElementRequest request);

    [JsonRpcMethod("Click")]
    Task<RpcResult> ClickAsync(ClickRequest request);

    [JsonRpcMethod("DoubleClick")]
    Task<RpcResult> DoubleClickAsync(DoubleClickRequest request);

    [JsonRpcMethod("RightClick")]
    Task<RpcResult> RightClickAsync(RightClickRequest request);

    [JsonRpcMethod("SetText")]
    Task<RpcResult> SetTextAsync(SetTextRequest request);

    [JsonRpcMethod("SendKeys")]
    Task<RpcResult> SendKeysAsync(SendKeysRequest request);

    [JsonRpcMethod("WaitUntil")]
    Task<RpcResult> WaitUntilAsync(WaitUntilRequest request);

    [JsonRpcMethod("RunFlow")]
    Task<RpcResult<RunFlowResponse>> RunFlowAsync(RunFlowRequest request);
}

