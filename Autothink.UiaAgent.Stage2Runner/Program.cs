// 说明:
// - Stage2Runner 是 UIA 自动化的“现场执行器”，负责读取配置与 selector 资产、启动/连接 Agent 并顺序执行 flows。
// - 同时提供 probe/check/verify 等运维能力，产出 summary/evidence pack 作为可提交的诊断证据。
// - 该入口不修改 RPC 契约，仅通过 JSON-RPC 调用 Agent 侧能力进行编排。
using System.Diagnostics;
using System.Linq;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Autothink.UiaAgent.Rpc.Contracts;
using FlaUI.Core;
using FlaUI.Core.AutomationElements;
using FlaUI.Core.Definitions;
using FlaUI.UIA3;
using StreamJsonRpc;

namespace Autothink.UiaAgent.Stage2Runner;

/// <summary>
/// Stage 2 Runner 控制台入口：负责解析参数、选择执行模式并调度核心流程。
/// </summary>
internal static class Program
{
    private static async Task<int> Main(string[] args)
    {
        RunnerOptions options = RunnerOptions.Parse(args);
        if (options.ShowHelp)
        {
            PrintUsage();
            return 0;
        }

        if (options.Verify)
        {
            return RunVerify(options);
        }

        string repoRoot = FindRepoRoot();
        RunnerConfig? config = null;
        string configDir = repoRoot;

        if (!string.IsNullOrWhiteSpace(options.ConfigPath))
        {
            config = LoadConfig(options.ConfigPath, out configDir);
        }

        string agentExe = ResolveAgentPath(options.AgentPath, config?.AgentPath, repoRoot, configDir);
        string demoExe = Path.Combine(repoRoot, "Autothink.UiaAgent.DemoTarget", "bin", "Release", "net8.0-windows", "Autothink.UiaAgent.DemoTarget.exe");

        Console.WriteLine($"RepoRoot: {repoRoot}");
        Console.WriteLine($"AgentExe: {agentExe}");
        Console.WriteLine($"DemoExe: {demoExe}");

        if (!File.Exists(agentExe))
        {
            Console.Error.WriteLine("Build Release first:");
            Console.Error.WriteLine("  dotnet build PLCCodeForge.sln -c Release");
            return 2;
        }

        if (options.Check)
        {
            string logsRoot = config is null
                ? Path.Combine(repoRoot, "logs")
                : ResolveLogsRoot(config, repoRoot, configDir);

            return await RunCheckAsync(agentExe, logsRoot, options);
        }

        if (options.Probe)
        {
            if (options.Demo)
            {
                Console.Error.WriteLine("Probe mode cannot be combined with --demo.");
                return 2;
            }

            if (config is null)
            {
                Console.Error.WriteLine("Probe mode requires --config <path>.");
                return 2;
            }

            string selectorsRoot = ResolveSelectorsRoot(options, config, repoRoot, configDir);
            string profile = ResolveProfile(options, config);
            string logsRoot = ResolveLogsRoot(config, repoRoot, configDir);

            Console.WriteLine($"SelectorsRoot: {selectorsRoot}");
            Console.WriteLine($"Profile: {profile}");
            Console.WriteLine($"LogsRoot: {logsRoot}");

            return await RunProbeAsync(agentExe, config, selectorsRoot, profile, logsRoot, configDir, options);
        }

        if (!options.Demo && config is not null)
        {
            string selectorsRoot = ResolveSelectorsRoot(options, config, repoRoot, configDir);
            string profile = ResolveProfile(options, config);
            string logsRoot = ResolveLogsRoot(config, repoRoot, configDir);

            Console.WriteLine($"SelectorsRoot: {selectorsRoot}");
            Console.WriteLine($"Profile: {profile}");
            Console.WriteLine($"LogsRoot: {logsRoot}");

            return await RunConfigAsync(agentExe, config, selectorsRoot, profile, logsRoot, configDir, options);
        }

        if (!File.Exists(demoExe))
        {
            Console.Error.WriteLine("DemoTarget not found. Build Release first:");
            Console.Error.WriteLine("  dotnet build PLCCodeForge.sln -c Release");
            return 2;
        }

        return await RunDemoAsync(agentExe, demoExe, repoRoot, options);
    }

    private static async Task<int> RunDemoAsync(string agentExe, string demoExe, string repoRoot, RunnerOptions options)
    {
        using var demo = Process.Start(new ProcessStartInfo
        {
            FileName = demoExe,
            UseShellExecute = false,
            CreateNoWindow = false,
        });

        if (demo is null)
        {
            Console.Error.WriteLine("Failed to start demo target.");
            return 3;
        }

        demo.WaitForInputIdle(5_000);

        using var agent = Process.Start(CreateAgentStartInfo(agentExe));

        if (agent is null)
        {
            demo.Kill(entireProcessTree: true);
            Console.Error.WriteLine("Failed to start agent.");
            return 4;
        }

        string runDir = CreateRunDirectory(Path.Combine(repoRoot, "logs"));
        Console.WriteLine($"RunDir: {runDir}");

        ConnectivitySession connectivity = await ConnectAndCheckAsync(agent, agentExe, options.CheckTimeoutMs, runDir);
        string connectivityPath = WriteConnectivityReport(runDir, connectivity.Report);
        Console.WriteLine($"Connectivity: {connectivityPath}");

        if (!connectivity.Report.Ok && !options.SkipCheck)
        {
            Console.Error.WriteLine($"Connectivity failed: {connectivity.Report.Error?.Message}");
            await CleanupAsync(agent, demo);
            return 6;
        }

        if (connectivity.Rpc is null)
        {
            Console.Error.WriteLine("Connectivity check failed to establish RPC.");
            await CleanupAsync(agent, demo);
            return 6;
        }

        using var handler = connectivity.Handler;
        using var rpc = connectivity.Rpc;

        string pong = await rpc.InvokeAsync<string>("Ping");
        Console.WriteLine($"Ping => {pong}");

        var openSessionRequest = new OpenSessionRequest
        {
            ProcessId = demo.Id,
            TimeoutMs = 10_000,
            BringToForeground = true,
        };

        RpcResult<OpenSessionResponse> sessionResult = await rpc.InvokeAsync<RpcResult<OpenSessionResponse>>("OpenSession", openSessionRequest);

        DumpRpc("OpenSession", openSessionRequest, sessionResult);

        if (!sessionResult.Ok || sessionResult.Value is null)
        {
            await CleanupAsync(agent, demo);
            return 5;
        }

        string sessionId = sessionResult.Value.SessionId;

        // 1) attach
        var attachRequest = new
        {
            sessionId,
            flowName = "autothink.attach",
            args = JsonSerializer.SerializeToElement((object?)null, JsonOptions),
            argsJson = (string?)null,
            timeoutMs = 30_000,
        };

        RpcResult<RunFlowResponse> attachResult = await rpc.InvokeAsync<RpcResult<RunFlowResponse>>("RunFlow", attachRequest);
        DumpRpc("RunFlow.autothink.attach", attachRequest, attachResult);

        // Selectors for DemoTarget (WinForms): use AutomationId + ControlType to be deterministic.
        ElementSelector editorSelector = new()
        {
            Path =
            [
                new SelectorStep { Search = SelectorSearchKinds.Descendants, ControlType = "Edit", AutomationId = "programEditor", Index = 0 },
            ],
        };

        ElementSelector openImportSelector = new()
        {
            Path =
            [
                new SelectorStep { Search = SelectorSearchKinds.Descendants, ControlType = "Button", AutomationId = "openImportButton", Index = 0 },
            ],
        };

        ElementSelector dialogSelector = new()
        {
            Path =
            [
                new SelectorStep { Search = SelectorSearchKinds.Descendants, ControlType = "Pane", AutomationId = "importPanel", Index = 0 },
            ],
        };

        ElementSelector pathInputSelector = new()
        {
            Path =
            [
                new SelectorStep { Search = SelectorSearchKinds.Descendants, ControlType = "Edit", AutomationId = "importPathTextBox", Index = 0 },
            ],
        };

        ElementSelector confirmSelector = new()
        {
            Path =
            [
                new SelectorStep { Search = SelectorSearchKinds.Descendants, ControlType = "Button", AutomationId = "confirmImportButton", Index = 0 },
            ],
        };

        ElementSelector buildButtonSelector = new()
        {
            Path =
            [
                new SelectorStep { Search = SelectorSearchKinds.Descendants, ControlType = "Button", AutomationId = "buildButton", Index = 0 },
            ],
        };

        // 2) importProgram.textPaste
        var importProgramArgs = new
        {
            programText = $"// DEMO {DateTimeOffset.Now:O}\r\nVAR\r\n  a : INT;\r\nEND_VAR\r\n",
            editorSelector,
            afterPasteWaitMs = 1000,
            verifyMode = "editorNotEmpty",
            findTimeoutMs = 10_000,
            clipboardTimeoutMs = 2_000,
            verifyTimeoutMs = 5_000,
            fallbackToType = true,
        };

        var importProgramRequest = new
        {
            sessionId,
            flowName = "autothink.importProgram.textPaste",
            args = JsonSerializer.SerializeToElement(importProgramArgs, JsonOptions),
            argsJson = JsonSerializer.Serialize(importProgramArgs, JsonOptions),
            timeoutMs = 30_000,
        };

        RpcResult<RunFlowResponse> importProgramResult = await rpc.InvokeAsync<RpcResult<RunFlowResponse>>("RunFlow", importProgramRequest);
        DumpRpc("RunFlow.autothink.importProgram.textPaste", importProgramRequest, importProgramResult);

        // 3) importVariables
        var importVariablesArgs = new
        {
            filePath = $"C:\\\\temp\\\\vars-{Guid.NewGuid():N}.xlsx",
            openImportDialogSteps = new[]
            {
                new
                {
                    action = "Click",
                    selector = openImportSelector,
                },
            },
            dialogSelector,
            filePathEditorSelector = pathInputSelector,
            confirmButtonSelector = confirmSelector,
            successCondition = new WaitCondition { Kind = WaitConditionKinds.ElementNotExists, Selector = dialogSelector },
            findTimeoutMs = 10_000,
            waitTimeoutMs = 10_000,
        };

        var importVariablesRequest = new
        {
            sessionId,
            flowName = "autothink.importVariables",
            args = JsonSerializer.SerializeToElement(importVariablesArgs, JsonOptions),
            argsJson = JsonSerializer.Serialize(importVariablesArgs, JsonOptions),
            timeoutMs = 30_000,
        };

        RpcResult<RunFlowResponse> importVariablesResult = await rpc.InvokeAsync<RpcResult<RunFlowResponse>>("RunFlow", importVariablesRequest);
        DumpRpc("RunFlow.autothink.importVariables", importVariablesRequest, importVariablesResult);

        // 4) build
        var buildArgs = new
        {
            buildButtonSelector = buildButtonSelector,
            waitCondition = new WaitCondition { Kind = WaitConditionKinds.ElementEnabled, Selector = buildButtonSelector },
            findTimeoutMs = 10_000,
            timeoutMs = 15_000,
        };

        var buildRequest = new
        {
            sessionId,
            flowName = "autothink.build",
            args = JsonSerializer.SerializeToElement(buildArgs, JsonOptions),
            argsJson = JsonSerializer.Serialize(buildArgs, JsonOptions),
            timeoutMs = 30_000,
        };

        RpcResult<RunFlowResponse> buildResult = await rpc.InvokeAsync<RpcResult<RunFlowResponse>>("RunFlow", buildRequest);
        DumpRpc("RunFlow.autothink.build", buildRequest, buildResult);

        _ = await rpc.InvokeAsync<RpcResult>("CloseSession", new CloseSessionRequest { SessionId = sessionId });
        await CleanupAsync(agent, demo);
        return 0;
    }

    private static async Task<int> RunCheckAsync(string agentExe, string logsRoot, RunnerOptions options)
    {
        using var agent = Process.Start(CreateAgentStartInfo(agentExe));

        if (agent is null)
        {
            Console.Error.WriteLine("Failed to start agent.");
            return 4;
        }

        string runDir = CreateRunDirectory(logsRoot);
        Console.WriteLine($"RunDir: {runDir}");

        ConnectivitySession connectivity = await ConnectAndCheckAsync(agent, agentExe, options.CheckTimeoutMs, runDir);
        string connectivityPath = WriteConnectivityReport(runDir, connectivity.Report);

        Console.WriteLine($"Connectivity: {connectivityPath}");
        if (connectivity.Report.Ok)
        {
            Console.WriteLine("Connectivity check: OK");
            await CleanupAsync(agent, demo: null);
            return 0;
        }

        Console.Error.WriteLine($"Connectivity check: FAIL - {connectivity.Report.Error?.Message}");
        if (!string.IsNullOrWhiteSpace(connectivity.Report.Error?.Hint))
        {
            Console.Error.WriteLine($"Hint: {connectivity.Report.Error?.Hint}");
        }

        await CleanupAsync(agent, demo: null);
        return 6;
    }

    private static int RunVerify(RunnerOptions options)
    {
        if (string.IsNullOrWhiteSpace(options.EvidencePath))
        {
            Console.Error.WriteLine("Verify requires --evidence <dir>.");
            return 2;
        }

        string packDir = Path.GetFullPath(options.EvidencePath);
        string summaryPath = Path.Combine(packDir, "evidence_summary.v1.json");
        if (!File.Exists(summaryPath))
        {
            Console.Error.WriteLine($"Evidence summary not found: {summaryPath}");
            return 2;
        }

        EvidenceSummaryReport? evidence = ReadJsonFile<EvidenceSummaryReport>(summaryPath);
        if (evidence is null)
        {
            Console.Error.WriteLine("Failed to parse evidence_summary.v1.json.");
            return 2;
        }

        var errors = new List<string>();
        string[] requiredFiles =
        {
            "summary.json",
            "selector_check_report.json",
            "step_logs.json",
            "evidence_summary.v1.json",
        };

        foreach (string required in requiredFiles)
        {
            string requiredPath = Path.Combine(packDir, required);
            if (!File.Exists(requiredPath))
            {
                errors.Add($"Missing required file: {required}");
            }
        }

        foreach (KeyValuePair<string, string> digest in evidence.Digests)
        {
            string filePath = Path.Combine(packDir, digest.Key);
            if (!File.Exists(filePath))
            {
                errors.Add($"Digest file missing: {digest.Key}");
                continue;
            }

            string actual = ComputeSha256(filePath);
            if (!string.Equals(actual, digest.Value, StringComparison.OrdinalIgnoreCase))
            {
                errors.Add($"Digest mismatch: {digest.Key}");
            }
        }

        string selectorPath = Path.Combine(packDir, "selector_check_report.json");
        SelectorCheckReport? selectorReport = File.Exists(selectorPath)
            ? ReadJsonFile<SelectorCheckReport>(selectorPath)
            : null;

        if (selectorReport is null)
        {
            errors.Add("selector_check_report.json missing or invalid.");
        }
        else
        {
            if (!string.Equals(selectorReport.PackVersion, "v1", StringComparison.OrdinalIgnoreCase))
            {
                errors.Add($"selector_check_report packVersion mismatch: {selectorReport.PackVersion}");
            }

            if (selectorReport.MissingKeys is not null && selectorReport.MissingKeys.Count > 0)
            {
                errors.Add($"selector_check_report missingKeys not empty: {selectorReport.MissingKeys.Count}");
            }
        }

        SummaryReport? runSummary = null;
        string runSummaryPath = Path.Combine(packDir, "summary.json");
        if (File.Exists(runSummaryPath))
        {
            runSummary = ReadJsonFile<SummaryReport>(runSummaryPath);
        }

        string buildOutcomePath = Path.Combine(packDir, "build_outcome.json");
        if (File.Exists(buildOutcomePath))
        {
            BuildOutcomeReport? buildOutcome = ReadJsonFile<BuildOutcomeReport>(buildOutcomePath);
            if (buildOutcome is null)
            {
                errors.Add("build_outcome.json invalid.");
            }
            else if (runSummary?.Build is null)
            {
                errors.Add("build_outcome.json exists but summary.build is missing.");
            }
            else if (!string.Equals(buildOutcome.Outcome, runSummary.Build.Outcome, StringComparison.Ordinal))
            {
                errors.Add($"build_outcome mismatch: summary={runSummary.Build.Outcome}, file={buildOutcome.Outcome}");
            }
        }
        else if (runSummary?.Build is not null)
        {
            errors.Add("summary.build exists but build_outcome.json is missing.");
        }

        if (errors.Count > 0)
        {
            Console.Error.WriteLine("Evidence verify: FAIL");
            foreach (string error in errors)
            {
                Console.Error.WriteLine($"  - {error}");
            }

            return 6;
        }

        Console.WriteLine("Evidence verify: OK");
        return 0;
    }

    private static async Task<int> RunConfigAsync(
        string agentExe,
        RunnerConfig config,
        string selectorsRoot,
        string profile,
        string logsRoot,
        string configDir,
        RunnerOptions options)
    {
        using var agent = Process.Start(CreateAgentStartInfo(agentExe));

        if (agent is null)
        {
            Console.Error.WriteLine("Failed to start agent.");
            return 4;
        }

        if (config.Session is null)
        {
            Console.Error.WriteLine("Config missing: session.");
            await CleanupAsync(agent, demo: null);
            return 5;
        }

        string runDir = CreateRunDirectory(logsRoot);
        Console.WriteLine($"RunDir: {runDir}");
        string? packVersion = NormalizeSelectorPackVersion(config.SelectorPackVersion);
        Console.WriteLine($"SelectorPackVersion: {packVersion ?? "(default)"}");

        EvidencePackConfig evidencePack = config.EvidencePack ?? new EvidencePackConfig();
        UiStateRecoveryConfig uiStateConfig = config.UiStateRecovery ?? new UiStateRecoveryConfig();
        UiStateRecoveryState? uiStateState = null;
        List<StepLogEntry>? runnerSteps = null;

        var summaries = new List<FlowRunSummary>();
        string? stoppedBecause = null;
        InputsSourceSummary? inputsSummary = null;
        BuildSummary? buildSummary = null;
        SelectorProfileLoadResult attachProfile;
        SelectorProfileLoadResult importVariablesProfile;
        SelectorProfileLoadResult importProgramProfile;
        SelectorProfileLoadResult buildProfile;

        try
        {
            attachProfile = LoadProfileWithOverrides(selectorsRoot, profile, "autothink.attach", packVersion);
            importVariablesProfile = LoadProfileWithOverrides(selectorsRoot, profile, "autothink.importVariables", packVersion);
            importProgramProfile = LoadProfileWithOverrides(selectorsRoot, profile, "autothink.importProgram.textPaste", packVersion);
            buildProfile = LoadProfileWithOverrides(selectorsRoot, profile, "autothink.build", packVersion);
        }
        catch (Exception ex) when (ex is FileNotFoundException or InvalidOperationException)
        {
            Console.Error.WriteLine($"Selector profile load failed: {ex.Message}");
            summaries.Add(new FlowRunSummary
            {
                Name = "autothink.attach",
                Ok = false,
                ErrorKind = RpcErrorKinds.ConfigError,
                ErrorMessage = ex.Message,
                Root = "mainWindow",
                SelectorsFile = string.Empty,
            });
            stoppedBecause = "selectorCheckFailed";
            WriteSummary(runDir, profile, summaries, null, null, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
            await CleanupAsync(agent, demo: null);
            return 6;
        }

        var profilesForCheck = new Dictionary<string, SelectorProfileLoadResult>(StringComparer.Ordinal)
        {
            ["autothink.attach"] = attachProfile,
            ["autothink.importVariables"] = importVariablesProfile,
            ["autothink.importProgram.textPaste"] = importProgramProfile,
            ["autothink.build"] = buildProfile,
        };

        SelectorCheckResult selectorCheck = RunSelectorCheck(runDir, packVersion, profilesForCheck);

        Console.WriteLine($"SelectorCheck: {selectorCheck.ReportPath}");
        if (!selectorCheck.Ok)
        {
            string missingMessage = BuildMissingKeysMessage(selectorCheck.Report.MissingKeys);
            summaries.Add(new FlowRunSummary
            {
                Name = "autothink.attach",
                Ok = false,
                ErrorKind = RpcErrorKinds.ConfigError,
                ErrorMessage = missingMessage,
                Root = "mainWindow",
                SelectorsFile = attachProfile.SelectorsFile,
            });
            stoppedBecause = "selectorCheckFailed";
            AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
            WriteSummary(runDir, profile, summaries, null, null, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
            await CleanupAsync(agent, demo: null);
            return 6;
        }

        ConnectivitySession connectivity = await ConnectAndCheckAsync(agent, agentExe, options.CheckTimeoutMs, runDir);
        string connectivityPath = WriteConnectivityReport(runDir, connectivity.Report);
        Console.WriteLine($"Connectivity: {connectivityPath}");

        if (!connectivity.Report.Ok && !options.SkipCheck)
        {
            Console.Error.WriteLine($"Connectivity failed: {connectivity.Report.Error?.Message}");
            summaries.Add(new FlowRunSummary
            {
                Name = "autothink.attach",
                Ok = false,
                ErrorKind = "NotRun",
                ErrorMessage = "Skipped due to connectivity failure",
                Root = "mainWindow",
                SelectorsFile = attachProfile.SelectorsFile,
            });

            AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
            WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
            await CleanupAsync(agent, demo: null);
            return 6;
        }

        if (connectivity.Rpc is null)
        {
            Console.Error.WriteLine("Connectivity check failed to establish RPC.");
            await CleanupAsync(agent, demo: null);
            return 6;
        }

        using var handler = connectivity.Handler;
        using var rpc = connectivity.Rpc;

        bool summaryWritten = false;

        var attachIndex = SelectorKeyIndex.FromProfile(attachProfile.Profile);

        var openSessionRequest = new OpenSessionRequest
        {
            ProcessId = config.Session.ProcessId,
            ProcessName = config.Session.ProcessName,
            MainWindowTitleContains = config.Session.MainWindowTitleContains,
            TimeoutMs = config.Session.TimeoutMs > 0 ? config.Session.TimeoutMs : 10_000,
            BringToForeground = config.Session.BringToForeground,
        };

        RpcResult<OpenSessionResponse> sessionResult;
        try
        {
            sessionResult = await rpc.InvokeAsync<RpcResult<OpenSessionResponse>>("OpenSession", openSessionRequest);
            DumpRpc("OpenSession", openSessionRequest, sessionResult);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"OpenSession failed: {ex.Message}");
            var summary = new FlowRunSummary
            {
                Name = "autothink.attach",
                Ok = false,
                ErrorKind = "RpcError",
                ErrorMessage = ex.Message,
                FailedStepId = "OpenSession",
                Root = "mainWindow",
                SelectorsFile = attachProfile.SelectorsFile,
            };
            summaries.Add(summary);
            AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
            WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
            summaryWritten = true;
            await CleanupAsync(agent, demo: null);
            return 6;
        }

        if (!sessionResult.Ok || sessionResult.Value is null)
        {
            var summary = new FlowRunSummary
            {
                Name = "autothink.attach",
                Ok = false,
                ErrorKind = sessionResult.Error?.Kind ?? "ConfigError",
                ErrorMessage = sessionResult.Error?.Message,
                FailedStepId = "OpenSession",
                Root = "mainWindow",
                SelectorsFile = attachProfile.SelectorsFile,
            };
            summaries.Add(summary);
            AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
            WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
            summaryWritten = true;
            await CleanupAsync(agent, demo: null);
            return 6;
        }

        string sessionId = sessionResult.Value.SessionId;
        uiStateState = InitializeUiStateRecovery(uiStateConfig, attachProfile.Profile, sessionResult.Value.ProcessId);
        runnerSteps = uiStateState?.RunnerSteps;

        try
        {
            FlowRunResult attachResult = await RunFlowAndLogAsync(rpc, sessionId, "autothink.attach", args: null, runDir, config.FlowTimeoutMs);
            FlowRunSummary attachSummary = CreateSummary("autothink.attach", attachResult, root: "mainWindow", attachProfile.SelectorsFile, attachIndex);
            summaries.Add(attachSummary);
            if (!attachSummary.Ok)
            {
                stoppedBecause = "flowFailed:autothink.attach";
                AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
                WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                summaryWritten = true;
                return 10;
            }

            ImportVariablesConfig importVariables = config.ImportVariables ?? new ImportVariablesConfig();
            ImportProgramConfig importProgram = config.ImportProgram ?? new ImportProgramConfig();
            BuildConfig build = config.Build ?? new BuildConfig();
            bool runImportVariables = !config.SkipImportVariables;
            bool runImportProgram = !config.SkipImportProgram;
            bool runBuild = !config.SkipBuild;
            bool allowPartial = config.AllowPartial;

            SelectorProfileLoadResult? popupProfile = null;
            if ((runImportVariables && importVariables.EnablePopupHandling) ||
                (runImportProgram && importProgram.EnablePopupHandling) ||
                (runBuild && build.EnablePopupHandling))
            {
                popupProfile = LoadProfileWithOverrides(selectorsRoot, profile, "autothink.popups", packVersion);
            }

            var importVariablesIndex = SelectorKeyIndex.FromProfile(importVariablesProfile.Profile);

            FlowInputsResolution inputs = ResolveInputs(config, configDir);
            string resolvedInputsPath = WriteResolvedInputs(runDir, inputs);
            inputsSummary = new InputsSourceSummary
            {
                Mode = inputs.Mode,
                CommIrPath = inputs.CommIrPath,
                ResolvedInputsPath = resolvedInputsPath,
                Warnings = inputs.Warnings,
            };

            if (!inputs.Ok)
            {
                var inputSummary = new FlowRunSummary
                {
                    Name = "autothink.importVariables",
                    Ok = false,
                    ErrorKind = inputs.Error?.Kind ?? RpcErrorKinds.InvalidArgument,
                    ErrorMessage = inputs.Error?.Message ?? "inputsSource resolution failed",
                    Root = NormalizeRoot(importVariables.SearchRoot),
                    SelectorsFile = importVariablesProfile.SelectorsFile,
                };

                summaries.Add(inputSummary);
                stoppedBecause = "flowFailed:inputsSource";
                AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
                WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                summaryWritten = true;
                return 11;
            }

            if (runImportVariables)
            {
                try
                {
                    string filePath = ResolvePath(importVariables.FilePath ?? inputs.VariablesFilePath, configDir, required: true, "variablesFilePath");

                    ElementSelector filePathEditorSelector = ResolveSelector(importVariables.FilePathEditorSelector, importVariables.FilePathEditorSelectorKey, importVariablesProfile.Profile, "filePathEditorSelector");
                    ElementSelector confirmButtonSelector = ResolveSelector(importVariables.ConfirmButtonSelector, importVariables.ConfirmButtonSelectorKey, importVariablesProfile.Profile, "confirmButtonSelector");
                    ElementSelector? dialogSelector = ResolveSelectorOptional(importVariables.DialogSelector, importVariables.DialogSelectorKey, importVariablesProfile.Profile);

                    var openDialogSteps = BuildDialogSteps(importVariables.OpenImportDialogSteps, importVariablesProfile.Profile);
                    WaitCondition? successCondition = ResolveWaitCondition(importVariables.SuccessCondition, importVariablesProfile.Profile);

                    PopupArgs importVariablesPopup = BuildPopupArgs(
                        importVariables.EnablePopupHandling,
                        importVariables.PopupSearchRoot,
                        importVariables.PopupTimeoutMs,
                        importVariables.AllowPopupOk,
                        importVariables.PopupDialogSelector,
                        importVariables.PopupDialogSelectorKey,
                        importVariables.PopupOkButtonSelector,
                        importVariables.PopupOkButtonSelectorKey,
                        importVariables.PopupCancelButtonSelector,
                        importVariables.PopupCancelButtonSelectorKey,
                        popupProfile?.Profile,
                        "importVariables");

                    var importVariablesArgs = new
                    {
                        filePath,
                        openImportDialogSteps = openDialogSteps,
                        dialogSelector,
                        filePathEditorSelector,
                        confirmButtonSelector,
                        successCondition,
                        findTimeoutMs = importVariables.FindTimeoutMs,
                        waitTimeoutMs = importVariables.WaitTimeoutMs,
                        searchRoot = importVariables.SearchRoot,
                        enablePopupHandling = importVariablesPopup.Enable,
                        popupSearchRoot = importVariablesPopup.SearchRoot,
                        popupTimeoutMs = importVariablesPopup.TimeoutMs,
                        allowPopupOk = importVariablesPopup.AllowOk,
                        popupDialogSelector = importVariablesPopup.DialogSelector,
                        popupOkButtonSelector = importVariablesPopup.OkButtonSelector,
                        popupCancelButtonSelector = importVariablesPopup.CancelButtonSelector,
                    };

                    FlowRunResult importVariablesResult = await RunFlowWithRecoveryAsync(
                        rpc,
                        sessionId,
                        "autothink.importVariables",
                        importVariablesArgs,
                        runDir,
                        config.FlowTimeoutMs,
                        uiStateState);
                    FlowRunSummary importVariablesSummary = CreateSummary(
                        "autothink.importVariables",
                        importVariablesResult,
                        NormalizeRoot(importVariables.SearchRoot),
                        importVariablesProfile.SelectorsFile,
                        importVariablesIndex);
                    summaries.Add(importVariablesSummary);
                    if (!importVariablesSummary.Ok)
                    {
                        if (!allowPartial)
                        {
                            stoppedBecause = "flowFailed:autothink.importVariables";
                            AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
                            WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                            summaryWritten = true;
                            return 11;
                        }
                    }
                }
                catch (Exception ex) when (ex is KeyNotFoundException or InvalidOperationException or ArgumentException)
                {
                    var summary = new FlowRunSummary
                    {
                        Name = "autothink.importVariables",
                        Ok = false,
                        ErrorKind = RpcErrorKinds.InvalidArgument,
                        ErrorMessage = ex.Message,
                        Root = NormalizeRoot(importVariables.SearchRoot),
                        SelectorsFile = importVariablesProfile.SelectorsFile,
                    };
                    summaries.Add(summary);
                    if (!allowPartial)
                    {
                        stoppedBecause = "flowFailed:autothink.importVariables";
                        AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
                        WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                        summaryWritten = true;
                        return 11;
                    }
                }
            }
            else
            {
                summaries.Add(new FlowRunSummary
                {
                    Name = "autothink.importVariables",
                    Ok = false,
                    ErrorKind = "NotRun",
                    ErrorMessage = "Skipped by config",
                    Root = NormalizeRoot(importVariables.SearchRoot),
                    SelectorsFile = importVariablesProfile.SelectorsFile,
                });
            }

            var importProgramIndex = SelectorKeyIndex.FromProfile(importProgramProfile.Profile);

            if (runImportProgram)
            {
                try
                {
                    string programTextPath = ResolvePath(inputs.ProgramTextPath, configDir, required: true, "programTextPath");
                    string programText = File.ReadAllText(programTextPath);

                    var openProgramSteps = BuildDialogSteps(importProgram.OpenProgramSteps, importProgramProfile.Profile);
                    ElementSelector? editorRootSelector = ResolveSelectorOptional(importProgram.EditorRootSelector, importProgram.EditorRootSelectorKey, importProgramProfile.Profile);
                    ElementSelector editorSelector = ResolveSelector(importProgram.EditorSelector, importProgram.EditorSelectorKey, importProgramProfile.Profile, "editorSelector");
                    ElementSelector? verifySelector = ResolveSelectorOptional(importProgram.VerifySelector, importProgram.VerifySelectorKey, importProgramProfile.Profile);

                    PopupArgs importProgramPopup = BuildPopupArgs(
                        importProgram.EnablePopupHandling,
                        importProgram.PopupSearchRoot,
                        importProgram.PopupTimeoutMs,
                        importProgram.AllowPopupOk,
                        importProgram.PopupDialogSelector,
                        importProgram.PopupDialogSelectorKey,
                        importProgram.PopupOkButtonSelector,
                        importProgram.PopupOkButtonSelectorKey,
                        importProgram.PopupCancelButtonSelector,
                        importProgram.PopupCancelButtonSelectorKey,
                        popupProfile?.Profile,
                        "importProgram");

                    var importProgramArgs = new
                    {
                        programText,
                        openProgramSteps,
                        editorRootSelector,
                        editorSelector,
                        afterPasteWaitMs = importProgram.AfterPasteWaitMs,
                        verifyMode = importProgram.VerifyMode,
                        verifySelector,
                        findTimeoutMs = importProgram.FindTimeoutMs,
                        clipboardTimeoutMs = importProgram.ClipboardTimeoutMs,
                    verifyTimeoutMs = importProgram.VerifyTimeoutMs,
                    fallbackToType = importProgram.FallbackToType,
                    preferClipboard = importProgram.PreferClipboard,
                    clipboardRetry = importProgram.ClipboardRetry,
                    clipboardHealthCheck = importProgram.ClipboardHealthCheck,
                    forceFallbackOnClipboardFailure = importProgram.ForceFallbackOnClipboardFailure,
                    searchRoot = importProgram.SearchRoot,
                    enablePopupHandling = importProgramPopup.Enable,
                        popupSearchRoot = importProgramPopup.SearchRoot,
                        popupTimeoutMs = importProgramPopup.TimeoutMs,
                        allowPopupOk = importProgramPopup.AllowOk,
                        popupDialogSelector = importProgramPopup.DialogSelector,
                        popupOkButtonSelector = importProgramPopup.OkButtonSelector,
                        popupCancelButtonSelector = importProgramPopup.CancelButtonSelector,
                    };

                    FlowRunResult importProgramResult = await RunFlowWithRecoveryAsync(
                        rpc,
                        sessionId,
                        "autothink.importProgram.textPaste",
                        importProgramArgs,
                        runDir,
                        config.FlowTimeoutMs,
                        uiStateState);
                    FlowRunSummary importProgramSummary = CreateSummary(
                        "autothink.importProgram.textPaste",
                        importProgramResult,
                        NormalizeRoot(importProgram.SearchRoot),
                        importProgramProfile.SelectorsFile,
                        importProgramIndex);
                    summaries.Add(importProgramSummary);
                    if (!importProgramSummary.Ok)
                    {
                        if (!allowPartial)
                        {
                            stoppedBecause = "flowFailed:autothink.importProgram.textPaste";
                            AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
                            WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                            summaryWritten = true;
                            return 12;
                        }
                    }
                }
                catch (Exception ex) when (ex is KeyNotFoundException or InvalidOperationException or ArgumentException)
                {
                    var summary = new FlowRunSummary
                    {
                        Name = "autothink.importProgram.textPaste",
                        Ok = false,
                        ErrorKind = RpcErrorKinds.InvalidArgument,
                        ErrorMessage = ex.Message,
                        Root = NormalizeRoot(importProgram.SearchRoot),
                        SelectorsFile = importProgramProfile.SelectorsFile,
                    };
                    summaries.Add(summary);
                    if (!allowPartial)
                    {
                        stoppedBecause = "flowFailed:autothink.importProgram.textPaste";
                        AppendNotRunSummaries(summaries, selectorsRoot, profile, packVersion, config, dueToFailure: true);
                        WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                        summaryWritten = true;
                        return 12;
                    }
                }
            }
            else
            {
                summaries.Add(new FlowRunSummary
                {
                    Name = "autothink.importProgram.textPaste",
                    Ok = false,
                    ErrorKind = "NotRun",
                    ErrorMessage = "Skipped by config",
                    Root = NormalizeRoot(importProgram.SearchRoot),
                    SelectorsFile = importProgramProfile.SelectorsFile,
                });
            }

            var buildIndex = SelectorKeyIndex.FromProfile(buildProfile.Profile);

            if (runBuild)
            {
                try
                {
                    ElementSelector buildButtonSelector = ResolveSelector(build.BuildButtonSelector, build.BuildButtonSelectorKey, buildProfile.Profile, "buildButtonSelector");
                    WaitCondition? waitCondition = ResolveWaitCondition(build.WaitCondition, buildProfile.Profile);

                    if (waitCondition is null)
                    {
                        Console.Error.WriteLine("Build.WaitCondition is required.");
                        FlowRunSummary buildInvalidSummary = new()
                        {
                            Name = "autothink.build",
                            Ok = false,
                            ErrorKind = "InvalidArgument",
                            ErrorMessage = "Build.WaitCondition is required",
                            Root = NormalizeRoot(build.SearchRoot),
                            SelectorsFile = buildProfile.SelectorsFile,
                        };
                        summaries.Add(buildInvalidSummary);
                        stoppedBecause = "flowFailed:autothink.build";
                    WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                        summaryWritten = true;
                        return 13;
                    }

                    ElementSelector? optionalCloseSelector = ResolveSelectorOptional(build.OptionalCloseDialogSelector, build.OptionalCloseDialogSelectorKey, buildProfile.Profile);
                    IReadOnlyList<ElementSelector>? unexpectedSelectors = ResolveSelectorsOptional(build.UnexpectedSelectors, build.UnexpectedSelectorKeys, buildProfile.Profile);

                    PopupArgs buildPopup = BuildPopupArgs(
                        build.EnablePopupHandling,
                        build.PopupSearchRoot,
                        build.PopupTimeoutMs,
                        build.AllowPopupOk,
                        build.PopupDialogSelector,
                        build.PopupDialogSelectorKey,
                        build.PopupOkButtonSelector,
                        build.PopupOkButtonSelectorKey,
                        build.PopupCancelButtonSelector,
                        build.PopupCancelButtonSelectorKey,
                        popupProfile?.Profile,
                        "build");

                    BuildOutcomeConfig? buildOutcomeConfig = build.BuildOutcome;
                    ElementSelector? buildOutcomeSuccess = ResolveSelectorOptional(buildOutcomeConfig?.SuccessSelector, buildOutcomeConfig?.SuccessSelectorKey, buildProfile.Profile);
                    ElementSelector? buildOutcomeFailure = ResolveSelectorOptional(buildOutcomeConfig?.FailureSelector, buildOutcomeConfig?.FailureSelectorKey, buildProfile.Profile);
                    ElementSelector? buildOutcomeTextProbe = ResolveSelectorOptional(buildOutcomeConfig?.TextProbeSelector, buildOutcomeConfig?.TextProbeSelectorKey, buildProfile.Profile);

                    object? buildOutcomeArgs = null;
                    if (buildOutcomeConfig is not null)
                    {
                        buildOutcomeArgs = new
                        {
                            mode = buildOutcomeConfig.Mode,
                            successSelector = buildOutcomeSuccess,
                            failureSelector = buildOutcomeFailure,
                            textProbeSelector = buildOutcomeTextProbe,
                            successTextContains = buildOutcomeConfig.SuccessTextContains,
                            timeoutMs = buildOutcomeConfig.TimeoutMs,
                        };
                    }

                    var buildArgs = new
                    {
                        buildButtonSelector,
                        waitCondition,
                        timeoutMs = build.TimeoutMs,
                        findTimeoutMs = build.FindTimeoutMs,
                        optionalCloseDialogSelector = optionalCloseSelector,
                        unexpectedSelectors,
                        searchRoot = build.SearchRoot,
                        buildOutcome = buildOutcomeArgs,
                        enablePopupHandling = buildPopup.Enable,
                        popupSearchRoot = buildPopup.SearchRoot,
                        popupTimeoutMs = buildPopup.TimeoutMs,
                        allowPopupOk = buildPopup.AllowOk,
                        popupDialogSelector = buildPopup.DialogSelector,
                        popupOkButtonSelector = buildPopup.OkButtonSelector,
                        popupCancelButtonSelector = buildPopup.CancelButtonSelector,
                    };

                    FlowRunResult buildResult = await RunFlowWithRecoveryAsync(
                        rpc,
                        sessionId,
                        "autothink.build",
                        buildArgs,
                        runDir,
                        config.FlowTimeoutMs,
                        uiStateState);
                    BuildOutcomeReport outcomeReport = BuildBuildOutcomeReport(buildResult.Result.StepLog);
                    string outcomePath = WriteBuildOutcomeReport(runDir, outcomeReport);
                    buildSummary = new BuildSummary
                    {
                        Outcome = outcomeReport.Outcome,
                        EvidencePath = outcomePath,
                    };

                    FlowRunSummary buildFlowSummary = CreateSummary(
                        "autothink.build",
                        buildResult,
                        NormalizeRoot(build.SearchRoot),
                        buildProfile.SelectorsFile,
                        buildIndex);
                    summaries.Add(buildFlowSummary);
                    if (!buildFlowSummary.Ok)
                    {
                        if (!allowPartial)
                        {
                            stoppedBecause = "flowFailed:autothink.build";
                            WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                            summaryWritten = true;
                            return 14;
                        }
                    }
                }
                catch (Exception ex) when (ex is KeyNotFoundException or InvalidOperationException or ArgumentException)
                {
                    var summary = new FlowRunSummary
                    {
                        Name = "autothink.build",
                        Ok = false,
                        ErrorKind = RpcErrorKinds.InvalidArgument,
                        ErrorMessage = ex.Message,
                        Root = NormalizeRoot(build.SearchRoot),
                        SelectorsFile = buildProfile.SelectorsFile,
                    };
                    summaries.Add(summary);
                    if (!allowPartial)
                    {
                        stoppedBecause = "flowFailed:autothink.build";
                        WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
                        summaryWritten = true;
                        return 14;
                    }
                }
            }
            else
            {
                summaries.Add(new FlowRunSummary
                {
                    Name = "autothink.build",
                    Ok = false,
                    ErrorKind = "NotRun",
                    ErrorMessage = "Skipped by config",
                    Root = NormalizeRoot(build.SearchRoot),
                    SelectorsFile = buildProfile.SelectorsFile,
                });
            }

            WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
            summaryWritten = true;
            Console.WriteLine("Stage2Runner completed: OK");
            return 0;
        }
        finally
        {
            try
            {
                _ = await rpc.InvokeAsync<RpcResult>("CloseSession", new CloseSessionRequest { SessionId = sessionId });
            }
            catch
            {
                // ignore
            }

            if (!summaryWritten && !string.IsNullOrWhiteSpace(runDir))
            {
                WriteSummary(runDir, profile, summaries, connectivity.Report, connectivityPath, stoppedBecause, inputsSummary, buildSummary, evidencePack, runnerSteps, uiStateState);
            }

            await CleanupAsync(agent, demo: null);

            if (uiStateState?.Automation is not null)
            {
                uiStateState.Automation.Dispose();
            }

            if (uiStateState?.Application is not null)
            {
                uiStateState.Application.Dispose();
            }
        }
    }

    private static async Task<int> RunProbeAsync(
        string agentExe,
        RunnerConfig config,
        string selectorsRoot,
        string profile,
        string logsRoot,
        string configDir,
        RunnerOptions options)
    {
        string? packVersion = NormalizeSelectorPackVersion(config.SelectorPackVersion);
        Console.WriteLine($"SelectorPackVersion: {packVersion ?? "(default)"}");

        if (config.Session is null)
        {
            Console.Error.WriteLine("Config missing: session.");
            return 5;
        }

        if (string.IsNullOrWhiteSpace(options.ProbeFlow))
        {
            Console.Error.WriteLine("Probe requires --probeFlow <flowName>.");
            return 5;
        }

        string flowName = options.ProbeFlow.Trim();
        string? normalizedRoot = NormalizeProbeRoot(options.ProbeSearchRoot);
        if (normalizedRoot is null)
        {
            Console.Error.WriteLine("Probe searchRoot must be mainWindow/desktop.");
            return 5;
        }

        int timeoutMs = options.ProbeTimeoutMs > 0 ? options.ProbeTimeoutMs : 5_000;

        SelectorProfileLoadResult profileResult = LoadProfileWithOverrides(selectorsRoot, profile, flowName, packVersion);
        List<string> keys = ResolveProbeKeys(options.ProbeKeys, profileResult.Profile);

        string runDir = CreateRunDirectory(logsRoot);
        Console.WriteLine($"RunDir: {runDir}");
        Console.WriteLine($"ProbeFlow: {flowName}");
        Console.WriteLine($"ProbeKeys: {string.Join(",", keys)}");
        Console.WriteLine($"ProbeRoot: {normalizedRoot}");
        Console.WriteLine($"ProbeTimeoutMs: {timeoutMs}");

        var entries = new List<ProbeEntry>();

        using var agent = Process.Start(CreateAgentStartInfo(agentExe));

        if (agent is null)
        {
            Console.Error.WriteLine("Failed to start agent.");
            return 4;
        }

        ConnectivitySession connectivity = await ConnectAndCheckAsync(agent, agentExe, options.CheckTimeoutMs, runDir);
        string connectivityPath = WriteConnectivityReport(runDir, connectivity.Report);
        Console.WriteLine($"Connectivity: {connectivityPath}");

        if (!connectivity.Report.Ok && !options.SkipCheck)
        {
            Console.Error.WriteLine($"Connectivity failed: {connectivity.Report.Error?.Message}");
            await CleanupAsync(agent, demo: null);
            return 6;
        }

        if (connectivity.Rpc is null)
        {
            Console.Error.WriteLine("Connectivity check failed to establish RPC.");
            await CleanupAsync(agent, demo: null);
            return 6;
        }

        using var handler = connectivity.Handler;
        using var rpc = connectivity.Rpc;

        var openSessionRequest = new OpenSessionRequest
        {
            ProcessId = config.Session.ProcessId,
            ProcessName = config.Session.ProcessName,
            MainWindowTitleContains = config.Session.MainWindowTitleContains,
            TimeoutMs = config.Session.TimeoutMs > 0 ? config.Session.TimeoutMs : 10_000,
            BringToForeground = config.Session.BringToForeground,
        };

        RpcResult<OpenSessionResponse>? sessionResult = null;
        RpcError? openError = null;

        try
        {
            sessionResult = await rpc.InvokeAsync<RpcResult<OpenSessionResponse>>("OpenSession", openSessionRequest);
            DumpRpc("OpenSession", openSessionRequest, sessionResult);

            if (!sessionResult.Ok || sessionResult.Value is null)
            {
                openError = sessionResult.Error ?? new RpcError { Kind = RpcErrorKinds.ConfigError, Message = "OpenSession failed" };
            }
        }
        catch (Exception ex)
        {
            openError = new RpcError { Kind = "RpcError", Message = ex.Message };
        }

        UIA3Automation? automation = null;
        Application? app = null;
        Window? mainWindow = null;

        if (openError is null && sessionResult?.Value is not null)
        {
            if (!TryAttachApplication(sessionResult.Value.ProcessId, out app, out automation, out mainWindow, out string? attachError))
            {
                Console.Error.WriteLine($"Probe attach warning: {attachError}");
            }
        }

        try
        {
            if (openError is not null)
            {
                foreach (string key in keys)
                {
                    entries.Add(BuildProbeFailure(flowName, key, normalizedRoot, profileResult.Profile, openError));
                }
            }
            else
            {
                string sessionId = sessionResult!.Value!.SessionId;
                foreach (string key in keys)
                {
                    if (!profileResult.Profile.Selectors.TryGetValue(key, out ElementSelector? selector) || selector is null)
                    {
                        entries.Add(new ProbeEntry
                        {
                            FlowName = flowName,
                            SelectorKey = key,
                            Root = normalizedRoot,
                            Ok = false,
                            ErrorKind = RpcErrorKinds.InvalidArgument,
                            ErrorMessage = "Selector key not found",
                        });
                        continue;
                    }

                    var entry = await ProbeSelectorAsync(
                        rpc,
                        sessionId,
                        flowName,
                        key,
                        selector,
                        normalizedRoot,
                        timeoutMs,
                        automation,
                        mainWindow);
                    entries.Add(entry);
                }
            }

            string probePath = WriteProbeReport(runDir, flowName, normalizedRoot, profileResult.SelectorsFile, entries);
            Console.WriteLine($"Probe written: {probePath}");
        }
        finally
        {
            try
            {
                if (sessionResult?.Value is not null)
                {
                    _ = await rpc.InvokeAsync<RpcResult>("CloseSession", new CloseSessionRequest { SessionId = sessionResult.Value.SessionId });
                }
            }
            catch
            {
                // ignore
            }

            await CleanupAsync(agent, demo: null);

            if (automation is not null)
            {
                automation.Dispose();
            }

            if (app is not null)
            {
                app.Dispose();
            }
        }

        return openError is null ? 0 : 6;
    }

    private static async Task<FlowRunResult> RunFlowAndLogAsync(
        JsonRpc rpc,
        string sessionId,
        string flowName,
        object? args,
        string runDir,
        int flowTimeoutMs)
    {
        string? argsJson = args is null ? null : JsonSerializer.Serialize(args, JsonOptions);
        JsonElement argsElement = JsonSerializer.SerializeToElement(args, JsonOptions);
        var payload = new
        {
            sessionId,
            flowName,
            args = argsElement,
            argsJson,
            timeoutMs = flowTimeoutMs > 0 ? flowTimeoutMs : 30_000,
        };

        RpcResult<RunFlowResponse> result = await rpc.InvokeAsync<RpcResult<RunFlowResponse>>("RunFlow", payload);

        DumpRpc($"RunFlow.{flowName}", payload, result);
        string logFile = WriteStepLog(runDir, flowName, result.StepLog);

        if (!result.Ok)
        {
            Console.Error.WriteLine($"Flow failed: {flowName}");
        }

        return new FlowRunResult
        {
            FlowName = flowName,
            Result = result,
            LogFile = logFile,
        };
    }

    private static async Task<FlowRunResult> RunFlowWithRecoveryAsync(
        JsonRpc rpc,
        string sessionId,
        string flowName,
        object? args,
        string runDir,
        int flowTimeoutMs,
        UiStateRecoveryState? uiStateState)
    {
        if (uiStateState is not null && uiStateState.Enabled)
        {
            TryHandleUnexpectedUiState(uiStateState, flowName, stage: "Preflight");
        }

        FlowRunResult runResult = await RunFlowAndLogAsync(rpc, sessionId, flowName, args, runDir, flowTimeoutMs);

        if (runResult.Result.Ok || uiStateState is null || !uiStateState.Enabled)
        {
            return runResult;
        }

        if (!ShouldAttemptUiStateRecovery(runResult.Result))
        {
            return runResult;
        }

        int maxAttempts = Math.Max(1, uiStateState.MaxAttempts);
        for (int attempt = 1; attempt <= maxAttempts; attempt++)
        {
            UiStateHandlerOutcome outcome = TryHandleUnexpectedUiState(uiStateState, flowName, stage: $"Retry{attempt}");
            if (!outcome.Handled)
            {
                break;
            }

            if (!outcome.Success)
            {
                ApplyUiStateRecoveryFailure(runResult, outcome);
                return runResult;
            }

            runResult = await RunFlowAndLogAsync(rpc, sessionId, flowName, args, runDir, flowTimeoutMs);
            if (runResult.Result.Ok)
            {
                return runResult;
            }

            if (!ShouldAttemptUiStateRecovery(runResult.Result))
            {
                return runResult;
            }
        }

        return runResult;
    }

    private static UiStateRecoveryState InitializeUiStateRecovery(
        UiStateRecoveryConfig config,
        SelectorProfileFile selectorProfile,
        int processId)
    {
        var state = new UiStateRecoveryState
        {
            Enabled = config.Enable,
            MaxAttempts = config.MaxAttempts > 0 ? config.MaxAttempts : 2,
            SearchRoot = NormalizeUiStateRoot(config.SearchRoot) ?? "desktop",
            SelectorProfile = selectorProfile,
        };

        if (!state.Enabled)
        {
            return state;
        }

        if (!TryAttachApplication(processId, out Application? app, out UIA3Automation? automation, out Window? mainWindow, out string? error))
        {
            Console.Error.WriteLine($"UIStateRecovery attach failed: {error}");
            state.Enabled = false;
            return state;
        }

        state.Application = app;
        state.Automation = automation;
        state.MainWindow = mainWindow;
        return state;
    }

    private static UiStateHandlerOutcome TryHandleUnexpectedUiState(
        UiStateRecoveryState state,
        string flowName,
        string stage)
    {
        if (!state.Enabled || state.Automation is null || state.MainWindow is null || state.SelectorProfile is null)
        {
            return new UiStateHandlerOutcome { Handled = false };
        }

        state.Attempts++;
        UiStateHandlerOutcome outcome = new UiStateHandlerOutcome { Handled = false };

        foreach (Func<UiStateHandlerOutcome> handler in BuildUiStateHandlers(state, flowName, stage))
        {
            outcome = handler();
            if (!outcome.Handled)
            {
                continue;
            }

            state.Handled++;
            state.LastHandler = outcome.HandlerName;

            state.Entries.Add(new UiStateRecoveryEntry
            {
                FlowName = flowName,
                HandlerName = outcome.HandlerName,
                Stage = stage,
                SelectorKeys = outcome.SelectorKeys,
                Success = outcome.Success,
                Warning = outcome.Warning,
                ErrorMessage = outcome.ErrorMessage,
                StartedAtUtc = outcome.StartedAtUtc == default ? DateTimeOffset.UtcNow : outcome.StartedAtUtc,
                DurationMs = outcome.DurationMs,
            });

            state.RunnerSteps.Add(BuildUiStateStep(flowName, stage, outcome, state.SearchRoot));
            return outcome;
        }

        return outcome;
    }

    private static IEnumerable<Func<UiStateHandlerOutcome>> BuildUiStateHandlers(
        UiStateRecoveryState state,
        string flowName,
        string stage)
    {
        SelectorProfileFile profile = state.SelectorProfile ?? new SelectorProfileFile();

        if (TryResolveGlobalSelector(profile, "global.popupRoot", out ElementSelector? popupRoot) &&
            TryResolveGlobalSelector(profile, "global.popupNoButton", out ElementSelector? popupNo) &&
            popupRoot is not null &&
            popupNo is not null)
        {
            yield return () => TryHandlePopup(
                state,
                handlerName: "SavePrompt.No",
                flowName,
                stage,
                popupRoot,
                popupNo,
                selectorKeys: new[] { "global.popupRoot", "global.popupNoButton" },
                warning: false);
        }

        if (TryResolveGlobalSelector(profile, "global.popupRoot", out popupRoot) &&
            TryResolveGlobalSelector(profile, "global.popupOkButton", out ElementSelector? popupOk) &&
            TryResolveGlobalSelector(profile, "global.popupWarningText", out ElementSelector? warningText) &&
            popupRoot is not null &&
            popupOk is not null &&
            warningText is not null)
        {
            yield return () => TryHandlePopup(
                state,
                handlerName: "LicensePrompt.Ok",
                flowName,
                stage,
                popupRoot,
                popupOk,
                selectorKeys: new[] { "global.popupRoot", "global.popupWarningText", "global.popupOkButton" },
                warning: true,
                requiredIndicator: warningText);
        }

        if (TryResolveGlobalSelector(profile, "global.popupRoot", out popupRoot) &&
            TryResolveGlobalSelector(profile, "global.popupOkButton", out popupOk) &&
            popupRoot is not null &&
            popupOk is not null)
        {
            yield return () => TryHandlePopup(
                state,
                handlerName: "GenericOk",
                flowName,
                stage,
                popupRoot,
                popupOk,
                selectorKeys: new[] { "global.popupRoot", "global.popupOkButton" },
                warning: false);
        }
    }

    private static UiStateHandlerOutcome TryHandlePopup(
        UiStateRecoveryState state,
        string handlerName,
        string flowName,
        string stage,
        ElementSelector popupRootSelector,
        ElementSelector buttonSelector,
        IReadOnlyList<string> selectorKeys,
        bool warning,
        ElementSelector? requiredIndicator = null)
    {
        DateTimeOffset started = DateTimeOffset.UtcNow;
        var stopwatch = Stopwatch.StartNew();
        AutomationElement rootElement = ResolveUiStateRoot(state);
        bool allowFallback = !string.Equals(state.SearchRoot, "desktop", StringComparison.OrdinalIgnoreCase);
        if (!TryFindElement(rootElement, state.Automation!, popupRootSelector, out AutomationElement? popupRoot, allowFallback))
        {
            return new UiStateHandlerOutcome { Handled = false };
        }

        if (requiredIndicator is not null && !TryFindElement(popupRoot!, state.Automation!, requiredIndicator, out _, allowDesktopFallback: false))
        {
            return new UiStateHandlerOutcome { Handled = false };
        }

        if (!TryFindElement(popupRoot!, state.Automation!, buttonSelector, out AutomationElement? button, allowDesktopFallback: false))
        {
            return new UiStateHandlerOutcome { Handled = false };
        }

        try
        {
            button?.Click();
            stopwatch.Stop();
            return new UiStateHandlerOutcome
            {
                Handled = true,
                Success = true,
                Warning = warning,
                HandlerName = handlerName,
                SelectorKeys = selectorKeys,
                StartedAtUtc = started,
                DurationMs = stopwatch.ElapsedMilliseconds,
            };
        }
        catch (Exception ex)
        {
            stopwatch.Stop();
            return new UiStateHandlerOutcome
            {
                Handled = true,
                Success = false,
                Warning = warning,
                HandlerName = handlerName,
                SelectorKeys = selectorKeys,
                ErrorMessage = ex.Message,
                StartedAtUtc = started,
                DurationMs = stopwatch.ElapsedMilliseconds,
            };
        }
    }

    private static StepLogEntry BuildUiStateStep(
        string flowName,
        string stage,
        UiStateHandlerOutcome outcome,
        string root)
    {
        var step = new StepLogEntry
        {
            StepId = "UnexpectedUIState",
            Action = "Handle unexpected UI state",
            StartedAtUtc = outcome.StartedAtUtc == default ? DateTimeOffset.UtcNow : outcome.StartedAtUtc,
            FinishedAtUtc = outcome.StartedAtUtc == default
                ? DateTimeOffset.UtcNow
                : outcome.StartedAtUtc.AddMilliseconds(outcome.DurationMs),
            DurationMs = outcome.DurationMs,
            Outcome = outcome.Warning ? StepOutcomes.Warning : outcome.Success ? StepOutcomes.Success : StepOutcomes.Fail,
            Parameters = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["flowName"] = flowName,
                ["stage"] = stage,
                ["handlerName"] = outcome.HandlerName,
                ["matchedSelectorKeys"] = string.Join(",", outcome.SelectorKeys),
                ["root"] = root,
            },
        };

        if (!outcome.Success)
        {
            step.Error = new RpcError
            {
                Kind = RpcErrorKinds.UnexpectedUIState,
                Message = outcome.ErrorMessage ?? "UI state handler failed",
            };
        }

        return step;
    }

    private static void ApplyUiStateRecoveryFailure(FlowRunResult runResult, UiStateHandlerOutcome outcome)
    {
        runResult.Result.Ok = false;
        runResult.Result.Error = new RpcError
        {
            Kind = RpcErrorKinds.UnexpectedUIState,
            Message = outcome.ErrorMessage ?? $"UI state handler failed: {outcome.HandlerName}",
        };
    }

    private static bool ShouldAttemptUiStateRecovery(RpcResult<RunFlowResponse> result)
    {
        string? kind = result.Error?.Kind;
        if (string.IsNullOrWhiteSpace(kind))
        {
            return false;
        }

        return string.Equals(kind, RpcErrorKinds.FindError, StringComparison.Ordinal) ||
            string.Equals(kind, RpcErrorKinds.TimeoutError, StringComparison.Ordinal) ||
            string.Equals(kind, RpcErrorKinds.ActionError, StringComparison.Ordinal) ||
            string.Equals(kind, RpcErrorKinds.UnexpectedUIState, StringComparison.Ordinal);
    }

    private static bool TryResolveGlobalSelector(SelectorProfileFile profile, string key, out ElementSelector? selector)
    {
        if (profile.Selectors.TryGetValue(key, out ElementSelector? found) && found is not null && found.Path.Count > 0)
        {
            selector = found;
            return true;
        }

        selector = null;
        return false;
    }

    private static AutomationElement ResolveUiStateRoot(UiStateRecoveryState state)
    {
        if (string.Equals(state.SearchRoot, "desktop", StringComparison.OrdinalIgnoreCase))
        {
            return state.Automation!.GetDesktop();
        }

        return state.MainWindow!;
    }

    private static bool TryFindElement(
        AutomationElement root,
        UIA3Automation automation,
        ElementSelector selector,
        out AutomationElement? element,
        bool allowDesktopFallback)
    {
        ProbeMatchResult match = EvaluateSelector(root, selector);
        if (match.Ok && match.Element is not null)
        {
            element = match.Element;
            return true;
        }

        if (allowDesktopFallback)
        {
            ProbeMatchResult desktopMatch = EvaluateSelector(automation.GetDesktop(), selector);
            if (desktopMatch.Ok && desktopMatch.Element is not null)
            {
                element = desktopMatch.Element;
                return true;
            }
        }

        element = null;
        return false;
    }

    private static string? NormalizeUiStateRoot(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return "desktop";
        }

        string trimmed = value.Trim();
        if (string.Equals(trimmed, "mainWindow", StringComparison.OrdinalIgnoreCase))
        {
            return "mainWindow";
        }

        if (string.Equals(trimmed, "desktop", StringComparison.OrdinalIgnoreCase))
        {
            return "desktop";
        }

        return null;
    }

    private static async Task<ConnectivitySession> ConnectAndCheckAsync(
        Process agent,
        string agentPath,
        int timeoutMs,
        string runDir)
    {
        int effectiveTimeoutMs = timeoutMs > 0 ? timeoutMs : 2_000;
        var report = new ConnectivityReport
        {
            AgentPath = agentPath,
            WorkingDir = agent.StartInfo.WorkingDirectory ?? string.Empty,
        };

        report.Methods["OpenSession"] = false;
        report.Methods["CloseSession"] = false;
        report.Methods["FindElement"] = false;
        report.Methods["Click"] = false;
        report.Methods["SetText"] = false;
        report.Methods["SendKeys"] = false;
        report.Methods["WaitUntil"] = false;

        var stopwatch = Stopwatch.StartNew();
        string? handshakeLine = null;

        try
        {
            int remaining = GetRemainingMs(stopwatch, effectiveTimeoutMs);
            if (remaining <= 0)
            {
                report.Error = new ConnectivityError
                {
                    Message = "Connectivity check timed out before handshake.",
                    Hint = "Increase --timeoutMs or ensure agent starts quickly.",
                };
                report.DurationMs = stopwatch.ElapsedMilliseconds;
                return new ConnectivitySession { Report = report };
            }

            using var cts = new CancellationTokenSource(TimeSpan.FromMilliseconds(remaining));
            handshakeLine = await ReadLineAsync(agent.StandardOutput.BaseStream, cts.Token);
        }
        catch (OperationCanceledException)
        {
            report.Error = new ConnectivityError
            {
                Message = "Handshake timeout (READY not received).",
                Hint = "Ensure the agent is the correct executable and can start normally.",
            };
            report.DurationMs = stopwatch.ElapsedMilliseconds;
            return new ConnectivitySession { Report = report };
        }
        catch (Exception ex)
        {
            report.Error = new ConnectivityError
            {
                Message = $"Handshake read failed: {ex.Message}",
                Hint = "Check agent stdout and startup errors.",
            };
            report.DurationMs = stopwatch.ElapsedMilliseconds;
            return new ConnectivitySession { Report = report };
        }

        if (!string.IsNullOrWhiteSpace(handshakeLine))
        {
            report.StdoutHead.Add(handshakeLine);
        }

        report.HandshakeReady = string.Equals(handshakeLine, "READY", StringComparison.Ordinal);
        if (!report.HandshakeReady)
        {
            report.Error = new ConnectivityError
            {
                Message = "Handshake mismatch (expected READY).",
                Hint = "Agent may not be the intended UiaRpcService host.",
            };
            report.DurationMs = stopwatch.ElapsedMilliseconds;
            return new ConnectivitySession { Report = report };
        }

        var handler = new HeaderDelimitedMessageHandler(agent.StandardInput.BaseStream, agent.StandardOutput.BaseStream);
        var rpc = new JsonRpc(handler);
        rpc.StartListening();

        int remainingPing = GetRemainingMs(stopwatch, effectiveTimeoutMs);
        var pingResult = await InvokeWithTimeoutAsync(
            () => rpc.InvokeAsync<string>("Ping"),
            remainingPing);

        if (!pingResult.Completed)
        {
            report.Error = new ConnectivityError
            {
                Message = "Ping timed out.",
                Hint = "Increase --timeoutMs or check agent responsiveness.",
            };
            report.DurationMs = stopwatch.ElapsedMilliseconds;
            return new ConnectivitySession { Report = report, Handler = handler, Rpc = rpc };
        }

        if (pingResult.Exception is RemoteMethodNotFoundException)
        {
            report.Error = new ConnectivityError
            {
                Message = "Ping method not found.",
                Hint = "Agent RPC target is not UiaRpcService (wrong executable or host).",
            };
            report.DurationMs = stopwatch.ElapsedMilliseconds;
            return new ConnectivitySession { Report = report, Handler = handler, Rpc = rpc };
        }

        if (pingResult.Exception is not null)
        {
            report.Error = new ConnectivityError
            {
                Message = $"Ping failed: {pingResult.Exception.Message}",
                Hint = "Agent RPC channel is unstable or not compatible.",
            };
            report.DurationMs = stopwatch.ElapsedMilliseconds;
            return new ConnectivitySession { Report = report, Handler = handler, Rpc = rpc };
        }

        report.PingOk = string.Equals(pingResult.Result, "pong", StringComparison.OrdinalIgnoreCase);
        if (!report.PingOk)
        {
            report.Error = new ConnectivityError
            {
                Message = "Ping returned unexpected response.",
                Hint = "Agent RPC target may be incorrect.",
            };
            report.DurationMs = stopwatch.ElapsedMilliseconds;
            return new ConnectivitySession { Report = report, Handler = handler, Rpc = rpc };
        }

        await ProbeMethodAsync(
            () => rpc.InvokeAsync<RpcResult<OpenSessionResponse>>("OpenSession", new object?[] { null }),
            "OpenSession",
            report,
            stopwatch,
            effectiveTimeoutMs);

        await ProbeMethodAsync(
            () => rpc.InvokeAsync<RpcResult>("CloseSession", new object?[] { null }),
            "CloseSession",
            report,
            stopwatch,
            effectiveTimeoutMs);

        await ProbeMethodAsync(
            () => rpc.InvokeAsync<RpcResult<FindElementResponse>>("FindElement", new object?[] { null }),
            "FindElement",
            report,
            stopwatch,
            effectiveTimeoutMs);

        await ProbeMethodAsync(
            () => rpc.InvokeAsync<RpcResult>("Click", new object?[] { null }),
            "Click",
            report,
            stopwatch,
            effectiveTimeoutMs);

        await ProbeMethodAsync(
            () => rpc.InvokeAsync<RpcResult>("SetText", new object?[] { null }),
            "SetText",
            report,
            stopwatch,
            effectiveTimeoutMs);

        await ProbeMethodAsync(
            () => rpc.InvokeAsync<RpcResult>("SendKeys", new object?[] { null }),
            "SendKeys",
            report,
            stopwatch,
            effectiveTimeoutMs);

        await ProbeMethodAsync(
            () => rpc.InvokeAsync<RpcResult>("WaitUntil", new object?[] { null }),
            "WaitUntil",
            report,
            stopwatch,
            effectiveTimeoutMs);

        if (report.Methods.Values.Any(value => !value))
        {
            string missing = string.Join(", ", report.Methods.Where(kv => !kv.Value).Select(kv => kv.Key));
            report.Error = new ConnectivityError
            {
                Message = $"Missing RPC methods: {missing}",
                Hint = "You may be running the minimal AgentHost (Ping only). Use the full UiaRpcService host.",
            };
        }

        report.Ok = report.HandshakeReady && report.PingOk && report.Methods.Values.All(value => value);
        report.DurationMs = stopwatch.ElapsedMilliseconds;

        return new ConnectivitySession { Report = report, Handler = handler, Rpc = rpc };
    }

    private static async Task<ProbeEntry> ProbeSelectorAsync(
        JsonRpc rpc,
        string sessionId,
        string flowName,
        string key,
        ElementSelector selector,
        string root,
        int timeoutMs,
        UIA3Automation? automation,
        Window? mainWindow)
    {
        var entry = new ProbeEntry
        {
            FlowName = flowName,
            SelectorKey = key,
            Root = root,
            Selector = selector,
            UsedIndex = BuildUsedIndex(selector),
        };

        var sw = Stopwatch.StartNew();

        RpcResult<FindElementResponse>? rpcResult = null;
        RpcError? rpcError = null;
        bool rpcUsed = false;

        if (string.Equals(root, "mainWindow", StringComparison.OrdinalIgnoreCase))
        {
            rpcUsed = true;
            try
            {
                var request = new FindElementRequest
                {
                    SessionId = sessionId,
                    Selector = selector,
                    TimeoutMs = timeoutMs,
                };
                rpcResult = await rpc.InvokeAsync<RpcResult<FindElementResponse>>("FindElement", request);
                if (!rpcResult.Ok)
                {
                    rpcError = rpcResult.Error;
                }
            }
            catch (Exception ex)
            {
                rpcError = new RpcError { Kind = "RpcError", Message = ex.Message };
            }
        }

        ProbeMatchResult? match = null;
        ProbeMatchResult? desktopMatch = null;

        if (automation is not null && mainWindow is not null)
        {
            AutomationElement rootElement = string.Equals(root, "desktop", StringComparison.OrdinalIgnoreCase)
                ? automation.GetDesktop()
                : mainWindow;

            match = EvaluateSelector(rootElement, selector);

            if (!match.Ok && string.Equals(root, "mainWindow", StringComparison.OrdinalIgnoreCase))
            {
                desktopMatch = EvaluateSelector(automation.GetDesktop(), selector);
            }
        }

        if (match is not null)
        {
            entry.Ok = match.Ok;
            entry.MatchedCount = match.MatchedCount;
            if (!match.Ok)
            {
                entry.ErrorKind = MapProbeFailureToErrorKind(match.FailureKind);
                entry.ErrorMessage = match.FailureMessage;
            }
            else if (match.Element is not null)
            {
                entry.Element = SnapshotElement(match.Element);
            }

            entry.Suggestions = BuildProbeSuggestions(selector, match, root, desktopMatch);
        }
        else
        {
            entry.Ok = rpcResult?.Ok == true;
            if (!entry.Ok)
            {
                entry.ErrorKind = rpcError?.Kind ?? "RpcError";
                entry.ErrorMessage = rpcError?.Message ?? (rpcUsed ? "FindElement failed" : "Probe requires session");
            }
        }

        sw.Stop();
        entry.ElapsedMs = sw.ElapsedMilliseconds;
        return entry;
    }

    private static SelectorProfileLoadResult LoadProfileWithOverrides(
        string selectorsRoot,
        string profile,
        string flowName,
        string? packVersion)
    {
        var merged = new SelectorProfileFile
        {
            SchemaVersion = 1,
            Selectors = new Dictionary<string, ElementSelector>(StringComparer.Ordinal),
        };

        bool usedLocal = false;
        string? localPathUsed = null;
        var loadedFiles = new List<string>();

        (string baseBaselinePath, string baseLocalPath) = GetBaseProfilePaths(selectorsRoot, profile, packVersion);
        if (File.Exists(baseBaselinePath))
        {
            SelectorProfileFile baseProfile = LoadProfileFile(baseBaselinePath);
            MergeSelectors(merged.Selectors, baseProfile.Selectors, overwrite: true);
            Console.WriteLine($"Loaded base profile: {baseBaselinePath}");
            loadedFiles.Add(baseBaselinePath);
        }
        else if (!string.IsNullOrWhiteSpace(packVersion))
        {
            throw new FileNotFoundException($"Selector base profile not found: {baseBaselinePath}");
        }

        if (File.Exists(baseLocalPath))
        {
            SelectorProfileFile baseLocal = LoadProfileFile(baseLocalPath);
            MergeSelectors(merged.Selectors, baseLocal.Selectors, overwrite: true);
            usedLocal = true;
            localPathUsed ??= baseLocalPath;
            Console.WriteLine($"Loaded base local override: {baseLocalPath}");
            loadedFiles.Add(baseLocalPath);
        }

        (string baselinePath, string localPath) = GetProfilePaths(selectorsRoot, profile, flowName);
        SelectorProfileFile baseline = LoadProfileFile(baselinePath);
        Console.WriteLine($"Loaded baseline profile: {baselinePath}");
        MergeSelectors(merged.Selectors, baseline.Selectors, overwrite: false);
        loadedFiles.Add(baselinePath);

        if (File.Exists(localPath))
        {
            SelectorProfileFile local = LoadProfileFile(localPath);
            MergeSelectors(merged.Selectors, local.Selectors, overwrite: true);
            usedLocal = true;
            localPathUsed = localPath;
            Console.WriteLine($"Loaded local override: {localPath}");
            loadedFiles.Add(localPath);
        }

        return new SelectorProfileLoadResult
        {
            Profile = merged,
            BaselinePath = baselinePath,
            LocalPath = localPathUsed,
            UsedLocal = usedLocal,
            BaseBaselinePath = baseBaselinePath,
            BaseLocalPath = File.Exists(baseLocalPath) ? baseLocalPath : null,
            LoadedFiles = loadedFiles,
        };
    }

    private static (string baselinePath, string localPath) GetProfilePaths(string selectorsRoot, string profile, string flowName)
    {
        string suffix = GetFlowSuffix(flowName);
        string baselineFile = $"{profile}.{suffix}.json";
        string localFile = $"{profile}.{suffix}.local.json";
        string baselinePath = Path.Combine(selectorsRoot, baselineFile);
        string localPath = Path.Combine(selectorsRoot, localFile);
        return (baselinePath, localPath);
    }

    private static (string baselinePath, string localPath) GetBaseProfilePaths(string selectorsRoot, string profile, string? packVersion)
    {
        string suffix = string.IsNullOrWhiteSpace(packVersion) ? "base" : $"{packVersion}.base";
        string localSuffix = string.IsNullOrWhiteSpace(packVersion) ? "base.local" : $"{packVersion}.local";
        string baselineFile = $"{profile}.{suffix}.json";
        string localFile = $"{profile}.{localSuffix}.json";
        string baselinePath = Path.Combine(selectorsRoot, baselineFile);
        string localPath = Path.Combine(selectorsRoot, localFile);
        return (baselinePath, localPath);
    }

    private static SelectorProfileFile LoadProfileFile(string path)
    {
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"Selector profile not found: {path}");
        }

        string json = File.ReadAllText(path, Encoding.UTF8);
        SelectorProfileFile? profileFile = JsonSerializer.Deserialize<SelectorProfileFile>(json, ConfigJsonOptions);

        if (profileFile is null)
        {
            throw new InvalidOperationException($"Failed to parse selector profile: {path}");
        }

        if (profileFile.Selectors is null)
        {
            profileFile.Selectors = new Dictionary<string, ElementSelector>(StringComparer.Ordinal);
        }

        return profileFile;
    }

    private static void MergeSelectors(
        Dictionary<string, ElementSelector> target,
        Dictionary<string, ElementSelector> source,
        bool overwrite)
    {
        foreach (KeyValuePair<string, ElementSelector> kv in source)
        {
            if (overwrite || !target.ContainsKey(kv.Key))
            {
                target[kv.Key] = kv.Value;
            }
        }
    }

    private static string GetFlowSuffix(string flowName)
    {
        int dot = flowName.IndexOf('.');
        if (dot < 0 || dot == flowName.Length - 1)
        {
            return flowName;
        }

        return flowName[(dot + 1)..];
    }

    private static ElementSelector ResolveSelector(
        ElementSelector? inlineSelector,
        string? selectorKey,
        SelectorProfileFile profile,
        string context)
    {
        if (inlineSelector is not null && inlineSelector.Path.Count > 0)
        {
            return inlineSelector;
        }

        if (!string.IsNullOrWhiteSpace(selectorKey))
        {
            if (profile.Selectors.TryGetValue(selectorKey, out ElementSelector? selector))
            {
                return selector;
            }

            throw new KeyNotFoundException($"Selector key not found: {selectorKey} ({context})");
        }

        throw new InvalidOperationException($"Selector is required: {context}");
    }

    private static ElementSelector? ResolveSelectorOptional(
        ElementSelector? inlineSelector,
        string? selectorKey,
        SelectorProfileFile profile)
    {
        if (inlineSelector is not null && inlineSelector.Path.Count > 0)
        {
            return inlineSelector;
        }

        if (!string.IsNullOrWhiteSpace(selectorKey))
        {
            if (profile.Selectors.TryGetValue(selectorKey, out ElementSelector? selector))
            {
                return selector;
            }

            throw new KeyNotFoundException($"Selector key not found: {selectorKey}");
        }

        return null;
    }

    private static IReadOnlyList<ElementSelector>? ResolveSelectorsOptional(
        IReadOnlyList<ElementSelector>? inlineSelectors,
        IReadOnlyList<string>? selectorKeys,
        SelectorProfileFile profile)
    {
        if (inlineSelectors is not null && inlineSelectors.Count > 0)
        {
            return inlineSelectors;
        }

        if (selectorKeys is null || selectorKeys.Count == 0)
        {
            return null;
        }

        var result = new List<ElementSelector>(selectorKeys.Count);
        foreach (string key in selectorKeys)
        {
            if (!profile.Selectors.TryGetValue(key, out ElementSelector? selector))
            {
                throw new KeyNotFoundException($"Selector key not found: {key}");
            }

            result.Add(selector);
        }

        return result;
    }

    private static PopupArgs BuildPopupArgs(
        bool enable,
        string? popupSearchRoot,
        int popupTimeoutMs,
        bool allowPopupOk,
        ElementSelector? dialogSelector,
        string? dialogSelectorKey,
        ElementSelector? okSelector,
        string? okSelectorKey,
        ElementSelector? cancelSelector,
        string? cancelSelectorKey,
        SelectorProfileFile? popupProfile,
        string context)
    {
        string? normalizedRoot = NormalizePopupRoot(popupSearchRoot);
        if (normalizedRoot is null)
        {
            throw new InvalidOperationException($"{context}.popupSearchRoot must be mainWindow/desktop");
        }

        var result = new PopupArgs
        {
            Enable = enable,
            SearchRoot = normalizedRoot,
            TimeoutMs = popupTimeoutMs > 0 ? popupTimeoutMs : 1500,
            AllowOk = allowPopupOk,
        };

        if (!enable)
        {
            return result;
        }

        if (popupProfile is null)
        {
            throw new InvalidOperationException("Popup selector profile not loaded.");
        }

        string dialogKey = string.IsNullOrWhiteSpace(dialogSelectorKey) ? "popupDialog" : dialogSelectorKey;
        string okKey = string.IsNullOrWhiteSpace(okSelectorKey) ? "popupOkButton" : okSelectorKey;
        string cancelKey = string.IsNullOrWhiteSpace(cancelSelectorKey) ? "popupCancelButton" : cancelSelectorKey;

        result.DialogSelector = ResolveSelector(dialogSelector, dialogKey, popupProfile, $"{context}.popupDialogSelector");
        result.OkButtonSelector = ResolveSelectorOptional(okSelector, okKey, popupProfile);
        result.CancelButtonSelector = ResolveSelectorOptional(cancelSelector, cancelKey, popupProfile);

        if (result.CancelButtonSelector is null && (!result.AllowOk || result.OkButtonSelector is null))
        {
            throw new InvalidOperationException($"{context} requires popupCancelButtonSelector unless allowPopupOk is true and popupOkButtonSelector is provided");
        }

        return result;
    }

    private static IReadOnlyList<object>? BuildDialogSteps(
        IReadOnlyList<ImportDialogStepConfig>? steps,
        SelectorProfileFile profile)
    {
        if (steps is null || steps.Count == 0)
        {
            return null;
        }

        var result = new List<object>(steps.Count);
        foreach (ImportDialogStepConfig step in steps)
        {
            ElementSelector? selector = ResolveSelectorOptional(step.Selector, step.SelectorKey, profile);
            WaitCondition? condition = ResolveWaitCondition(step.Condition, profile);

            result.Add(new
            {
                action = step.Action,
                selector,
                text = step.Text,
                mode = step.Mode,
                keys = step.Keys,
                condition,
                timeoutMs = step.TimeoutMs,
            });
        }

        return result;
    }

    private static WaitCondition? ResolveWaitCondition(WaitConditionConfig? config, SelectorProfileFile profile)
    {
        if (config is null)
        {
            return null;
        }

        ElementSelector? selector = ResolveSelectorOptional(config.Selector, config.SelectorKey, profile);
        return new WaitCondition
        {
            Kind = string.IsNullOrWhiteSpace(config.Kind) ? WaitConditionKinds.ElementExists : config.Kind,
            Selector = selector,
        };
    }

    private static string ResolvePath(string? path, string baseDir, bool required, string label)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            if (required)
            {
                throw new InvalidOperationException($"Missing {label}.");
            }

            return string.Empty;
        }

        string resolved = Path.IsPathRooted(path)
            ? path
            : Path.GetFullPath(Path.Combine(baseDir, path));

        if (required && !File.Exists(resolved))
        {
            throw new FileNotFoundException($"{label} not found: {resolved}");
        }

        return resolved;
    }

    private static FlowInputsResolution ResolveInputs(RunnerConfig config, string configDir)
    {
        var resolution = new FlowInputsResolution();
        var warnings = new List<string>();

        InputsSourceConfig source = config.InputsSource ?? new InputsSourceConfig();
        string mode = string.IsNullOrWhiteSpace(source.Mode) ? "inline" : source.Mode.Trim();

        if (string.Equals(mode, "fromCommIr", StringComparison.OrdinalIgnoreCase))
        {
            resolution.Mode = "fromCommIr";
            string commIrPath;
            try
            {
                commIrPath = ResolvePath(source.CommIrPath, configDir, required: true, "inputsSource.commIrPath");
            }
            catch (Exception ex)
            {
                resolution.Ok = false;
                resolution.Error = new InputsResolutionError { Message = ex.Message };
                return resolution;
            }

            resolution.CommIrPath = commIrPath;

            CommIrReadResult read;
            try
            {
                read = CommIrReader.Read(commIrPath);
            }
            catch (Exception ex)
            {
                resolution.Ok = false;
                resolution.Error = new InputsResolutionError { Message = $"Failed to read commIr: {ex.Message}" };
                return resolution;
            }

            string commIrDir = Path.GetDirectoryName(commIrPath) ?? configDir;

            string? variablesPath = read.Inputs.VariablesFilePath;
            if (!string.IsNullOrWhiteSpace(variablesPath))
            {
                resolution.VariablesFilePath = ResolvePath(variablesPath, commIrDir, required: false, "variablesFilePath");
                resolution.VariablesSource = read.Inputs.VariablesSource;
            }
            else if (!string.IsNullOrWhiteSpace(config.VariablesFilePath))
            {
                resolution.VariablesFilePath = ResolvePath(config.VariablesFilePath, configDir, required: false, "variablesFilePath");
                resolution.VariablesSource = "config.variablesFilePath";
                warnings.Add("variablesFilePath missing in comm_ir; using config override.");
            }

            string? programPath = read.Inputs.ProgramTextPath;
            if (!string.IsNullOrWhiteSpace(programPath))
            {
                resolution.ProgramTextPath = ResolvePath(programPath, commIrDir, required: false, "programTextPath");
                resolution.ProgramSource = read.Inputs.ProgramSource;
            }
            else if (!string.IsNullOrWhiteSpace(config.ProgramTextPath))
            {
                resolution.ProgramTextPath = ResolvePath(config.ProgramTextPath, configDir, required: false, "programTextPath");
                resolution.ProgramSource = "config.programTextPath";
                warnings.Add("programTextPath missing in comm_ir; using config override.");
            }

            resolution.OutputDir = read.Inputs.OutputDir;
            resolution.ProjectName = read.Inputs.ProjectName;

            if (string.IsNullOrWhiteSpace(resolution.OutputDir))
            {
                resolution.OutputDir = TryDeriveOutputDir(commIrPath);
            }

            if (string.IsNullOrWhiteSpace(resolution.VariablesFilePath))
            {
                resolution.Ok = false;
                resolution.Error = new InputsResolutionError { Message = "variablesFilePath not resolved" };
                resolution.Warnings = warnings.Count > 0 ? warnings : null;
                return resolution;
            }

            if (string.IsNullOrWhiteSpace(resolution.ProgramTextPath))
            {
                resolution.Ok = false;
                resolution.Error = new InputsResolutionError { Message = "programTextPath not resolved" };
                resolution.Warnings = warnings.Count > 0 ? warnings : null;
                return resolution;
            }

            resolution.Ok = true;
            resolution.Warnings = warnings.Count > 0 ? warnings : null;
            return resolution;
        }

        if (!string.Equals(mode, "inline", StringComparison.OrdinalIgnoreCase))
        {
            resolution.Mode = mode;
            resolution.Ok = false;
            resolution.Error = new InputsResolutionError { Message = $"Unsupported inputsSource.mode: {mode}" };
            return resolution;
        }

        resolution.Mode = "inline";
        if (string.IsNullOrWhiteSpace(config.VariablesFilePath) || string.IsNullOrWhiteSpace(config.ProgramTextPath))
        {
            resolution.Ok = false;
            resolution.Error = new InputsResolutionError { Message = "inline inputsSource requires variablesFilePath and programTextPath" };
            return resolution;
        }

        resolution.VariablesFilePath = ResolvePath(config.VariablesFilePath, configDir, required: false, "variablesFilePath");
        resolution.ProgramTextPath = ResolvePath(config.ProgramTextPath, configDir, required: false, "programTextPath");
        resolution.VariablesSource = "config.variablesFilePath";
        resolution.ProgramSource = "config.programTextPath";
        resolution.Ok = true;
        resolution.Warnings = warnings.Count > 0 ? warnings : null;
        return resolution;
    }

    private static string WriteResolvedInputs(string runDir, FlowInputsResolution inputs)
    {
        string path = Path.Combine(runDir, "resolved_inputs.json");
        string json = JsonSerializer.Serialize(inputs, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        return path;
    }

    private static string? TryDeriveOutputDir(string commIrPath)
    {
        string? dir = Path.GetDirectoryName(commIrPath);
        if (string.IsNullOrWhiteSpace(dir))
        {
            return null;
        }

        if (string.Equals(Path.GetFileName(dir), "ir", StringComparison.OrdinalIgnoreCase))
        {
            return Path.GetDirectoryName(dir);
        }

        return null;
    }

    private static List<string> ResolveProbeKeys(string? value, SelectorProfileFile profile)
    {
        if (!string.IsNullOrWhiteSpace(value))
        {
            return value.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                .Where(key => !string.IsNullOrWhiteSpace(key))
                .Distinct(StringComparer.Ordinal)
                .ToList();
        }

        return profile.Selectors.Keys
            .OrderBy(key => key, StringComparer.Ordinal)
            .ToList();
    }

    private static string CreateRunDirectory(string logsRoot)
    {
        string timestamp = DateTimeOffset.Now.ToString("yyyyMMdd-HHmmss");
        string runDir = Path.Combine(logsRoot, timestamp);
        Directory.CreateDirectory(runDir);
        return runDir;
    }

    private static string WriteStepLog(string runDir, string flowName, StepLog stepLog)
    {
        string fileName = $"{flowName}.json";
        string path = Path.Combine(runDir, fileName);
        string json = JsonSerializer.Serialize(stepLog, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        return path;
    }

    private static string WriteConnectivityReport(string runDir, ConnectivityReport report)
    {
        string path = Path.Combine(runDir, "connectivity.json");
        string json = JsonSerializer.Serialize(report, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        return path;
    }

    private static string WriteProbeReport(
        string runDir,
        string flowName,
        string root,
        string selectorsFile,
        IReadOnlyList<ProbeEntry> entries)
    {
        var report = new ProbeReport
        {
            FlowName = flowName,
            Root = root,
            SelectorsFile = selectorsFile,
            GeneratedAtUtc = DateTimeOffset.UtcNow,
            Entries = entries,
        };

        string fileName = $"probe.{flowName}.json";
        string path = Path.Combine(runDir, fileName);
        string json = JsonSerializer.Serialize(report, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        return path;
    }

    private static string WriteSelectorCheckReport(string runDir, SelectorCheckReport report)
    {
        string path = Path.Combine(runDir, "selector_check_report.json");
        string json = JsonSerializer.Serialize(report, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        return path;
    }

    private static string WriteBuildOutcomeReport(string runDir, BuildOutcomeReport report)
    {
        string path = Path.Combine(runDir, "build_outcome.json");
        string json = JsonSerializer.Serialize(report, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        return path;
    }

    private static FlowRunSummary CreateSummary(
        string flowName,
        FlowRunResult runResult,
        string root,
        string selectorsFile,
        SelectorKeyIndex keyIndex)
    {
        var summary = new FlowRunSummary
        {
            Name = flowName,
            Ok = runResult.Result.Ok,
            ErrorKind = runResult.Result.Error?.Kind,
            ErrorMessage = runResult.Result.Error?.Message,
            Root = root,
            SelectorsFile = selectorsFile,
            LogFile = runResult.LogFile,
        };

        StepLogEntry? failed = runResult.Result.StepLog?.Steps
            .FirstOrDefault(step => string.Equals(step.Outcome, StepOutcomes.Fail, StringComparison.Ordinal));

        if (failed is not null)
        {
            summary.FailedStepId = failed.StepId;
            summary.SelectorKey = keyIndex.FindKey(failed.Selector);
            if (string.IsNullOrWhiteSpace(summary.ErrorKind) && failed.Error is not null)
            {
                summary.ErrorKind = failed.Error.Kind;
                summary.ErrorMessage = failed.Error.Message;
            }
        }

        if (!summary.Ok && string.IsNullOrWhiteSpace(summary.ErrorKind))
        {
            summary.ErrorKind = "RpcError";
        }

        if (!summary.Ok && !string.IsNullOrWhiteSpace(summary.SelectorKey))
        {
            if (string.IsNullOrWhiteSpace(summary.ErrorMessage))
            {
                summary.ErrorMessage = $"selectorKey={summary.SelectorKey}";
            }
            else if (!summary.ErrorMessage.Contains(summary.SelectorKey, StringComparison.Ordinal))
            {
                summary.ErrorMessage = $"{summary.ErrorMessage} (selectorKey={summary.SelectorKey})";
            }
        }

        summary.DurationMs = GetStepLogDuration(runResult.Result.StepLog);
        summary.PopupHandledCount = CountPopupHandled(runResult.Result.StepLog, out string? lastTitle);
        summary.LastPopupTitle = lastTitle;

        if (string.Equals(flowName, "autothink.importProgram.textPaste", StringComparison.Ordinal))
        {
            summary.Clipboard = BuildClipboardSummary(runResult.Result.StepLog);
        }

        return summary;
    }

    private static ClipboardSummary BuildClipboardSummary(StepLog? stepLog)
    {
        var summary = new ClipboardSummary();

        if (stepLog?.Steps is null || stepLog.Steps.Count == 0)
        {
            return summary;
        }

        List<StepLogEntry> attempts = stepLog.Steps
            .Where(step => step.StepId.StartsWith("SetClipboardText.Attempt", StringComparison.Ordinal))
            .ToList();

        summary.Attempted = attempts.Count > 0;
        summary.Ok = attempts.Any(step => string.Equals(step.Outcome, StepOutcomes.Success, StringComparison.Ordinal));
        summary.Retries = attempts.Count > 0 ? Math.Max(0, attempts.Count - 1) : 0;

        if (summary.Attempted && !summary.Ok)
        {
            StepLogEntry? lastAttempt = attempts.LastOrDefault();
            if (lastAttempt?.Parameters is not null && lastAttempt.Parameters.TryGetValue("failureKind", out string? failureKind))
            {
                summary.FailureKind = failureKind;
            }
            else if (lastAttempt?.Error is not null)
            {
                summary.FailureKind = lastAttempt.Error.Kind;
            }
        }

        summary.UsedFallback = stepLog.Steps.Any(step =>
            string.Equals(step.StepId, "FallbackTypeText", StringComparison.Ordinal) &&
            string.Equals(step.Outcome, StepOutcomes.Success, StringComparison.Ordinal));

        List<StepLogEntry> healthAttempts = stepLog.Steps
            .Where(step => step.StepId.StartsWith("ClipboardHealthCheck.Attempt", StringComparison.Ordinal))
            .ToList();

        summary.HealthCheckAttempted = healthAttempts.Count > 0;
        summary.HealthCheckOk = healthAttempts.Any(step => string.Equals(step.Outcome, StepOutcomes.Success, StringComparison.Ordinal));
        summary.HealthCheckRetries = healthAttempts.Count > 0 ? Math.Max(0, healthAttempts.Count - 1) : 0;

        if (summary.HealthCheckAttempted && !summary.HealthCheckOk)
        {
            StepLogEntry? lastAttempt = healthAttempts.LastOrDefault();
            if (lastAttempt?.Parameters is not null && lastAttempt.Parameters.TryGetValue("failureKind", out string? failureKind))
            {
                summary.HealthCheckFailureKind = failureKind;
            }
            else if (lastAttempt?.Error is not null)
            {
                summary.HealthCheckFailureKind = lastAttempt.Error.Kind;
            }
        }

        return summary;
    }

    private static BuildOutcomeReport BuildBuildOutcomeReport(StepLog? stepLog)
    {
        var report = new BuildOutcomeReport();

        if (stepLog?.Steps is null || stepLog.Steps.Count == 0)
        {
            report.Outcome = "Unknown";
            report.ErrorMessage = "BuildOutcome step missing";
            return report;
        }

        StepLogEntry? outcomeStep = stepLog.Steps.LastOrDefault(step =>
            string.Equals(step.StepId, "BuildOutcome", StringComparison.Ordinal));

        if (outcomeStep is null)
        {
            report.Outcome = "Unknown";
            report.ErrorMessage = "BuildOutcome step missing";
            return report;
        }

        string? outcome = GetParameter(outcomeStep.Parameters, "outcome");
        report.Outcome = string.IsNullOrWhiteSpace(outcome) ? "Unknown" : outcome;
        report.UsedMode = GetParameter(outcomeStep.Parameters, "mode");
        report.StartedAtUtc = outcomeStep.StartedAtUtc;
        report.FinishedAtUtc = outcomeStep.FinishedAtUtc;
        report.DurationMs = outcomeStep.DurationMs;

        report.SelectorEvidence = new BuildOutcomeSelectorEvidence
        {
            SuccessHit = ParseOptionalBool(GetParameter(outcomeStep.Parameters, "successHit")),
            FailureHit = ParseOptionalBool(GetParameter(outcomeStep.Parameters, "failureHit")),
        };

        report.TextEvidence = new BuildOutcomeTextEvidence
        {
            Probed = ParseOptionalBool(GetParameter(outcomeStep.Parameters, "textProbed")) ?? false,
            LastTextSample = GetParameter(outcomeStep.Parameters, "textSample"),
            MatchedToken = GetParameter(outcomeStep.Parameters, "matchedToken"),
            Source = GetParameter(outcomeStep.Parameters, "textSampleSource"),
        };

        if (outcomeStep.Error is not null)
        {
            report.ErrorKind = outcomeStep.Error.Kind;
            report.ErrorMessage = outcomeStep.Error.Message;
        }

        return report;
    }

    private static string? GetParameter(Dictionary<string, string>? parameters, string key)
    {
        if (parameters is null)
        {
            return null;
        }

        return parameters.TryGetValue(key, out string? value) ? value : null;
    }

    private static bool? ParseOptionalBool(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        return bool.TryParse(value, out bool parsed) ? parsed : null;
    }

    private static long? GetStepLogDuration(StepLog? stepLog)
    {
        if (stepLog?.Steps is null || stepLog.Steps.Count == 0)
        {
            return null;
        }

        DateTimeOffset started = stepLog.Steps.Min(step => step.StartedAtUtc);
        DateTimeOffset finished = stepLog.Steps
            .Select(step => step.FinishedAtUtc == default ? step.StartedAtUtc : step.FinishedAtUtc)
            .Max();

        return (long)(finished - started).TotalMilliseconds;
    }

    private static int CountPopupHandled(StepLog? stepLog, out string? lastTitle)
    {
        lastTitle = null;
        if (stepLog?.Steps is null || stepLog.Steps.Count == 0)
        {
            return 0;
        }

        int count = 0;
        foreach (StepLogEntry step in stepLog.Steps)
        {
            if (!step.StepId.StartsWith("PopupDismissed", StringComparison.Ordinal))
            {
                continue;
            }

            count++;
            if (step.Parameters is not null && step.Parameters.TryGetValue("title", out string? title) && !string.IsNullOrWhiteSpace(title))
            {
                lastTitle = title;
            }
        }

        return count;
    }

    private static void WriteSummary(
        string runDir,
        string profile,
        IReadOnlyList<FlowRunSummary> flows,
        ConnectivityReport? connectivity,
        string? connectivityPath,
        string? stoppedBecause,
        InputsSourceSummary? inputsSource = null,
        BuildSummary? buildSummary = null,
        EvidencePackConfig? evidencePack = null,
        IReadOnlyList<StepLogEntry>? runnerSteps = null,
        UiStateRecoveryState? uiStateState = null)
    {
        UiStateRecoverySummary? uiStateSummary = BuildUiStateRecoverySummary(runDir, uiStateState);

        var report = new SummaryReport
        {
            Profile = profile,
            RunDir = runDir,
            Flows = flows,
            GeneratedAtUtc = DateTimeOffset.UtcNow,
            ConnectivityOk = connectivity?.Ok ?? false,
            ConnectivityFailedReason = connectivity?.Error?.Message,
            ConnectivityHint = connectivity?.Error?.Hint,
            ConnectivityReport = connectivityPath,
            StoppedBecause = stoppedBecause,
            InputsSource = inputsSource,
            Build = buildSummary,
            UiStateRecovery = uiStateSummary,
        };

        string path = Path.Combine(runDir, "summary.json");
        string json = JsonSerializer.Serialize(report, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        Console.WriteLine($"Summary written: {path}");

        if (ShouldGenerateEvidencePack(evidencePack))
        {
            EvidencePackResult packResult = WriteEvidencePack(runDir, report, runnerSteps);
            Console.WriteLine($"EvidencePack: {packResult.PackDir}");
        }
    }

    private static bool ShouldGenerateEvidencePack(EvidencePackConfig? evidencePack)
    {
        return evidencePack?.Enable ?? true;
    }

    private static EvidencePackResult WriteEvidencePack(
        string runDir,
        SummaryReport report,
        IReadOnlyList<StepLogEntry>? runnerSteps)
    {
        string packDir = Path.Combine(runDir, "evidence_pack_v1");
        Directory.CreateDirectory(packDir);

        var digests = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

        CopyEvidenceFile(Path.Combine(runDir, "summary.json"), packDir, digests, required: true);
        CopyEvidenceFile(Path.Combine(runDir, "selector_check_report.json"), packDir, digests, required: true);

        string stepLogsPath = WriteStepLogBundle(runDir, report, runnerSteps);
        CopyEvidenceFile(stepLogsPath, packDir, digests, required: true);

        CopyEvidenceFile(Path.Combine(runDir, "build_outcome.json"), packDir, digests, required: false);
        CopyEvidenceFile(Path.Combine(runDir, "unexpected_ui_state.json"), packDir, digests, required: false);
        CopyEvidenceFile(Path.Combine(runDir, "resolved_inputs.json"), packDir, digests, required: false);

        EvidenceKeyMetrics metrics = BuildEvidenceKeyMetrics(runDir, report);
        var evidenceSummary = new EvidenceSummaryReport
        {
            PackVersion = "v1",
            CreatedAtUtc = DateTimeOffset.UtcNow,
            RunDir = runDir,
            Flows = report.Flows
                .Select(flow => new EvidenceFlowSummary
                {
                    Name = flow.Name,
                    Ok = flow.Ok,
                    ErrorKind = flow.ErrorKind,
                })
                .ToList(),
            Digests = digests,
            KeyMetrics = metrics,
        };

        string summaryPath = Path.Combine(packDir, "evidence_summary.v1.json");
        string summaryJson = JsonSerializer.Serialize(evidenceSummary, JsonOptions);
        File.WriteAllText(summaryPath, summaryJson, Encoding.UTF8);

        return new EvidencePackResult
        {
            PackDir = packDir,
            SummaryPath = summaryPath,
        };
    }

    private static EvidenceKeyMetrics BuildEvidenceKeyMetrics(string runDir, SummaryReport report)
    {
        int missingKeysCount = 0;
        string selectorPath = Path.Combine(runDir, "selector_check_report.json");
        SelectorCheckReport? selectorReport = File.Exists(selectorPath)
            ? ReadJsonFile<SelectorCheckReport>(selectorPath)
            : null;

        if (selectorReport?.MissingKeys is not null)
        {
            missingKeysCount = selectorReport.MissingKeys.Count;
        }

        return new EvidenceKeyMetrics
        {
            MissingKeysCount = missingKeysCount,
            BuildOutcome = report.Build?.Outcome ?? "Unknown",
        };
    }

    private static bool CopyEvidenceFile(
        string sourcePath,
        string packDir,
        Dictionary<string, string> digests,
        bool required)
    {
        if (!File.Exists(sourcePath))
        {
            if (required)
            {
                Console.Error.WriteLine($"Evidence file missing: {sourcePath}");
            }

            return false;
        }

        string fileName = Path.GetFileName(sourcePath);
        string dest = Path.Combine(packDir, fileName);
        File.Copy(sourcePath, dest, overwrite: true);
        digests[fileName] = ComputeSha256(dest);
        return true;
    }

    private static string WriteStepLogBundle(
        string runDir,
        SummaryReport report,
        IReadOnlyList<StepLogEntry>? runnerSteps)
    {
        var bundle = new StepLogBundle
        {
            GeneratedAtUtc = DateTimeOffset.UtcNow,
        };

        foreach (FlowRunSummary flow in report.Flows)
        {
            StepLog? stepLog = null;
            string logFile = flow.LogFile ?? string.Empty;
            if (!string.IsNullOrWhiteSpace(logFile) && File.Exists(logFile))
            {
                stepLog = ReadJsonFile<StepLog>(logFile);
            }

            bundle.Flows.Add(new StepLogBundleFlow
            {
                Name = flow.Name,
                LogFile = logFile,
                StepLog = stepLog,
            });
        }

        if (runnerSteps is not null && runnerSteps.Count > 0)
        {
            bundle.RunnerSteps = runnerSteps.ToList();
        }

        string path = Path.Combine(runDir, "step_logs.json");
        string json = JsonSerializer.Serialize(bundle, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        return path;
    }

    private static UiStateRecoverySummary? BuildUiStateRecoverySummary(string runDir, UiStateRecoveryState? state)
    {
        if (state is null || !state.Enabled)
        {
            return null;
        }

        if (string.IsNullOrWhiteSpace(state.EvidencePath))
        {
            state.EvidencePath = WriteUiStateRecoveryReport(runDir, state);
        }

        return new UiStateRecoverySummary
        {
            Attempts = state.Attempts,
            Handled = state.Handled,
            LastHandler = state.LastHandler,
            EvidencePath = state.EvidencePath,
        };
    }

    private static string WriteUiStateRecoveryReport(string runDir, UiStateRecoveryState state)
    {
        var report = new UiStateRecoveryReport
        {
            GeneratedAtUtc = DateTimeOffset.UtcNow,
            Attempts = state.Attempts,
            Handled = state.Handled,
            Entries = state.Entries.ToArray(),
        };

        string path = Path.Combine(runDir, "unexpected_ui_state.json");
        string json = JsonSerializer.Serialize(report, JsonOptions);
        File.WriteAllText(path, json, Encoding.UTF8);
        return path;
    }

    private static T? ReadJsonFile<T>(string path)
    {
        try
        {
            string json = File.ReadAllText(path, Encoding.UTF8);
            return JsonSerializer.Deserialize<T>(json, JsonOptions);
        }
        catch
        {
            return default;
        }
    }

    private static string ComputeSha256(string path)
    {
        using var stream = File.OpenRead(path);
        using var sha256 = SHA256.Create();
        byte[] hash = sha256.ComputeHash(stream);
        return Convert.ToHexString(hash).ToLowerInvariant();
    }

    private static SelectorCheckResult RunSelectorCheck(
        string runDir,
        string? packVersion,
        IReadOnlyDictionary<string, SelectorProfileLoadResult> profiles)
    {
        SelectorCheckReport report = BuildSelectorCheckReport(packVersion, profiles);
        string reportPath = WriteSelectorCheckReport(runDir, report);
        bool ok = report.MissingKeys.Count == 0;
        return new SelectorCheckResult
        {
            Ok = ok,
            Report = report,
            ReportPath = reportPath,
        };
    }

    private static SelectorCheckReport BuildSelectorCheckReport(
        string? packVersion,
        IReadOnlyDictionary<string, SelectorProfileLoadResult> profiles)
    {
        var report = new SelectorCheckReport
        {
            PackVersion = string.IsNullOrWhiteSpace(packVersion) ? "default" : packVersion,
        };

        List<SelectorRequirement> requirements = GetSelectorPackRequirements(packVersion).ToList();
        var flowReports = new List<SelectorCheckFlowReport>(requirements.Count);
        var requiredKeys = new List<string>();
        var missingKeys = new List<string>();

        foreach (SelectorRequirement requirement in requirements)
        {
            profiles.TryGetValue(requirement.FlowName, out SelectorProfileLoadResult? profileResult);
            Dictionary<string, ElementSelector> selectors = profileResult?.Profile.Selectors ?? new Dictionary<string, ElementSelector>(StringComparer.Ordinal);

            var flowReport = new SelectorCheckFlowReport
            {
                FlowName = requirement.FlowName,
                RequiredKeys = requirement.RequiredKeys.ToList(),
                RequiredAnyOf = requirement.RequiredAnyOf.Select(group => group.ToArray()).ToList(),
            };

            foreach (string key in requirement.RequiredKeys)
            {
                requiredKeys.Add($"{requirement.FlowName}:{key}");
                if (!selectors.ContainsKey(key))
                {
                    flowReport.MissingKeys.Add(key);
                    missingKeys.Add($"{requirement.FlowName}:{key}");
                }
            }

            foreach (string[] group in requirement.RequiredAnyOf)
            {
                string formatted = $"{requirement.FlowName}:(anyOf:{string.Join("|", group)})";
                requiredKeys.Add(formatted);
                bool hasAny = group.Any(key => selectors.ContainsKey(key));
                if (!hasAny)
                {
                    flowReport.MissingAnyOf.Add(group);
                    missingKeys.Add(formatted);
                }
            }

            flowReports.Add(flowReport);
        }

        var loadedFiles = profiles.Values
            .SelectMany(result => result.LoadedFiles)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .OrderBy(path => path, StringComparer.OrdinalIgnoreCase)
            .ToList();

        report.RequiredKeys = requiredKeys;
        report.MissingKeys = missingKeys;
        report.Flows = flowReports;
        report.LoadedFiles = loadedFiles;
        report.GeneratedAtUtc = DateTimeOffset.UtcNow;

        return report;
    }

    private static IEnumerable<SelectorRequirement> GetSelectorPackRequirements(string? packVersion)
    {
        if (!string.Equals(packVersion, "v1", StringComparison.OrdinalIgnoreCase))
        {
            return Array.Empty<SelectorRequirement>();
        }

        return new[]
        {
            new SelectorRequirement(
                "autothink.attach",
                new[] { "mainWindow" },
                Array.Empty<string[]>()),
            new SelectorRequirement(
                "autothink.importVariables",
                new[]
                {
                    "importVariablesMenuOrButton",
                    "importVariablesDialogRoot",
                    "importVariablesFilePathEdit",
                    "importVariablesOkButton",
                    "importVariablesDoneIndicator",
                },
                Array.Empty<string[]>()),
            new SelectorRequirement(
                "autothink.importProgram.textPaste",
                new[]
                {
                    "importProgramMenuOrButton",
                    "programEditorRoot",
                    "programEditorTextArea",
                    "programPastedIndicator",
                },
                Array.Empty<string[]>()),
            new SelectorRequirement(
                "autothink.build",
                new[]
                {
                    "buildButton",
                },
                new[]
                {
                    new[] { "buildOutputPane", "buildStatus" },
                    new[] { "buildSucceededIndicator", "buildFinishedIndicator" },
                }),
        };
    }

    private static string BuildMissingKeysMessage(IReadOnlyList<string> missingKeys)
    {
        if (missingKeys.Count == 0)
        {
            return "selector-check missingKeys: []";
        }

        return $"selector-check missingKeys: [{string.Join(", ", missingKeys)}]";
    }

    private static void AppendNotRunSummaries(
        List<FlowRunSummary> summaries,
        string selectorsRoot,
        string profile,
        string? packVersion,
        RunnerConfig config,
        bool dueToFailure)
    {
        ImportVariablesConfig importVariables = config.ImportVariables ?? new ImportVariablesConfig();
        ImportProgramConfig importProgram = config.ImportProgram ?? new ImportProgramConfig();
        BuildConfig build = config.Build ?? new BuildConfig();

        bool runImportVariables = !config.SkipImportVariables;
        bool runImportProgram = !config.SkipImportProgram;
        bool runBuild = !config.SkipBuild;

        AddNotRunSummary(
            summaries,
            selectorsRoot,
            profile,
            packVersion,
            "autothink.importVariables",
            NormalizeRoot(importVariables.SearchRoot),
            dueToFailure && runImportVariables ? "Skipped due to previous failure" : "Skipped by config");
        AddNotRunSummary(
            summaries,
            selectorsRoot,
            profile,
            packVersion,
            "autothink.importProgram.textPaste",
            NormalizeRoot(importProgram.SearchRoot),
            dueToFailure && runImportProgram ? "Skipped due to previous failure" : "Skipped by config");
        AddNotRunSummary(
            summaries,
            selectorsRoot,
            profile,
            packVersion,
            "autothink.build",
            NormalizeRoot(build.SearchRoot),
            dueToFailure && runBuild ? "Skipped due to previous failure" : "Skipped by config");
    }

    private static void AddNotRunSummary(
        List<FlowRunSummary> summaries,
        string selectorsRoot,
        string profile,
        string? packVersion,
        string flowName,
        string root,
        string reason)
    {
        if (summaries.Any(summary => string.Equals(summary.Name, flowName, StringComparison.Ordinal)))
        {
            return;
        }

        SelectorProfileLoadResult profileResult = LoadProfileWithOverrides(selectorsRoot, profile, flowName, packVersion);
        summaries.Add(new FlowRunSummary
        {
            Name = flowName,
            Ok = false,
            ErrorKind = "NotRun",
            ErrorMessage = reason,
            Root = root,
            SelectorsFile = profileResult.SelectorsFile,
        });
    }

    private static ProbeEntry BuildProbeFailure(
        string flowName,
        string key,
        string root,
        SelectorProfileFile profile,
        RpcError error)
    {
        var entry = new ProbeEntry
        {
            FlowName = flowName,
            SelectorKey = key,
            Root = root,
            Ok = false,
            ErrorKind = error.Kind,
            ErrorMessage = error.Message,
        };

        if (profile.Selectors.TryGetValue(key, out ElementSelector? selector))
        {
            entry.Selector = selector;
            entry.UsedIndex = BuildUsedIndex(selector);
        }
        else
        {
            entry.ErrorKind = RpcErrorKinds.InvalidArgument;
            entry.ErrorMessage = "Selector key not found";
        }

        return entry;
    }

    private static string NormalizeRoot(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return "mainWindow";
        }

        if (string.Equals(value.Trim(), "desktop", StringComparison.OrdinalIgnoreCase))
        {
            return "desktop";
        }

        return "mainWindow";
    }

    private static string? NormalizeSelectorPackVersion(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        return value.Trim();
    }

    private static string? NormalizeProbeRoot(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return "mainWindow";
        }

        string trimmed = value.Trim();
        if (string.Equals(trimmed, "desktop", StringComparison.OrdinalIgnoreCase))
        {
            return "desktop";
        }

        if (string.Equals(trimmed, "mainWindow", StringComparison.OrdinalIgnoreCase))
        {
            return "mainWindow";
        }

        return null;
    }

    private static string? NormalizePopupRoot(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return "desktop";
        }

        string trimmed = value.Trim();
        if (string.Equals(trimmed, "desktop", StringComparison.OrdinalIgnoreCase))
        {
            return "desktop";
        }

        if (string.Equals(trimmed, "mainWindow", StringComparison.OrdinalIgnoreCase))
        {
            return "mainWindow";
        }

        return null;
    }

    private static RunnerConfig LoadConfig(string path, out string configDir)
    {
        string fullPath = Path.GetFullPath(path);
        configDir = Path.GetDirectoryName(fullPath) ?? Directory.GetCurrentDirectory();

        string json = File.ReadAllText(fullPath, Encoding.UTF8);
        RunnerConfig? config = JsonSerializer.Deserialize<RunnerConfig>(json, ConfigJsonOptions);
        if (config is null)
        {
            throw new InvalidOperationException($"Failed to parse config: {fullPath}");
        }

        return config;
    }

    private static string ResolveSelectorsRoot(RunnerOptions options, RunnerConfig config, string repoRoot, string configDir)
    {
        string? value = options.SelectorsRoot ?? config.SelectorsRoot;
        if (string.IsNullOrWhiteSpace(value))
        {
            value = Path.Combine(repoRoot, "Docs", "组态软件自动操作", "Selectors");
        }

        return Path.IsPathRooted(value)
            ? value
            : Path.GetFullPath(Path.Combine(configDir, value));
    }

    private static string ResolveProfile(RunnerOptions options, RunnerConfig config)
    {
        string? value = options.Profile ?? config.Profile;
        return string.IsNullOrWhiteSpace(value) ? "autothink" : value.Trim();
    }

    private static string ResolveLogsRoot(RunnerConfig config, string repoRoot, string configDir)
    {
        string? value = config.LogsRoot;
        if (string.IsNullOrWhiteSpace(value))
        {
            value = Path.Combine(repoRoot, "logs");
        }

        return Path.IsPathRooted(value)
            ? value
            : Path.GetFullPath(Path.Combine(configDir, value));
    }

    private static string ResolveAgentPath(string? cliOverride, string? configOverride, string repoRoot, string configDir)
    {
        string? value = !string.IsNullOrWhiteSpace(cliOverride) ? cliOverride : configOverride;
        string baseDir = !string.IsNullOrWhiteSpace(cliOverride) ? repoRoot : configDir;

        if (string.IsNullOrWhiteSpace(value))
        {
            value = Path.Combine(repoRoot, "Autothink.UiaAgent", "bin", "Release", "net8.0-windows", "Autothink.UiaAgent.exe");
            baseDir = repoRoot;
        }

        string path = value.Trim();
        if (!Path.IsPathRooted(path))
        {
            path = Path.GetFullPath(Path.Combine(baseDir, path));
        }

        return path;
    }

    private static ProcessStartInfo CreateAgentStartInfo(string agentExe)
    {
        return new ProcessStartInfo
        {
            FileName = agentExe,
            WorkingDirectory = Path.GetDirectoryName(agentExe) ?? Directory.GetCurrentDirectory(),
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8,
        };
    }

    private static void PrintUsage()
    {
        Console.WriteLine("Stage2Runner usage:");
        Console.WriteLine("  --config <path>         Run config-based flow chain");
        Console.WriteLine("  --selectorsRoot <path>  Override selector profile root");
        Console.WriteLine("  --profile <name>        Selector profile prefix (default: autothink)");
        Console.WriteLine("  --agentPath <path>      Override agent executable path");
        Console.WriteLine("  --verify               Verify evidence pack (use with --evidence)");
        Console.WriteLine("  --evidence <dir>        Evidence pack directory for verify");
        Console.WriteLine("  --probe                Run selector probe mode (requires --config)");
        Console.WriteLine("  --probeFlow <flow>      Flow name for probe (e.g. autothink.build)");
        Console.WriteLine("  --probeKeys <k1,k2>     Selector keys to probe (default: all keys)");
        Console.WriteLine("  --probeSearchRoot <r>   mainWindow/desktop (default: mainWindow)");
        Console.WriteLine("  --probeTimeoutMs <ms>   Find timeout (default: 5000)");
        Console.WriteLine("  --check                Run connectivity check only");
        Console.WriteLine("  --skipCheck            Skip connectivity check before running flows/probe");
        Console.WriteLine("  --timeoutMs <ms>        Connectivity timeout (default: 2000)");
        Console.WriteLine("  --demo                  Run DemoTarget scenario");
        Console.WriteLine("  --help                  Show usage");
    }

    private static string FindRepoRoot()
    {
        string dir = AppContext.BaseDirectory;
        while (!string.IsNullOrWhiteSpace(dir))
        {
            if (File.Exists(Path.Combine(dir, "PLCCodeForge.sln")))
            {
                return dir;
            }

            string parent = Path.GetFullPath(Path.Combine(dir, ".."));
            if (string.Equals(parent, dir, StringComparison.OrdinalIgnoreCase))
            {
                break;
            }

            dir = parent;
        }

        return Directory.GetCurrentDirectory();
    }

    private static void DumpRpc(string title, object requestParams, object response)
    {
        Console.WriteLine();
        Console.WriteLine($"=== {title} ===");
        Console.WriteLine("Request:");
        Console.WriteLine(JsonSerializer.Serialize(requestParams, JsonOptions));
        Console.WriteLine("Response:");
        Console.WriteLine(JsonSerializer.Serialize(response, JsonOptions));
    }

    private static bool TryAttachApplication(
        int processId,
        out Application? app,
        out UIA3Automation? automation,
        out Window? mainWindow,
        out string? error)
    {
        app = null;
        automation = null;
        mainWindow = null;
        error = null;

        if (processId <= 0)
        {
            error = "ProcessId is missing.";
            return false;
        }

        try
        {
            app = Application.Attach(processId);
            automation = new UIA3Automation();
            mainWindow = app.GetMainWindow(automation);
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            if (automation is not null)
            {
                automation.Dispose();
            }

            if (app is not null)
            {
                app.Dispose();
            }

            app = null;
            automation = null;
            mainWindow = null;
            return false;
        }
    }

    private static async Task<string> ReadLineAsync(Stream stream, CancellationToken cancellationToken)
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
                throw new InvalidOperationException("READY line too long.");
            }
        }

        return Encoding.UTF8.GetString(bytes.ToArray());
    }

    private static int GetRemainingMs(Stopwatch stopwatch, int totalMs)
    {
        int remaining = totalMs - (int)stopwatch.ElapsedMilliseconds;
        return remaining > 0 ? remaining : 0;
    }

    private static async Task<InvokeResult<T>> InvokeWithTimeoutAsync<T>(Func<Task<T>> action, int timeoutMs)
    {
        var result = new InvokeResult<T>();

        if (timeoutMs <= 0)
        {
            result.Completed = false;
            return result;
        }

        try
        {
            Task<T> task = action();
            Task delay = Task.Delay(timeoutMs);
            Task completed = await Task.WhenAny(task, delay);
            if (completed != task)
            {
                result.Completed = false;
                return result;
            }

            result.Result = await task;
            result.Completed = true;
            return result;
        }
        catch (Exception ex)
        {
            result.Completed = true;
            result.Exception = ex;
            return result;
        }
    }

    private static async Task ProbeMethodAsync(
        Func<Task> call,
        string methodName,
        ConnectivityReport report,
        Stopwatch stopwatch,
        int timeoutMs)
    {
        int remaining = GetRemainingMs(stopwatch, timeoutMs);
        if (remaining <= 0)
        {
            report.Methods[methodName] = false;
            report.Error ??= new ConnectivityError
            {
                Message = "Connectivity check timed out while probing methods.",
                Hint = "Increase --timeoutMs or reduce startup load.",
            };
            return;
        }

        InvokeResult<int> invokeResult = await InvokeWithTimeoutAsync(async () =>
        {
            await call();
            return 0;
        }, remaining);

        if (!invokeResult.Completed)
        {
            report.Methods[methodName] = false;
            report.Error ??= new ConnectivityError
            {
                Message = "Connectivity check timed out while probing methods.",
                Hint = "Increase --timeoutMs or reduce startup load.",
            };
            return;
        }

        if (invokeResult.Exception is RemoteMethodNotFoundException)
        {
            report.Methods[methodName] = false;
            return;
        }

        report.Methods[methodName] = true;
    }

    private static string? BuildUsedIndex(ElementSelector selector)
    {
        if (selector.Path is null || selector.Path.Count == 0)
        {
            return null;
        }

        var parts = new List<string>();
        for (int i = 0; i < selector.Path.Count; i++)
        {
            if (selector.Path[i].Index is int index)
            {
                parts.Add($"step{i}:{index}");
            }
        }

        return parts.Count == 0 ? null : string.Join(";", parts);
    }

    private static ProbeMatchResult EvaluateSelector(AutomationElement root, ElementSelector selector)
    {
        var result = new ProbeMatchResult();

        if (selector.Path is null || selector.Path.Count == 0)
        {
            result.Ok = false;
            result.FailureKind = ProbeFailureKinds.InvalidSelector;
            result.FailureMessage = "Selector.Path must contain at least one step.";
            return result;
        }

        AutomationElement current = root;
        int? lastMatchedCount = null;

        for (int i = 0; i < selector.Path.Count; i++)
        {
            SelectorStep step = selector.Path[i];

            ControlType? controlTypeFilter = null;
            if (!string.IsNullOrWhiteSpace(step.ControlType))
            {
                if (!ProbeControlTypeMap.TryGetValue(step.ControlType.Trim(), out ControlType mapped))
                {
                    result.Ok = false;
                    result.FailureKind = ProbeFailureKinds.InvalidControlType;
                    result.FailureMessage = $"Invalid ControlType: {step.ControlType}";
                    return result;
                }

                controlTypeFilter = mapped;
            }

            bool hasAnyFilter =
                !string.IsNullOrWhiteSpace(step.AutomationId) ||
                !string.IsNullOrWhiteSpace(step.AutomationIdContains) ||
                !string.IsNullOrWhiteSpace(step.Name) ||
                !string.IsNullOrWhiteSpace(step.NameContains) ||
                !string.IsNullOrWhiteSpace(step.ClassName) ||
                !string.IsNullOrWhiteSpace(step.ClassNameContains) ||
                controlTypeFilter is not null;

            if (!hasAnyFilter)
            {
                result.Ok = false;
                result.FailureKind = ProbeFailureKinds.InvalidSelector;
                result.FailureMessage = $"Step {i} has no filters.";
                return result;
            }

            AutomationElement[] candidates = step.Search switch
            {
                SelectorSearchKinds.Children => current.FindAllChildren(),
                SelectorSearchKinds.Descendants => current.FindAllDescendants(),
                _ => current.FindAllDescendants(),
            };

            AutomationElement[] matches = candidates
                .Where(e => MatchesStep(e, step, controlTypeFilter))
                .ToArray();

            lastMatchedCount = matches.Length;

            if (matches.Length == 0)
            {
                result.Ok = false;
                result.FailureKind = ProbeFailureKinds.NotFound;
                result.FailureMessage = $"No matches at step {i}.";
                result.MatchedCount = 0;
                return result;
            }

            AutomationElement selected;
            if (step.Index is int index)
            {
                if (index < 0 || index >= matches.Length)
                {
                    result.Ok = false;
                    result.FailureKind = ProbeFailureKinds.IndexOutOfRange;
                    result.FailureMessage = $"Index {index} out of range at step {i}.";
                    result.MatchedCount = matches.Length;
                    return result;
                }

                selected = matches[index];
            }
            else
            {
                if (matches.Length != 1)
                {
                    result.Ok = false;
                    result.FailureKind = ProbeFailureKinds.Ambiguous;
                    result.FailureMessage = $"Multiple matches ({matches.Length}) at step {i}.";
                    result.MatchedCount = matches.Length;
                    return result;
                }

                selected = matches[0];
            }

            current = selected;
        }

        result.Ok = true;
        result.Element = current;
        result.MatchedCount = lastMatchedCount;
        return result;
    }

    private static bool MatchesStep(AutomationElement element, SelectorStep step, ControlType? controlTypeFilter)
    {
        if (!MatchesText(
                element.Properties.AutomationId.ValueOrDefault,
                step.AutomationId,
                step.AutomationIdContains,
                step.IgnoreCase))
        {
            return false;
        }

        if (!MatchesText(
                element.Properties.Name.ValueOrDefault,
                step.Name,
                step.NameContains,
                step.IgnoreCase,
                normalizeWhitespace: step.NormalizeWhitespace))
        {
            return false;
        }

        if (!MatchesText(
                element.Properties.ClassName.ValueOrDefault,
                step.ClassName,
                step.ClassNameContains,
                step.IgnoreCase))
        {
            return false;
        }

        if (controlTypeFilter is not null &&
            element.Properties.ControlType.ValueOrDefault != controlTypeFilter)
        {
            return false;
        }

        return true;
    }

    private static bool MatchesText(
        string? actual,
        string? expectedExact,
        string? expectedContains,
        bool ignoreCase,
        bool normalizeWhitespace = false)
    {
        StringComparison comparison = ignoreCase ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal;
        string value = actual ?? string.Empty;
        string? exact = expectedExact;
        string? contains = expectedContains;

        if (normalizeWhitespace)
        {
            value = NormalizeWhitespace(value);
            exact = NormalizeWhitespace(exact);
            contains = NormalizeWhitespace(contains);
        }

        if (!string.IsNullOrWhiteSpace(exact))
        {
            return string.Equals(value, exact, comparison);
        }

        if (!string.IsNullOrWhiteSpace(contains))
        {
            return value.Contains(contains, comparison);
        }

        return true;
    }

    private static string NormalizeWhitespace(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return string.Empty;
        }

        var sb = new StringBuilder(value.Length);
        bool inWhitespace = false;

        foreach (char c in value)
        {
            if (char.IsWhiteSpace(c))
            {
                if (!inWhitespace)
                {
                    sb.Append(' ');
                    inWhitespace = true;
                }

                continue;
            }

            sb.Append(c);
            inWhitespace = false;
        }

        return sb.ToString().Trim();
    }

    private static string MapProbeFailureToErrorKind(string? failureKind)
    {
        return failureKind switch
        {
            ProbeFailureKinds.InvalidSelector => RpcErrorKinds.InvalidArgument,
            ProbeFailureKinds.InvalidControlType => RpcErrorKinds.InvalidArgument,
            ProbeFailureKinds.IndexOutOfRange => RpcErrorKinds.InvalidArgument,
            _ => RpcErrorKinds.FindError,
        };
    }

    private static ProbeElementSnapshot SnapshotElement(AutomationElement element)
    {
        ControlType controlType = element.Properties.ControlType.ValueOrDefault;
        string controlTypeName = controlType.ToString();

        string? rect = null;
        try
        {
            var bounds = element.Properties.BoundingRectangle.ValueOrDefault;
            rect = $"{bounds.Left},{bounds.Top},{bounds.Width},{bounds.Height}";
        }
        catch
        {
            rect = null;
        }

        return new ProbeElementSnapshot
        {
            Name = element.Properties.Name.ValueOrDefault,
            ClassName = element.Properties.ClassName.ValueOrDefault,
            AutomationId = element.Properties.AutomationId.ValueOrDefault,
            ControlType = controlTypeName,
            IsEnabled = element.Properties.IsEnabled.ValueOrDefault,
            BoundingRect = rect,
        };
    }

    private static IReadOnlyList<string>? BuildProbeSuggestions(
        ElementSelector selector,
        ProbeMatchResult match,
        string root,
        ProbeMatchResult? desktopMatch)
    {
        var suggestions = new List<string>();

        if (!match.Ok)
        {
            if (string.Equals(match.FailureKind, ProbeFailureKinds.Ambiguous, StringComparison.Ordinal))
            {
                suggestions.Add("Matches multiple elements; add SelectorStep.Index or tighten contains filters.");
            }
            else if (string.Equals(match.FailureKind, ProbeFailureKinds.NotFound, StringComparison.Ordinal))
            {
                if (string.Equals(root, "mainWindow", StringComparison.OrdinalIgnoreCase) && desktopMatch?.Ok == true)
                {
                    suggestions.Add("Try searchRoot=desktop in flow args/RunnerConfig.");
                }

                suggestions.Add("Consider NameContains/IgnoreCase/NormalizeWhitespace or AutomationIdContains/ClassNameContains.");
            }
            else if (string.Equals(match.FailureKind, ProbeFailureKinds.IndexOutOfRange, StringComparison.Ordinal))
            {
                suggestions.Add("Index out of range; adjust SelectorStep.Index to 0..matches-1.");
            }
            else if (string.Equals(match.FailureKind, ProbeFailureKinds.InvalidControlType, StringComparison.Ordinal))
            {
                suggestions.Add("ControlType is invalid; use a valid FlaUI control type name.");
            }
            else if (string.Equals(match.FailureKind, ProbeFailureKinds.InvalidSelector, StringComparison.Ordinal))
            {
                suggestions.Add("Selector step must include at least one filter.");
            }
        }
        else if (match.MatchedCount is > 1)
        {
            suggestions.Add("Multiple matches detected; keep Index or tighten contains filters.");
        }

        if (match.Element is not null)
        {
            string? name = match.Element.Properties.Name.ValueOrDefault;
            bool hasWhitespaceVariance = HasWhitespaceVariance(name);

            foreach (SelectorStep step in selector.Path)
            {
                if (!string.IsNullOrWhiteSpace(step.NameContains))
                {
                    if (!step.IgnoreCase)
                    {
                        suggestions.Add("NameContains may benefit from IgnoreCase=true.");
                    }

                    if (!step.NormalizeWhitespace && hasWhitespaceVariance)
                    {
                        suggestions.Add("Enable NormalizeWhitespace for NameContains to reduce whitespace variance.");
                    }
                }
            }
        }

        return suggestions.Count == 0 ? null : suggestions;
    }

    private static bool HasWhitespaceVariance(string? value)
    {
        if (string.IsNullOrEmpty(value))
        {
            return false;
        }

        bool prevWhite = false;
        foreach (char c in value)
        {
            if (char.IsWhiteSpace(c))
            {
                if (prevWhite || c == '\n' || c == '\r' || c == '\t')
                {
                    return true;
                }

                prevWhite = true;
            }
            else
            {
                prevWhite = false;
            }
        }

        return false;
    }

    private static async Task CleanupAsync(Process? agent, Process? demo)
    {
        try
        {
            if (agent is not null && !agent.HasExited)
            {
                agent.Kill(entireProcessTree: true);
            }
        }
        catch
        {
            // ignore
        }

        try
        {
            if (demo is not null && !demo.HasExited)
            {
                demo.Kill(entireProcessTree: true);
            }
        }
        catch
        {
            // ignore
        }

        try { if (agent is not null) await agent.WaitForExitAsync(CancellationToken.None); } catch { }
        try { if (demo is not null) await demo.WaitForExitAsync(CancellationToken.None); } catch { }
    }

    private static readonly Dictionary<string, ControlType> ProbeControlTypeMap = BuildProbeControlTypeMap();

    private static Dictionary<string, ControlType> BuildProbeControlTypeMap()
    {
        var map = new Dictionary<string, ControlType>(StringComparer.OrdinalIgnoreCase);

        foreach (PropertyInfo prop in typeof(ControlType).GetProperties(BindingFlags.Public | BindingFlags.Static))
        {
            if (prop.PropertyType != typeof(ControlType))
            {
                continue;
            }

            if (prop.GetValue(null) is ControlType ct)
            {
                map[ prop.Name ] = ct;
            }
        }

        foreach (FieldInfo field in typeof(ControlType).GetFields(BindingFlags.Public | BindingFlags.Static))
        {
            if (field.FieldType != typeof(ControlType))
            {
                continue;
            }

            if (field.GetValue(null) is ControlType ct)
            {
                map[ field.Name ] = ct;
            }
        }

        return map;
    }

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        Converters = { new JsonElementSafeConverter() },
    };

    private static readonly JsonSerializerOptions ConfigJsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    private static readonly JsonSerializerOptions SelectorJsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    private sealed class JsonElementSafeConverter : JsonConverter<JsonElement>
    {
        public override JsonElement Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
        {
            return JsonDocument.ParseValue(ref reader).RootElement.Clone();
        }

        public override void Write(Utf8JsonWriter writer, JsonElement value, JsonSerializerOptions options)
        {
            try
            {
                value.WriteTo(writer);
            }
            catch
            {
                writer.WriteStringValue("[unavailable]");
            }
        }
    }

    private sealed class FlowRunResult
    {
        public string FlowName { get; set; } = string.Empty;

        public RpcResult<RunFlowResponse> Result { get; set; } = new();

        public string LogFile { get; set; } = string.Empty;
    }

    private sealed class ConnectivitySession
    {
        public HeaderDelimitedMessageHandler? Handler { get; set; }

        public JsonRpc? Rpc { get; set; }

        public ConnectivityReport Report { get; set; } = new();
    }

    private sealed class ConnectivityReport
    {
        public bool Ok { get; set; }

        public string AgentPath { get; set; } = string.Empty;

        public string WorkingDir { get; set; } = string.Empty;

        public bool HandshakeReady { get; set; }

        public bool PingOk { get; set; }

        public Dictionary<string, bool> Methods { get; set; } = new(StringComparer.Ordinal);

        public List<string> StdoutHead { get; set; } = new();

        public ConnectivityError? Error { get; set; }

        public long DurationMs { get; set; }
    }

    private sealed class ConnectivityError
    {
        public string Kind { get; set; } = RpcErrorKinds.ConfigError;

        public string Message { get; set; } = string.Empty;

        public string? Hint { get; set; }
    }

    private sealed class ProbeReport
    {
        public string FlowName { get; set; } = string.Empty;

        public string Root { get; set; } = "mainWindow";

        public string SelectorsFile { get; set; } = string.Empty;

        public IReadOnlyList<ProbeEntry> Entries { get; set; } = Array.Empty<ProbeEntry>();

        public DateTimeOffset GeneratedAtUtc { get; set; }
    }

    private sealed class ProbeEntry
    {
        public string FlowName { get; set; } = string.Empty;

        public string SelectorKey { get; set; } = string.Empty;

        public string Root { get; set; } = "mainWindow";

        public bool Ok { get; set; }

        public string? ErrorKind { get; set; }

        public string? ErrorMessage { get; set; }

        public int? MatchedCount { get; set; }

        public string? UsedIndex { get; set; }

        public long ElapsedMs { get; set; }

        public ElementSelector? Selector { get; set; }

        public ProbeElementSnapshot? Element { get; set; }

        public IReadOnlyList<string>? Suggestions { get; set; }
    }

    private sealed class ProbeElementSnapshot
    {
        public string? Name { get; set; }

        public string? ClassName { get; set; }

        public string? AutomationId { get; set; }

        public string? ControlType { get; set; }

        public bool? IsEnabled { get; set; }

        public string? BoundingRect { get; set; }
    }

    private sealed class ProbeMatchResult
    {
        public bool Ok { get; set; }

        public string? FailureKind { get; set; }

        public string? FailureMessage { get; set; }

        public int? MatchedCount { get; set; }

        public AutomationElement? Element { get; set; }
    }

    private sealed class InvokeResult<T>
    {
        public bool Completed { get; set; }

        public T? Result { get; set; }

        public Exception? Exception { get; set; }
    }

    private sealed class PopupArgs
    {
        public bool Enable { get; set; }

        public string SearchRoot { get; set; } = "desktop";

        public int TimeoutMs { get; set; } = 1500;

        public bool AllowOk { get; set; }

        public ElementSelector? DialogSelector { get; set; }

        public ElementSelector? OkButtonSelector { get; set; }

        public ElementSelector? CancelButtonSelector { get; set; }
    }

    private static class ProbeFailureKinds
    {
        public const string InvalidSelector = "InvalidSelector";
        public const string InvalidControlType = "InvalidControlType";
        public const string NotFound = "NotFound";
        public const string Ambiguous = "Ambiguous";
        public const string IndexOutOfRange = "IndexOutOfRange";
    }

    private sealed class FlowRunSummary
    {
        public string Name { get; set; } = string.Empty;

        public bool Ok { get; set; }

        public string? ErrorKind { get; set; }

        public string? ErrorMessage { get; set; }

        public string? FailedStepId { get; set; }

        public string? SelectorKey { get; set; }

        public string Root { get; set; } = "mainWindow";

        public string SelectorsFile { get; set; } = string.Empty;

        public string LogFile { get; set; } = string.Empty;

        public long? DurationMs { get; set; }

        public int PopupHandledCount { get; set; }

        public string? LastPopupTitle { get; set; }

        public ClipboardSummary? Clipboard { get; set; }
    }

    private sealed class ClipboardSummary
    {
        public bool Attempted { get; set; }

        public bool Ok { get; set; }

        public string? FailureKind { get; set; }

        public int Retries { get; set; }

        public bool UsedFallback { get; set; }

        public bool HealthCheckAttempted { get; set; }

        public bool HealthCheckOk { get; set; }

        public string? HealthCheckFailureKind { get; set; }

        public int HealthCheckRetries { get; set; }
    }

    private sealed class SummaryReport
    {
        public string Profile { get; set; } = string.Empty;

        public string RunDir { get; set; } = string.Empty;

        public IReadOnlyList<FlowRunSummary> Flows { get; set; } = Array.Empty<FlowRunSummary>();

        public DateTimeOffset GeneratedAtUtc { get; set; }

        public bool ConnectivityOk { get; set; }

        public string? ConnectivityFailedReason { get; set; }

        public string? ConnectivityHint { get; set; }

        public string? ConnectivityReport { get; set; }

        public string? StoppedBecause { get; set; }

        public InputsSourceSummary? InputsSource { get; set; }

        public BuildSummary? Build { get; set; }

        public UiStateRecoverySummary? UiStateRecovery { get; set; }
    }

    private sealed class EvidencePackResult
    {
        public string PackDir { get; set; } = string.Empty;

        public string SummaryPath { get; set; } = string.Empty;
    }

    private sealed class EvidenceSummaryReport
    {
        public string PackVersion { get; set; } = "v1";

        public DateTimeOffset CreatedAtUtc { get; set; }

        public string RunDir { get; set; } = string.Empty;

        public IReadOnlyList<EvidenceFlowSummary> Flows { get; set; } = Array.Empty<EvidenceFlowSummary>();

        public Dictionary<string, string> Digests { get; set; } = new(StringComparer.OrdinalIgnoreCase);

        public EvidenceKeyMetrics KeyMetrics { get; set; } = new();
    }

    private sealed class EvidenceFlowSummary
    {
        public string Name { get; set; } = string.Empty;

        public bool Ok { get; set; }

        public string? ErrorKind { get; set; }
    }

    private sealed class EvidenceKeyMetrics
    {
        public int MissingKeysCount { get; set; }

        public string BuildOutcome { get; set; } = "Unknown";
    }

    private sealed class UiStateRecoverySummary
    {
        public int Attempts { get; set; }

        public int Handled { get; set; }

        public string? LastHandler { get; set; }

        public string? EvidencePath { get; set; }
    }

    private sealed class UiStateRecoveryReport
    {
        public DateTimeOffset GeneratedAtUtc { get; set; }

        public int Attempts { get; set; }

        public int Handled { get; set; }

        public IReadOnlyList<UiStateRecoveryEntry> Entries { get; set; } = Array.Empty<UiStateRecoveryEntry>();
    }

    private sealed class UiStateRecoveryEntry
    {
        public string FlowName { get; set; } = string.Empty;

        public string HandlerName { get; set; } = string.Empty;

        public string Stage { get; set; } = string.Empty;

        public IReadOnlyList<string> SelectorKeys { get; set; } = Array.Empty<string>();

        public bool Success { get; set; }

        public bool Warning { get; set; }

        public string? ErrorMessage { get; set; }

        public DateTimeOffset StartedAtUtc { get; set; }

        public long DurationMs { get; set; }
    }

    private sealed class StepLogBundle
    {
        public DateTimeOffset GeneratedAtUtc { get; set; }

        public List<StepLogBundleFlow> Flows { get; set; } = new();

        public List<StepLogEntry>? RunnerSteps { get; set; }
    }

    private sealed class StepLogBundleFlow
    {
        public string Name { get; set; } = string.Empty;

        public string LogFile { get; set; } = string.Empty;

        public StepLog? StepLog { get; set; }
    }

    private sealed class UiStateRecoveryState
    {
        public bool Enabled { get; set; }

        public int MaxAttempts { get; set; } = 2;

        public string SearchRoot { get; set; } = "desktop";

        public UIA3Automation? Automation { get; set; }

        public Application? Application { get; set; }

        public Window? MainWindow { get; set; }

        public SelectorProfileFile? SelectorProfile { get; set; }

        public List<UiStateRecoveryEntry> Entries { get; } = new();

        public List<StepLogEntry> RunnerSteps { get; } = new();

        public int Attempts { get; set; }

        public int Handled { get; set; }

        public string? LastHandler { get; set; }

        public string? EvidencePath { get; set; }
    }

    private sealed class UiStateHandlerOutcome
    {
        public bool Handled { get; set; }

        public bool Success { get; set; }

        public bool Warning { get; set; }

        public string HandlerName { get; set; } = string.Empty;

        public IReadOnlyList<string> SelectorKeys { get; set; } = Array.Empty<string>();

        public string? ErrorMessage { get; set; }

        public DateTimeOffset StartedAtUtc { get; set; }

        public long DurationMs { get; set; }
    }

    private sealed class BuildSummary
    {
        public string Outcome { get; set; } = "Unknown";

        public string? EvidencePath { get; set; }
    }

    private sealed class BuildOutcomeReport
    {
        public string Outcome { get; set; } = "Unknown";

        public string? UsedMode { get; set; }

        public BuildOutcomeSelectorEvidence? SelectorEvidence { get; set; }

        public BuildOutcomeTextEvidence? TextEvidence { get; set; }

        public DateTimeOffset? StartedAtUtc { get; set; }

        public DateTimeOffset? FinishedAtUtc { get; set; }

        public long? DurationMs { get; set; }

        public string? ErrorKind { get; set; }

        public string? ErrorMessage { get; set; }
    }

    private sealed class BuildOutcomeSelectorEvidence
    {
        public bool? SuccessHit { get; set; }

        public bool? FailureHit { get; set; }
    }

    private sealed class BuildOutcomeTextEvidence
    {
        public bool Probed { get; set; }

        public string? LastTextSample { get; set; }

        public string? MatchedToken { get; set; }

        public string? Source { get; set; }
    }

    private sealed class SelectorCheckResult
    {
        public bool Ok { get; set; }

        public SelectorCheckReport Report { get; set; } = new();

        public string ReportPath { get; set; } = string.Empty;
    }

    private sealed class SelectorCheckReport
    {
        public string PackVersion { get; set; } = "default";

        public IReadOnlyList<string> RequiredKeys { get; set; } = Array.Empty<string>();

        public IReadOnlyList<string> MissingKeys { get; set; } = Array.Empty<string>();

        public IReadOnlyList<string> LoadedFiles { get; set; } = Array.Empty<string>();

        public IReadOnlyList<SelectorCheckFlowReport> Flows { get; set; } = Array.Empty<SelectorCheckFlowReport>();

        public DateTimeOffset GeneratedAtUtc { get; set; }
    }

    private sealed class SelectorCheckFlowReport
    {
        public string FlowName { get; set; } = string.Empty;

        public List<string> RequiredKeys { get; set; } = new();

        public List<string[]> RequiredAnyOf { get; set; } = new();

        public List<string> MissingKeys { get; set; } = new();

        public List<string[]> MissingAnyOf { get; set; } = new();
    }

    private sealed class InputsSourceSummary
    {
        public string Mode { get; set; } = "inline";

        public string? CommIrPath { get; set; }

        public string? ResolvedInputsPath { get; set; }

        public IReadOnlyList<string>? Warnings { get; set; }
    }

    private sealed class SelectorRequirement
    {
        public SelectorRequirement(string flowName, IReadOnlyList<string> requiredKeys, IReadOnlyList<string[]> requiredAnyOf)
        {
            FlowName = flowName;
            RequiredKeys = requiredKeys;
            RequiredAnyOf = requiredAnyOf;
        }

        public string FlowName { get; }

        public IReadOnlyList<string> RequiredKeys { get; }

        public IReadOnlyList<string[]> RequiredAnyOf { get; }
    }

    private sealed class SelectorKeyIndex
    {
        private readonly Dictionary<string, string> selectorToKey = new(StringComparer.Ordinal);

        public static SelectorKeyIndex FromProfile(SelectorProfileFile profile)
        {
            var index = new SelectorKeyIndex();
            foreach (KeyValuePair<string, ElementSelector> kv in profile.Selectors)
            {
                index.Register(kv.Key, kv.Value);
            }

            return index;
        }

        public void Register(string key, ElementSelector selector)
        {
            string json = SerializeSelector(selector);
            selectorToKey[json] = key;
        }

        public string? FindKey(ElementSelector? selector)
        {
            if (selector is null)
            {
                return null;
            }

            string json = SerializeSelector(selector);
            return selectorToKey.TryGetValue(json, out string? key) ? key : null;
        }

        private static string SerializeSelector(ElementSelector selector)
        {
            return JsonSerializer.Serialize(selector, SelectorJsonOptions);
        }
    }

    private sealed class SelectorProfileLoadResult
    {
        public SelectorProfileFile Profile { get; set; } = new();

        public string BaseBaselinePath { get; set; } = string.Empty;

        public string? BaseLocalPath { get; set; }

        public string BaselinePath { get; set; } = string.Empty;

        public string? LocalPath { get; set; }

        public bool UsedLocal { get; set; }

        public IReadOnlyList<string> LoadedFiles { get; set; } = Array.Empty<string>();

        public string SelectorsFile => UsedLocal && !string.IsNullOrWhiteSpace(LocalPath) ? LocalPath! : BaselinePath;
    }

    private sealed class RunnerOptions
    {
        public string? ConfigPath { get; set; }

        public string? SelectorsRoot { get; set; }

        public string? Profile { get; set; }

        public string? AgentPath { get; set; }

        public bool Demo { get; set; }

        public bool Probe { get; set; }

        public bool Verify { get; set; }

        public string? EvidencePath { get; set; }

        public string? ProbeFlow { get; set; }

        public string? ProbeKeys { get; set; }

        public string? ProbeSearchRoot { get; set; }

        public int ProbeTimeoutMs { get; set; }

        public bool Check { get; set; }

        public bool SkipCheck { get; set; }

        public int CheckTimeoutMs { get; set; }

        public bool ShowHelp { get; set; }

        public static RunnerOptions Parse(string[] args)
        {
            var options = new RunnerOptions();

            for (int i = 0; i < args.Length; i++)
            {
                string arg = args[i];
                switch (arg)
                {
                    case "verify":
                        options.Verify = true;
                        break;
                    case "--config":
                        options.ConfigPath = i + 1 < args.Length ? args[++i] : null;
                        break;
                    case "--selectorsRoot":
                        options.SelectorsRoot = i + 1 < args.Length ? args[++i] : null;
                        break;
                    case "--profile":
                        options.Profile = i + 1 < args.Length ? args[++i] : null;
                        break;
                    case "--agentPath":
                        options.AgentPath = i + 1 < args.Length ? args[++i] : null;
                        break;
                    case "--demo":
                        options.Demo = true;
                        break;
                    case "--verify":
                        options.Verify = true;
                        break;
                    case "--evidence":
                        options.EvidencePath = i + 1 < args.Length ? args[++i] : null;
                        break;
                    case "--probe":
                        options.Probe = true;
                        break;
                    case "--probeFlow":
                        options.ProbeFlow = i + 1 < args.Length ? args[++i] : null;
                        break;
                    case "--probeKeys":
                        options.ProbeKeys = i + 1 < args.Length ? args[++i] : null;
                        break;
                    case "--probeSearchRoot":
                        options.ProbeSearchRoot = i + 1 < args.Length ? args[++i] : null;
                        break;
                    case "--probeTimeoutMs":
                        if (i + 1 < args.Length && int.TryParse(args[++i], out int timeout))
                        {
                            options.ProbeTimeoutMs = timeout;
                        }

                        break;
                    case "--check":
                        options.Check = true;
                        break;
                    case "--skipCheck":
                        options.SkipCheck = true;
                        break;
                    case "--timeoutMs":
                        if (i + 1 < args.Length && int.TryParse(args[++i], out int checkTimeout))
                        {
                            options.CheckTimeoutMs = checkTimeout;
                        }

                        break;
                    case "--help":
                    case "-h":
                        options.ShowHelp = true;
                        break;
                }
            }

            return options;
        }
    }

    private sealed class RunnerConfig
    {
        public SessionConfig Session { get; set; } = new();

        public string? AgentPath { get; set; }

        public InputsSourceConfig? InputsSource { get; set; }

        public EvidencePackConfig? EvidencePack { get; set; }

        public UiStateRecoveryConfig? UiStateRecovery { get; set; }

        public string? SelectorPackVersion { get; set; }

        public bool SkipImportVariables { get; set; }

        public bool SkipImportProgram { get; set; }

        public bool SkipBuild { get; set; }

        public bool AllowPartial { get; set; }

        public string? SelectorsRoot { get; set; }

        public string? Profile { get; set; }

        public string? LogsRoot { get; set; }

        public string? ProgramTextPath { get; set; }

        public string? VariablesFilePath { get; set; }

        public ImportProgramConfig? ImportProgram { get; set; }

        public ImportVariablesConfig? ImportVariables { get; set; }

        public BuildConfig? Build { get; set; }

        public int FlowTimeoutMs { get; set; } = 30_000;
    }

    private sealed class SessionConfig
    {
        public int? ProcessId { get; set; }

        public string? ProcessName { get; set; }

        public string? MainWindowTitleContains { get; set; }

        public int TimeoutMs { get; set; } = 10_000;

        public bool BringToForeground { get; set; } = true;
    }

    private sealed class SelectorProfileFile
    {
        public int SchemaVersion { get; set; } = 1;

        public Dictionary<string, ElementSelector> Selectors { get; set; } = new(StringComparer.Ordinal);
    }

    private sealed class ImportProgramConfig
    {
        public List<ImportDialogStepConfig>? OpenProgramSteps { get; set; }

        public string? EditorRootSelectorKey { get; set; }

        public ElementSelector? EditorRootSelector { get; set; }

        public string? EditorSelectorKey { get; set; }

        public ElementSelector? EditorSelector { get; set; }

        public string? VerifySelectorKey { get; set; }

        public ElementSelector? VerifySelector { get; set; }

        public string? VerifyMode { get; set; }

        public int? AfterPasteWaitMs { get; set; }

        public int FindTimeoutMs { get; set; } = 10_000;

        public int ClipboardTimeoutMs { get; set; } = 2_000;

        public int VerifyTimeoutMs { get; set; } = 5_000;

        public bool? FallbackToType { get; set; } = true;

        public bool PreferClipboard { get; set; } = true;

        public ClipboardRetryConfig? ClipboardRetry { get; set; }

        public bool ClipboardHealthCheck { get; set; }

        public bool ForceFallbackOnClipboardFailure { get; set; }

        public string? SearchRoot { get; set; }

        public bool EnablePopupHandling { get; set; }

        public string? PopupSearchRoot { get; set; }

        public int PopupTimeoutMs { get; set; } = 1500;

        public bool AllowPopupOk { get; set; }

        public string? PopupDialogSelectorKey { get; set; }

        public ElementSelector? PopupDialogSelector { get; set; }

        public string? PopupOkButtonSelectorKey { get; set; }

        public ElementSelector? PopupOkButtonSelector { get; set; }

        public string? PopupCancelButtonSelectorKey { get; set; }

        public ElementSelector? PopupCancelButtonSelector { get; set; }
    }

    private sealed class ImportVariablesConfig
    {
        public string? FilePath { get; set; }

        public List<ImportDialogStepConfig>? OpenImportDialogSteps { get; set; }

        public string? DialogSelectorKey { get; set; }

        public ElementSelector? DialogSelector { get; set; }

        public string? FilePathEditorSelectorKey { get; set; }

        public ElementSelector? FilePathEditorSelector { get; set; }

        public string? ConfirmButtonSelectorKey { get; set; }

        public ElementSelector? ConfirmButtonSelector { get; set; }

        public WaitConditionConfig? SuccessCondition { get; set; }

        public int FindTimeoutMs { get; set; } = 10_000;

        public int WaitTimeoutMs { get; set; } = 30_000;

        public string? SearchRoot { get; set; }

        public bool EnablePopupHandling { get; set; }

        public string? PopupSearchRoot { get; set; }

        public int PopupTimeoutMs { get; set; } = 1500;

        public bool AllowPopupOk { get; set; }

        public string? PopupDialogSelectorKey { get; set; }

        public ElementSelector? PopupDialogSelector { get; set; }

        public string? PopupOkButtonSelectorKey { get; set; }

        public ElementSelector? PopupOkButtonSelector { get; set; }

        public string? PopupCancelButtonSelectorKey { get; set; }

        public ElementSelector? PopupCancelButtonSelector { get; set; }
    }

    private sealed class ImportDialogStepConfig
    {
        public string? Action { get; set; }

        public string? SelectorKey { get; set; }

        public ElementSelector? Selector { get; set; }

        public string? Text { get; set; }

        public string? Mode { get; set; }

        public string? Keys { get; set; }

        public WaitConditionConfig? Condition { get; set; }

        public int? TimeoutMs { get; set; }
    }

    private sealed class WaitConditionConfig
    {
        public string? Kind { get; set; }

        public string? SelectorKey { get; set; }

        public ElementSelector? Selector { get; set; }
    }

    private sealed class ClipboardRetryConfig
    {
        public int Times { get; set; } = 3;

        public int IntervalMs { get; set; } = 200;
    }

    private sealed class EvidencePackConfig
    {
        public bool Enable { get; set; } = true;
    }

    private sealed class UiStateRecoveryConfig
    {
        public bool Enable { get; set; } = true;

        public int MaxAttempts { get; set; } = 2;

        public string SearchRoot { get; set; } = "desktop";
    }

    private sealed class BuildConfig
    {
        public string? BuildButtonSelectorKey { get; set; }

        public ElementSelector? BuildButtonSelector { get; set; }

        public WaitConditionConfig? WaitCondition { get; set; }

        public BuildOutcomeConfig? BuildOutcome { get; set; }

        public int TimeoutMs { get; set; } = 60_000;

        public int FindTimeoutMs { get; set; } = 10_000;

        public string? OptionalCloseDialogSelectorKey { get; set; }

        public ElementSelector? OptionalCloseDialogSelector { get; set; }

        public IReadOnlyList<string>? UnexpectedSelectorKeys { get; set; }

        public IReadOnlyList<ElementSelector>? UnexpectedSelectors { get; set; }

        public string? SearchRoot { get; set; }

        public bool EnablePopupHandling { get; set; }

        public string? PopupSearchRoot { get; set; }

        public int PopupTimeoutMs { get; set; } = 1500;

        public bool AllowPopupOk { get; set; }

        public string? PopupDialogSelectorKey { get; set; }

        public ElementSelector? PopupDialogSelector { get; set; }

        public string? PopupOkButtonSelectorKey { get; set; }

        public ElementSelector? PopupOkButtonSelector { get; set; }

        public string? PopupCancelButtonSelectorKey { get; set; }

        public ElementSelector? PopupCancelButtonSelector { get; set; }
    }

    private sealed class BuildOutcomeConfig
    {
        public string? Mode { get; set; }

        public string? SuccessSelectorKey { get; set; }

        public ElementSelector? SuccessSelector { get; set; }

        public string? FailureSelectorKey { get; set; }

        public ElementSelector? FailureSelector { get; set; }

        public string? TextProbeSelectorKey { get; set; }

        public ElementSelector? TextProbeSelector { get; set; }

        public IReadOnlyList<string>? SuccessTextContains { get; set; }

        public int TimeoutMs { get; set; } = 60_000;
    }
}


