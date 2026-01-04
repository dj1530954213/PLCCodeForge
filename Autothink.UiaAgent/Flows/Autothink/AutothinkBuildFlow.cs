using System.Linq;
using System.Text.Json;
using Autothink.UiaAgent.Flows;
using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Uia;
using FlaUI.Core.AutomationElements;

namespace Autothink.UiaAgent.Flows.Autothink;

internal sealed class AutothinkBuildFlow : IFlow
{
    private static readonly TimeSpan DefaultPollInterval = TimeSpan.FromMilliseconds(200);
    private const string SearchRootMainWindow = "mainWindow";
    private const string SearchRootDesktop = "desktop";
    private const string BuildOutcomeModeWaitSelector = "waitSelector";
    private const string BuildOutcomeModeReadText = "readTextContains";
    private const string BuildOutcomeModeEither = "either";
    private const string BuildOutcomeSuccess = "Success";
    private const string BuildOutcomeFail = "Fail";
    private const string BuildOutcomeUnknown = "Unknown";

    public string Name => FlowNames.AutothinkBuild;

    public bool IsImplemented => true;

    public RpcResult<RunFlowResponse> Run(FlowContext context, JsonElement? args)
    {
        ArgumentNullException.ThrowIfNull(context);

        var result = new RpcResult<RunFlowResponse> { StepLog = context.StepLog };

        StepLogEntry validateStep = context.StartStep(stepId: "ValidateArgs", action: "Validate args");
        if (!TryParseArgs(args, out ParsedBuildArgs parsed, out RpcError? parseError))
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

        AutomationElement searchRoot = ResolveSearchRoot(parsed.SearchRoot, mainWindow, context.Session.Automation, out string searchRootKind);
        PopupHandlingOptions? popupHandling = BuildPopupHandling(parsed);
        PopupHandling.TryHandle(context, mainWindow, popupHandling, "BeforeBuild");

        if (!TryFindWithSearchRoot(
                context,
                mainWindow,
                searchRoot,
                searchRootKind,
                parsed.BuildButtonSelector,
                stepId: "FindBuildButton",
                action: "Find build button",
                findTimeoutMs: parsed.FindTimeoutMs,
                out AutomationElement? buildTrigger,
                out RpcError? findError) ||
            buildTrigger is null)
        {
            result.Ok = false;
            result.Error = findError;
            return result;
        }

        StepLogEntry clickStep = context.StartStep(stepId: "ClickBuild", action: "Click build button", selector: parsed.BuildButtonSelector);
        try
        {
            buildTrigger.Click();
            context.MarkSuccess(clickStep);
        }
        catch (Exception ex)
        {
            return Fail(result, context, clickStep, RpcErrorKinds.ActionError, "Failed to click build trigger", ex);
        }

        PopupHandling.TryHandle(context, mainWindow, popupHandling, "AfterClickBuild");

        // Wait completion + detect unexpected dialogs (optional).
        StepLogEntry waitStep = context.StartStep(
            stepId: "WaitBuildDone",
            action: "Wait build done",
            selector: parsed.WaitCondition.Selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["kind"] = parsed.WaitCondition.Kind,
                ["timeoutMs"] = parsed.WaitTimeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        AddRootParameters(waitStep.Parameters, searchRootKind);

        try
        {
            AutomationElement desktop = context.Session.Automation.GetDesktop();
            bool ok = Waiter.PollUntil(
                predicate: () =>
                {
                    if (parsed.UnexpectedSelectors is not null)
                    {
                        foreach (ElementSelector sel in parsed.UnexpectedSelectors)
                        {
                            if (sel.Path.Count == 0)
                            {
                                continue;
                            }

                            bool matched = string.Equals(searchRootKind, SearchRootDesktop, StringComparison.OrdinalIgnoreCase)
                                ? ElementFinder.TryFind(desktop, context.Session.Automation, sel, out _, out _, out _)
                                : ElementFinder.TryFind(mainWindow, context.Session.Automation, sel, out _, out _, out _) ||
                                  ElementFinder.TryFind(desktop, context.Session.Automation, sel, out _, out _, out _);

                            if (matched)
                            {
                                return false;
                            }
                        }
                    }

                    return EvaluateWaitCondition(mainWindow, desktop, context.Session.Automation, parsed.WaitCondition, searchRootKind);
                },
                timeout: TimeSpan.FromMilliseconds(parsed.WaitTimeoutMs),
                interval: DefaultPollInterval);

            if (!ok)
            {
                // If unexpected selector exists, map to UnexpectedUIState; otherwise TimeoutError.
                RpcError? unexpected = TryDetectUnexpected(mainWindow, desktop, context.Session.Automation, parsed.UnexpectedSelectors, searchRootKind);
                RpcError err = unexpected ?? new RpcError { Kind = RpcErrorKinds.TimeoutError, Message = "Build wait timed out" };
                context.MarkFailure(waitStep, err);
                result.Ok = false;
                result.Error = err;
                return result;
            }

            context.MarkSuccess(waitStep);
        }
        catch (Exception ex)
        {
            return Fail(result, context, waitStep, RpcErrorKinds.ActionError, "Wait build done failed", ex);
        }

        PopupHandling.TryHandle(context, mainWindow, popupHandling, "AfterBuildDone");

        StepLogEntry outcomeStep = context.StartStep(
            stepId: "BuildOutcome",
            action: "Evaluate build outcome",
            selector: parsed.BuildOutcome.PrimarySelector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["mode"] = parsed.BuildOutcome.Mode,
                ["timeoutMs"] = parsed.BuildOutcome.TimeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        AddRootParameters(outcomeStep.Parameters, searchRootKind);

        BuildOutcomeResult outcome;
        try
        {
            outcome = EvaluateBuildOutcome(
                context,
                mainWindow,
                searchRootKind,
                parsed.BuildOutcome);
        }
        catch (Exception ex)
        {
            return Fail(result, context, outcomeStep, RpcErrorKinds.ActionError, "Build outcome evaluation failed", ex);
        }

        ApplyOutcomeParameters(outcomeStep, outcome);

        RpcError? outcomeError = null;
        if (string.Equals(outcome.Outcome, BuildOutcomeSuccess, StringComparison.Ordinal))
        {
            context.MarkSuccess(outcomeStep);
        }
        else
        {
            outcomeError = MapOutcomeToError(outcome);
            context.MarkFailure(outcomeStep, outcomeError);
        }

        var response = new RunFlowResponse
        {
            Data = JsonSerializer.SerializeToElement(new
            {
                waitedKind = parsed.WaitCondition.Kind,
                buildOutcome = new
                {
                    outcome = outcome.Outcome,
                    usedMode = outcome.UsedMode,
                    selectorEvidence = new
                    {
                        successHit = outcome.SuccessHit,
                        failureHit = outcome.FailureHit,
                    },
                    textEvidence = new
                    {
                        probed = outcome.TextProbed,
                        lastTextSample = outcome.LastTextSample,
                        matchedToken = outcome.MatchedToken,
                        source = outcome.TextSource,
                    },
                    startedAtUtc = outcomeStep.StartedAtUtc,
                    finishedAtUtc = outcomeStep.FinishedAtUtc,
                    durationMs = outcomeStep.DurationMs,
                },
            }),
        };

        if (outcomeError is not null)
        {
            result.Ok = false;
            result.Error = outcomeError;
            result.Value = response;
            return result;
        }

        if (parsed.OptionalCloseDialogSelector is not null)
        {
            if (!TryCloseDialog(context, mainWindow, searchRoot, searchRootKind, parsed.OptionalCloseDialogSelector, parsed.FindTimeoutMs, out RpcError? closeError))
            {
                result.Ok = false;
                result.Error = closeError;
                result.Value = response;
                return result;
            }
        }

        result.Ok = true;
        result.Value = response;
        return result;
    }

    private static PopupHandlingOptions? BuildPopupHandling(ParsedBuildArgs parsed)
    {
        return parsed.PopupHandling;
    }

    private static RpcError? TryDetectUnexpected(
        AutomationElement mainWindow,
        AutomationElement desktop,
        FlaUI.UIA3.UIA3Automation automation,
        IReadOnlyList<ElementSelector>? selectors,
        string searchRootKind)
    {
        if (selectors is null)
        {
            return null;
        }

        foreach (ElementSelector sel in selectors)
        {
            if (sel.Path.Count == 0)
            {
                continue;
            }

            bool matched = string.Equals(searchRootKind, SearchRootDesktop, StringComparison.OrdinalIgnoreCase)
                ? ElementFinder.TryFind(desktop, automation, sel, out _, out _, out _)
                : ElementFinder.TryFind(mainWindow, automation, sel, out _, out _, out _) ||
                  ElementFinder.TryFind(desktop, automation, sel, out _, out _, out _);

            if (matched)
            {
                return new RpcError
                {
                    Kind = RpcErrorKinds.UnexpectedUIState,
                    Message = "Unexpected UI state detected during build",
                    Details = new Dictionary<string, string>(StringComparer.Ordinal)
                    {
                        ["selectorHint"] = "unexpectedSelectorMatched",
                    },
                };
            }
        }

        return null;
    }

    private static bool TryCloseDialog(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        string searchRootKind,
        ElementSelector selector,
        int findTimeoutMs,
        out RpcError? error)
    {
        StepLogEntry step = context.StartStep(
            stepId: "CloseDialog",
            action: "Close dialog",
            selector: selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["timeoutMs"] = findTimeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        AddRootParameters(step.Parameters, searchRootKind);

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
            RpcError warn = MapFindFailureToError(failureKind, details);
            context.MarkWarning(step, warn);
            error = null;
            return true;
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
            error = CreateError(RpcErrorKinds.ActionError, "Failed to close dialog", ex);
            context.MarkFailure(step, error);
            return false;
        }
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

    private static bool TryFindWithSearchRoot(
        FlowContext context,
        Window mainWindow,
        AutomationElement searchRoot,
        string searchRootKind,
        ElementSelector selector,
        string stepId,
        string action,
        int findTimeoutMs,
        out AutomationElement? element,
        out RpcError? error)
    {
        element = null;
        error = null;

        StepLogEntry step = context.StartStep(
            stepId: stepId,
            action: action,
            selector: selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["timeoutMs"] = findTimeoutMs.ToString(),
                ["root"] = searchRootKind,
            });

        AddRootParameters(step.Parameters, searchRootKind);

        string? lastFailure = null;
        Dictionary<string, string>? lastDetails = null;

        bool ok = string.Equals(searchRootKind, SearchRootDesktop, StringComparison.OrdinalIgnoreCase)
            ? TryFindWithinRoot(searchRoot, context.Session.Automation, selector, findTimeoutMs, out element, out lastFailure, out lastDetails)
            : TryFindElementWithFallbackRoots(mainWindow, context.Session.Automation, selector, findTimeoutMs, out element, out lastFailure, out lastDetails);

        if (ok)
        {
            context.MarkSuccess(step);
            error = null;
            return true;
        }

        RpcError err = MapFindFailureToError(lastFailure, lastDetails);
        context.MarkFailure(step, err);
        error = err;
        return false;
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
                if (ElementFinder.TryFind(desktop, automation, selector, out AutomationElement? eDesktop, out _, out _) && eDesktop is not null)
                {
                    return eDesktop.Properties.IsEnabled.ValueOrDefault;
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

    private static BuildOutcomeResult EvaluateBuildOutcome(
        FlowContext context,
        Window mainWindow,
        string searchRootKind,
        ParsedBuildOutcome buildOutcome)
    {
        var result = new BuildOutcomeResult
        {
            UsedMode = buildOutcome.Mode,
            SuccessHit = buildOutcome.SuccessSelector is null ? null : false,
            FailureHit = buildOutcome.FailureSelector is null ? null : false,
        };

        bool checkSelector = buildOutcome.Mode == BuildOutcomeModeWaitSelector ||
            buildOutcome.Mode == BuildOutcomeModeEither;
        bool checkText = buildOutcome.Mode == BuildOutcomeModeReadText ||
            buildOutcome.Mode == BuildOutcomeModeEither;

        AutomationElement desktop = context.Session.Automation.GetDesktop();
        DateTimeOffset deadline = DateTimeOffset.UtcNow.AddMilliseconds(buildOutcome.TimeoutMs);

        while (DateTimeOffset.UtcNow <= deadline)
        {
            if (buildOutcome.FailureSelector is not null &&
                TryFindForOutcome(mainWindow, desktop, context.Session.Automation, buildOutcome.FailureSelector, searchRootKind, out _))
            {
                result.FailureHit = true;
                result.Outcome = BuildOutcomeFail;
                return result;
            }

            if (checkSelector && buildOutcome.SuccessSelector is not null &&
                TryFindForOutcome(mainWindow, desktop, context.Session.Automation, buildOutcome.SuccessSelector, searchRootKind, out _))
            {
                result.SuccessHit = true;
                result.Outcome = BuildOutcomeSuccess;
                return result;
            }

            if (checkText && buildOutcome.TextProbeSelector is not null &&
                TryFindForOutcome(mainWindow, desktop, context.Session.Automation, buildOutcome.TextProbeSelector, searchRootKind, out AutomationElement? textElement) &&
                textElement is not null)
            {
                result.TextProbed = true;
                if (TryReadElementText(textElement, out string text, out string? source))
                {
                    result.LastTextSample = TruncateForLog(text, 256);
                    result.TextSource = source;
                    string? matched = MatchToken(text, buildOutcome.SuccessTextContains);
                    if (!string.IsNullOrWhiteSpace(matched))
                    {
                        result.MatchedToken = matched;
                        result.Outcome = BuildOutcomeSuccess;
                        return result;
                    }
                }
            }

            Thread.Sleep(DefaultPollInterval);
        }

        result.Outcome = BuildOutcomeUnknown;
        return result;
    }

    private static bool TryFindForOutcome(
        Window mainWindow,
        AutomationElement desktop,
        FlaUI.UIA3.UIA3Automation automation,
        ElementSelector selector,
        string searchRootKind,
        out AutomationElement? element)
    {
        element = null;

        if (string.Equals(searchRootKind, SearchRootDesktop, StringComparison.OrdinalIgnoreCase))
        {
            return ElementFinder.TryFind(desktop, automation, selector, out element, out _, out _);
        }

        if (ElementFinder.TryFind(mainWindow, automation, selector, out element, out _, out _))
        {
            return true;
        }

        return ElementFinder.TryFind(desktop, automation, selector, out element, out _, out _);
    }

    private static bool TryReadElementText(AutomationElement element, out string text, out string? source)
    {
        text = string.Empty;
        source = null;

        try
        {
            var value = element.Patterns.Value.PatternOrDefault;
            if (value is not null)
            {
                text = value.Value.ValueOrDefault ?? string.Empty;
                source = "ValuePattern";
                return true;
            }

            var tp = element.Patterns.Text.PatternOrDefault;
            if (tp is not null)
            {
                text = tp.DocumentRange.GetText(2048) ?? string.Empty;
                source = "TextPattern";
                return true;
            }

            var legacy = element.Patterns.LegacyIAccessible.PatternOrDefault;
            if (legacy is not null)
            {
                string? legacyValue = legacy.Value.ValueOrDefault;
                if (!string.IsNullOrWhiteSpace(legacyValue))
                {
                    text = legacyValue;
                    source = "LegacyValue";
                    return true;
                }

                string? legacyName = legacy.Name.ValueOrDefault;
                if (!string.IsNullOrWhiteSpace(legacyName))
                {
                    text = legacyName;
                    source = "LegacyName";
                    return true;
                }
            }

            string name = element.Name ?? string.Empty;
            if (!string.IsNullOrWhiteSpace(name))
            {
                text = name;
                source = "Name";
                return true;
            }
        }
        catch
        {
            return false;
        }

        return false;
    }

    private static string? MatchToken(string text, IReadOnlyList<string> tokens)
    {
        if (string.IsNullOrWhiteSpace(text) || tokens.Count == 0)
        {
            return null;
        }

        foreach (string token in tokens)
        {
            if (text.Contains(token, StringComparison.OrdinalIgnoreCase))
            {
                return token;
            }
        }

        return null;
    }

    private static string TruncateForLog(string text, int maxLength)
    {
        if (string.IsNullOrEmpty(text) || text.Length <= maxLength)
        {
            return text;
        }

        return text[..maxLength];
    }

    private static void ApplyOutcomeParameters(StepLogEntry step, BuildOutcomeResult outcome)
    {
        step.Parameters ??= new Dictionary<string, string>(StringComparer.Ordinal);
        step.Parameters["outcome"] = outcome.Outcome;

        if (outcome.SuccessHit.HasValue)
        {
            step.Parameters["successHit"] = outcome.SuccessHit.Value.ToString();
        }

        if (outcome.FailureHit.HasValue)
        {
            step.Parameters["failureHit"] = outcome.FailureHit.Value.ToString();
        }

        step.Parameters["textProbed"] = outcome.TextProbed.ToString();

        if (!string.IsNullOrWhiteSpace(outcome.MatchedToken))
        {
            step.Parameters["matchedToken"] = outcome.MatchedToken;
        }

        if (!string.IsNullOrWhiteSpace(outcome.LastTextSample))
        {
            step.Parameters["textSample"] = outcome.LastTextSample;
        }

        if (!string.IsNullOrWhiteSpace(outcome.TextSource))
        {
            step.Parameters["textSampleSource"] = outcome.TextSource;
        }
    }

    private static RpcError MapOutcomeToError(BuildOutcomeResult outcome)
    {
        if (string.Equals(outcome.Outcome, BuildOutcomeFail, StringComparison.Ordinal))
        {
            return new RpcError
            {
                Kind = RpcErrorKinds.UnexpectedUIState,
                Message = "Build failed (failure selector matched)",
            };
        }

        return new RpcError
        {
            Kind = RpcErrorKinds.TimeoutError,
            Message = "Build outcome unknown",
        };
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

    private static string? NormalizeBuildOutcomeMode(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return BuildOutcomeModeWaitSelector;
        }

        string trimmed = value.Trim();
        if (string.Equals(trimmed, BuildOutcomeModeWaitSelector, StringComparison.OrdinalIgnoreCase))
        {
            return BuildOutcomeModeWaitSelector;
        }

        if (string.Equals(trimmed, BuildOutcomeModeReadText, StringComparison.OrdinalIgnoreCase))
        {
            return BuildOutcomeModeReadText;
        }

        if (string.Equals(trimmed, BuildOutcomeModeEither, StringComparison.OrdinalIgnoreCase))
        {
            return BuildOutcomeModeEither;
        }

        return null;
    }

    private static ElementSelector? NormalizeSelector(ElementSelector? selector)
    {
        if (selector is null || selector.Path is null || selector.Path.Count == 0)
        {
            return null;
        }

        return selector;
    }

    private static IReadOnlyList<string> FilterTokens(List<string>? tokens)
    {
        if (tokens is null || tokens.Count == 0)
        {
            return Array.Empty<string>();
        }

        return tokens
            .Where(token => !string.IsNullOrWhiteSpace(token))
            .Select(token => token.Trim())
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToList();
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

    internal static bool TryParseArgs(JsonElement? args, out ParsedBuildArgs parsed, out RpcError? error)
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

        BuildArgs? raw;
        try
        {
            raw = args.Value.Deserialize<BuildArgs>(JsonOptions);
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

        ElementSelector? buildSelector = raw.BuildButtonSelector ?? raw.BuildTriggerSelector;
        if (buildSelector is null || buildSelector.Path.Count == 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "BuildButtonSelector must be provided" };
            return false;
        }

        WaitCondition? waitCondition = raw.WaitCondition ?? raw.CompletedCondition;
        if (waitCondition is null)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "WaitCondition must be provided" };
            return false;
        }

        if (waitCondition.Selector is null || waitCondition.Selector.Path.Count == 0)
        {
            error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "WaitCondition selector must be provided" };
            return false;
        }

        int findTimeoutMs = raw.FindTimeoutMs > 0 ? raw.FindTimeoutMs : 10_000;
        int waitTimeoutMs = raw.TimeoutMs > 0 ? raw.TimeoutMs : (raw.WaitTimeoutMs > 0 ? raw.WaitTimeoutMs : 60_000);
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

        ElementSelector? optionalClose = raw.OptionalCloseDialogSelector is not null && raw.OptionalCloseDialogSelector.Path.Count > 0
            ? raw.OptionalCloseDialogSelector
            : null;

        ParsedBuildOutcome buildOutcome = new();
        if (raw.BuildOutcome is null)
        {
            buildOutcome.Mode = BuildOutcomeModeWaitSelector;
            buildOutcome.SuccessSelector = waitCondition.Selector;
            buildOutcome.TimeoutMs = waitTimeoutMs;
            buildOutcome.PrimarySelector = waitCondition.Selector;
        }
        else
        {
            string? normalizedMode = NormalizeBuildOutcomeMode(raw.BuildOutcome.Mode);
            if (normalizedMode is null)
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "BuildOutcome.Mode must be waitSelector/readTextContains/either" };
                return false;
            }

            buildOutcome.Mode = normalizedMode;
            buildOutcome.SuccessSelector = NormalizeSelector(raw.BuildOutcome.SuccessSelector);
            buildOutcome.FailureSelector = NormalizeSelector(raw.BuildOutcome.FailureSelector);
            buildOutcome.TextProbeSelector = NormalizeSelector(raw.BuildOutcome.TextProbeSelector);
            buildOutcome.TimeoutMs = raw.BuildOutcome.TimeoutMs > 0 ? raw.BuildOutcome.TimeoutMs : waitTimeoutMs;
            buildOutcome.SuccessTextContains = FilterTokens(raw.BuildOutcome.SuccessTextContains);

            bool hasSuccess = buildOutcome.SuccessSelector is not null;
            bool hasFailure = buildOutcome.FailureSelector is not null;
            bool hasTextProbe = buildOutcome.TextProbeSelector is not null;
            bool hasTokens = buildOutcome.SuccessTextContains.Count > 0;

            if (buildOutcome.Mode == BuildOutcomeModeWaitSelector && !hasSuccess && !hasFailure)
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "BuildOutcome requires successSelector or failureSelector for waitSelector mode" };
                return false;
            }

            if (buildOutcome.Mode == BuildOutcomeModeReadText)
            {
                if (!hasTextProbe)
                {
                    error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "BuildOutcome.TextProbeSelector is required for readTextContains mode" };
                    return false;
                }

                if (!hasTokens)
                {
                    error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "BuildOutcome.SuccessTextContains is required for readTextContains mode" };
                    return false;
                }
            }

            if (buildOutcome.Mode == BuildOutcomeModeEither && !((hasSuccess || hasFailure) || (hasTextProbe && hasTokens)))
            {
                error = new RpcError { Kind = RpcErrorKinds.InvalidArgument, Message = "BuildOutcome requires selector or text probe configuration for either mode" };
                return false;
            }

            buildOutcome.PrimarySelector = buildOutcome.SuccessSelector ?? buildOutcome.TextProbeSelector ?? buildOutcome.FailureSelector;
        }

        parsed = new ParsedBuildArgs
        {
            BuildButtonSelector = buildSelector,
            WaitCondition = waitCondition,
            UnexpectedSelectors = raw.UnexpectedSelectors,
            OptionalCloseDialogSelector = optionalClose,
            FindTimeoutMs = findTimeoutMs,
            WaitTimeoutMs = waitTimeoutMs,
            SearchRoot = normalizedRoot,
            PopupHandling = popupHandling,
            BuildOutcome = buildOutcome,
        };

        error = null;
        return true;
    }

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    internal sealed class BuildArgs
    {
        public ElementSelector? BuildButtonSelector { get; set; }

        public WaitCondition? WaitCondition { get; set; }

        public BuildOutcomeArgs? BuildOutcome { get; set; }

        public int TimeoutMs { get; set; }

        public string? SearchRoot { get; set; }

        public ElementSelector? OptionalCloseDialogSelector { get; set; }

        public ElementSelector? BuildTriggerSelector { get; set; }

        public WaitCondition? CompletedCondition { get; set; }

        public List<ElementSelector>? UnexpectedSelectors { get; set; }

        public int FindTimeoutMs { get; set; } = 10_000;

        public int WaitTimeoutMs { get; set; } = 60_000;

        public bool EnablePopupHandling { get; set; }

        public string? PopupSearchRoot { get; set; }

        public int PopupTimeoutMs { get; set; } = 1500;

        public bool AllowPopupOk { get; set; }

        public ElementSelector? PopupDialogSelector { get; set; }

        public ElementSelector? PopupOkButtonSelector { get; set; }

        public ElementSelector? PopupCancelButtonSelector { get; set; }
    }

    internal sealed class ParsedBuildArgs
    {
        public ElementSelector BuildButtonSelector { get; set; } = new();

        public WaitCondition WaitCondition { get; set; } = new();

        public IReadOnlyList<ElementSelector>? UnexpectedSelectors { get; set; }

        public ElementSelector? OptionalCloseDialogSelector { get; set; }

        public int FindTimeoutMs { get; set; } = 10_000;

        public int WaitTimeoutMs { get; set; } = 60_000;

        public string SearchRoot { get; set; } = SearchRootMainWindow;

        public PopupHandlingOptions? PopupHandling { get; set; }

        public ParsedBuildOutcome BuildOutcome { get; set; } = new();
    }

    internal sealed class BuildOutcomeArgs
    {
        public string? Mode { get; set; }

        public ElementSelector? SuccessSelector { get; set; }

        public ElementSelector? FailureSelector { get; set; }

        public ElementSelector? TextProbeSelector { get; set; }

        public List<string>? SuccessTextContains { get; set; }

        public int TimeoutMs { get; set; }
    }

    internal sealed class ParsedBuildOutcome
    {
        public string Mode { get; set; } = BuildOutcomeModeWaitSelector;

        public ElementSelector? SuccessSelector { get; set; }

        public ElementSelector? FailureSelector { get; set; }

        public ElementSelector? TextProbeSelector { get; set; }

        public IReadOnlyList<string> SuccessTextContains { get; set; } = Array.Empty<string>();

        public int TimeoutMs { get; set; } = 60_000;

        public ElementSelector? PrimarySelector { get; set; }
    }

    private sealed class BuildOutcomeResult
    {
        public string Outcome { get; set; } = BuildOutcomeUnknown;

        public string UsedMode { get; set; } = BuildOutcomeModeWaitSelector;

        public bool? SuccessHit { get; set; }

        public bool? FailureHit { get; set; }

        public bool TextProbed { get; set; }

        public string? MatchedToken { get; set; }

        public string? LastTextSample { get; set; }

        public string? TextSource { get; set; }
    }
}
