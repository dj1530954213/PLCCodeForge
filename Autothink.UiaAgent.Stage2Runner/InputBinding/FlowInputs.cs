using Autothink.UiaAgent.Rpc.Contracts;

namespace Autothink.UiaAgent.Stage2Runner;

internal sealed class InputsSourceConfig
{
    public string Mode { get; set; } = "inline";

    public string? CommIrPath { get; set; }
}

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

internal sealed class InputsResolutionError
{
    public string Kind { get; set; } = RpcErrorKinds.InvalidArgument;

    public string Message { get; set; } = string.Empty;
}
