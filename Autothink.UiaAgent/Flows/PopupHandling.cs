using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Uia;
using FlaUI.Core.AutomationElements;

namespace Autothink.UiaAgent.Flows;

internal sealed class PopupHandlingOptions
{
    public bool Enabled { get; set; }

    public string SearchRoot { get; set; } = "desktop";

    public int TimeoutMs { get; set; } = 1500;

    public bool AllowOk { get; set; }

    public ElementSelector? DialogSelector { get; set; }

    public ElementSelector? OkButtonSelector { get; set; }

    public ElementSelector? CancelButtonSelector { get; set; }
}

internal static class PopupHandling
{
    private static readonly TimeSpan DefaultPollInterval = TimeSpan.FromMilliseconds(200);

    public static void TryHandle(FlowContext context, Window mainWindow, PopupHandlingOptions? options, string stepTag)
    {
        if (options is null || !options.Enabled)
        {
            return;
        }

        AutomationElement root = ResolveRoot(context, mainWindow, options.SearchRoot, out string rootKind);

        if (options.DialogSelector is null || options.DialogSelector.Path.Count == 0)
        {
            StepLogEntry invalid = context.StartStep(
                stepId: $"PopupDetected.{stepTag}",
                action: "Detect popup (invalid selector)");
            context.MarkWarning(invalid, new RpcError
            {
                Kind = RpcErrorKinds.InvalidArgument,
                Message = "Popup dialog selector is missing",
            });
            return;
        }

        StepLogEntry detectStep = context.StartStep(
            stepId: $"PopupDetected.{stepTag}",
            action: "Detect popup",
            selector: options.DialogSelector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["timeoutMs"] = options.TimeoutMs.ToString(),
                ["root"] = rootKind,
            });

        AutomationElement? dialog = null;
        string? lastFailure = null;
        Dictionary<string, string>? lastDetails = null;

        bool found = Waiter.PollUntil(
            predicate: () =>
            {
                bool ok = ElementFinder.TryFind(root, context.Session.Automation, options.DialogSelector, out AutomationElement? element, out string? fk, out Dictionary<string, string>? det);
                if (ok)
                {
                    dialog = element;
                    return true;
                }

                lastFailure = fk;
                lastDetails = det;
                return false;
            },
            timeout: TimeSpan.FromMilliseconds(options.TimeoutMs),
            interval: DefaultPollInterval);

        if (!found || dialog is null)
        {
            detectStep.Parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
            detectStep.Parameters["found"] = "false";
            if (!string.IsNullOrWhiteSpace(lastFailure))
            {
                detectStep.Parameters["failureKind"] = lastFailure;
            }

            context.MarkSuccess(detectStep);
            return;
        }

        detectStep.Parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
        detectStep.Parameters["found"] = "true";
        detectStep.Parameters["title"] = dialog.Properties.Name.ValueOrDefault ?? string.Empty;
        context.MarkSuccess(detectStep);

        ElementSelector? targetSelector = null;
        AutomationElement? button = null;
        string buttonKind = "cancel";

        if (options.CancelButtonSelector is not null && options.CancelButtonSelector.Path.Count > 0)
        {
            if (ElementFinder.TryFind(dialog, context.Session.Automation, options.CancelButtonSelector, out AutomationElement? candidate, out _, out _) &&
                candidate is not null)
            {
                targetSelector = options.CancelButtonSelector;
                button = candidate;
            }
        }

        if (button is null && options.AllowOk && options.OkButtonSelector is not null && options.OkButtonSelector.Path.Count > 0)
        {
            if (ElementFinder.TryFind(dialog, context.Session.Automation, options.OkButtonSelector, out AutomationElement? candidate, out _, out _) &&
                candidate is not null)
            {
                targetSelector = options.OkButtonSelector;
                button = candidate;
                buttonKind = "ok";
            }
        }

        StepLogEntry dismissStep = context.StartStep(
            stepId: $"PopupDismissed.{stepTag}",
            action: "Dismiss popup",
            selector: targetSelector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["root"] = rootKind,
                ["button"] = buttonKind,
                ["title"] = dialog.Properties.Name.ValueOrDefault ?? string.Empty,
            });

        if (button is null)
        {
            context.MarkWarning(dismissStep, new RpcError
            {
                Kind = RpcErrorKinds.FindError,
                Message = "Popup button not found",
            });
            return;
        }

        try
        {
            button.Click();
            context.MarkSuccess(dismissStep);
        }
        catch (Exception ex)
        {
            context.MarkWarning(dismissStep, new RpcError
            {
                Kind = RpcErrorKinds.ActionError,
                Message = "Popup dismiss failed",
                Details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["exceptionType"] = ex.GetType().FullName ?? ex.GetType().Name,
                    ["exceptionMessage"] = ex.Message,
                },
            });
        }
    }

    private static AutomationElement ResolveRoot(
        FlowContext context,
        Window mainWindow,
        string searchRoot,
        out string normalizedRoot)
    {
        if (string.Equals(searchRoot, "desktop", StringComparison.OrdinalIgnoreCase))
        {
            normalizedRoot = "desktop";
            return context.Session.Automation.GetDesktop();
        }

        normalizedRoot = "mainWindow";
        return mainWindow;
    }
}
