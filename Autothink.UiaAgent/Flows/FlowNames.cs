namespace Autothink.UiaAgent.Flows;

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
