// 说明:
// - Runner 的“输入绑定层”数据结构：把外部配置/CommIR 解析成可供各 flow 使用的统一输入。
// - 这些类型只在 Runner 内部使用，不属于 RPC 契约的一部分。
using Autothink.UiaAgent.Rpc.Contracts;

namespace Autothink.UiaAgent.Stage2Runner;

/// <summary>
/// 输入来源配置：决定从“手写 inline 配置”还是“CommIR 文件”解析实际路径。
/// </summary>
internal sealed class InputsSourceConfig
{
    public string Mode { get; set; } = "inline";

    public string? CommIrPath { get; set; }
}

/// <summary>
/// 输入解析结果：在 Runner 中统一保存 variables/program 等关键路径与诊断信息。
/// </summary>
internal sealed class FlowInputsResolution
{
    public bool Ok { get; set; }

    public string Mode { get; set; } = "inline";

    public string? CommIrPath { get; set; }

    public string? VariablesFilePath { get; set; }

    public string? ProgramTextPath { get; set; }

    public string? OutputDir { get; set; }

    public string? ProjectName { get; set; }

    public string? VariablesSource { get; set; }

    public string? ProgramSource { get; set; }

    public List<string>? Warnings { get; set; }

    public InputsResolutionError? Error { get; set; }
}

/// <summary>
/// 输入解析失败时的结构化错误。
/// </summary>
internal sealed class InputsResolutionError
{
    public string Kind { get; set; } = RpcErrorKinds.InvalidArgument;

    public string Message { get; set; } = string.Empty;
}
