// 说明:
// - DemoTarget 是 UIA 自动化的稳定演示目标窗口，便于 CI/开发机验证 selector 与 flow。
// - 不依赖真实 AUTOTHINK 环境，用于快速回归与示例演示。
namespace Autothink.UiaAgent.DemoTarget;

/// <summary>
/// DemoTarget WinForms 入口。
/// </summary>
internal static class Program
{
    [STAThread]
    private static void Main()
    {
        ApplicationConfiguration.Initialize();
        Application.Run(new MainForm());
    }
}
