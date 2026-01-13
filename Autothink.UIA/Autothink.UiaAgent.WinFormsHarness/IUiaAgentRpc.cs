// 说明:
// - WinFormsHarness 侧 RPC 代理接口，覆盖完整的 UIA 原子操作与 RunFlow。
// - 仅用于开发/现场联调，不对外发布为 SDK。
using Autothink.UiaAgent.Rpc.Contracts;
using StreamJsonRpc;

namespace Autothink.UiaAgent.WinFormsHarness;

/// <summary>
/// WinForms 测试台使用的 RPC 代理接口。
/// </summary>
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

    [JsonRpcMethod("ClickAt")]
    Task<RpcResult> ClickAtAsync(ClickAtRequest request);

    [JsonRpcMethod("RightClickAt")]
    Task<RpcResult> RightClickAtAsync(RightClickAtRequest request);

    [JsonRpcMethod("ClickRel")]
    Task<RpcResult> ClickRelAsync(ClickRelRequest request);

    [JsonRpcMethod("SetText")]
    Task<RpcResult> SetTextAsync(SetTextRequest request);

    [JsonRpcMethod("SendKeys")]
    Task<RpcResult> SendKeysAsync(SendKeysRequest request);

    [JsonRpcMethod("WaitUntil")]
    Task<RpcResult> WaitUntilAsync(WaitUntilRequest request);

    [JsonRpcMethod("RunFlow")]
    Task<RpcResult<RunFlowResponse>> RunFlowAsync(RunFlowRequest request);
}
