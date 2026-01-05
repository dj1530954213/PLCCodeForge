// 说明:
// - ElementSelector 是 UIA 查找的路径表达式，也是 selector 资产的核心结构。
// - 对外作为 JSON 传输，与 Runner/前端共享。
namespace Autothink.UiaAgent.Rpc.Contracts;

/// <summary>
/// 元素选择器：用于在一个 scope（例如主窗口）下定位目标控件。
/// </summary>
/// <remarks>
/// 选择器采用“路径”表达：每一步在上一步命中的元素之下继续查找。
/// </remarks>
public sealed class ElementSelector
{
    /// <summary>
    /// 路径步骤（至少 1 个）。
    /// </summary>
    public List<SelectorStep> Path { get; set; } = new();
}

/// <summary>
/// 选择器路径中的一步。
/// </summary>
public sealed class SelectorStep
{
    /// <summary>
    /// 查找范围：Descendants / Children。
    /// </summary>
    /// <remarks>
    /// 默认 Descendants（更鲁棒，但可能更慢且更易命中多个）。
    /// </remarks>
    public string Search { get; set; } = SelectorSearchKinds.Descendants;

    /// <summary>
    /// AutomationId 匹配（可选）。
    /// </summary>
    public string? AutomationId { get; set; }

    /// <summary>
    /// AutomationId 包含匹配（可选）。
    /// </summary>
    public string? AutomationIdContains { get; set; }

    /// <summary>
    /// Name 匹配（可选）。
    /// </summary>
    public string? Name { get; set; }

    /// <summary>
    /// Name 包含匹配（可选）。
    /// </summary>
    public string? NameContains { get; set; }

    /// <summary>
    /// 是否对 Name/NameContains 进行空白归一（可选）。
    /// </summary>
    /// <remarks>
    /// 归一规则：连续空白 -> 单空格，Trim。
    /// </remarks>
    public bool NormalizeWhitespace { get; set; }

    /// <summary>
    /// 字符串匹配是否忽略大小写（可选；默认 false）。
    /// </summary>
    /// <remarks>
    /// 当前仅作用于 Name/NameContains；其它字段仍为精确匹配（Ordinal）。
    /// </remarks>
    public bool IgnoreCase { get; set; }

    /// <summary>
    /// ClassName 匹配（可选）。
    /// </summary>
    public string? ClassName { get; set; }

    /// <summary>
    /// ClassName 包含匹配（可选）。
    /// </summary>
    public string? ClassNameContains { get; set; }

    /// <summary>
    /// ControlType 匹配（可选）。
    /// </summary>
    /// <remarks>
    /// 约定使用 FlaUI 常见类型名，例如：Button / Edit / Tree / DataGrid / Window。
    /// </remarks>
    public string? ControlType { get; set; }

    /// <summary>
    /// 当多个元素命中时选择第几个（从 0 开始；可选）。
    /// </summary>
    public int? Index { get; set; }
}

/// <summary>
/// <see cref="SelectorStep.Search"/> 的约定取值。
/// </summary>
public static class SelectorSearchKinds
{
    /// <summary>在所有后代中查找。</summary>
    public const string Descendants = "Descendants";

    /// <summary>仅在直接子元素中查找。</summary>
    public const string Children = "Children";
}
