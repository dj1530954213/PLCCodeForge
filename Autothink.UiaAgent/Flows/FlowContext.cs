using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Uia;

namespace Autothink.UiaAgent.Flows;

/// <summary>
/// Flow 执行上下文。
/// </summary>
internal sealed class FlowContext
{
    public FlowContext(string sessionId, UiaSession session, TimeSpan timeout, StepLog stepLog)
    {
        this.SessionId = sessionId;
        this.Session = session;
        this.Timeout = timeout;
        this.StepLog = stepLog;
    }

    public string SessionId { get; }

    public UiaSession Session { get; }

    public TimeSpan Timeout { get; }

    public StepLog StepLog { get; }

    public StepLogEntry StartStep(string stepId, string action, ElementSelector? selector = null, Dictionary<string, string>? parameters = null)
    {
        if (selector is not null)
        {
            string? rule = ElementFinder.DescribeMatchRules(selector);
            if (!string.IsNullOrWhiteSpace(rule))
            {
                parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
                parameters["matchRule"] = rule;
            }
        }

        var entry = new StepLogEntry
        {
            StepId = stepId,
            Action = action,
            Selector = selector,
            Parameters = parameters,
            StartedAtUtc = DateTimeOffset.UtcNow,
            Outcome = StepOutcomes.Fail,
        };

        this.StepLog.Steps.Add(entry);
        return entry;
    }

    public bool TrySetClipboardText(string? text, out RpcError? error, int? timeoutMs = null, bool warnOnFailure = false)
    {
        int effectiveTimeoutMs = timeoutMs is > 0 ? timeoutMs.Value : 2_000;
        string value = text ?? string.Empty;

        StepLogEntry step = this.StartStep(
            stepId: "SetClipboardText",
            action: "SetClipboardText",
            selector: null,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["timeoutMs"] = effectiveTimeoutMs.ToString(),
                ["textLength"] = value.Length.ToString(),
            });

        try
        {
            ClipboardText.SetTextWithRetry(value, TimeSpan.FromMilliseconds(effectiveTimeoutMs), TimeSpan.FromMilliseconds(50));
            this.MarkSuccess(step);
            error = null;
            return true;
        }
        catch (Exception ex)
        {
            error = new RpcError
            {
                Kind = RpcErrorKinds.ActionError,
                Message = "SetClipboardText failed",
                Details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["exceptionType"] = ex.GetType().FullName ?? ex.GetType().Name,
                    ["exceptionMessage"] = ex.Message,
                },
            };

            if (warnOnFailure)
            {
                this.MarkWarning(step, error);
            }
            else
            {
                this.MarkFailure(step, error);
            }

            return false;
        }
    }

    public void MarkSuccess(StepLogEntry step)
    {
        step.FinishedAtUtc = DateTimeOffset.UtcNow;
        step.DurationMs = (long)(step.FinishedAtUtc - step.StartedAtUtc).TotalMilliseconds;
        step.Outcome = StepOutcomes.Success;
    }

    public void MarkWarning(StepLogEntry step, RpcError error)
    {
        step.FinishedAtUtc = DateTimeOffset.UtcNow;
        step.DurationMs = (long)(step.FinishedAtUtc - step.StartedAtUtc).TotalMilliseconds;
        step.Outcome = StepOutcomes.Warning;
        step.Error = error;
    }

    public void MarkFailure(StepLogEntry step, RpcError error)
    {
        step.FinishedAtUtc = DateTimeOffset.UtcNow;
        step.DurationMs = (long)(step.FinishedAtUtc - step.StartedAtUtc).TotalMilliseconds;
        step.Outcome = StepOutcomes.Fail;
        step.Error = error;
    }
}
