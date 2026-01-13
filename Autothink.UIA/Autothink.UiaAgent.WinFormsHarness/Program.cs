// 说明:
// - WinForms 测试台入口，用于手工验证 UIA Agent 的 RPC 与选择器效果。
// - 该进程仅用于本机调试/现场联调，不作为产品交付组件。
namespace Autothink.UiaAgent.WinFormsHarness;

/// <summary>
/// WinFormsHarness 启动入口。
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
