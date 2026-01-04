using System.Diagnostics;
using System.Text;
using StreamJsonRpc;

namespace Autothink.UiaAgent.WinFormsHarness;

internal sealed class AgentRpcClient : IAsyncDisposable
{
    private Process? process;
    private HeaderDelimitedMessageHandler? messageHandler;
    private JsonRpc? rpc;

    public event Action<string>? StderrLine;

    public JsonRpc? Rpc => this.rpc;

    public bool IsRunning => this.process is not null && !this.process.HasExited && this.rpc is not null;

    public async Task StartAsync(string agentExePath, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(agentExePath))
        {
            throw new ArgumentException("Agent exe path must be provided.", nameof(agentExePath));
        }

        if (!File.Exists(agentExePath))
        {
            throw new FileNotFoundException("Agent exe not found.", agentExePath);
        }

        if (this.process is not null)
        {
            throw new InvalidOperationException("Agent is already started.");
        }

        var startInfo = new ProcessStartInfo
        {
            FileName = agentExePath,
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8,
        };

        this.process = Process.Start(startInfo) ?? throw new InvalidOperationException("Failed to start agent process.");

        // sidecar handshake: the agent prints a single READY line to stdout before starting JSON-RPC framing.
        string readyLine = await ReadAsciiLineAsync(this.process.StandardOutput.BaseStream, cancellationToken);
        if (!string.Equals(readyLine.Trim(), "READY", StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"Unexpected READY handshake: '{readyLine}'.");
        }

        this.messageHandler = new HeaderDelimitedMessageHandler(this.process.StandardInput.BaseStream, this.process.StandardOutput.BaseStream);
        this.rpc = new JsonRpc(this.messageHandler);
        this.rpc.StartListening();

        _ = Task.Run(() => PumpStderrAsync(this.process, cancellationToken), cancellationToken);
    }

    public async Task StopAsync(CancellationToken cancellationToken)
    {
        if (this.process is null)
        {
            return;
        }

        try
        {
            this.rpc?.Dispose();
            if (this.messageHandler is not null)
            {
                await this.messageHandler.DisposeAsync();
            }
        }
        finally
        {
            this.rpc = null;
            this.messageHandler = null;

            if (!this.process.HasExited)
            {
                this.process.Kill(entireProcessTree: true);
            }

            await this.process.WaitForExitAsync(cancellationToken);
            this.process.Dispose();
            this.process = null;
        }
    }

    public async ValueTask DisposeAsync()
    {
        try
        {
            await this.StopAsync(CancellationToken.None);
        }
        catch
        {
            // ignore
        }
    }

    private static async Task<string> ReadAsciiLineAsync(Stream stream, CancellationToken cancellationToken)
    {
        var bytes = new List<byte>(64);
        var buffer = new byte[1];

        while (true)
        {
            int read = await stream.ReadAsync(buffer, 0, 1, cancellationToken);
            if (read == 0)
            {
                break;
            }

            byte b = buffer[0];
            if (b == (byte)'\n')
            {
                break;
            }

            if (b != (byte)'\r')
            {
                bytes.Add(b);
            }

            if (bytes.Count > 4096)
            {
                throw new InvalidOperationException("READY line is too long.");
            }
        }

        return Encoding.UTF8.GetString(bytes.ToArray());
    }

    private async Task PumpStderrAsync(Process p, CancellationToken cancellationToken)
    {
        try
        {
            while (!p.HasExited && !cancellationToken.IsCancellationRequested)
            {
                string? line = await p.StandardError.ReadLineAsync(cancellationToken);
                if (line is null)
                {
                    break;
                }

                this.StderrLine?.Invoke(line);
            }
        }
        catch
        {
            // ignore
        }
    }
}
