// 说明:
// - 控件类型映射：把配置中的字符串（如 "Button"）转换为 FlaUI ControlType。
// - 供 selector 解析与 ElementFinder 使用，提升可读性与一致性。
using System.Collections.Concurrent;
using System.Reflection;
using FlaUI.Core.Definitions;

namespace Autothink.UiaAgent.Uia;

/// <summary>
/// ControlType 映射表：从字符串名称解析为 FlaUI 的 ControlType。
/// </summary>
internal static class ControlTypeMap
{
    private static readonly ConcurrentDictionary<string, ControlType> map = new(StringComparer.OrdinalIgnoreCase);

    static ControlTypeMap()
    {
        // 通过反射收集 FlaUI 预定义的 ControlType（静态属性/字段）。
        foreach (PropertyInfo prop in typeof(ControlType).GetProperties(BindingFlags.Public | BindingFlags.Static))
        {
            if (prop.PropertyType != typeof(ControlType))
            {
                continue;
            }

            if (prop.GetValue(null) is ControlType ct)
            {
                _ = map.TryAdd(prop.Name, ct);
            }
        }

        foreach (FieldInfo field in typeof(ControlType).GetFields(BindingFlags.Public | BindingFlags.Static))
        {
            if (field.FieldType != typeof(ControlType))
            {
                continue;
            }

            if (field.GetValue(null) is ControlType ct)
            {
                _ = map.TryAdd(field.Name, ct);
            }
        }
    }

    public static bool TryGet(string? name, out ControlType controlType)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            controlType = default!;
            return false;
        }

        if (map.TryGetValue(name.Trim(), out ControlType ct))
        {
            controlType = ct;
            return true;
        }

        controlType = default!;
        return false;
    }
}
