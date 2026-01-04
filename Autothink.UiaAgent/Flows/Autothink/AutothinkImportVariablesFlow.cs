using System.Text.Json;
using Autothink.UiaAgent.Flows;
using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Uia;
using FlaUI.Core.AutomationElements;
using FlaUI.Core.Input;
using FlaUI.Core.WindowsAPI;

namespace Autothink.UiaAgent.Flows.Autothink;

internal sealed class AutothinkImportVariablesFlow : IFlow
{
    private static readonly TimeSpan DefaultPollInterval = TimeSpan.FromMilliseconds(200);

    private const string StepActionClick = "Click";
    private const string StepActionDoubleClick = "DoubleClick";
    private const string StepActionRightClick = "RightClick";
    private const string StepActionSetText = "SetText";
    private const string StepActionSendKeys = "SendKeys";
    private const string StepActionWaitUntil = "WaitUntil";
    private const string SearchRootMainWindow = "mainWindow";
    private const string SearchRootDesktop = "desktop";

    public string Name => FlowNames.AutothinkImportVariables;

    public bool IsImplemented => true;

    public RpcResult<RunFlowResponse> Run(FlowContext context, JsonElement? args)
    {
        ArgumentNullException.ThrowIfNull(context);

        var result = new RpcResult<RunFlowResponse> { StepLog = context.StepLog };

        StepLogEntry validateStep = context.StartStep(stepId: "ValidateArgs", action: "Validate args");
        if (!TryParseArgs(args, out ParsedImportVariablesArgs parsed, out RpcError? parseError))
        {
            return Fail(result, context, validateStep, parseError ?? new RpcError
            {
                Kind = RpcErrorKinds.InvalidArgument,
                Message = "Invalid args",
            });
        }

        context.MarkSuccess(validateStep);

        StepLogEntry mainWindowStep = context.StartStep(stepId: "GetMainWindow", action: "Get main window");
        Window mainWindow;
        try
        {
            mainWindow = context.Session.GetMainWindow(context.Timeout);
            context.MarkSuccess(mainWindowStep);
        }
        catch (Exception ex)
        {
            return Fail(result, context, mainWindowStep, RpcErrorKinds.ConfigError, "Failed to get main window", ex);
        }

        StepLogEntry bringToForegroundStep = context.StartStep(stepId: "BringToForeground", action: "Bring main window to foreground");
        try
        {
            mainWindow.Focus();
            context.MarkSuccess(bringToForegroundStep);
        }
        catch (Exception ex)
        {
            context.MarkWarning(bringToForegroundStep, CreateError(RpcErrorKinds.ActionError, "Failed to bring window to foreground", ex));
        }

        AutomationElement searchRoot = ResolveSearchRoot(parsed.SearchRoot!, mainWindow, context.Session.Automation, out string searchRootKind);
        PopupHandlingOptions? popupHandling = BuildPopupHandling(parsed);
        PopupHandling.TryHandle(context, mainWindow, popupHandling, "BeforeOpenImport");

        if (!TryOpenImportDialog(context, mainWindow, searchRoot, parsed, searchRootKind, out RpcError? openError))
        {
            result.Ok = false;
            result.Error = openError;
            return result;
        }

        if (parsed.DialogSelector is not null && parsed.DialogSelector.Path.Count > 0)
        {
            RpcResult<RunFlowResponse>? waitOpenResult = WaitUntil(
                context,
                mainWindow,
                searchRoot,
                searchRootKind,
                stepId: "WaitDialogOpen",
                action: "Wait import dialog open",
                condition: new WaitCondition
                {
                    Kind = WaitConditionKinds.ElementExists,
                    Selector = parsed.DialogSelector,
                },
                timeoutMs: Math.Min(parsed.WaitTimeoutMs, 10_000));

            if (waitOpenResult is not null)
            {
                return waitOpenResult;
            }
        }

        if (!TrySetFilePath(context, mainWindow, searchRoot, parsed, searchRootKind, out RpcError? setPathError))
        {
            result.Ok = false;
            result.Error = setPathError;
            return result;
        }

        if (!TryConfirmImport(context, mainWindow, searchRoot, parsed, searchRootKind, out RpcError? confirmError))
        {
            result.Ok = false;
            result.Error = confirmError;
            return result;
        }

        PopupHandling.TryHandle(context, mainWindow, popupHandling, "AfterConfirmImport");

        WaitCondition completed = parsed.CompletedCondition ?? new WaitCondition
        {
            Kind = WaitConditionKinds.ElementNotExists,
            Selector = parsed.DialogSelector,
        };

        RpcResult<RunFlowResponse>? waitDone = WaitUntil(
            context,
            mainWindow,
            searchRoot,
            searchRootKind,
            stepId: "WaitImportDone",
            action: "Wait import done",
            condition: completed,
            timeoutMs: parsed.WaitTimeoutMs);

        if (waitDone is not null)
        {
            return waitDone;
        }

        PopupHandling.TryHandle(context, mainWindow, popupHandling, "AfterImportDone");

        result.Ok = true;
        result.Value = new RunFlowResponse
        {
            Data = JsonSerializer.SerializeToElement(new
            {
                path = parsed.FilePath,
            }),
        };
        return result;
    }

    private static PopupHandlingOptions? BuildPopupHandling(ParsedImportVariablesArgs parsed)
    {
        return parsed.PopupHandling;
    }

    private static bool TryOpenImportDialog(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        ParsedImportVariablesArgs parsed,
        string searchRootKind,
        out RpcError? error)
    {
        if (parsed.OpenImportDialogSteps is { Count: > 0 })
        {
            for (int i = 0; i < parsed.OpenImportDialogSteps.Count; i++)
            {
                if (!TryExecuteDialogStep(context, mainWindow, searchRoot, parsed, searchRootKind, parsed.OpenImportDialogSteps[i], i, out error))
                {
                    return false;
                }
            }

            error = null;
            return true;
        }

        if (parsed.OpenImportSelector is null || parsed.OpenImportSelector.Path.Count == 0)
        {
            StepLogEntry skipped = context.StartStep(stepId: "OpenImportDialog", action: "Open import dialog (skipped)");
            skipped.Parameters = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["skipped"] = "true",
            };
            context.MarkSuccess(skipped);
            error = null;
            return true;
        }

        StepLogEntry openStep = context.StartStep(
            stepId: "OpenImportDialog",
            action: "Open import dialog",
            selector: parsed.OpenImportSelector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["timeoutMs"] = parsed.FindTimeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        AddRootParameters(openStep.Parameters, searchRootKind);

        if (!TryFindElementWithSearchRoot(
                mainWindow,
                searchRoot,
                context.Session.Automation,
                parsed.OpenImportSelector,
                parsed.FindTimeoutMs,
                searchRootKind,
                out AutomationElement? element,
                out string? failureKind,
                out Dictionary<string, string>? details) ||
            element is null)
        {
            RpcError err = MapFindFailureToError(failureKind, details);
            context.MarkFailure(openStep, err);
            error = err;
            return false;
        }

        try
        {
            element.Click();
            context.MarkSuccess(openStep);
            error = null;
            return true;
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.ActionError, "Failed to open import dialog", ex);
            context.MarkFailure(openStep, error);
            return false;
        }
    }
    private static bool TryExecuteDialogStep(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        ParsedImportVariablesArgs parsed,
        string searchRootKind,
        ImportDialogStepAction stepAction,
        int index,
        out RpcError? error)
    {
        string stepId = index == 0 ? "OpenImportDialog" : $"OpenImportDialog.{index + 1}";
        string? actionKind = NormalizeActionKind(stepAction.Action);

        StepLogEntry step = context.StartStep(
            stepId: stepId,
            action: "Open import dialog step",
            selector: stepAction.Selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["action"] = actionKind ?? string.Empty,
            });

        AddRootParameters(step.Parameters, searchRootKind);

        if (actionKind is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "OpenImportDialogSteps action must be provided" };
            context.MarkFailure(step, error);
            return false;
        }

        int findTimeoutMs = stepAction.TimeoutMs.GetValueOrDefault(parsed.FindTimeoutMs);
        int waitTimeoutMs = stepAction.TimeoutMs.GetValueOrDefault(parsed.WaitTimeoutMs);

        if (actionKind == StepActionSendKeys)
        {
            if (string.IsNullOrWhiteSpace(stepAction.Keys))
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "SendKeys action requires keys" };
                context.MarkFailure(step, error);
                return false;
            }

            step.Parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
            step.Parameters["keys"] = stepAction.Keys;

            try
            {
                if (!SendKeysParser.TryParse(stepAction.Keys, out ParsedSendKeys? parsedKeys, out string? parseError) || parsedKeys is null)
                {
                    error = new RpcError
                    {
                        Kind = RpcErrorKinds.InvalidArgument,
                        Message = "Failed to parse keys",
                        Details = new Dictionary<string, string>(StringComparer.Ordinal)
                        {
                            ["error"] = parseError ?? string.Empty,
                        },
                    };
                    context.MarkFailure(step, error);
                    return false;
                }

                switch (parsedKeys.Kind)
                {
                    case ParsedSendKeysKinds.Text:
                        Keyboard.Type(parsedKeys.Text ?? string.Empty);
                        break;
                    case ParsedSendKeysKinds.Key:
                        if (parsedKeys.Key is null)
                        {
                            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Key is missing" };
                            context.MarkFailure(step, error);
                            return false;
                        }

                        Keyboard.Press(parsedKeys.Key.Value);
                        Keyboard.Release(parsedKeys.Key.Value);
                        break;
                    case ParsedSendKeysKinds.Chord:
                        if (parsedKeys.Key is null)
                        {
                            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Chord key is missing" };
                            context.MarkFailure(step, error);
                            return false;
                        }

                        foreach (VirtualKeyShort m in parsedKeys.Modifiers)
                        {
                            Keyboard.Press(m);
                        }

                        Keyboard.Press(parsedKeys.Key.Value);
                        Keyboard.Release(parsedKeys.Key.Value);

                        for (int i = parsedKeys.Modifiers.Length - 1; i >= 0; i--)
                        {
                            Keyboard.Release(parsedKeys.Modifiers[i]);
                        }

                        break;
                    default:
                        error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Unsupported parsed keys kind" };
                        context.MarkFailure(step, error);
                        return false;
                }

                context.MarkSuccess(step);
                error = null;
                return true;
            }
            catch (Exception ex)
            {
                error = CreateError(RpcErrorKinds.ActionError, "SendKeys failed", ex);
                context.MarkFailure(step, error);
                return false;
            }
        }

        if (actionKind == StepActionWaitUntil)
        {
            if (stepAction.Condition is null)
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "WaitUntil action requires condition" };
                context.MarkFailure(step, error);
                return false;
            }

            step.Parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
            step.Parameters["kind"] = stepAction.Condition.Kind ?? string.Empty;
            step.Parameters["timeoutMs"] = waitTimeoutMs.ToString();
            AddRootParameters(step.Parameters, searchRootKind);

            try
            {
                AutomationElement desktop = context.Session.Automation.GetDesktop();
                bool ok = Waiter.PollUntil(
                    predicate: () => EvaluateWaitCondition(mainWindow, desktop, context.Session.Automation, stepAction.Condition, searchRootKind),
                    timeout: TimeSpan.FromMilliseconds(waitTimeoutMs),
                    interval: DefaultPollInterval);

                if (!ok)
                {
                    error = new RpcError { Kind = RpcErrorKinds.TimeoutError, Message = "WaitUntil timed out" };
                    context.MarkFailure(step, error);
                    return false;
                }

                context.MarkSuccess(step);
                error = null;
                return true;
            }
            catch (Exception ex)
            {
                error = CreateError(RpcErrorKinds.ActionError, "WaitUntil failed", ex);
                context.MarkFailure(step, error);
                return false;
            }
        }

        ElementSelector? selector = stepAction.Selector;
        if (selector is null || selector.Path.Count == 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Selector must be provided for action" };
            context.MarkFailure(step, error);
            return false;
        }

        if (!TryFindElementWithSearchRoot(
                mainWindow,
                searchRoot,
                context.Session.Automation,
                selector,
                findTimeoutMs,
                searchRootKind,
                out AutomationElement? element,
                out string? failureKind,
                out Dictionary<string, string>? details) ||
            element is null)
        {
            error = MapFindFailureToError(failureKind, details);
            context.MarkFailure(step, error);
            return false;
        }

        step.Parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
        step.Parameters["timeoutMs"] = findTimeoutMs.ToString();
        AddRootParameters(step.Parameters, searchRootKind);

        try
        {
            switch (actionKind)
            {
                case StepActionClick:
                    element.Click();
                    break;
                case StepActionDoubleClick:
                    element.DoubleClick();
                    break;
                case StepActionRightClick:
                    element.RightClick();
                    break;
                case StepActionSetText:
                    string text = stepAction.Text ?? string.Empty;
                    string mode = string.IsNullOrWhiteSpace(stepAction.Mode) ? SetTextModes.Replace : stepAction.Mode!;
                    step.Parameters["textLength"] = text.Length.ToString();
                    step.Parameters["mode"] = mode;
                    if (!TrySetTextOnElement(element, text, mode))
                    {
                        error = new RpcError { Kind = RpcErrorKinds.ActionError, Message = "SetText failed" };
                        context.MarkFailure(step, error);
                        return false;
                    }

                    break;
                default:
                    error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Unsupported action" };
                    context.MarkFailure(step, error);
                    return false;
            }

            context.MarkSuccess(step);
            error = null;
            return true;
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.ActionError, "OpenImportDialog step failed", ex);
            context.MarkFailure(step, error);
            return false;
        }
    }

    private static bool TrySetFilePath(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        ParsedImportVariablesArgs parsed,
        string searchRootKind,
        out RpcError? error)
    {
        StepLogEntry step = context.StartStep(
            stepId: "SetFilePath",
            action: "Set file path",
            selector: parsed.PathInputSelector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["textLength"] = parsed.FilePath.Length.ToString(),
                ["mode"] = SetTextModes.Replace,
                ["root"] = searchRootKind,
            });

        AddRootParameters(step.Parameters, searchRootKind);

        if (!TryFindElementWithSearchRoot(
                mainWindow,
                searchRoot,
                context.Session.Automation,
                parsed.PathInputSelector,
                parsed.FindTimeoutMs,
                searchRootKind,
                out AutomationElement? element,
                out string? failureKind,
                out Dictionary<string, string>? details) ||
            element is null)
        {
            error = MapFindFailureToError(failureKind, details);
            context.MarkFailure(step, error);
            return false;
        }

        try
        {
            if (!TrySetTextOnElement(element, parsed.FilePath, SetTextModes.Replace))
            {
                error = new RpcError { Kind = RpcErrorKinds.ActionError, Message = "Set file path failed" };
                context.MarkFailure(step, error);
                return false;
            }

            context.MarkSuccess(step);
            error = null;
            return true;
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.ActionError, "Set file path failed", ex);
            context.MarkFailure(step, error);
            return false;
        }
    }

    private static bool TryConfirmImport(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        ParsedImportVariablesArgs parsed,
        string searchRootKind,
        out RpcError? error)
    {
        StepLogEntry step = context.StartStep(
            stepId: "ConfirmImport",
            action: "Confirm import",
            selector: parsed.ConfirmSelector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["timeoutMs"] = parsed.FindTimeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        AddRootParameters(step.Parameters, searchRootKind);

        if (!TryFindElementWithSearchRoot(
                mainWindow,
                searchRoot,
                context.Session.Automation,
                parsed.ConfirmSelector,
                parsed.FindTimeoutMs,
                searchRootKind,
                out AutomationElement? element,
                out string? failureKind,
                out Dictionary<string, string>? details) ||
            element is null)
        {
            error = MapFindFailureToError(failureKind, details);
            context.MarkFailure(step, error);
            return false;
        }

        try
        {
            element.Click();
            context.MarkSuccess(step);
            error = null;
            return true;
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.ActionError, "Failed to confirm import", ex);
            context.MarkFailure(step, error);
            return false;
        }
    }

    private static bool TrySetTextOnElement(AutomationElement element, string text, string mode)
    {
        var valuePattern = element.Patterns.Value.PatternOrDefault;
        if (valuePattern is not null && string.Equals(mode, SetTextModes.Replace, StringComparison.OrdinalIgnoreCase))
        {
            valuePattern.SetValue(text);
            return true;
        }

        element.Focus();

        if (string.Equals(mode, SetTextModes.CtrlAReplace, StringComparison.OrdinalIgnoreCase))
        {
            Keyboard.Press((VirtualKeyShort)0x11); // CTRL
            Keyboard.Type((VirtualKeyShort)0x41); // A
            Keyboard.Release((VirtualKeyShort)0x11);
        }

        Keyboard.Type(text);
        return true;
    }

    private static bool TryFindElementWithFallbackRoots(
        Window mainWindow,
        FlaUI.UIA3.UIA3Automation automation,
        ElementSelector selector,
        int findTimeoutMs,
        out AutomationElement? element,
        out string? failureKind,
        out Dictionary<string, string>? details)
    {
        element = null;
        failureKind = null;
        details = null;

        TimeSpan timeout = TimeSpan.FromMilliseconds(findTimeoutMs);
        DateTimeOffset deadline = DateTimeOffset.UtcNow.Add(timeout);
        DateTimeOffset split = DateTimeOffset.UtcNow.Add(TimeSpan.FromMilliseconds(Math.Max(200, findTimeoutMs / 2)));

        while (DateTimeOffset.UtcNow <= split)
        {
            bool ok = ElementFinder.TryFind(mainWindow, automation, selector, out AutomationElement? found, out string? fk, out Dictionary<string, string>? det);
            if (ok)
            {
                element = found;
                return true;
            }

            failureKind = fk;
            details = det;
            Thread.Sleep(DefaultPollInterval);
        }

        AutomationElement desktop = automation.GetDesktop();
        while (DateTimeOffset.UtcNow <= deadline)
        {
            bool ok = ElementFinder.TryFind(desktop, automation, selector, out AutomationElement? found, out string? fk, out Dictionary<string, string>? det);
            if (ok)
            {
                element = found;
                return true;
            }

            failureKind = fk;
            details = det;
            Thread.Sleep(DefaultPollInterval);
        }

        return false;
    }

    private static bool TryFindElementWithSearchRoot(
        Window mainWindow,
        AutomationElement searchRoot,
        FlaUI.UIA3.UIA3Automation automation,
        ElementSelector selector,
        int findTimeoutMs,
        string searchRootKind,
        out AutomationElement? element,
        out string? failureKind,
        out Dictionary<string, string>? details)
    {
        if (string.Equals(searchRootKind, SearchRootDesktop, StringComparison.OrdinalIgnoreCase))
        {
            return TryFindWithinRoot(searchRoot, automation, selector, findTimeoutMs, out element, out failureKind, out details);
        }

        return TryFindElementWithFallbackRoots(mainWindow, automation, selector, findTimeoutMs, out element, out failureKind, out details);
    }

    private static bool TryFindWithinRoot(
        AutomationElement root,
        FlaUI.UIA3.UIA3Automation automation,
        ElementSelector selector,
        int findTimeoutMs,
        out AutomationElement? element,
        out string? failureKind,
        out Dictionary<string, string>? details)
    {
        element = null;
        failureKind = null;
        details = null;

        TimeSpan timeout = TimeSpan.FromMilliseconds(findTimeoutMs);
        DateTimeOffset deadline = DateTimeOffset.UtcNow.Add(timeout);

        while (DateTimeOffset.UtcNow <= deadline)
        {
            bool ok = ElementFinder.TryFind(root, automation, selector, out AutomationElement? found, out string? fk, out Dictionary<string, string>? det);
            if (ok)
            {
                element = found;
                return true;
            }

            failureKind = fk;
            details = det;
            Thread.Sleep(DefaultPollInterval);
        }

        return false;
    }

    private static string? NormalizeActionKind(string? action)
    {
        if (string.IsNullOrWhiteSpace(action))
        {
            return null;
        }

        if (string.Equals(action, StepActionClick, StringComparison.OrdinalIgnoreCase))
        {
            return StepActionClick;
        }

        if (string.Equals(action, StepActionDoubleClick, StringComparison.OrdinalIgnoreCase))
        {
            return StepActionDoubleClick;
        }

        if (string.Equals(action, StepActionRightClick, StringComparison.OrdinalIgnoreCase))
        {
            return StepActionRightClick;
        }

        if (string.Equals(action, StepActionSetText, StringComparison.OrdinalIgnoreCase))
        {
            return StepActionSetText;
        }

        if (string.Equals(action, StepActionSendKeys, StringComparison.OrdinalIgnoreCase))
        {
            return StepActionSendKeys;
        }

        if (string.Equals(action, StepActionWaitUntil, StringComparison.OrdinalIgnoreCase))
        {
            return StepActionWaitUntil;
        }

        return null;
    }

    private static string? NormalizeSearchRoot(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return SearchRootMainWindow;
        }

        string trimmed = value.Trim();
        if (string.Equals(trimmed, SearchRootMainWindow, StringComparison.OrdinalIgnoreCase))
        {
            return SearchRootMainWindow;
        }

        if (string.Equals(trimmed, SearchRootDesktop, StringComparison.OrdinalIgnoreCase))
        {
            return SearchRootDesktop;
        }

        return null;
    }

    private static string? NormalizePopupSearchRoot(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return SearchRootDesktop;
        }

        string trimmed = value.Trim();
        if (string.Equals(trimmed, SearchRootMainWindow, StringComparison.OrdinalIgnoreCase))
        {
            return SearchRootMainWindow;
        }

        if (string.Equals(trimmed, SearchRootDesktop, StringComparison.OrdinalIgnoreCase))
        {
            return SearchRootDesktop;
        }

        return null;
    }

    private static AutomationElement ResolveSearchRoot(
        string searchRoot,
        Window mainWindow,
        FlaUI.UIA3.UIA3Automation automation,
        out string normalizedRoot)
    {
        if (string.Equals(searchRoot, SearchRootDesktop, StringComparison.OrdinalIgnoreCase))
        {
            normalizedRoot = SearchRootDesktop;
            return automation.GetDesktop();
        }

        normalizedRoot = SearchRootMainWindow;
        return mainWindow;
    }

    private static void AddRootParameters(Dictionary<string, string>? parameters, string searchRootKind)
    {
        if (parameters is null)
        {
            return;
        }

        parameters["root"] = searchRootKind;
        if (string.Equals(searchRootKind, SearchRootMainWindow, StringComparison.OrdinalIgnoreCase))
        {
            parameters["roots"] = "MainWindow->Desktop";
        }
    }
    private static RpcResult<RunFlowResponse>? WaitUntil(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        string searchRootKind,
        string stepId,
        string action,
        WaitCondition condition,
        int timeoutMs)
    {
        StepLogEntry step = context.StartStep(
            stepId: stepId,
            action: action,
            selector: condition.Selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["kind"] = condition.Kind,
                ["timeoutMs"] = timeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        AddRootParameters(step.Parameters, searchRootKind);

        try
        {
            AutomationElement desktop = context.Session.Automation.GetDesktop();
            bool ok = Waiter.PollUntil(
                predicate: () => EvaluateWaitCondition(mainWindow, desktop, context.Session.Automation, condition, searchRootKind),
                timeout: TimeSpan.FromMilliseconds(timeoutMs),
                interval: DefaultPollInterval);

            if (!ok)
            {
                var err = new RpcError { Kind = RpcErrorKinds.TimeoutError, Message = "WaitUntil timed out" };
                context.MarkFailure(step, err);
                return new RpcResult<RunFlowResponse> { Ok = false, Error = err, StepLog = context.StepLog };
            }

            context.MarkSuccess(step);
            return null;
        }
        catch (Exception ex)
        {
            RpcError err = CreateError(RpcErrorKinds.ActionError, "WaitUntil failed", ex);
            context.MarkFailure(step, err);
            return new RpcResult<RunFlowResponse> { Ok = false, Error = err, StepLog = context.StepLog };
        }
    }

    private static bool EvaluateWaitCondition(
        AutomationElement mainWindow,
        AutomationElement desktop,
        FlaUI.UIA3.UIA3Automation automation,
        WaitCondition condition,
        string searchRootKind)
    {
        string kind = condition.Kind ?? WaitConditionKinds.ElementExists;
        ElementSelector? selector = condition.Selector;
        bool useDesktopOnly = string.Equals(searchRootKind, SearchRootDesktop, StringComparison.OrdinalIgnoreCase);

        if (kind == WaitConditionKinds.ElementExists)
        {
            if (selector is null)
            {
                return false;
            }

            if (useDesktopOnly)
            {
                return ElementFinder.TryFind(desktop, automation, selector, out _, out _, out _);
            }

            return ElementFinder.TryFind(mainWindow, automation, selector, out _, out _, out _) ||
                ElementFinder.TryFind(desktop, automation, selector, out _, out _, out _);
        }

        if (kind == WaitConditionKinds.ElementNotExists)
        {
            if (selector is null)
            {
                return true;
            }

            if (useDesktopOnly)
            {
                return !ElementFinder.TryFind(desktop, automation, selector, out _, out _, out _);
            }

            return !ElementFinder.TryFind(mainWindow, automation, selector, out _, out _, out _) &&
                !ElementFinder.TryFind(desktop, automation, selector, out _, out _, out _);
        }

        if (kind == WaitConditionKinds.ElementEnabled)
        {
            if (selector is null)
            {
                return false;
            }

            if (useDesktopOnly)
            {
                if (ElementFinder.TryFind(desktop, automation, selector, out AutomationElement? e, out _, out _) && e is not null)
                {
                    return e.Properties.IsEnabled.ValueOrDefault;
                }

                return false;
            }

            if (ElementFinder.TryFind(mainWindow, automation, selector, out AutomationElement? eMain, out _, out _) && eMain is not null)
            {
                return eMain.Properties.IsEnabled.ValueOrDefault;
            }

            if (ElementFinder.TryFind(desktop, automation, selector, out AutomationElement? e2, out _, out _) && e2 is not null)
            {
                return e2.Properties.IsEnabled.ValueOrDefault;
            }

            return false;
        }

        return false;
    }

    private static RpcResult<RunFlowResponse> Fail(RpcResult<RunFlowResponse> result, FlowContext context, StepLogEntry step, string kind, string message)
    {
        var err = new RpcError { Kind = kind, Message = message };
        context.MarkFailure(step, err);
        result.Ok = false;
        result.Error = err;
        return result;
    }

    private static RpcResult<RunFlowResponse> Fail(RpcResult<RunFlowResponse> result, FlowContext context, StepLogEntry step, string kind, string message, Exception ex)
    {
        RpcError err = CreateError(kind, message, ex);
        context.MarkFailure(step, err);
        result.Ok = false;
        result.Error = err;
        return result;
    }

    private static RpcResult<RunFlowResponse> Fail(RpcResult<RunFlowResponse> result, FlowContext context, StepLogEntry step, RpcError err)
    {
        context.MarkFailure(step, err);
        result.Ok = false;
        result.Error = err;
        return result;
    }

    private static RpcError CreateError(string kind, string message, Exception ex)
    {
        return new RpcError
        {
            Kind = kind,
            Message = message,
            Details = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["exceptionType"] = ex.GetType().FullName ?? ex.GetType().Name,
                ["exceptionMessage"] = ex.Message,
            },
        };
    }

    private static RpcError MapFindFailureToError(string? failureKind, Dictionary<string, string>? details)
    {
        var d = details is null
            ? null
            : new Dictionary<string, string>(details, StringComparer.Ordinal)
            {
                ["failureKind"] = failureKind ?? string.Empty,
            };

        switch (failureKind)
        {
            case FinderFailureKinds.InvalidSelector:
                return new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Invalid selector", Details = d };
            case FinderFailureKinds.InvalidControlType:
                return new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Invalid control type", Details = d };
            case FinderFailureKinds.IndexOutOfRange:
                return new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Selector index out of range", Details = d };
            case FinderFailureKinds.Ambiguous:
                return new RpcError { Kind = RpcErrorKinds.FindError, Message = "Selector matched multiple elements", Details = d };
            case FinderFailureKinds.NotFound:
            default:
                return new RpcError { Kind = RpcErrorKinds.FindError, Message = "Element not found", Details = d };
        }
    }

    internal static bool TryParseArgs(JsonElement? args, out ParsedImportVariablesArgs parsed, out RpcError? error)
    {
        parsed = null!;
        error = null;

        if (args is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Args must be provided" };
            return false;
        }

        if (args.Value.ValueKind is JsonValueKind.Undefined or JsonValueKind.Null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Args must be provided" };
            return false;
        }

        ImportVariablesArgs? raw;
        try
        {
            raw = args.Value.Deserialize<ImportVariablesArgs>(JsonOptions);
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.InvalidArgument, "Failed to parse args", ex);
            return false;
        }

        if (raw is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Args must not be null" };
            return false;
        }

        string filePath = !string.IsNullOrWhiteSpace(raw.FilePath) ? raw.FilePath : raw.Path ?? string.Empty;
        if (string.IsNullOrWhiteSpace(filePath))
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "FilePath must be provided" };
            return false;
        }

        ElementSelector? pathSelector = raw.FilePathEditorSelector ?? raw.PathInputSelector;
        if (pathSelector is null || pathSelector.Path.Count == 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "FilePathEditorSelector must be provided" };
            return false;
        }

        ElementSelector? confirmSelector = raw.ConfirmButtonSelector ?? raw.ConfirmSelector;
        if (confirmSelector is null || confirmSelector.Path.Count == 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "ConfirmButtonSelector must be provided" };
            return false;
        }

        ElementSelector? dialogSelector = raw.DialogSelector is not null && raw.DialogSelector.Path.Count > 0 ? raw.DialogSelector : null;
        WaitCondition? completedCondition = raw.SuccessCondition ?? raw.CompletedCondition;
        if (completedCondition is null && dialogSelector is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "SuccessCondition or DialogSelector must be provided for completion wait" };
            return false;
        }

        int findTimeoutMs = raw.FindTimeoutMs > 0 ? raw.FindTimeoutMs : 10_000;
        int waitTimeoutMs = raw.WaitTimeoutMs > 0 ? raw.WaitTimeoutMs : 30_000;
        string? normalizedRoot = NormalizeSearchRoot(raw.SearchRoot);
        if (normalizedRoot is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "SearchRoot must be mainWindow/desktop" };
            return false;
        }

        string? popupRoot = NormalizePopupSearchRoot(raw.PopupSearchRoot);
        if (popupRoot is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "PopupSearchRoot must be mainWindow/desktop" };
            return false;
        }

        if (raw.PopupTimeoutMs <= 0)
        {
            raw.PopupTimeoutMs = 1500;
        }

        PopupHandlingOptions? popupHandling = null;
        if (raw.EnablePopupHandling)
        {
            if (raw.PopupDialogSelector is null || raw.PopupDialogSelector.Path is null || raw.PopupDialogSelector.Path.Count == 0)
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "PopupDialogSelector must be provided when popup handling is enabled" };
                return false;
            }

            bool hasCancel = raw.PopupCancelButtonSelector is not null && raw.PopupCancelButtonSelector.Path is not null && raw.PopupCancelButtonSelector.Path.Count > 0;
            bool hasOk = raw.PopupOkButtonSelector is not null && raw.PopupOkButtonSelector.Path is not null && raw.PopupOkButtonSelector.Path.Count > 0;

            if (!hasCancel && !(raw.AllowPopupOk && hasOk))
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "PopupCancelButtonSelector is required unless AllowPopupOk is true and PopupOkButtonSelector is provided" };
                return false;
            }

            popupHandling = new PopupHandlingOptions
            {
                Enabled = true,
                SearchRoot = popupRoot,
                TimeoutMs = raw.PopupTimeoutMs,
                AllowOk = raw.AllowPopupOk,
                DialogSelector = raw.PopupDialogSelector,
                OkButtonSelector = raw.PopupOkButtonSelector,
                CancelButtonSelector = raw.PopupCancelButtonSelector,
            };
        }

        if (raw.OpenImportDialogSteps is { Count: > 0 })
        {
            foreach (ImportDialogStepAction step in raw.OpenImportDialogSteps)
            {
                string? actionKind = NormalizeActionKind(step.Action);
                if (actionKind is null)
                {
                    error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "OpenImportDialogSteps action must be provided" };
                    return false;
                }

                if (actionKind == StepActionSendKeys && string.IsNullOrWhiteSpace(step.Keys))
                {
                    error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "SendKeys action requires keys" };
                    return false;
                }

                if (actionKind == StepActionWaitUntil && step.Condition is null)
                {
                    error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "WaitUntil action requires condition" };
                    return false;
                }

                if (actionKind != StepActionSendKeys && actionKind != StepActionWaitUntil)
                {
                    if (step.Selector is null || step.Selector.Path.Count == 0)
                    {
                        error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Selector must be provided for action" };
                        return false;
                    }
                }
            }
        }

        parsed = new ParsedImportVariablesArgs
        {
            FilePath = filePath,
            OpenImportDialogSteps = raw.OpenImportDialogSteps,
            OpenImportSelector = raw.OpenImportSelector,
            DialogSelector = dialogSelector,
            PathInputSelector = pathSelector,
            ConfirmSelector = confirmSelector,
            CompletedCondition = completedCondition,
            FindTimeoutMs = findTimeoutMs,
            WaitTimeoutMs = waitTimeoutMs,
            SearchRoot = normalizedRoot,
            PopupHandling = popupHandling,
        };

        error = null;
        return true;
    }

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };
    internal sealed class ImportVariablesArgs
    {
        public string? FilePath { get; set; }

        public string? Path { get; set; }

        public string? SearchRoot { get; set; }

        public List<ImportDialogStepAction>? OpenImportDialogSteps { get; set; }

        public ElementSelector? OpenImportSelector { get; set; }

        public ElementSelector? DialogSelector { get; set; }

        public ElementSelector? FilePathEditorSelector { get; set; }

        public ElementSelector? PathInputSelector { get; set; }

        public ElementSelector? ConfirmButtonSelector { get; set; }

        public ElementSelector? ConfirmSelector { get; set; }

        public WaitCondition? SuccessCondition { get; set; }

        public WaitCondition? CompletedCondition { get; set; }

        public int FindTimeoutMs { get; set; } = 10_000;

        public int WaitTimeoutMs { get; set; } = 30_000;

        public bool EnablePopupHandling { get; set; }

        public string? PopupSearchRoot { get; set; }

        public int PopupTimeoutMs { get; set; } = 1500;

        public bool AllowPopupOk { get; set; }

        public ElementSelector? PopupDialogSelector { get; set; }

        public ElementSelector? PopupOkButtonSelector { get; set; }

        public ElementSelector? PopupCancelButtonSelector { get; set; }
    }

    internal sealed class ParsedImportVariablesArgs
    {
        public string FilePath { get; set; } = string.Empty;

        public List<ImportDialogStepAction>? OpenImportDialogSteps { get; set; }

        public ElementSelector? OpenImportSelector { get; set; }

        public ElementSelector? DialogSelector { get; set; }

        public ElementSelector PathInputSelector { get; set; } = new();

        public ElementSelector ConfirmSelector { get; set; } = new();

        public WaitCondition? CompletedCondition { get; set; }

        public int FindTimeoutMs { get; set; } = 10_000;

        public int WaitTimeoutMs { get; set; } = 30_000;

        public string SearchRoot { get; set; } = SearchRootMainWindow;

        public PopupHandlingOptions? PopupHandling { get; set; }
    }

    internal sealed class ImportDialogStepAction
    {
        public string? Action { get; set; }

        public ElementSelector? Selector { get; set; }

        public string? Text { get; set; }

        public string? Mode { get; set; }

        public string? Keys { get; set; }

        public WaitCondition? Condition { get; set; }

        public int? TimeoutMs { get; set; }
    }
}
