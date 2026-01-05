// 说明:
// - ElementFinder 是 selector 到 UIA 元素的核心匹配引擎。
// - 按“路径步骤”逐层查找，并负责 Name/AutomationId/ControlType 等过滤逻辑与错误分类。
using Autothink.UiaAgent.Rpc.Contracts;
using FlaUI.Core.AutomationElements;
using FlaUI.Core.Definitions;
using FlaUI.UIA3;

namespace Autothink.UiaAgent.Uia;

/// <summary>
/// UIA 元素查找器：将 ElementSelector 转换为具体的 AutomationElement。
/// </summary>
internal static class ElementFinder
{
    /// <summary>
    /// 在给定 root 下按 selector 查找元素（单次尝试，不包含重试/等待）。
    /// </summary>
    public static bool TryFind(
        AutomationElement root,
        UIA3Automation automation,
        ElementSelector selector,
        out AutomationElement? element,
        out string? failureKind,
        out Dictionary<string, string>? details)
    {
        ArgumentNullException.ThrowIfNull(root);
        ArgumentNullException.ThrowIfNull(automation);
        ArgumentNullException.ThrowIfNull(selector);

        element = null;
        failureKind = null;
        details = null;

        if (selector.Path is null || selector.Path.Count == 0)
        {
            failureKind = FinderFailureKinds.InvalidSelector;
            details = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["reason"] = "selector.Path must contain at least 1 step",
            };
            return false;
        }

        AutomationElement current = root;
        for (int i = 0; i < selector.Path.Count; i++)
        {
            SelectorStep step = selector.Path[i];

            // Note: to keep the core robust and avoid depending on ConditionFactory composition semantics,
            // we do a broad query (Children/Descendants) then filter in-process.

            ControlType? controlTypeFilter = null;
            if (!string.IsNullOrWhiteSpace(step.ControlType))
            {
                if (!ControlTypeMap.TryGet(step.ControlType, out ControlType mapped))
                {
                    failureKind = FinderFailureKinds.InvalidControlType;
                    details = new Dictionary<string, string>(StringComparer.Ordinal)
                    {
                        ["stepIndex"] = i.ToString(),
                        ["controlType"] = step.ControlType,
                    };
                    return false;
                }

                controlTypeFilter = mapped;
            }

            bool hasAnyFilter =
                !string.IsNullOrWhiteSpace(step.AutomationId) ||
                !string.IsNullOrWhiteSpace(step.AutomationIdContains) ||
                !string.IsNullOrWhiteSpace(step.Name) ||
                !string.IsNullOrWhiteSpace(step.NameContains) ||
                !string.IsNullOrWhiteSpace(step.ClassName) ||
                !string.IsNullOrWhiteSpace(step.ClassNameContains) ||
                controlTypeFilter is not null;

            if (!hasAnyFilter)
            {
                failureKind = FinderFailureKinds.InvalidSelector;
                details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["stepIndex"] = i.ToString(),
                    ["reason"] = "Each selector step must specify at least one filter (AutomationId/Name/ClassName/ControlType).",
                };
                return false;
            }

            AutomationElement[] candidates = step.Search switch
            {
                SelectorSearchKinds.Children => current.FindAllChildren(),
                SelectorSearchKinds.Descendants => current.FindAllDescendants(),
                _ => current.FindAllDescendants(),
            };

            AutomationElement[] matches = candidates
                .Where(e => MatchesStep(e, step, controlTypeFilter))
                .ToArray();

            if (matches.Length == 0)
            {
                failureKind = FinderFailureKinds.NotFound;
                details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["stepIndex"] = i.ToString(),
                };
                return false;
            }

            AutomationElement selected;
            if (step.Index is int index)
            {
                if (index < 0 || index >= matches.Length)
                {
                    failureKind = FinderFailureKinds.IndexOutOfRange;
                    details = new Dictionary<string, string>(StringComparer.Ordinal)
                    {
                        ["stepIndex"] = i.ToString(),
                        ["index"] = index.ToString(),
                        ["matches"] = matches.Length.ToString(),
                    };
                    return false;
                }

                selected = matches[index];
            }
            else
            {
                if (matches.Length != 1)
                {
                    failureKind = FinderFailureKinds.Ambiguous;
                    details = new Dictionary<string, string>(StringComparer.Ordinal)
                    {
                        ["stepIndex"] = i.ToString(),
                        ["matches"] = matches.Length.ToString(),
                        ["hint"] = "Specify SelectorStep.Index to select one element deterministically.",
                    };
                    return false;
                }

                selected = matches[0];
            }

            current = selected;
        }

        element = current;
        return true;
    }

    private static bool MatchesStep(AutomationElement element, SelectorStep step, ControlType? controlTypeFilter)
    {
        if (!MatchesText(
                element.Properties.AutomationId.ValueOrDefault,
                step.AutomationId,
                step.AutomationIdContains,
                step.IgnoreCase))
        {
            return false;
        }

        if (!MatchesText(
                element.Properties.Name.ValueOrDefault,
                step.Name,
                step.NameContains,
                step.IgnoreCase,
                normalizeWhitespace: step.NormalizeWhitespace))
        {
            return false;
        }

        if (!MatchesText(
                element.Properties.ClassName.ValueOrDefault,
                step.ClassName,
                step.ClassNameContains,
                step.IgnoreCase))
        {
            return false;
        }

        if (controlTypeFilter is not null && element.Properties.ControlType.ValueOrDefault != controlTypeFilter)
        {
            return false;
        }

        return true;
    }

    internal static bool MatchesText(
        string? actual,
        string? expectedExact,
        string? expectedContains,
        bool ignoreCase,
        bool normalizeWhitespace = false)
    {
        StringComparison comparison = ignoreCase ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal;
        string a = actual ?? string.Empty;
        string? exact = expectedExact;
        string? contains = expectedContains;

        if (normalizeWhitespace)
        {
            a = NormalizeWhitespace(a);
            exact = NormalizeWhitespace(exact);
            contains = NormalizeWhitespace(contains);
        }

        if (!string.IsNullOrWhiteSpace(exact))
        {
            return string.Equals(a, exact, comparison);
        }

        if (!string.IsNullOrWhiteSpace(contains))
        {
            return a.Contains(contains, comparison);
        }

        return true;
    }

    internal static string? DescribeMatchRules(ElementSelector selector)
    {
        if (selector.Path is null || selector.Path.Count == 0)
        {
            return null;
        }

        var rules = new List<string>();

        for (int i = 0; i < selector.Path.Count; i++)
        {
            SelectorStep step = selector.Path[i];

            if (!string.IsNullOrWhiteSpace(step.AutomationId) && !string.IsNullOrWhiteSpace(step.AutomationIdContains))
            {
                rules.Add($"step{i}.AutomationId=exact");
            }

            if (!string.IsNullOrWhiteSpace(step.Name) && !string.IsNullOrWhiteSpace(step.NameContains))
            {
                rules.Add($"step{i}.Name=exact");
            }

            if (!string.IsNullOrWhiteSpace(step.ClassName) && !string.IsNullOrWhiteSpace(step.ClassNameContains))
            {
                rules.Add($"step{i}.ClassName=exact");
            }
        }

        if (rules.Count == 0)
        {
            return null;
        }

        return string.Join(";", rules);
    }

    private static string NormalizeWhitespace(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return string.Empty;
        }

        var sb = new System.Text.StringBuilder(value.Length);
        bool inWhitespace = false;

        foreach (char c in value)
        {
            if (char.IsWhiteSpace(c))
            {
                if (!inWhitespace)
                {
                    sb.Append(' ');
                    inWhitespace = true;
                }

                continue;
            }

            sb.Append(c);
            inWhitespace = false;
        }

        return sb.ToString().Trim();
    }
}

internal static class FinderFailureKinds
{
    public const string InvalidSelector = "InvalidSelector";
    public const string InvalidControlType = "InvalidControlType";
    public const string NotFound = "NotFound";
    public const string Ambiguous = "Ambiguous";
    public const string IndexOutOfRange = "IndexOutOfRange";
}
