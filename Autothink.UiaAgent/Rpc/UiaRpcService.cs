using System.Diagnostics;
using System.Text.Json;
using Autothink.UiaAgent.Flows;
using Autothink.UiaAgent.Rpc.Contracts;
using Autothink.UiaAgent.Uia;
using FlaUI.Core;
using FlaUI.Core.AutomationElements;
using FlaUI.Core.Input;
using FlaUI.Core.WindowsAPI;
using FlaUI.UIA3;

namespace Autothink.UiaAgent.Rpc;

/// <summary>
/// UIA Agent 的 JSON-RPC 对外入口。
/// </summary>
/// <remarks>
/// 约定：
/// - 该类型的方法将作为 JSON-RPC method 暴露。
/// - 业务日志不要写 stdout（stdout 保留给协议帧）。
/// - 不以异常作为业务失败语义：失败应返回 <see cref="RpcResult"/> / <see cref="RpcResult{T}"/> 的 Error。
/// </remarks>
internal sealed class UiaRpcService
{
    private static readonly TimeSpan DefaultPollInterval = TimeSpan.FromMilliseconds(200);

    /// <summary>
    /// 连通性/健康检查：用于宿主快速确认 RPC 通路可用。
    /// </summary>
    /// <returns>固定返回 "pong"。</returns>
    public string Ping() => "pong";

    /// <summary>
    /// 统一流程入口：按 FlowName 运行流程层逻辑（Stage 2）。
    /// </summary>
    public RpcResult<RunFlowResponse> RunFlow(RunFlowRequest? request)
    {
        var result = new RpcResult<RunFlowResponse>();

        StepLogEntry validateStep = StartStep(
            result.StepLog,
            stepId: "ValidateRequest",
            action: "Validate RunFlow request",
            parameters: request is null ? null : new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["sessionId"] = request.SessionId,
                ["flowName"] = request.FlowName,
                ["timeoutMs"] = request.TimeoutMs.ToString(),
            });

        if (request is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "request must not be null");
        }

        if (string.IsNullOrWhiteSpace(request.SessionId))
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "SessionId must be provided");
        }

        if (string.IsNullOrWhiteSpace(request.FlowName))
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "FlowName must be provided");
        }

        if (request.TimeoutMs <= 0)
        {
            request.TimeoutMs = 30_000;
        }

        MarkSuccess(validateStep);

        // Dispatch by flow name first so callers can get InvalidArgument/NotImplemented
        // without requiring a valid session in the current process.
        if (!FlowNames.IsKnown(request.FlowName))
        {
            string available = string.Join(", ", FlowRegistry.KnownFlowNames);
            StepLogEntry dispatchStep = StartStep(
                result.StepLog,
                stepId: "DispatchFlow",
                action: "Dispatch flow",
                parameters: new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["flowName"] = request.FlowName,
                    ["availableFlows"] = available,
                });

            var error = new RpcError
            {
                Kind = RpcErrorKinds.InvalidArgument,
                Message = "Unknown flow",
                Details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["flowName"] = request.FlowName,
                    ["availableFlows"] = available,
                },
            };

            return Fail(result, dispatchStep, error);
        }

        if (FlowRegistry.TryGet(request.FlowName, out IFlow? flow) && flow is not null && !flow.IsImplemented)
        {
            StepLogEntry dispatchStep = StartStep(
                result.StepLog,
                stepId: "DispatchFlow",
                action: "Dispatch flow",
                parameters: new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["flowName"] = request.FlowName,
                });
            MarkSuccess(dispatchStep);

            StepLogEntry notImplementedStep = StartStep(
                result.StepLog,
                stepId: "NotImplemented",
                action: "Flow not implemented",
                parameters: new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["flowName"] = request.FlowName,
                });

            var error = new RpcError
            {
                Kind = RpcErrorKinds.NotImplemented,
                Message = "Flow is registered but not implemented yet",
                Details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["flowName"] = request.FlowName,
                },
            };

            return Fail(result, notImplementedStep, error);
        }

        JsonElement? args = request.Args.ValueKind is JsonValueKind.Undefined or JsonValueKind.Null
            ? null
            : request.Args;

        if (args is null && !string.IsNullOrWhiteSpace(request.ArgsJson))
        {
            StepLogEntry parseArgs = StartStep(
                result.StepLog,
                stepId: "ParseArgsJson",
                action: "Parse args json",
                parameters: new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["length"] = request.ArgsJson.Length.ToString(),
                });

            try
            {
                using JsonDocument doc = JsonDocument.Parse(request.ArgsJson);
                args = doc.RootElement.Clone();
                MarkSuccess(parseArgs);
            }
            catch (Exception ex)
            {
                return Fail(result, parseArgs, RpcErrorKinds.InvalidArgument, "Failed to parse ArgsJson", ex);
            }
        }

        if (!TryGetSession(request.SessionId, result.StepLog, out UiaSession session, out RpcError? sessionError))
        {
            result.Ok = false;
            result.Error = sessionError;
            return result;
        }

        var timeout = TimeSpan.FromMilliseconds(request.TimeoutMs);
        var ctx = new FlowContext(request.SessionId, session, timeout, result.StepLog);

        try
        {
            return FlowDispatcher.Dispatch(ctx, request.FlowName, args);
        }
        catch (Exception ex)
        {
            // 兜底：FlowDispatcher 内部应尽量结构化处理错误；这里避免异常逃逸破坏协议。
            return Fail(result, StartStep(result.StepLog, stepId: "RunFlowException", action: "RunFlowException"), RpcErrorKinds.ActionError, "RunFlow failed", ex);
        }
    }

    /// <summary>
    /// 打开/附加到目标软件进程并建立会话。
    /// </summary>
    public RpcResult<OpenSessionResponse> OpenSession(OpenSessionRequest? request)
    {
        var result = new RpcResult<OpenSessionResponse>();

        StepLogEntry validateStep = StartStep(
            result.StepLog,
            stepId: "ValidateRequest",
            action: "Validate OpenSession request",
            parameters: request is null ? null : new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["processId"] = request.ProcessId?.ToString() ?? string.Empty,
                ["processName"] = request.ProcessName ?? string.Empty,
                ["mainWindowTitleContains"] = request.MainWindowTitleContains ?? string.Empty,
                ["timeoutMs"] = request.TimeoutMs.ToString(),
                ["bringToForeground"] = request.BringToForeground.ToString(),
            });

        if (request is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "request must not be null");
        }

        if (request.TimeoutMs <= 0)
        {
            request.TimeoutMs = 10_000;
        }

        if (request.ProcessId is null && string.IsNullOrWhiteSpace(request.ProcessName))
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "ProcessId or ProcessName must be provided");
        }

        MarkSuccess(validateStep);

        UiaSession? session = null;
        try
        {
            var timeout = TimeSpan.FromMilliseconds(request.TimeoutMs);

            StepLogEntry attachStep = StartStep(result.StepLog, stepId: "Attach", action: "Attach target process");
            FlaUI.Core.Application app = AttachApplication(request, timeout, out int processId, out string? mainTitle);

            session = UiaSessionRegistry.Create(app);

            MarkSuccess(attachStep);

            StepLogEntry mainWindowStep = StartStep(result.StepLog, stepId: "GetMainWindow", action: "Get main window");
            Window mainWindow = session.GetMainWindow(timeout);
            MarkSuccess(mainWindowStep);

            if (request.BringToForeground)
            {
                StepLogEntry focusStep = StartStep(result.StepLog, stepId: "BringToForeground", action: "Bring main window to foreground");
                try
                {
                    mainWindow.Focus();
                    MarkSuccess(focusStep);
                }
                catch (Exception ex)
                {
                    // 置前失败不视为致命错误，但记录为 Warning。
                    MarkWarning(focusStep, CreateError(RpcErrorKinds.ActionError, "Failed to bring window to foreground", ex));
                }
            }

            result.Ok = true;
            result.Value = new OpenSessionResponse
            {
                SessionId = session.SessionId,
                ProcessId = processId,
                MainWindowTitle = mainTitle ?? mainWindow.Title,
            };

            return result;
        }
        catch (Exception ex)
        {
            RpcError error = CreateError(RpcErrorKinds.ConfigError, "OpenSession failed", ex);
            AppendFailStep(result.StepLog, stepId: "OpenSession", action: "OpenSession", error);
            result.Ok = false;
            result.Error = error;

            if (session is not null)
            {
                if (UiaSessionRegistry.TryRemove(session.SessionId, out UiaSession? removed) && removed is not null)
                {
                    removed.Dispose();
                }
            }

            return result;
        }
    }

    /// <summary>
    /// 关闭会话。
    /// </summary>
    public RpcResult CloseSession(CloseSessionRequest? request)
    {
        var result = new RpcResult();

        StepLogEntry validateStep = StartStep(
            result.StepLog,
            stepId: "ValidateRequest",
            action: "Validate CloseSession request",
            parameters: request is null ? null : new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["sessionId"] = request.SessionId,
            });

        if (request is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "request must not be null");
        }

        if (string.IsNullOrWhiteSpace(request.SessionId))
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "SessionId must be provided");
        }

        MarkSuccess(validateStep);

        StepLogEntry closeStep = StartStep(result.StepLog, stepId: "CloseSession", action: "CloseSession");
        if (!UiaSessionRegistry.TryRemove(request.SessionId, out UiaSession? session) || session is null)
        {
            return Fail(result, closeStep, RpcErrorKinds.ConfigError, "Session not found", new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["sessionId"] = request.SessionId,
            });
        }

        session.Dispose();
        MarkSuccess(closeStep);
        result.Ok = true;
        return result;
    }

    /// <summary>
    /// 查找元素。
    /// </summary>
    public RpcResult<FindElementResponse> FindElement(FindElementRequest? request)
    {
        var result = new RpcResult<FindElementResponse>();

        StepLogEntry validateStep = StartStep(result.StepLog, stepId: "ValidateRequest", action: "Validate FindElement request");
        if (request is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "request must not be null");
        }

        if (string.IsNullOrWhiteSpace(request.SessionId))
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "SessionId must be provided");
        }

        if (request.TimeoutMs <= 0)
        {
            request.TimeoutMs = 5_000;
        }

        MarkSuccess(validateStep);

        if (!TryGetSession(request.SessionId, result.StepLog, out UiaSession session, out RpcError? sessionError))
        {
            result.Ok = false;
            result.Error = sessionError;
            return result;
        }

        var timeout = TimeSpan.FromMilliseconds(request.TimeoutMs);

        StepLogEntry mainWindowStep = StartStep(result.StepLog, stepId: "GetMainWindow", action: "Get main window");
        Window mainWindow;
        try
        {
            mainWindow = session.GetMainWindow(timeout);
            MarkSuccess(mainWindowStep);
        }
        catch (Exception ex)
        {
            return Fail(result, mainWindowStep, RpcErrorKinds.ConfigError, "Failed to get main window", ex);
        }

        StepLogEntry findStep = StartStep(
            result.StepLog,
            stepId: "FindElement",
            action: "FindElement",
            selector: request.Selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["timeoutMs"] = request.TimeoutMs.ToString(),
            });

        AutomationElement? found = null;
        string? lastFailure = null;
        Dictionary<string, string>? lastDetails = null;

        bool ok = Waiter.PollUntil(
            predicate: () =>
            {
                bool foundNow = ElementFinder.TryFind(mainWindow, session.Automation, request.Selector, out AutomationElement? e, out string? fk, out Dictionary<string, string>? det);
                if (foundNow)
                {
                    found = e;
                    return true;
                }

                lastFailure = fk;
                lastDetails = det;
                return false;
            },
            timeout: timeout,
            interval: DefaultPollInterval);

        if (!ok || found is null)
        {
            RpcError err = MapFindFailureToError(lastFailure, lastDetails);
            return Fail(result, findStep, err);
        }

        MarkSuccess(findStep);

        result.Ok = true;
        result.Value = new FindElementResponse
        {
            Element = new ElementRef
            {
                SessionId = session.SessionId,
                Selector = request.Selector,
                RuntimeId = found.Properties.RuntimeId.ValueOrDefault,
                CapturedAtUtc = DateTimeOffset.UtcNow,
            },
        };

        return result;
    }

    /// <summary>
    /// 单击元素。
    /// </summary>
    public RpcResult Click(ClickRequest? request)
    {
        return PerformElementAction(
            request,
            stepId: "Click",
            action: "Click",
            act: element => element.Click());
    }

    /// <summary>
    /// 双击元素。
    /// </summary>
    public RpcResult DoubleClick(DoubleClickRequest? request)
    {
        return PerformElementAction(
            request,
            stepId: "DoubleClick",
            action: "DoubleClick",
            act: element => element.DoubleClick());
    }

    /// <summary>
    /// 右键元素。
    /// </summary>
    public RpcResult RightClick(RightClickRequest? request)
    {
        return PerformElementAction(
            request,
            stepId: "RightClick",
            action: "RightClick",
            act: element => element.RightClick());
    }

    /// <summary>
    /// 设置文本。
    /// </summary>
    public RpcResult SetText(SetTextRequest? request)
    {
        var result = new RpcResult();
        StepLogEntry validateStep = StartStep(result.StepLog, stepId: "ValidateRequest", action: "Validate SetText request");
        if (request is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "request must not be null");
        }

        if (string.IsNullOrWhiteSpace(request.Text))
        {
            request.Text = string.Empty;
        }

        if (request.Element is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "Element must be provided");
        }

        if (!IsSetTextModeSupported(request.Mode))
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "Unsupported SetText mode", new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["mode"] = request.Mode,
            });
        }

        MarkSuccess(validateStep);

        if (!TryResolveElement(result.StepLog, request.Element, out AutomationElement element, out RpcError? resolveError))
        {
            result.Ok = false;
            result.Error = resolveError;
            return result;
        }

        StepLogEntry setTextStep = StartStep(
            result.StepLog,
            stepId: "SetText",
            action: "SetText",
            selector: request.Element.Selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["mode"] = request.Mode,
                ["textLength"] = request.Text.Length.ToString(),
            });

        try
        {
            // Prefer ValuePattern when available and mode is Replace.
            if (request.Mode == SetTextModes.Replace)
            {
                var valuePattern = element.Patterns.Value.PatternOrDefault;
                if (valuePattern is not null)
                {
                    valuePattern.SetValue(request.Text);
                    MarkSuccess(setTextStep);
                    result.Ok = true;
                    return result;
                }
            }

            // Fallback: focus + keyboard.
            element.Focus();

            if (request.Mode != SetTextModes.Append)
            {
                Keyboard.Press((VirtualKeyShort)0x11); // CTRL
                Keyboard.Type((VirtualKeyShort)0x41); // A
                Keyboard.Release((VirtualKeyShort)0x11);
            }

            Keyboard.Type(request.Text);

            MarkSuccess(setTextStep);
            result.Ok = true;
            return result;
        }
        catch (Exception ex)
        {
            return Fail(result, setTextStep, RpcErrorKinds.ActionError, "SetText failed", ex);
        }
    }

    /// <summary>
    /// 发送按键。
    /// </summary>
    public RpcResult SendKeys(SendKeysRequest? request)
    {
        var result = new RpcResult();

        StepLogEntry validateStep = StartStep(result.StepLog, stepId: "ValidateRequest", action: "Validate SendKeys request");
        if (request is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "request must not be null");
        }

        if (string.IsNullOrWhiteSpace(request.SessionId))
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "SessionId must be provided");
        }

        MarkSuccess(validateStep);

        if (!TryGetSession(request.SessionId, result.StepLog, out UiaSession session, out RpcError? sessionError))
        {
            result.Ok = false;
            result.Error = sessionError;
            return result;
        }

        StepLogEntry sendStep = StartStep(
            result.StepLog,
            stepId: "SendKeys",
            action: "SendKeys",
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["keys"] = request.Keys,
            });

        try
        {
            // best-effort: make sure main window is focused
            try
            {
                session.GetMainWindow(TimeSpan.FromMilliseconds(1_000)).Focus();
            }
            catch
            {
                // ignore
            }

            if (!SendKeysParser.TryParse(request.Keys, out ParsedSendKeys? parsed, out string? parseError) || parsed is null)
            {
                return Fail(result, sendStep, RpcErrorKinds.InvalidArgument, "Failed to parse keys", new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["error"] = parseError ?? string.Empty,
                });
            }

            switch (parsed.Kind)
            {
                case ParsedSendKeysKinds.Text:
                    Keyboard.Type(parsed.Text ?? string.Empty);
                    break;
                case ParsedSendKeysKinds.Key:
                    if (parsed.Key is null)
                    {
                        return Fail(result, sendStep, RpcErrorKinds.InvalidArgument, "Key is missing");
                    }

                    Keyboard.Press(parsed.Key.Value);
                    Keyboard.Release(parsed.Key.Value);
                    break;
                case ParsedSendKeysKinds.Chord:
                    if (parsed.Key is null)
                    {
                        return Fail(result, sendStep, RpcErrorKinds.InvalidArgument, "Chord key is missing");
                    }

                    foreach (VirtualKeyShort m in parsed.Modifiers)
                    {
                        Keyboard.Press(m);
                    }

                    Keyboard.Press(parsed.Key.Value);
                    Keyboard.Release(parsed.Key.Value);

                    for (int i = parsed.Modifiers.Length - 1; i >= 0; i--)
                    {
                        Keyboard.Release(parsed.Modifiers[i]);
                    }

                    break;
                default:
                    return Fail(result, sendStep, RpcErrorKinds.InvalidArgument, "Unsupported parsed keys kind", new Dictionary<string, string>(StringComparer.Ordinal)
                    {
                        ["kind"] = parsed.Kind,
                    });
            }

            MarkSuccess(sendStep);
            result.Ok = true;
            return result;
        }
        catch (Exception ex)
        {
            return Fail(result, sendStep, RpcErrorKinds.ActionError, "SendKeys failed", ex);
        }
    }

    /// <summary>
    /// 等待直到满足可观测条件。
    /// </summary>
    public RpcResult WaitUntil(WaitUntilRequest? request)
    {
        var result = new RpcResult();

        StepLogEntry validateStep = StartStep(result.StepLog, stepId: "ValidateRequest", action: "Validate WaitUntil request");
        if (request is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "request must not be null");
        }

        if (string.IsNullOrWhiteSpace(request.SessionId))
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "SessionId must be provided");
        }

        if (request.TimeoutMs <= 0)
        {
            request.TimeoutMs = 5_000;
        }

        if (request.Condition is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "Condition must be provided");
        }

        MarkSuccess(validateStep);

        if (!TryGetSession(request.SessionId, result.StepLog, out UiaSession session, out RpcError? sessionError))
        {
            result.Ok = false;
            result.Error = sessionError;
            return result;
        }

        var timeout = TimeSpan.FromMilliseconds(request.TimeoutMs);

        StepLogEntry waitStep = StartStep(
            result.StepLog,
            stepId: "WaitUntil",
            action: "WaitUntil",
            selector: request.Condition.Selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["kind"] = request.Condition.Kind,
                ["timeoutMs"] = request.TimeoutMs.ToString(),
            });

        try
        {
            Window mainWindow = session.GetMainWindow(TimeSpan.FromMilliseconds(2_000));

            bool satisfied = Waiter.PollUntil(
                predicate: () => EvaluateWaitCondition(mainWindow, session.Automation, request.Condition),
                timeout: timeout,
                interval: DefaultPollInterval);

            if (!satisfied)
            {
                return Fail(result, waitStep, RpcErrorKinds.TimeoutError, "WaitUntil timed out");
            }

            MarkSuccess(waitStep);
            result.Ok = true;
            return result;
        }
        catch (Exception ex)
        {
            return Fail(result, waitStep, RpcErrorKinds.ActionError, "WaitUntil failed", ex);
        }
    }

    private static FlaUI.Core.Application AttachApplication(OpenSessionRequest request, TimeSpan timeout, out int processId, out string? mainTitle)
    {
        processId = 0;
        mainTitle = null;

        if (request.ProcessId is int pid)
        {
            var proc = Process.GetProcessById(pid);
            processId = proc.Id;
            mainTitle = proc.MainWindowTitle;
            return FlaUI.Core.Application.Attach(proc);
        }

        string processName = request.ProcessName ?? string.Empty;
        Process[] candidates = Process.GetProcessesByName(processName);
        if (candidates.Length == 0)
        {
            throw new InvalidOperationException($"Process not found: {processName}");
        }

        Process? selected = null;
        DateTimeOffset deadline = DateTimeOffset.UtcNow.Add(timeout);

        while (selected is null && DateTimeOffset.UtcNow <= deadline)
        {
            foreach (Process p in candidates)
            {
                if (!string.IsNullOrWhiteSpace(request.MainWindowTitleContains))
                {
                    if (p.MainWindowTitle.Contains(request.MainWindowTitleContains, StringComparison.OrdinalIgnoreCase))
                    {
                        selected = p;
                        break;
                    }
                }
                else
                {
                    if (!string.IsNullOrWhiteSpace(p.MainWindowTitle))
                    {
                        selected = p;
                        break;
                    }
                }
            }

            if (selected is null)
            {
                Thread.Sleep(DefaultPollInterval);
                candidates = Process.GetProcessesByName(processName);
                if (candidates.Length == 0)
                {
                    break;
                }
            }
        }

        selected ??= candidates[0];
        processId = selected.Id;
        mainTitle = selected.MainWindowTitle;
        return FlaUI.Core.Application.Attach(selected);
    }

    private static bool IsSetTextModeSupported(string mode)
    {
        return mode == SetTextModes.Replace || mode == SetTextModes.Append || mode == SetTextModes.CtrlAReplace;
    }

    private static bool EvaluateWaitCondition(Window mainWindow, UiaSession session, WaitCondition condition)
    {
        return EvaluateWaitCondition(mainWindow, session.Automation, condition);
    }

    private static bool EvaluateWaitCondition(Window mainWindow, UIA3Automation automation, WaitCondition condition)
    {
        string kind = condition.Kind ?? WaitConditionKinds.ElementExists;

        if (kind == WaitConditionKinds.ElementExists)
        {
            if (condition.Selector is null)
            {
                return false;
            }

            return ElementFinder.TryFind(mainWindow, automation, condition.Selector, out _, out _, out _);
        }

        if (kind == WaitConditionKinds.ElementNotExists)
        {
            if (condition.Selector is null)
            {
                return true;
            }

            return !ElementFinder.TryFind(mainWindow, automation, condition.Selector, out _, out _, out _);
        }

        if (kind == WaitConditionKinds.ElementEnabled)
        {
            if (condition.Selector is null)
            {
                return false;
            }

            if (!ElementFinder.TryFind(mainWindow, automation, condition.Selector, out AutomationElement? e, out _, out _) || e is null)
            {
                return false;
            }

            return e.Properties.IsEnabled.ValueOrDefault;
        }

        return false;
    }

    private static bool TryGetSession(string sessionId, StepLog stepLog, out UiaSession session, out RpcError? error)
    {
        StepLogEntry step = StartStep(stepLog, stepId: "ResolveSession", action: "ResolveSession", parameters: new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["sessionId"] = sessionId,
        });

        if (!UiaSessionRegistry.TryGet(sessionId, out UiaSession? found) || found is null)
        {
            session = null!;
            error = new RpcError
            {
                Kind = RpcErrorKinds.ConfigError,
                Message = "Session not found",
                Details = new Dictionary<string, string>(StringComparer.Ordinal)
                {
                    ["sessionId"] = sessionId,
                },
            };

            MarkFailure(step, error);
            return false;
        }

        session = found;
        MarkSuccess(step);
        error = null;
        return true;
    }

    private static bool TryResolveElement(StepLog stepLog, ElementRef elementRef, out AutomationElement element, out RpcError? error)
    {
        element = null!;
        error = null;

        StepLogEntry resolveStep = StartStep(
            stepLog,
            stepId: "ResolveElement",
            action: "ResolveElement",
            selector: elementRef.Selector,
            parameters: new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["sessionId"] = elementRef.SessionId,
            });

        if (!TryGetSession(elementRef.SessionId, stepLog, out UiaSession session, out RpcError? sessionError))
        {
            MarkFailure(resolveStep, sessionError ?? new RpcError { Kind = RpcErrorKinds.ConfigError, Message = "Session not found" });
            error = sessionError;
            return false;
        }

        Window mainWindow;
        try
        {
            mainWindow = session.GetMainWindow(TimeSpan.FromMilliseconds(2_000));
        }
        catch (Exception ex)
        {
            error = CreateError(RpcErrorKinds.ConfigError, "Failed to get main window", ex);
            MarkFailure(resolveStep, error);
            return false;
        }

        bool ok = ElementFinder.TryFind(mainWindow, session.Automation, elementRef.Selector, out AutomationElement? found, out string? failureKind, out Dictionary<string, string>? details);
        if (!ok || found is null)
        {
            RpcError err = MapFindFailureToError(failureKind, details);
            // ElementRef 场景：找不到更倾向于 StaleElement。
            if (err.Kind == RpcErrorKinds.FindError)
            {
                err.Kind = RpcErrorKinds.StaleElement;
            }

            error = err;
            MarkFailure(resolveStep, err);
            return false;
        }

        element = found;
        MarkSuccess(resolveStep);
        return true;
    }

    private static RpcResult PerformElementAction(ElementRefRequest? request, string stepId, string action, Action<AutomationElement> act)
    {
        var result = new RpcResult();
        StepLogEntry validateStep = StartStep(result.StepLog, stepId: "ValidateRequest", action: $"Validate {action} request");

        if (request is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "request must not be null");
        }

        if (request.Element is null)
        {
            return Fail(result, validateStep, RpcErrorKinds.InvalidArgument, "Element must be provided");
        }

        MarkSuccess(validateStep);

        if (!TryResolveElement(result.StepLog, request.Element, out AutomationElement element, out RpcError? resolveError))
        {
            result.Ok = false;
            result.Error = resolveError;
            return result;
        }

        StepLogEntry actionStep = StartStep(result.StepLog, stepId: stepId, action: action, selector: request.Element.Selector);
        try
        {
            act(element);
            MarkSuccess(actionStep);
            result.Ok = true;
            return result;
        }
        catch (Exception ex)
        {
            return Fail(result, actionStep, RpcErrorKinds.ActionError, $"{action} failed", ex);
        }
    }

    private static RpcResult Fail(RpcResult result, StepLogEntry step, string kind, string message, Dictionary<string, string>? details = null)
    {
        var error = new RpcError { Kind = kind, Message = message, Details = details };
        return Fail(result, step, error);
    }

    private static RpcResult Fail(RpcResult result, StepLogEntry step, string kind, string message, Exception ex)
    {
        RpcError error = CreateError(kind, message, ex);
        return Fail(result, step, error);
    }

    private static RpcResult Fail(RpcResult result, StepLogEntry step, RpcError error)
    {
        result.Ok = false;
        result.Error = error;
        MarkFailure(step, error);
        return result;
    }

    private static RpcResult<T> Fail<T>(RpcResult<T> result, StepLogEntry step, string kind, string message, Dictionary<string, string>? details = null)
    {
        var error = new RpcError { Kind = kind, Message = message, Details = details };
        return Fail(result, step, error);
    }

    private static RpcResult<T> Fail<T>(RpcResult<T> result, StepLogEntry step, string kind, string message, Exception ex)
    {
        RpcError error = CreateError(kind, message, ex);
        return Fail(result, step, error);
    }

    private static RpcResult<T> Fail<T>(RpcResult<T> result, StepLogEntry step, RpcError error)
    {
        result.Ok = false;
        result.Error = error;
        MarkFailure(step, error);
        return result;
    }

    private static RpcError CreateError(string kind, string message, Exception? ex)
    {
        var details = new Dictionary<string, string>(StringComparer.Ordinal);
        if (ex is not null)
        {
            details["exceptionType"] = ex.GetType().FullName ?? ex.GetType().Name;
            details["exceptionMessage"] = ex.Message;
        }

        return new RpcError
        {
            Kind = kind,
            Message = message,
            Details = details.Count == 0 ? null : details,
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

    private static StepLogEntry StartStep(
        StepLog stepLog,
        string stepId,
        string action,
        ElementSelector? selector = null,
        Dictionary<string, string>? parameters = null)
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

        stepLog.Steps.Add(entry);
        return entry;
    }

    private static void MarkSuccess(StepLogEntry step)
    {
        step.FinishedAtUtc = DateTimeOffset.UtcNow;
        step.DurationMs = (long)(step.FinishedAtUtc - step.StartedAtUtc).TotalMilliseconds;
        step.Outcome = StepOutcomes.Success;
    }

    private static void MarkWarning(StepLogEntry step, RpcError error)
    {
        step.FinishedAtUtc = DateTimeOffset.UtcNow;
        step.DurationMs = (long)(step.FinishedAtUtc - step.StartedAtUtc).TotalMilliseconds;
        step.Outcome = StepOutcomes.Warning;
        step.Error = error;
    }

    private static void MarkFailure(StepLogEntry step, RpcError error)
    {
        step.FinishedAtUtc = DateTimeOffset.UtcNow;
        step.DurationMs = (long)(step.FinishedAtUtc - step.StartedAtUtc).TotalMilliseconds;
        step.Outcome = StepOutcomes.Fail;
        step.Error = error;
    }

    private static void AppendFailStep(StepLog stepLog, string stepId, string action, RpcError error)
    {
        DateTimeOffset startedAtUtc = DateTimeOffset.UtcNow;
        DateTimeOffset finishedAtUtc = startedAtUtc;

        stepLog.Steps.Add(new StepLogEntry
        {
            StepId = stepId,
            Action = action,
            StartedAtUtc = startedAtUtc,
            FinishedAtUtc = finishedAtUtc,
            DurationMs = 0,
            Outcome = StepOutcomes.Fail,
            Error = error,
        });
    }
}
