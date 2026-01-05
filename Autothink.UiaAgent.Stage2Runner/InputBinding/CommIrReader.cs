// 说明:
// - 读取 CommIR v1 输入清单并抽取 variables/program 等路径。
// - 该解析逻辑只用于 Runner 的“输入绑定层”，不影响 RPC 契约。
using System.Text.Json;

namespace Autothink.UiaAgent.Stage2Runner;

/// <summary>
/// 从 CommIR 中解析出的关键输入路径集合。
/// </summary>
internal sealed class CommIrInputs
{
    public string? VariablesFilePath { get; set; }

    public string? VariablesSource { get; set; }

    public string? ProgramTextPath { get; set; }

    public string? ProgramSource { get; set; }

    public string? OutputDir { get; set; }

    public string? ProjectName { get; set; }
}

/// <summary>
/// 解析结果：包含已解析输入与警告信息。
/// </summary>
internal sealed class CommIrReadResult
{
    public CommIrInputs Inputs { get; set; } = new();

    public List<string> Warnings { get; set; } = new();
}

/// <summary>
/// CommIR 解析器：为 Runner 解析 CommIR v1 结构并做路径映射。
/// </summary>
internal static class CommIrReader
{
    public static CommIrReadResult Read(string commIrPath)
    {
        string json = File.ReadAllText(commIrPath);
        using JsonDocument doc = JsonDocument.Parse(json);
        JsonElement root = doc.RootElement;

        var result = new CommIrReadResult();
        string? outputDir = TryGetString(root, "outputs", "outputDir");
        string? projectName = TryGetString(root, "projectName") ?? TryGetString(root, "sources", "projectName");

        result.Inputs.OutputDir = outputDir;
        result.Inputs.ProjectName = projectName;

        string? variables = TryGetString(root, "inputs", "variablesFilePath");
        if (!string.IsNullOrWhiteSpace(variables))
        {
            result.Inputs.VariablesFilePath = variables;
            result.Inputs.VariablesSource = "inputs.variablesFilePath";
        }
        else
        {
            variables = TryGetString(root, "outputs", "variablesFilePath");
            if (!string.IsNullOrWhiteSpace(variables))
            {
                result.Inputs.VariablesFilePath = variables;
                result.Inputs.VariablesSource = "outputs.variablesFilePath";
            }
            else
            {
                variables = TryGetString(root, "sources", "unionXlsxPath");
                if (!string.IsNullOrWhiteSpace(variables))
                {
                    result.Inputs.VariablesFilePath = variables;
                    result.Inputs.VariablesSource = "sources.unionXlsxPath";
                }
            }
        }

        string? program = TryGetString(root, "inputs", "programTextPath");
        if (!string.IsNullOrWhiteSpace(program))
        {
            result.Inputs.ProgramTextPath = program;
            result.Inputs.ProgramSource = "inputs.programTextPath";
        }
        else
        {
            program = TryGetString(root, "outputs", "programTextPath");
            if (!string.IsNullOrWhiteSpace(program))
            {
                result.Inputs.ProgramTextPath = program;
                result.Inputs.ProgramSource = "outputs.programTextPath";
            }
            else
            {
                program = TryGetString(root, "sources", "programTextPath");
                if (!string.IsNullOrWhiteSpace(program))
                {
                    result.Inputs.ProgramTextPath = program;
                    result.Inputs.ProgramSource = "sources.programTextPath";
                }
            }
        }

        return result;
    }

    private static string? TryGetString(JsonElement element, params string[] path)
    {
        JsonElement current = element;
        foreach (string segment in path)
        {
            if (current.ValueKind != JsonValueKind.Object || !current.TryGetProperty(segment, out JsonElement next))
            {
                return null;
            }

            current = next;
        }

        return current.ValueKind == JsonValueKind.String ? current.GetString() : null;
    }
}
