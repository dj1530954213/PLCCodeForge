// 说明:
// - FlowName 常量表：对外接口的唯一真源，避免字符串散落与拼写不一致。
// - 与 RunFlow 契约绑定，大小写敏感。
namespace Autothink.UiaAgent.Flows;

/// <summary>
/// 固定的 FlowName 集合（Stage 2 约束）。
/// </summary>
internal static class FlowNames
{
    public const string AutothinkAttach = "autothink.attach";
    public const string AutothinkImportVariables = "autothink.importVariables";
    public const string AutothinkImportProgramTextPaste = "autothink.importProgram.textPaste";
    public const string AutothinkBuild = "autothink.build";

    public static readonly string[] AllOrdered =
    [
        AutothinkAttach,
        AutothinkImportVariables,
        AutothinkImportProgramTextPaste,
        AutothinkBuild,
    ];

    private static readonly HashSet<string> All = new(StringComparer.Ordinal)
    {
        AutothinkAttach,
        AutothinkImportVariables,
        AutothinkImportProgramTextPaste,
        AutothinkBuild,
    };

    public static bool IsKnown(string? flowName)
    {
        return !string.IsNullOrWhiteSpace(flowName) && All.Contains(flowName);
    }
}
