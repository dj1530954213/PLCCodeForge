// 说明:
// - ElementRef 是元素引用的序列化形式，用于跨 RPC 调用复用目标元素。
// - 该引用可能失效，调用方需按错误类型重试/重新查找。
namespace Autothink.UiaAgent.Rpc.Contracts;

/// <summary>
/// 元素引用：用于在后续 RPC 调用中指向同一个“目标元素”。
/// </summary>
/// <remarks>
/// ElementRef 必须被视为“可失效”的：UI 刷新后可能需要重新定位或返回 StaleElement 错误。
/// </remarks>
public sealed class ElementRef
{
    /// <summary>
    /// 所属会话标识。
    /// </summary>
    public string SessionId { get; set; } = string.Empty;

    /// <summary>
    /// 用于重定位该元素的选择器。
    /// </summary>
    public ElementSelector Selector { get; set; } = new();

    /// <summary>
    /// 捕获该引用时的 RuntimeId（可选）。
    /// </summary>
    /// <remarks>
    /// RuntimeId 通常能帮助诊断“是否元素已变化”；但不能作为长期稳定标识。
    /// </remarks>
    public int[]? RuntimeId { get; set; }

    /// <summary>
    /// 捕获时间（UTC）。
    /// </summary>
    public DateTimeOffset CapturedAtUtc { get; set; }
}
