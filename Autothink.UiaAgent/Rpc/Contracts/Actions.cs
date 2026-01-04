namespace Autothink.UiaAgent.Rpc.Contracts;

/// <summary>
/// 查找元素请求。
/// </summary>
public sealed class FindElementRequest
{
    /// <summary>
    /// 会话标识。
    /// </summary>
    public string SessionId { get; set; } = string.Empty;

    /// <summary>
    /// 元素选择器。
    /// </summary>
    public ElementSelector Selector { get; set; } = new();

    /// <summary>
    /// 等待超时（毫秒）。
    /// </summary>
    public int TimeoutMs { get; set; } = 5_000;
}

/// <summary>
/// 查找元素返回值。
/// </summary>
public sealed class FindElementResponse
{
    /// <summary>
    /// 找到的元素引用。
    /// </summary>
    public ElementRef Element { get; set; } = new();
}

/// <summary>
/// 以元素引用为目标的请求基类。
/// </summary>
public abstract class ElementRefRequest
{
    /// <summary>
    /// 目标元素引用。
    /// </summary>
    public ElementRef Element { get; set; } = new();
}

/// <summary>
/// 点击请求。
/// </summary>
public sealed class ClickRequest : ElementRefRequest;

/// <summary>
/// 双击请求。
/// </summary>
public sealed class DoubleClickRequest : ElementRefRequest;

/// <summary>
/// 右键请求。
/// </summary>
public sealed class RightClickRequest : ElementRefRequest;

/// <summary>
/// 设置文本请求。
/// </summary>
public sealed class SetTextRequest : ElementRefRequest
{
    /// <summary>
    /// 要设置的文本。
    /// </summary>
    public string Text { get; set; } = string.Empty;

    /// <summary>
    /// 输入模式：Replace / Append / CtrlAReplace。
    /// </summary>
    public string Mode { get; set; } = SetTextModes.Replace;
}

/// <summary>
/// <see cref="SetTextRequest.Mode"/> 的约定取值。
/// </summary>
public static class SetTextModes
{
    /// <summary>清空后写入。</summary>
    public const string Replace = "Replace";

    /// <summary>追加。</summary>
    public const string Append = "Append";

    /// <summary>先 Ctrl+A 再写入（用于某些控件更可靠）。</summary>
    public const string CtrlAReplace = "CtrlAReplace";
}

/// <summary>
/// 发送按键请求。
/// </summary>
public sealed class SendKeysRequest
{
    /// <summary>
    /// 会话标识。
    /// </summary>
    public string SessionId { get; set; } = string.Empty;

    /// <summary>
    /// 按键字符串（例如："CTRL+V"、"ENTER"）。
    /// </summary>
    public string Keys { get; set; } = string.Empty;
}

/// <summary>
/// 等待条件请求。
/// </summary>
public sealed class WaitUntilRequest
{
    /// <summary>
    /// 会话标识。
    /// </summary>
    public string SessionId { get; set; } = string.Empty;

    /// <summary>
    /// 等待超时（毫秒）。
    /// </summary>
    public int TimeoutMs { get; set; } = 5_000;

    /// <summary>
    /// 等待条件。
    /// </summary>
    public WaitCondition Condition { get; set; } = new();
}

/// <summary>
/// 可观测等待条件。
/// </summary>
public sealed class WaitCondition
{
    /// <summary>
    /// 条件类型：ElementExists / ElementNotExists / ElementEnabled。
    /// </summary>
    public string Kind { get; set; } = WaitConditionKinds.ElementExists;

    /// <summary>
    /// 目标元素选择器（可选，取决于 Kind）。
    /// </summary>
    public ElementSelector? Selector { get; set; }
}

/// <summary>
/// <see cref="WaitCondition.Kind"/> 的约定取值。
/// </summary>
public static class WaitConditionKinds
{
    /// <summary>等待元素存在。</summary>
    public const string ElementExists = "ElementExists";

    /// <summary>等待元素消失。</summary>
    public const string ElementNotExists = "ElementNotExists";

    /// <summary>等待元素可用（Enabled）。</summary>
    public const string ElementEnabled = "ElementEnabled";
}
