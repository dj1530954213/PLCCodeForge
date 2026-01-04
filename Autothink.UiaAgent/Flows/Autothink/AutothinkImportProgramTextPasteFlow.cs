using System.Text.Json;
using Autothink.UiaAgent.Flows;
using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Uia;
using FlaUI.Core.AutomationElements;
using FlaUI.Core.Input;
using FlaUI.Core.WindowsAPI;

namespace Autothink.UiaAgent.Flows.Autothink;

internal sealed class AutothinkImportProgramTextPasteFlow : IFlow
{
    private static readonly TimeSpan DefaultPollInterval = TimeSpan.FromMilliseconds(200);
    private const string StepActionClick = "Click";
    private const string StepActionDoubleClick = "DoubleClick";
    private const string StepActionRightClick = "RightClick";
    private const string StepActionSetText = "SetText";
    private const string StepActionSendKeys = "SendKeys";
    private const string StepActionWaitUntil = "WaitUntil";
    private const string VerifyModeNone = "none";
    private const string VerifyModeEditorNotEmpty = "editorNotEmpty";
    private const string VerifyModeElementExists = "elementExists";
    private const string SearchRootMainWindow = "mainWindow";
    private const string SearchRootDesktop = "desktop";
    private const string ClipboardHealthCheckText = "CLIPBOARD_HEALTHCHECK";

    public string Name => FlowNames.AutothinkImportProgramTextPaste;

    public bool IsImplemented => true;

    public RpcResult<RunFlowResponse> Run(FlowContext context, JsonElement? args)
    {
        ArgumentNullException.ThrowIfNull(context);

        var result = new RpcResult<RunFlowResponse> { StepLog = context.StepLog };

        StepLogEntry validateStep = context.StartStep(stepId: "ValidateArgs", action: "Validate args");

        if (args is null)
        {
            return Fail(result, context, validateStep, RpcErrorKinds.InvalidArgument, "Args must be provided");
        }

        if (!TryParseArgs(args, out AutothinkImportProgramTextPasteArgs parsed, out RpcError? parseError))
        {
            return Fail(result, context, validateStep, parseError ?? new RpcError
            {
                Kind = RpcErrorKinds.InvalidArgument,
                Message = "Invalid args",
            });
        }

        context.MarkSuccess(validateStep);

        // Ensure main window exists and is foregrounded (best-effort).
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
            // Non-fatal.
            context.MarkWarning(bringToForegroundStep, CreateError(RpcErrorKinds.ActionError, "Failed to bring window to foreground", ex));
        }

        AutomationElement searchRoot = ResolveSearchRoot(parsed.SearchRoot!, mainWindow, context.Session.Automation, out string searchRootKind);
        PopupHandlingOptions? popupHandling = BuildPopupHandling(parsed);
        PopupHandling.TryHandle(context, mainWindow, popupHandling, "BeforePaste");

        if (!TryOpenProgramEntry(context, mainWindow, searchRoot, parsed, searchRootKind, out RpcError? openError))
        {
            result.Ok = false;
            result.Error = openError;
            return result;
        }

        if (parsed.EditorRootSelector is not null && parsed.EditorRootSelector.Path is not null && parsed.EditorRootSelector.Path.Count > 0)
        {
            RpcResult<RunFlowResponse>? waitEditorRoot = WaitUntil(
                context,
                mainWindow,
                searchRoot,
                searchRootKind,
                stepId: "WaitEditorRoot",
                action: "Wait editor root",
                condition: new WaitCondition
                {
                    Kind = WaitConditionKinds.ElementExists,
                    Selector = parsed.EditorRootSelector,
                },
                timeoutMs: parsed.FindTimeoutMs);

            if (waitEditorRoot is not null)
            {
                return waitEditorRoot;
            }
        }

        // Find editor (wait until timeout).
        StepLogEntry findEditorStep = context.StartStep(
            stepId: "FindEditor",
            action: "Find editor element",
            selector: parsed.EditorSelector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["timeoutMs"] = parsed.FindTimeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        AutomationElement? editor = null;
        string? lastFailure = null;
        Dictionary<string, string>? lastDetails = null;

        TimeSpan findTimeout = TimeSpan.FromMilliseconds(parsed.FindTimeoutMs > 0 ? parsed.FindTimeoutMs : (int)context.Timeout.TotalMilliseconds);

        bool found = Waiter.PollUntil(
            predicate: () =>
            {
                bool ok = ElementFinder.TryFind(searchRoot, context.Session.Automation, parsed.EditorSelector!, out AutomationElement? e, out string? fk, out Dictionary<string, string>? det);
                if (ok)
                {
                    editor = e;
                    return true;
                }

                lastFailure = fk;
                lastDetails = det;
                return false;
            },
            timeout: findTimeout,
            interval: DefaultPollInterval);

        if (!found || editor is null)
        {
            RpcError err = MapFindFailureToError(lastFailure, lastDetails);
            return Fail(result, context, findEditorStep, err);
        }

        context.MarkSuccess(findEditorStep);

        // Focus editor (required for CTRL+V).
        StepLogEntry focusStep = context.StartStep(stepId: "FocusEditor", action: "Focus editor", selector: parsed.EditorSelector);
        try
        {
            editor.Focus();
            context.MarkSuccess(focusStep);
        }
        catch (Exception ex)
        {
            return Fail(result, context, focusStep, RpcErrorKinds.ActionError, "Failed to focus editor", ex);
        }

        bool fallbackUsed = false;
        bool preferClipboard = parsed.PreferClipboard;

        if (preferClipboard && parsed.ClipboardHealthCheck)
        {
            ClipboardOperationResult healthCheck = TrySetClipboardWithRetry(
                context,
                stepId: "ClipboardHealthCheck",
                action: "Clipboard health check",
                text: ClipboardHealthCheckText,
                retry: parsed.ClipboardRetry!,
                warnOnFailure: true,
                out _);

            if (!healthCheck.Ok && parsed.ForceFallbackOnClipboardFailure)
            {
                preferClipboard = false;
            }
        }

        if (preferClipboard)
        {
            ClipboardOperationResult clipboardResult = TrySetClipboardWithRetry(
                context,
                stepId: "SetClipboardText",
                action: "SetClipboardText",
                text: parsed.ProgramText,
                retry: parsed.ClipboardRetry!,
                warnOnFailure: parsed.FallbackToType,
                out RpcError? clipboardError);

            if (!clipboardResult.Ok)
            {
                if (!parsed.FallbackToType)
                {
                    result.Ok = false;
                    result.Error = clipboardError;
                    return result;
                }

                StepLogEntry pasteStep = context.StartStep(stepId: "SendKeysPaste", action: "SendKeys CTRL+V");
                context.MarkWarning(pasteStep, new RpcError
                {
                    Kind = RpcErrorKinds.ActionError,
                    Message = "Clipboard unavailable; skipped CTRL+V",
                });

                if (!TryTypeFallback(context, parsed.ProgramText, parsed.TypeChunkSize, parsed.TypeChunkDelayMs, out RpcError? typeError))
                {
                    result.Ok = false;
                    result.Error = typeError;
                    return result;
                }

                fallbackUsed = true;
            }
            else
            {
                StepLogEntry pasteStep = context.StartStep(stepId: "SendKeysPaste", action: "SendKeys CTRL+V");
                try
                {
                    Keyboard.Press((VirtualKeyShort)0x11); // CTRL
                    Keyboard.Type((VirtualKeyShort)0x56); // V
                    Keyboard.Release((VirtualKeyShort)0x11);
                    context.MarkSuccess(pasteStep);
                }
                catch (Exception ex)
                {
                    if (!parsed.FallbackToType)
                    {
                        return Fail(result, context, pasteStep, RpcErrorKinds.ActionError, "CTRL+V paste failed", ex);
                    }

                    context.MarkWarning(pasteStep, CreateError(RpcErrorKinds.ActionError, "CTRL+V paste failed, will fallback to Keyboard.Type", ex));
                    if (!TryTypeFallback(context, parsed.ProgramText, parsed.TypeChunkSize, parsed.TypeChunkDelayMs, out RpcError? typeError))
                    {
                        result.Ok = false;
                        result.Error = typeError;
                        return result;
                    }

                    fallbackUsed = true;
                }
            }
        }
        else
        {
            StepLogEntry pasteStep = context.StartStep(stepId: "SendKeysPaste", action: "SendKeys CTRL+V");
            context.MarkWarning(pasteStep, new RpcError
            {
                Kind = RpcErrorKinds.ActionError,
                Message = "CTRL+V skipped (preferClipboard=false)",
            });

            if (!parsed.FallbackToType)
            {
                return Fail(result, context, pasteStep, RpcErrorKinds.InvalidArgument, "preferClipboard=false requires fallbackToType=true");
            }

            if (!TryTypeFallback(context, parsed.ProgramText, parsed.TypeChunkSize, parsed.TypeChunkDelayMs, out RpcError? typeError))
            {
                result.Ok = false;
                result.Error = typeError;
                return result;
            }

            fallbackUsed = true;
        }

        // Wait after paste (default 1000ms; allow 0 to skip).
        if (parsed.AfterPasteWaitMs is > 0)
        {
            StepLogEntry waitStep = context.StartStep(
                stepId: "WaitAfterPaste",
                action: "Wait after paste",
                parameters: new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["waitMs"] = parsed.AfterPasteWaitMs.Value.ToString(),
                    ["method"] = fallbackUsed ? "fallback" : "ctrlV",
                });

            Thread.Sleep(parsed.AfterPasteWaitMs.Value);
            context.MarkSuccess(waitStep);
        }

        PopupHandling.TryHandle(context, mainWindow, popupHandling, "AfterPaste");

        if (!TryVerifyPaste(
                context,
                searchRoot,
                editor,
                parsed.EditorSelector!,
                parsed.VerifySelector,
                parsed.VerifyMode!,
                parsed.VerifyTimeoutMs,
                stepId: "VerifyPaste",
                action: "Verify paste result",
                searchRootKind: searchRootKind,
                warnOnFailure: parsed.FallbackToType && !fallbackUsed,
                out RpcError? verifyError))
        {
            if (!parsed.FallbackToType || fallbackUsed || parsed.VerifyMode == VerifyModeNone)
            {
                result.Ok = false;
                result.Error = verifyError;
                return result;
            }

            if (!TryTypeFallback(context, parsed.ProgramText, parsed.TypeChunkSize, parsed.TypeChunkDelayMs, out RpcError? typeError))
            {
                result.Ok = false;
                result.Error = typeError;
                return result;
            }

            fallbackUsed = true;

            if (parsed.AfterPasteWaitMs is > 0)
            {
                StepLogEntry waitAfterFallback = context.StartStep(
                    stepId: "WaitAfterFallback",
                    action: "Wait after fallback typing",
                    parameters: new Dictionary<string, string>(StringComparer.Ordinal)
                    {
                        ["waitMs"] = parsed.AfterPasteWaitMs.Value.ToString(),
                    });

                Thread.Sleep(parsed.AfterPasteWaitMs.Value);
                context.MarkSuccess(waitAfterFallback);
            }

            PopupHandling.TryHandle(context, mainWindow, popupHandling, "AfterFallback");

            if (!TryVerifyPaste(
                    context,
                    searchRoot,
                    editor,
                    parsed.EditorSelector!,
                    parsed.VerifySelector,
                    parsed.VerifyMode!,
                    Math.Min(parsed.VerifyTimeoutMs, 3_000),
                    stepId: "VerifyPasteAfterFallback",
                    action: "Verify paste after fallback",
                    searchRootKind: searchRootKind,
                    warnOnFailure: false,
                    out RpcError? verifyAfterError))
            {
                result.Ok = false;
                result.Error = verifyAfterError;
                return result;
            }
        }

        result.Ok = true;
        result.Value = new RunFlowResponse
        {
            Data = JsonSerializer.SerializeToElement(new
            {
                textLength = parsed.ProgramText.Length,
                fallbackUsed,
                verifyMode = parsed.VerifyMode,
                preferClipboard = parsed.PreferClipboard,
            }),
        };
        return result;
    }

    private static PopupHandlingOptions? BuildPopupHandling(AutothinkImportProgramTextPasteArgs parsed)
    {
        if (!parsed.EnablePopupHandling)
        {
            return null;
        }

        return new PopupHandlingOptions
        {
            Enabled = true,
            SearchRoot = parsed.PopupSearchRoot ?? SearchRootDesktop,
            TimeoutMs = parsed.PopupTimeoutMs,
            AllowOk = parsed.AllowPopupOk,
            DialogSelector = parsed.PopupDialogSelector,
            OkButtonSelector = parsed.PopupOkButtonSelector,
            CancelButtonSelector = parsed.PopupCancelButtonSelector,
        };
    }

    private static bool TryOpenProgramEntry(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        AutothinkImportProgramTextPasteArgs parsed,
        string searchRootKind,
        out RpcError? error)
    {
        if (parsed.OpenProgramSteps is { Count: > 0 })
        {
            for (int i = 0; i < parsed.OpenProgramSteps.Count; i++)
            {
                if (!TryExecuteOpenStep(context, mainWindow, searchRoot, parsed, searchRootKind, parsed.OpenProgramSteps[i], i, out error))
                {
                    return false;
                }
            }

            error = null;
            return true;
        }

        StepLogEntry skipped = context.StartStep(stepId: "OpenProgramEntry", action: "Open program entry (skipped)");
        skipped.Parameters = new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["skipped"] = "true",
        };
        context.MarkSuccess(skipped);
        error = null;
        return true;
    }

    private static bool TryExecuteOpenStep(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        AutothinkImportProgramTextPasteArgs parsed,
        string searchRootKind,
        ImportProgramStepAction stepAction,
        int index,
        out RpcError? error)
    {
        string stepId = index == 0 ? "OpenProgramEntry" : $"OpenProgramEntry.{index + 1}";
        string? actionKind = NormalizeActionKind(stepAction.Action);

        StepLogEntry step = context.StartStep(
            stepId: stepId,
            action: "Open program entry step",
            selector: stepAction.Selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["action"] = actionKind ?? string.Empty,
            });

        AddRootParameters(step.Parameters, searchRootKind);

        if (actionKind is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "OpenProgramSteps action must be provided" };
            context.MarkFailure(step, error);
            return false;
        }

        int findTimeoutMs = stepAction.TimeoutMs.GetValueOrDefault(parsed.FindTimeoutMs);
        int waitTimeoutMs = stepAction.TimeoutMs.GetValueOrDefault(parsed.VerifyTimeoutMs);

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
        if (selector is null || selector.Path is null || selector.Path.Count == 0)
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
            error = CreateError(RpcErrorKinds.ActionError, "Open program entry step failed", ex);
            context.MarkFailure(step, error);
            return false;
        }
    }

    private static bool TryTypeFallback(FlowContext context, string text, int chunkSize, int chunkDelayMs, out RpcError? error)
    {
        StepLogEntry step = context.StartStep(
            stepId: "FallbackTypeText",
            action: "Fallback type text via Keyboard.Type",
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["chunkSize"] = chunkSize.ToString(),
                ["chunkDelayMs"] = chunkDelayMs.ToString(),
                ["textLength"] = text.Length.ToString(),
            });

        try
        {
            for (int i = 0; i < text.Length; i += chunkSize)
            {
                string chunk = text.Substring(i, Math.Min(chunkSize, text.Length - i));
                Keyboard.Type(chunk);

                if (chunkDelayMs > 0)
                {
                    Thread.Sleep(chunkDelayMs);
                }
            }

            context.MarkSuccess(step);
            error = null;
            return true;
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.ActionError, "Fallback typing failed", ex);
            context.MarkFailure(step, error);
            return false;
        }
    }

    private static bool TryVerifyPaste(
        FlowContext context,
        AutomationElement searchRoot,
        AutomationElement? editor,
        ElementSelector editorSelector,
        ElementSelector? verifySelector,
        string verifyMode,
        int timeoutMs,
        string stepId,
        string action,
        string searchRootKind,
        bool warnOnFailure,
        out RpcError? error)
    {
        ElementSelector? selectorForLog = verifyMode == VerifyModeElementExists ? verifySelector : editorSelector;

        StepLogEntry step = context.StartStep(
            stepId: stepId,
            action: action,
            selector: selectorForLog,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["mode"] = verifyMode,
                ["timeoutMs"] = timeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        if (verifyMode == VerifyModeNone)
        {
            context.MarkSuccess(step);
            error = null;
            return true;
        }

        string? verificationSource = null;

        try
        {
            bool ok = Waiter.PollUntil(
                predicate: () =>
                {
                    if (verifyMode == VerifyModeElementExists)
                    {
                        if (verifySelector is null || verifySelector.Path.Count == 0)
                        {
                            return false;
                        }

                        return ElementFinder.TryFind(searchRoot, context.Session.Automation, verifySelector, out _, out _, out _);
                    }

                    AutomationElement? target = ResolveEditor(searchRoot, context.Session.Automation, editorSelector, editor);
                    if (target is null)
                    {
                        return false;
                    }

                    if (!target.Properties.IsEnabled.ValueOrDefault)
                    {
                        return false;
                    }

                    if (TryGetEditorText(target, out string text, out string? source))
                    {
                        verificationSource = source ?? "ValuePattern";
                        return text.Length > 0;
                    }

                    verificationSource = "EnabledOnly";
                    return true;
                },
                timeout: TimeSpan.FromMilliseconds(timeoutMs),
                interval: DefaultPollInterval);

            if (!ok)
            {
                error = new RpcError { Kind = RpcErrorKinds.TimeoutError, Message = "Paste verification timed out" };
                if (warnOnFailure)
                {
                    context.MarkWarning(step, error);
                }
                else
                {
                    context.MarkFailure(step, error);
                }

                return false;
            }

            if (!string.IsNullOrWhiteSpace(verificationSource))
            {
                step.Parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
                step.Parameters["verification"] = verificationSource;
            }

            context.MarkSuccess(step);
            error = null;
            return true;
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.ActionError, "Paste verification failed", ex);
            if (warnOnFailure)
            {
                context.MarkWarning(step, error);
            }
            else
            {
                context.MarkFailure(step, error);
            }

            return false;
        }
    }

    private static AutomationElement? ResolveEditor(
        AutomationElement searchRoot,
        FlaUI.UIA3.UIA3Automation automation,
        ElementSelector editorSelector,
        AutomationElement? fallback)
    {
        if (ElementFinder.TryFind(searchRoot, automation, editorSelector, out AutomationElement? found, out _, out _) &&
            found is not null)
        {
            return found;
        }

        return fallback;
    }

    private static bool TryGetEditorText(AutomationElement editor, out string text, out string? source)
    {
        text = string.Empty;
        source = null;

        try
        {
            var value = editor.Patterns.Value.PatternOrDefault;
            if (value is not null)
            {
                text = value.Value.ValueOrDefault ?? string.Empty;
                source = "ValuePattern";
                return true;
            }

            var tp = editor.Patterns.Text.PatternOrDefault;
            if (tp is not null)
            {
                // Limit to avoid large allocations.
                text = tp.DocumentRange.GetText(2048) ?? string.Empty;
                source = "TextPattern";
                return true;
            }
        }
        catch
        {
            // best-effort: treat as unreadable.
        }

        return false;
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

    private static ClipboardOperationResult TrySetClipboardWithRetry(
        FlowContext context,
        string stepId,
        string action,
        string text,
        ClipboardRetryOptions retry,
        bool warnOnFailure,
        out RpcError? error)
    {
        int attempts = Math.Max(1, retry.Times);
        int intervalMs = Math.Max(0, retry.IntervalMs);

        StepLogEntry summary = context.StartStep(
            stepId: stepId,
            action: action,
            selector: null,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["attempts"] = attempts.ToString(),
                ["intervalMs"] = intervalMs.ToString(),
                ["textLength"] = text.Length.ToString(),
            });

        ClipboardOperationResult outcome = new();

        RpcError? lastError = null;
        ClipboardFailureKind? lastFailureKind = null;
        string? lastFailureMessage = null;

        for (int i = 1; i <= attempts; i++)
        {
            StepLogEntry attemptStep = context.StartStep(
                stepId: $"{stepId}.Attempt{i}",
                action: action,
                selector: null,
                parameters: new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["attempt"] = i.ToString(),
                    ["maxAttempts"] = attempts.ToString(),
                    ["intervalMs"] = intervalMs.ToString(),
                    ["textLength"] = text.Length.ToString(),
                });

            ClipboardAttemptResult attempt = ClipboardAdapter.TrySetText(text);
            if (attempt.Ok)
            {
                context.MarkSuccess(attemptStep);
                outcome.Ok = true;
                outcome.Attempts = i;
                context.MarkSuccess(summary);
                error = null;
                return outcome;
            }

            lastFailureKind = attempt.FailureKind;
            lastFailureMessage = attempt.Message;

            if (attemptStep.Parameters is not null)
            {
                attemptStep.Parameters["failureKind"] = attempt.FailureKind?.ToString() ?? string.Empty;
                if (!string.IsNullOrWhiteSpace(attempt.Message))
                {
                    attemptStep.Parameters["message"] = attempt.Message;
                }
            }

            lastError = new RpcError
            {
                Kind = RpcErrorKinds.ActionError,
                Message = $"Clipboard set failed ({attempt.FailureKind})",
                Details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["failureKind"] = attempt.FailureKind?.ToString() ?? string.Empty,
                    ["message"] = attempt.Message ?? string.Empty,
                    ["exceptionType"] = attempt.ExceptionType ?? string.Empty,
                },
            };

            bool finalAttempt = i == attempts;
            if (warnOnFailure || !finalAttempt)
            {
                context.MarkWarning(attemptStep, lastError);
            }
            else
            {
                context.MarkFailure(attemptStep, lastError);
            }

            if (!finalAttempt && intervalMs > 0)
            {
                Thread.Sleep(intervalMs);
            }
        }

        outcome.Ok = false;
        outcome.Attempts = attempts;
        outcome.FailureKind = lastFailureKind?.ToString();
        outcome.FailureMessage = lastFailureMessage;

        if (summary.Parameters is not null && lastFailureKind is not null)
        {
            summary.Parameters["failureKind"] = lastFailureKind.ToString() ?? string.Empty;
        }

        if (lastError is not null)
        {
            if (warnOnFailure)
            {
                context.MarkWarning(summary, lastError);
            }
            else
            {
                context.MarkFailure(summary, lastError);
            }
        }
        else
        {
            context.MarkFailure(summary, new RpcError { Kind = RpcErrorKinds.ActionError, Message = "Clipboard set failed" });
        }

        error = lastError;
        return outcome;
    }

    internal static bool TryParseArgs(JsonElement? args, out AutothinkImportProgramTextPasteArgs parsed, out RpcError? error)
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

        AutothinkImportProgramTextPasteArgs? parsedLocal;
        try
        {
            parsedLocal = args.Value.Deserialize<AutothinkImportProgramTextPasteArgs>(JsonOptions);
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.InvalidArgument, "Failed to parse args", ex);
            return false;
        }

        if (parsedLocal is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Args must not be null" };
            return false;
        }

        parsed = parsedLocal;

        if (string.IsNullOrWhiteSpace(parsed.ProgramText))
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "ProgramText must be provided" };
            return false;
        }

        if (parsed.EditorSelector is null || parsed.EditorSelector.Path is null || parsed.EditorSelector.Path.Count == 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "EditorSelector must be provided" };
            return false;
        }

        if (parsed.EditorRootSelector is not null && (parsed.EditorRootSelector.Path is null || parsed.EditorRootSelector.Path.Count == 0))
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "EditorRootSelector is invalid" };
            return false;
        }

        if (parsed.OpenProgramSteps is { Count: > 0 })
        {
            foreach (ImportProgramStepAction step in parsed.OpenProgramSteps)
            {
                string? actionKind = NormalizeActionKind(step.Action);
                if (actionKind is null)
                {
                    error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "OpenProgramSteps action must be provided" };
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
                    if (step.Selector is null || step.Selector.Path is null || step.Selector.Path.Count == 0)
                    {
                        error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "Selector must be provided for action" };
                        return false;
                    }
                }
            }
        }

        if (parsed.AfterPasteWaitMs is null)
        {
            parsed.AfterPasteWaitMs = 1_000;
        }

        if (parsed.AfterPasteWaitMs < 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "AfterPasteWaitMs must be >= 0" };
            return false;
        }

        string? normalizedVerifyMode = NormalizeVerifyMode(parsed.VerifyMode);
        if (normalizedVerifyMode is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "VerifyMode must be none/editorNotEmpty/elementExists" };
            return false;
        }

        parsed.VerifyMode = normalizedVerifyMode;

        if (normalizedVerifyMode == VerifyModeElementExists &&
            (parsed.VerifySelector is null || parsed.VerifySelector.Path is null || parsed.VerifySelector.Path.Count == 0))
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "VerifySelector must be provided when VerifyMode=elementExists" };
            return false;
        }

        string? normalizedRoot = NormalizeSearchRoot(parsed.SearchRoot);
        if (normalizedRoot is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "SearchRoot must be mainWindow/desktop" };
            return false;
        }

        parsed.SearchRoot = normalizedRoot;

        string? popupRoot = NormalizePopupSearchRoot(parsed.PopupSearchRoot);
        if (popupRoot is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "PopupSearchRoot must be mainWindow/desktop" };
            return false;
        }

        parsed.PopupSearchRoot = popupRoot;

        if (parsed.PopupTimeoutMs <= 0)
        {
            parsed.PopupTimeoutMs = 1500;
        }

        if (parsed.EnablePopupHandling)
        {
            if (parsed.PopupDialogSelector is null || parsed.PopupDialogSelector.Path is null || parsed.PopupDialogSelector.Path.Count == 0)
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "PopupDialogSelector must be provided when popup handling is enabled" };
                return false;
            }

            bool hasCancel = parsed.PopupCancelButtonSelector is not null && parsed.PopupCancelButtonSelector.Path is not null && parsed.PopupCancelButtonSelector.Path.Count > 0;
            bool hasOk = parsed.PopupOkButtonSelector is not null && parsed.PopupOkButtonSelector.Path is not null && parsed.PopupOkButtonSelector.Path.Count > 0;

            if (!hasCancel && !(parsed.AllowPopupOk && hasOk))
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "PopupCancelButtonSelector is required unless AllowPopupOk is true and PopupOkButtonSelector is provided" };
                return false;
            }
        }

        if (parsed.VerifyTimeoutMs <= 0)
        {
            parsed.VerifyTimeoutMs = 5_000;
        }

        if (parsed.ClipboardTimeoutMs <= 0)
        {
            parsed.ClipboardTimeoutMs = 2_000;
        }

        parsed.ClipboardRetry ??= new ClipboardRetryOptions();

        if (parsed.ClipboardRetry.Times <= 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "ClipboardRetry.Times must be >= 1" };
            return false;
        }

        if (parsed.ClipboardRetry.IntervalMs < 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "ClipboardRetry.IntervalMs must be >= 0" };
            return false;
        }

        if (!parsed.PreferClipboard && !parsed.FallbackToType)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "preferClipboard=false requires fallbackToType=true" };
            return false;
        }

        if (parsed.TypeChunkSize <= 0)
        {
            parsed.TypeChunkSize = 128;
        }

        if (parsed.TypeChunkDelayMs < 0)
        {
            parsed.TypeChunkDelayMs = 0;
        }

        if (parsed.FindTimeoutMs <= 0)
        {
            parsed.FindTimeoutMs = 10_000;
        }

        error = null;
        return true;
    }

    private static string? NormalizeVerifyMode(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return VerifyModeEditorNotEmpty;
        }

        string trimmed = value.Trim();
        if (string.Equals(trimmed, VerifyModeNone, StringComparison.OrdinalIgnoreCase))
        {
            return VerifyModeNone;
        }

        if (string.Equals(trimmed, VerifyModeEditorNotEmpty, StringComparison.OrdinalIgnoreCase))
        {
            return VerifyModeEditorNotEmpty;
        }

        if (string.Equals(trimmed, VerifyModeElementExists, StringComparison.OrdinalIgnoreCase))
        {
            return VerifyModeElementExists;
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

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    internal sealed class ImportProgramStepAction
    {
        public string? Action { get; set; }
        public ElementSelector? Selector { get; set; }
        public string? Text { get; set; }
        public string? Mode { get; set; }
        public string? Keys { get; set; }
        public WaitCondition? Condition { get; set; }
        public int? TimeoutMs { get; set; }
    }

    internal sealed class AutothinkImportProgramTextPasteArgs
    {
        public string ProgramText { get; set; } = string.Empty;
        public List<ImportProgramStepAction>? OpenProgramSteps { get; set; }
        public ElementSelector? EditorRootSelector { get; set; }
        public ElementSelector? EditorSelector { get; set; }
        public int? AfterPasteWaitMs { get; set; }
        public string? VerifyMode { get; set; }
        public ElementSelector? VerifySelector { get; set; }
        public string? SearchRoot { get; set; }
        public int FindTimeoutMs { get; set; } = 10_000;
        public int ClipboardTimeoutMs { get; set; } = 2_000;
        public int VerifyTimeoutMs { get; set; } = 5_000;
        public bool FallbackToType { get; set; } = true;
        public bool PreferClipboard { get; set; } = true;
        public ClipboardRetryOptions? ClipboardRetry { get; set; }
        public bool ClipboardHealthCheck { get; set; }
        public bool ForceFallbackOnClipboardFailure { get; set; }
        public int TypeChunkSize { get; set; } = 128;
        public int TypeChunkDelayMs { get; set; }
        public bool EnablePopupHandling { get; set; }
        public string? PopupSearchRoot { get; set; }
        public int PopupTimeoutMs { get; set; } = 1500;
        public bool AllowPopupOk { get; set; }
        public ElementSelector? PopupDialogSelector { get; set; }
        public ElementSelector? PopupOkButtonSelector { get; set; }
        public ElementSelector? PopupCancelButtonSelector { get; set; }
    }

    internal sealed class ClipboardRetryOptions
    {
        public int Times { get; set; } = 3;
        public int IntervalMs { get; set; } = 200;
    }

    private sealed class ClipboardOperationResult
    {
        public bool Ok { get; set; }
        public int Attempts { get; set; }
        public string? FailureKind { get; set; }
        public string? FailureMessage { get; set; }
    }
}
