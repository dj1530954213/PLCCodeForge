// 说明:
// - 解析 SendKeys 文本（如 "CTRL+V"、"ENTER" 或普通文本）为统一结构，供输入动作复用。
// - 该解析逻辑尽量简单可预测，避免引入过度复杂的键盘脚本语法。
using FlaUI.Core.WindowsAPI;

namespace Autothink.UiaAgent.Uia;

/// <summary>
/// SendKeys 解析结果：区分“组合键/单键/纯文本”三类。
/// </summary>
internal sealed record ParsedSendKeys
{
    public required string Kind { get; init; }
    public VirtualKeyShort[] Modifiers { get; init; } = Array.Empty<VirtualKeyShort>();
    public VirtualKeyShort? Key { get; init; }
    public string? Text { get; init; }
}

/// <summary>
/// SendKeys 解析器表示层：将用户输入字符串转换为 UIA 可执行的键盘动作。
/// </summary>
internal static class SendKeysParser
{
    public static bool TryParse(string? input, out ParsedSendKeys? parsed, out string? error)
    {
        parsed = null;
        error = null;

        if (string.IsNullOrWhiteSpace(input))
        {
            error = "keys must be non-empty";
            return false;
        }

        string trimmed = input.Trim();

        // If contains '+', treat as chord (e.g. CTRL+V)
        if (trimmed.Contains('+', StringComparison.Ordinal))
        {
            return TryParseChord(trimmed, out parsed, out error);
        }

        // Try parse as single special key (ENTER/TAB/ESC/...) 
        if (TryParseKeyToken(trimmed, out VirtualKeyShort key))
        {
            parsed = new ParsedSendKeys { Kind = ParsedSendKeysKinds.Key, Key = key };
            return true;
        }

        // Fallback: treat as plain text.
        parsed = new ParsedSendKeys { Kind = ParsedSendKeysKinds.Text, Text = trimmed };
        return true;
    }

    private static bool TryParseChord(string input, out ParsedSendKeys? parsed, out string? error)
    {
        parsed = null;
        error = null;

        string[] parts = input.Split('+', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length < 2)
        {
            error = "invalid chord";
            return false;
        }

        var modifiers = new List<VirtualKeyShort>();
        VirtualKeyShort? key = null;

        foreach (string part in parts)
        {
            if (TryParseModifier(part, out VirtualKeyShort modifier))
            {
                modifiers.Add(modifier);
                continue;
            }

            if (key is not null)
            {
                error = "chord must contain exactly one non-modifier key";
                return false;
            }

            if (!TryParseKeyToken(part, out VirtualKeyShort k))
            {
                error = $"unsupported key token: {part}";
                return false;
            }

            key = k;
        }

        if (key is null)
        {
            error = "chord must contain a non-modifier key";
            return false;
        }

        parsed = new ParsedSendKeys
        {
            Kind = ParsedSendKeysKinds.Chord,
            Modifiers = modifiers.ToArray(),
            Key = key,
        };
        return true;
    }

    private static bool TryParseModifier(string token, out VirtualKeyShort key)
    {
        key = default;
        string t = token.Trim().ToUpperInvariant();
        switch (t)
        {
            case "CTRL":
            case "CONTROL":
                key = (VirtualKeyShort)0x11; // VK_CONTROL
                return true;
            case "SHIFT":
                key = (VirtualKeyShort)0x10; // VK_SHIFT
                return true;
            case "ALT":
                key = (VirtualKeyShort)0x12; // VK_MENU
                return true;
            case "WIN":
            case "META":
                key = (VirtualKeyShort)0x5B; // VK_LWIN
                return true;
            default:
                return false;
        }
    }

    private static bool TryParseKeyToken(string token, out VirtualKeyShort key)
    {
        key = default;
        string t = token.Trim();
        if (t.Length == 1)
        {
            char c = char.ToUpperInvariant(t[0]);
            if (c is >= 'A' and <= 'Z')
            {
                key = (VirtualKeyShort)c; // ASCII == VK
                return true;
            }

            if (c is >= '0' and <= '9')
            {
                key = (VirtualKeyShort)c; // ASCII == VK
                return true;
            }
        }

        string u = t.ToUpperInvariant();
        switch (u)
        {
            case "ENTER":
            case "RETURN":
                key = (VirtualKeyShort)0x0D; // VK_RETURN
                return true;
            case "TAB":
                key = (VirtualKeyShort)0x09; // VK_TAB
                return true;
            case "ESC":
            case "ESCAPE":
                key = (VirtualKeyShort)0x1B; // VK_ESCAPE
                return true;
            case "BACKSPACE":
            case "BS":
                key = (VirtualKeyShort)0x08; // VK_BACK
                return true;
            case "DEL":
            case "DELETE":
                key = (VirtualKeyShort)0x2E; // VK_DELETE
                return true;
            case "SPACE":
                key = (VirtualKeyShort)0x20; // VK_SPACE
                return true;
            case "UP":
                key = (VirtualKeyShort)0x26; // VK_UP
                return true;
            case "DOWN":
                key = (VirtualKeyShort)0x28; // VK_DOWN
                return true;
            case "LEFT":
                key = (VirtualKeyShort)0x25; // VK_LEFT
                return true;
            case "RIGHT":
                key = (VirtualKeyShort)0x27; // VK_RIGHT
                return true;
        }

        // F1-F12
        if (u.Length is >= 2 and <= 3 && u[0] == 'F' && int.TryParse(u[1..], out int f) && f is >= 1 and <= 12)
        {
            key = (VirtualKeyShort)(0x70 + (f - 1)); // VK_F1..VK_F12
            return true;
        }

        return false;
    }
}

internal static class ParsedSendKeysKinds
{
    public const string Text = "Text";
    public const string Key = "Key";
    public const string Chord = "Chord";
}
