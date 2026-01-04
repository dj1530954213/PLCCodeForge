using System.Text.Json;
using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Flows.Autothink;

namespace Autothink.UiaAgent.Flows;

internal static class FlowRegistry
{
    private static readonly object Sync = new();

    private static readonly Dictionary<string, IFlow> Flows = new(StringComparer.Ordinal)
    {
        [FlowNames.AutothinkAttach] = new AutothinkAttachFlow(),
        [FlowNames.AutothinkImportVariables] = new AutothinkImportVariablesFlow(),
        [FlowNames.AutothinkImportProgramTextPaste] = new AutothinkImportProgramTextPasteFlow(),
        [FlowNames.AutothinkBuild] = new AutothinkBuildFlow(),
    };

    public static IReadOnlyList<string> KnownFlowNames => FlowNames.AllOrdered;

    public static bool TryGet(string flowName, out IFlow? flow)
    {
        lock (Sync)
        {
            return Flows.TryGetValue(flowName, out flow);
        }
    }

    public static void Register(IFlow flow)
    {
        ArgumentNullException.ThrowIfNull(flow);

        if (!FlowNames.IsKnown(flow.Name))
        {
            throw new ArgumentException($"Unknown flow name: {flow.Name}", nameof(flow));
        }

        lock (Sync)
        {
            Flows[flow.Name] = flow;
        }
    }

    internal static IDisposable OverrideForTests(string flowName, IFlow flow)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(flowName);
        ArgumentNullException.ThrowIfNull(flow);

        if (!FlowNames.IsKnown(flowName))
        {
            throw new ArgumentException($"Unknown flow name: {flowName}", nameof(flowName));
        }

        lock (Sync)
        {
            bool hadExisting = Flows.TryGetValue(flowName, out IFlow? existing);
            Flows[flowName] = flow;

            return new RevertAction(() =>
            {
                lock (Sync)
                {
                    if (hadExisting && existing is not null)
                    {
                        Flows[flowName] = existing;
                    }
                    else
                    {
                        _ = Flows.Remove(flowName);
                    }
                }
            });
        }
    }

    private sealed class RevertAction : IDisposable
    {
        private readonly Action action;
        private bool disposed;

        public RevertAction(Action action)
        {
            this.action = action;
        }

        public void Dispose()
        {
            if (this.disposed)
            {
                return;
            }

            this.disposed = true;
            this.action();
        }
    }

    private sealed class StubFlow : IFlow
    {
        public StubFlow(string name)
        {
            this.Name = name;
        }

        public string Name { get; }

        public bool IsImplemented => false;

        public RpcResult<RunFlowResponse> Run(FlowContext context, JsonElement? args)
        {
            throw new NotSupportedException("Flow is not implemented.");
        }
    }
}
