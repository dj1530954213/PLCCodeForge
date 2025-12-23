namespace Autothink.UiaAgent;

/// <summary>
/// 控制台进程入口。
/// </summary>
/// <remarks>
/// 该进程将作为 Tauri sidecar / 外部宿主启动的“工具进程”。
/// UIA/FlaUI 在实践中更稳定的运行方式是：
/// - 整个 Agent 生命周期都固定在同一个 STA 线程中执行（包含 RPC 回调）。
/// - 主线程只负责创建/等待 STA 线程，避免把 UIA 调用分散到线程池或其它线程。
/// </remarks>
internal static class Program
{
    /// <summary>
    /// 进程主入口。
    /// </summary>
    /// <param name="args">命令行参数（预留给未来：例如启用调试、选择 IPC 模式等）。</param>
    /// <returns>
    /// 退出码：
    /// - 0：正常退出。
    /// - 非 0：发生未处理异常或异常终止。
    /// </returns>
    private static int Main(string[] args)
    {
        // 默认退出码为非 0，确保“未明确成功”时不会被宿主误判为成功。
        int exitCode = 1;

        // STA 线程内发生的未处理异常会在这里保存，等待线程 Join 后统一输出到 stderr。
        // 这样做能避免在 RPC 期间把异常写到 stdout（stdout 会承载 JSON-RPC 协议数据）。
        Exception? fatalException = null;

        // UIA/FlaUI 要求 STA（Single-Threaded Apartment）。
        // 因此我们用一个专门的线程作为“Agent 主线程”，并显式设置为 STA。
        var thread = new Thread(() =>
        {
            try
            {
                exitCode = AgentHost.Run(args);
            }
            catch (Exception ex)
            {
                // 不在这里直接写 stdout，避免破坏 JSON-RPC 流；仅记录异常，交给主线程输出到 stderr。
                fatalException = ex;
                exitCode = 1;
            }
        });

        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();

        // 阻塞等待 STA 线程退出。
        // sidecar 场景下，宿主通常以进程存活作为“服务仍在运行”的信号。
        thread.Join();

        if (fatalException is not null)
        {
            // 约定：诊断/错误输出只写 stderr。
            Console.Error.WriteLine(fatalException);
        }

        return exitCode;
    }
}
