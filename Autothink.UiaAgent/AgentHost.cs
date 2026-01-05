using System.Collections.Concurrent;
using System.Runtime.ExceptionServices;
using Autothink.UiaAgent.Rpc;
using StreamJsonRpc;

namespace Autothink.UiaAgent;

/// <summary>
/// 负责 Agent 的生命周期管理与 IPC 主循环。
/// </summary>
/// <remarks>
/// 该进程设计为 sidecar：
/// - 通过 stdin/stdout 承载 JSON-RPC 协议数据（StreamJsonRpc）。
/// - 启动后立即输出一行 "READY" 到 stdout，作为宿主探测信号。
/// - 一旦进入 JSON-RPC 通信阶段，stdout 将被协议占用，因此业务日志应写到 stderr。
/// - 通过 <see cref="JsonRpc.SynchronizationContext"/> 将 RPC 调用调度回当前 STA 线程，
///   保证 UIA/FlaUI 相关操作始终在同一线程执行。
/// </remarks>
internal static class AgentHost
{
    /// <summary>
    /// 在 STA 线程上运行 Agent。
    /// </summary>
    /// <param name="args">命令行参数（预留扩展：例如启用调试、选择 IPC 模式等）。</param>
    /// <returns>退出码：0 表示正常退出；非 0 表示异常退出。</returns>
    internal static int Run(string[] args)
    {
        // StreamJsonRpc 在派发 RPC 方法调用时会使用 SynchronizationContext。
        // 默认情况下，控制台程序没有消息循环；这里我们提供一个最小单线程消息循环，
        // 以便把所有 RPC 回调串行化并固定在当前（STA）线程。
        using var synchronizationContext = new SingleThreadSynchronizationContext();
        SynchronizationContext.SetSynchronizationContext(synchronizationContext);

        try
        {
            // 诊断信息写 stderr，便于确认运行的是哪个版本以及支持哪些动作。
            string version = typeof(AgentHost).Assembly.GetName().Version?.ToString() ?? "unknown";
            string assemblyPath = typeof(AgentHost).Assembly.Location;
            DateTimeOffset buildTimeUtc = File.Exists(assemblyPath)
                ? File.GetLastWriteTimeUtc(assemblyPath)
                : DateTimeOffset.UtcNow;
            Console.Error.WriteLine($"[Agent] Version={version} BuildUtc={buildTimeUtc:O}");
            Console.Error.WriteLine("[Agent] OpenImportDialogSteps actions: Click, DoubleClick, RightClick, Hover, SetText, SendKeys, WaitUntil");

            // sidecar 约定：Agent 启动后尽快写出 READY，让宿主确认“进程已启动且已进入监听状态”。
            // 注意：这行写到 stdout；后续 stdout 会用于 JSON-RPC 数据流，因此不能再写业务日志到 stdout。
            Console.Out.WriteLine("READY");
            Console.Out.Flush();

            // 使用标准输入/输出作为双向管道。
            // 选择 OpenStandardInput/Output 而不是 Console.In/Out，避免引入额外的编码/缓冲语义。
            using Stream receivingStream = Console.OpenStandardInput();
            using Stream sendingStream = Console.OpenStandardOutput();

            // HeaderDelimitedMessageHandler 使用类似 LSP 的“Content-Length: ...\r\n\r\n{json}”格式分帧，
            // 能在纯字节流（stdin/stdout）上可靠地边界化 JSON-RPC 消息。
            using var messageHandler = new HeaderDelimitedMessageHandler(sendingStream, receivingStream);

            // RPC target：对外暴露 UIA/FlaUI 入口（OpenSession/Find/Click/RunFlow...）。
            var rpcTarget = new UiaRpcService();
            using var rpc = new JsonRpc(messageHandler, rpcTarget)
            {
                // 关键：将所有 RPC 方法调度回当前线程执行。
                // 对 UIA/FlaUI 来说，这能极大降低跨线程/COM apartment 问题。
                SynchronizationContext = synchronizationContext,
            };

            // 开始监听：后台会从 receivingStream 读取请求并派发到 rpcTarget。
            rpc.StartListening();

            // 当对端关闭连接（EOF）或发生协议层错误时，rpc.Completion 将完成。
            // 我们在完成后通知消息循环退出，从而让整个 Run 方法正常返回。
            // 注意这里使用 TaskScheduler.Default，避免回调被投递回我们自己的单线程上下文而产生死锁。
            _ = rpc.Completion.ContinueWith(
                _ => synchronizationContext.Complete(),
                CancellationToken.None,
                TaskContinuationOptions.None,
                TaskScheduler.Default);

            // 运行单线程消息循环：持续处理 SynchronizationContext.Post/Sent 投递的工作项。
            // 只有当 Complete() 被调用（例如 rpc.Completion 完成）时才会退出。
            synchronizationContext.Run();

            // 确保 RPC 真的结束，并将可能的异常（例如读取流失败/协议异常）向上传播。
            // Program.cs 会捕获并写入 stderr。
            rpc.Completion.GetAwaiter().GetResult();
            return 0;
        }
        finally
        {
            // 避免将该 SynchronizationContext 泄漏到调用栈之外。
            SynchronizationContext.SetSynchronizationContext(null);
        }
    }

    /// <summary>
    /// 最小实现的单线程 <see cref="SynchronizationContext"/>。
    /// </summary>
    /// <remarks>
    /// 控制台程序默认没有 UI 消息循环/调度器。
    /// 该实现提供：
    /// - <see cref="Post"/>：异步投递回调（不阻塞调用方）。
    /// - <see cref="Send"/>：同步投递回调（阻塞调用方直到回调执行完）。
    /// - <see cref="Run"/>：在拥有线程上顺序执行回调，充当消息循环。
    /// - <see cref="Complete"/>：结束消息循环。
    ///
    /// StreamJsonRpc 在需要回到指定线程执行 target 方法时，会使用该上下文来投递工作项。
    /// </remarks>
    private sealed class SingleThreadSynchronizationContext : SynchronizationContext, IDisposable
    {
        // 队列元素是 (回调, state)。BlockingCollection 既可用于生产/消费，也可用于“完成后退出”。
        private readonly BlockingCollection<(SendOrPostCallback Callback, object? State)> queue = new();

        // 记录拥有该上下文的线程 ID，用于在 Send 时做“同线程直接执行”的快速路径。
        private readonly int owningThreadId = Thread.CurrentThread.ManagedThreadId;

        public override void Post(SendOrPostCallback d, object? state)
        {
            ArgumentNullException.ThrowIfNull(d);

            // Post 语义是尽力投递：如果已经 Complete，则 TryAdd 会失败，我们选择静默丢弃，
            // 以便在连接关闭后更快退出。
            _ = this.queue.TryAdd((d, state));
        }

        public override void Send(SendOrPostCallback d, object? state)
        {
            ArgumentNullException.ThrowIfNull(d);

            // 如果调用方就在拥有线程上，直接执行即可，避免死锁。
            if (Thread.CurrentThread.ManagedThreadId == this.owningThreadId)
            {
                d(state);
                return;
            }

            // 连接已结束时不再接受新工作。
            if (this.queue.IsAddingCompleted)
            {
                throw new InvalidOperationException("The synchronization context is no longer accepting work.");
            }

            // Send 需要“同步返回”，因此用事件等待回调执行完。
            using var done = new ManualResetEventSlim(false);

            // 用 ExceptionDispatchInfo 保留异常的原始堆栈信息。
            ExceptionDispatchInfo? exception = null;

            void Wrapped(object? _)
            {
                try
                {
                    d(state);
                }
                catch (Exception ex)
                {
                    exception = ExceptionDispatchInfo.Capture(ex);
                }
                finally
                {
                    done.Set();
                }
            }

            if (!this.queue.TryAdd((Wrapped, null)))
            {
                throw new InvalidOperationException("The synchronization context is no longer accepting work.");
            }

            // 等待回调在拥有线程中执行完成。
            done.Wait();
            exception?.Throw();
        }

        /// <summary>
        /// 通知消息循环退出（幂等）。
        /// </summary>
        public void Complete()
        {
            if (!this.queue.IsAddingCompleted)
            {
                this.queue.CompleteAdding();
            }
        }

        /// <summary>
        /// 在拥有线程上运行消息循环。
        /// </summary>
        /// <remarks>
        /// 必须由创建该上下文的线程调用。
        /// </remarks>
        public void Run()
        {
            foreach ((SendOrPostCallback callback, object? state) in this.queue.GetConsumingEnumerable())
            {
                callback(state);
            }
        }

        public void Dispose() => this.queue.Dispose();
    }
}
