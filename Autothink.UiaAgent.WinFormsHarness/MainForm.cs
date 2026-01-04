using System.Text.Json;
using Autothink.UiaAgent.Rpc.Contracts;
using StreamJsonRpc;

namespace Autothink.UiaAgent.WinFormsHarness;

internal sealed class MainForm : Form
{
    private static readonly Font UiFont = new("Segoe UI", 12F, FontStyle.Regular, GraphicsUnit.Point);
    private const int WideTextWidth = 980;
    private const int MediumTextWidth = 520;
    private const int ButtonWidth = 160;
    private const int ButtonHeight = 44;

    private readonly AgentRpcClient client = new();

    private readonly TextBox agentExePathTextBox = new() { Width = WideTextWidth };
    private readonly Button browseAgentButton = new() { Text = "Browse..." };
    private readonly Button startAgentButton = new() { Text = "Start Agent" };
    private readonly Button stopAgentButton = new() { Text = "Stop Agent", Enabled = false };
    private readonly Button pingButton = new() { Text = "Ping", Enabled = false };

    private readonly TextBox processNameTextBox = new() { Text = "Autothink", Width = 240 };
    private readonly TextBox mainTitleContainsTextBox = new() { Text = "AUTOTHINK", Width = 320 };
    private readonly NumericUpDown sessionTimeoutMs = new() { Minimum = 100, Maximum = 600_000, Value = 10_000, Width = 140 };
    private readonly CheckBox bringToForegroundCheckBox = new() { Text = "BringToForeground", Checked = true, AutoSize = true };
    private readonly Button openSessionButton = new() { Text = "OpenSession", Enabled = false };
    private readonly Button closeSessionButton = new() { Text = "CloseSession", Enabled = false };
    private readonly TextBox sessionIdTextBox = new() { Width = 520 };

    private readonly TextBox selectorJsonTextBox = new() { Multiline = true, ScrollBars = ScrollBars.Vertical, Height = 200, Width = WideTextWidth };
    private readonly Button findElementButton = new() { Text = "FindElement", Enabled = false };
    private readonly Button clickButton = new() { Text = "Click", Enabled = false };
    private readonly Button doubleClickButton = new() { Text = "DoubleClick", Enabled = false };
    private readonly Button rightClickButton = new() { Text = "RightClick", Enabled = false };
    private readonly Button setTextButton = new() { Text = "SetText", Enabled = false };
    private readonly TextBox setTextValueTextBox = new() { Width = MediumTextWidth };
    private readonly ComboBox setTextModeComboBox = new() { Width = 200, DropDownStyle = ComboBoxStyle.DropDownList };

    private readonly Button sendKeysButton = new() { Text = "SendKeys", Enabled = false };
    private readonly TextBox sendKeysTextBox = new() { Text = "CTRL+V", Width = 240 };

    private readonly ComboBox waitKindComboBox = new() { Width = 240, DropDownStyle = ComboBoxStyle.DropDownList };
    private readonly NumericUpDown waitTimeoutMs = new() { Minimum = 100, Maximum = 600_000, Value = 5_000, Width = 140 };
    private readonly Button waitUntilButton = new() { Text = "WaitUntil", Enabled = false };

    private readonly Button runFlowButton = new() { Text = "RunFlow", Enabled = false };
    private readonly ComboBox flowNameComboBox = new() { Width = 360, DropDownStyle = ComboBoxStyle.DropDownList };
    private readonly TextBox flowArgsJsonTextBox = new() { Multiline = true, ScrollBars = ScrollBars.Vertical, Height = 180, Width = WideTextWidth, Text = "{}" };

    private readonly TextBox selectorsFileTextBox = new() { Width = WideTextWidth };
    private readonly Button browseSelectorsButton = new() { Text = "Browse..." };
    private readonly Button loadSelectorsButton = new() { Text = "Load" };
    private readonly ComboBox selectorKeyComboBox = new() { Width = 320, DropDownStyle = ComboBoxStyle.DropDownList };
    private readonly Button applySelectorButton = new() { Text = "Use Selector" };

    private readonly TextBox logTextBox = new()
    {
        Multiline = true,
        ScrollBars = ScrollBars.Both,
        WordWrap = false,
        Dock = DockStyle.Fill,
        ReadOnly = true,
    };

    private ElementRef? lastElement;
    private readonly Dictionary<string, ElementSelector> selectorCache = new(StringComparer.Ordinal);

    public MainForm()
    {
        this.Text = "Autothink.UiaAgent WinForms Harness";
        this.Width = 1600;
        this.Height = 1050;
        this.MinimumSize = new Size(1400, 900);
        this.Font = UiFont;
        this.AutoScaleMode = AutoScaleMode.Font;

        this.client.StderrLine += line => this.AppendLog($"[stderr] {line}");

        this.setTextModeComboBox.Items.AddRange([SetTextModes.Replace, SetTextModes.Append, SetTextModes.CtrlAReplace]);
        this.setTextModeComboBox.SelectedIndex = 0;

        this.waitKindComboBox.Items.AddRange([WaitConditionKinds.ElementExists, WaitConditionKinds.ElementNotExists, WaitConditionKinds.ElementEnabled]);
        this.waitKindComboBox.SelectedIndex = 0;

        this.flowNameComboBox.Items.AddRange(
        [
            "autothink.attach",
            "autothink.importVariables",
            "autothink.importProgram.textPaste",
            "autothink.build",
        ]);
        this.flowNameComboBox.SelectedIndex = 0;

        this.selectorJsonTextBox.Text =
            """
            {
              "Path": [
                { "Search": "Descendants", "ControlType": "Button", "Name": "确定", "Index": 0 }
              ]
            }
            """;

        this.browseAgentButton.Click += (_, _) => this.BrowseAgentExe();
        this.startAgentButton.Click += async (_, _) => await this.StartAgentAsync();
        this.stopAgentButton.Click += async (_, _) => await this.StopAgentAsync();
        this.pingButton.Click += async (_, _) => await this.PingAsync();
        this.openSessionButton.Click += async (_, _) => await this.OpenSessionAsync();
        this.closeSessionButton.Click += async (_, _) => await this.CloseSessionAsync();

        this.findElementButton.Click += async (_, _) => await this.FindElementAsync();
        this.clickButton.Click += async (_, _) => await this.ClickAsync();
        this.doubleClickButton.Click += async (_, _) => await this.DoubleClickAsync();
        this.rightClickButton.Click += async (_, _) => await this.RightClickAsync();
        this.setTextButton.Click += async (_, _) => await this.SetTextAsync();
        this.sendKeysButton.Click += async (_, _) => await this.SendKeysAsync();
        this.waitUntilButton.Click += async (_, _) => await this.WaitUntilAsync();
        this.runFlowButton.Click += async (_, _) => await this.RunFlowAsync();

        this.browseSelectorsButton.Click += (_, _) => this.BrowseSelectorsFile();
        this.loadSelectorsButton.Click += (_, _) => this.LoadSelectors();
        this.applySelectorButton.Click += (_, _) => this.ApplySelectedSelector();

        ApplyButtonStyle(
            this.browseAgentButton,
            this.startAgentButton,
            this.stopAgentButton,
            this.pingButton,
            this.openSessionButton,
            this.closeSessionButton,
            this.findElementButton,
            this.clickButton,
            this.doubleClickButton,
            this.rightClickButton,
            this.setTextButton,
            this.sendKeysButton,
            this.waitUntilButton,
            this.runFlowButton,
            this.browseSelectorsButton,
            this.loadSelectorsButton,
            this.applySelectorButton);

        var root = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 1,
            RowCount = 2,
        };
        root.RowStyles.Add(new RowStyle(SizeType.Absolute, 780));
        root.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        this.Controls.Add(root);

        var controlsPanel = new Panel { Dock = DockStyle.Fill, AutoScroll = true };
        root.Controls.Add(controlsPanel, 0, 0);
        root.Controls.Add(this.logTextBox, 0, 1);

        int y = 10;
        controlsPanel.Controls.Add(this.BuildAgentPanel(ref y));
        controlsPanel.Controls.Add(this.BuildSessionPanel(ref y));
        controlsPanel.Controls.Add(this.BuildSelectorPanel(ref y));
        controlsPanel.Controls.Add(this.BuildAtomicPanel(ref y));
        controlsPanel.Controls.Add(this.BuildFlowPanel(ref y));

        this.agentExePathTextBox.Text = TryGuessAgentExePath() ?? string.Empty;
        this.selectorsFileTextBox.Text = TryGuessSelectorsPath() ?? string.Empty;
    }

    private Control BuildAgentPanel(ref int y)
    {
        var group = new GroupBox
        {
            Text = "Agent Process",
            Left = 10,
            Top = y,
            Width = 1040,
            Height = 110,
        };

        var flow = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            WrapContents = true,
            AutoScroll = true,
            Padding = new Padding(6, 4, 6, 4),
        };

        flow.Controls.Add(new Label { Text = "AgentExe", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        flow.Controls.Add(this.agentExePathTextBox);
        flow.Controls.Add(this.browseAgentButton);
        flow.Controls.Add(this.startAgentButton);
        flow.Controls.Add(this.stopAgentButton);
        flow.Controls.Add(this.pingButton);

        group.Controls.Add(flow);
        y += group.Height + 10;
        return group;
    }

    private Control BuildSessionPanel(ref int y)
    {
        var group = new GroupBox
        {
            Text = "Session",
            Left = 10,
            Top = y,
            Width = 1040,
            Height = 130,
        };

        var flow = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            WrapContents = true,
            AutoScroll = true,
            Padding = new Padding(6, 4, 6, 4),
        };

        flow.Controls.Add(new Label { Text = "ProcessName", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        flow.Controls.Add(this.processNameTextBox);
        flow.Controls.Add(new Label { Text = "TitleContains", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        flow.Controls.Add(this.mainTitleContainsTextBox);
        flow.Controls.Add(new Label { Text = "TimeoutMs", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        flow.Controls.Add(this.sessionTimeoutMs);
        flow.Controls.Add(this.bringToForegroundCheckBox);
        flow.Controls.Add(this.openSessionButton);
        flow.Controls.Add(this.closeSessionButton);
        flow.Controls.Add(new Label { Text = "SessionId", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        flow.Controls.Add(this.sessionIdTextBox);

        group.Controls.Add(flow);
        y += group.Height + 10;
        return group;
    }

    private Control BuildSelectorPanel(ref int y)
    {
        var group = new GroupBox
        {
            Text = "Selector Profile",
            Left = 10,
            Top = y,
            Width = 1040,
            Height = 120,
        };

        var flow = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            WrapContents = true,
            AutoScroll = true,
            Padding = new Padding(6, 4, 6, 4),
        };

        flow.Controls.Add(new Label { Text = "SelectorsFile", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        flow.Controls.Add(this.selectorsFileTextBox);
        flow.Controls.Add(this.browseSelectorsButton);
        flow.Controls.Add(this.loadSelectorsButton);
        flow.Controls.Add(new Label { Text = "Key", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        flow.Controls.Add(this.selectorKeyComboBox);
        flow.Controls.Add(this.applySelectorButton);

        group.Controls.Add(flow);
        y += group.Height + 10;
        return group;
    }

    private Control BuildAtomicPanel(ref int y)
    {
        var group = new GroupBox
        {
            Text = "Atomic Actions (core)",
            Left = 10,
            Top = y,
            Width = 1040,
            Height = 320,
        };

        var panel = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            WrapContents = true,
            AutoScroll = true,
            Padding = new Padding(6, 4, 6, 4),
        };

        panel.Controls.Add(new Label { Text = "Selector JSON (ElementSelector)", AutoSize = true, Padding = new Padding(0, 6, 0, 0) });
        panel.SetFlowBreak(panel.Controls[panel.Controls.Count - 1], true);
        panel.Controls.Add(this.selectorJsonTextBox);
        panel.SetFlowBreak(this.selectorJsonTextBox, true);

        panel.Controls.Add(this.findElementButton);
        panel.Controls.Add(this.clickButton);
        panel.Controls.Add(this.doubleClickButton);
        panel.Controls.Add(this.rightClickButton);

        panel.Controls.Add(new Label { Text = "Text", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        panel.Controls.Add(this.setTextValueTextBox);
        panel.Controls.Add(new Label { Text = "Mode", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        panel.Controls.Add(this.setTextModeComboBox);
        panel.Controls.Add(this.setTextButton);

        panel.Controls.Add(new Label { Text = "Keys", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        panel.Controls.Add(this.sendKeysTextBox);
        panel.Controls.Add(this.sendKeysButton);

        panel.Controls.Add(new Label { Text = "WaitKind", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        panel.Controls.Add(this.waitKindComboBox);
        panel.Controls.Add(new Label { Text = "TimeoutMs", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        panel.Controls.Add(this.waitTimeoutMs);
        panel.Controls.Add(this.waitUntilButton);

        group.Controls.Add(panel);
        y += group.Height + 10;
        return group;
    }

    private Control BuildFlowPanel(ref int y)
    {
        var group = new GroupBox
        {
            Text = "RunFlow (Stage 2)",
            Left = 10,
            Top = y,
            Width = 1040,
            Height = 240,
        };

        var panel = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            WrapContents = true,
            AutoScroll = true,
            Padding = new Padding(6, 4, 6, 4),
        };

        panel.Controls.Add(new Label { Text = "FlowName", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        panel.Controls.Add(this.flowNameComboBox);
        panel.Controls.Add(this.runFlowButton);
        panel.SetFlowBreak(this.runFlowButton, true);

        panel.Controls.Add(new Label { Text = "Args JSON (optional)", AutoSize = true, Padding = new Padding(0, 6, 0, 0) });
        panel.SetFlowBreak(panel.Controls[panel.Controls.Count - 1], true);
        panel.Controls.Add(this.flowArgsJsonTextBox);

        group.Controls.Add(panel);
        y += group.Height + 10;
        return group;
    }

    private async Task StartAgentAsync()
    {
        try
        {
            this.AppendLog($"Starting agent: {this.agentExePathTextBox.Text}");
            await this.client.StartAsync(this.agentExePathTextBox.Text, CancellationToken.None);
            this.AppendLog("Agent READY + JSON-RPC connected.");

            this.startAgentButton.Enabled = false;
            this.stopAgentButton.Enabled = true;
            this.pingButton.Enabled = true;
            this.openSessionButton.Enabled = true;
            this.runFlowButton.Enabled = true;
        }
        catch (Exception ex)
        {
            this.AppendLog($"StartAgent failed: {ex}");
        }
    }

    private async Task StopAgentAsync()
    {
        try
        {
            this.AppendLog("Stopping agent...");
            await this.client.StopAsync(CancellationToken.None);
            this.AppendLog("Agent stopped.");
        }
        catch (Exception ex)
        {
            this.AppendLog($"StopAgent failed: {ex}");
        }
        finally
        {
            this.startAgentButton.Enabled = true;
            this.stopAgentButton.Enabled = false;
            this.pingButton.Enabled = false;
            this.openSessionButton.Enabled = false;
            this.closeSessionButton.Enabled = false;
            this.findElementButton.Enabled = false;
            this.clickButton.Enabled = false;
            this.doubleClickButton.Enabled = false;
            this.rightClickButton.Enabled = false;
            this.setTextButton.Enabled = false;
            this.sendKeysButton.Enabled = false;
            this.waitUntilButton.Enabled = false;
            this.runFlowButton.Enabled = false;
            this.sessionIdTextBox.Text = string.Empty;
            this.lastElement = null;
        }
    }

    private async Task PingAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            string pong = await rpc.InvokeAsync<string>("Ping");
            this.AppendLog($"Ping -> {pong}");
        }
        catch (Exception ex)
        {
            this.AppendLog($"Ping failed: {ex}");
        }
    }

    private async Task OpenSessionAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            RpcResult<OpenSessionResponse> result = await rpc.InvokeAsync<RpcResult<OpenSessionResponse>>("OpenSession", new OpenSessionRequest
            {
                ProcessName = this.processNameTextBox.Text,
                MainWindowTitleContains = string.IsNullOrWhiteSpace(this.mainTitleContainsTextBox.Text) ? null : this.mainTitleContainsTextBox.Text,
                TimeoutMs = (int)this.sessionTimeoutMs.Value,
                BringToForeground = this.bringToForegroundCheckBox.Checked,
            });

            this.AppendRpcResult("OpenSession", result);

            if (result.Ok && result.Value is not null)
            {
                this.sessionIdTextBox.Text = result.Value.SessionId;
                this.closeSessionButton.Enabled = true;
                this.findElementButton.Enabled = true;
                this.sendKeysButton.Enabled = true;
                this.waitUntilButton.Enabled = true;
            }
        }
        catch (Exception ex)
        {
            this.AppendLog($"OpenSession failed: {ex}");
        }
    }

    private async Task CloseSessionAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            string sessionId = this.RequireSessionId();
            RpcResult result = await rpc.InvokeAsync<RpcResult>("CloseSession", new CloseSessionRequest { SessionId = sessionId });
            this.AppendRpcResult("CloseSession", result);

            if (result.Ok)
            {
                this.sessionIdTextBox.Text = string.Empty;
                this.lastElement = null;
                this.closeSessionButton.Enabled = false;
                this.findElementButton.Enabled = false;
                this.clickButton.Enabled = false;
                this.doubleClickButton.Enabled = false;
                this.rightClickButton.Enabled = false;
                this.setTextButton.Enabled = false;
                this.sendKeysButton.Enabled = false;
                this.waitUntilButton.Enabled = false;
            }
        }
        catch (Exception ex)
        {
            this.AppendLog($"CloseSession failed: {ex}");
        }
    }

    private async Task FindElementAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            string sessionId = this.RequireSessionId();

            ElementSelector selector = DeserializeSelector(this.selectorJsonTextBox.Text);
            RpcResult<FindElementResponse> result = await rpc.InvokeAsync<RpcResult<FindElementResponse>>("FindElement", new FindElementRequest
            {
                SessionId = sessionId,
                TimeoutMs = 5_000,
                Selector = selector,
            });

            this.AppendRpcResult("FindElement", result);

            if (result.Ok && result.Value is not null)
            {
                this.lastElement = result.Value.Element;
                this.clickButton.Enabled = true;
                this.doubleClickButton.Enabled = true;
                this.rightClickButton.Enabled = true;
                this.setTextButton.Enabled = true;
                this.AppendLog("ElementRef captured for Click/SetText.");
            }
        }
        catch (Exception ex)
        {
            this.AppendLog($"FindElement failed: {ex}");
        }
    }

    private async Task ClickAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            ElementRef element = this.RequireLastElement();
            RpcResult result = await rpc.InvokeAsync<RpcResult>("Click", new ClickRequest { Element = element });
            this.AppendRpcResult("Click", result);
        }
        catch (Exception ex)
        {
            this.AppendLog($"Click failed: {ex}");
        }
    }

    private async Task DoubleClickAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            ElementRef element = this.RequireLastElement();
            RpcResult result = await rpc.InvokeAsync<RpcResult>("DoubleClick", new DoubleClickRequest { Element = element });
            this.AppendRpcResult("DoubleClick", result);
        }
        catch (Exception ex)
        {
            this.AppendLog($"DoubleClick failed: {ex}");
        }
    }

    private async Task RightClickAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            ElementRef element = this.RequireLastElement();
            RpcResult result = await rpc.InvokeAsync<RpcResult>("RightClick", new RightClickRequest { Element = element });
            this.AppendRpcResult("RightClick", result);
        }
        catch (Exception ex)
        {
            this.AppendLog($"RightClick failed: {ex}");
        }
    }

    private async Task SetTextAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            ElementRef element = this.RequireLastElement();
            string mode = this.setTextModeComboBox.SelectedItem?.ToString() ?? SetTextModes.Replace;
            RpcResult result = await rpc.InvokeAsync<RpcResult>("SetText", new SetTextRequest
            {
                Element = element,
                Text = this.setTextValueTextBox.Text,
                Mode = mode,
            });
            this.AppendRpcResult("SetText", result);
        }
        catch (Exception ex)
        {
            this.AppendLog($"SetText failed: {ex}");
        }
    }

    private async Task SendKeysAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            string sessionId = this.RequireSessionId();
            RpcResult result = await rpc.InvokeAsync<RpcResult>("SendKeys", new SendKeysRequest
            {
                SessionId = sessionId,
                Keys = this.sendKeysTextBox.Text,
            });
            this.AppendRpcResult("SendKeys", result);
        }
        catch (Exception ex)
        {
            this.AppendLog($"SendKeys failed: {ex}");
        }
    }

    private async Task WaitUntilAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            string sessionId = this.RequireSessionId();
            string kind = this.waitKindComboBox.SelectedItem?.ToString() ?? WaitConditionKinds.ElementExists;

            ElementSelector? selector = null;
            if (!string.IsNullOrWhiteSpace(this.selectorJsonTextBox.Text))
            {
                selector = DeserializeSelector(this.selectorJsonTextBox.Text);
            }

            RpcResult result = await rpc.InvokeAsync<RpcResult>("WaitUntil", new WaitUntilRequest
            {
                SessionId = sessionId,
                TimeoutMs = (int)this.waitTimeoutMs.Value,
                Condition = new WaitCondition { Kind = kind, Selector = selector },
            });

            this.AppendRpcResult("WaitUntil", result);
        }
        catch (Exception ex)
        {
            this.AppendLog($"WaitUntil failed: {ex}");
        }
    }

    private async Task RunFlowAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            string sessionId = this.sessionIdTextBox.Text;

            string rawArgs = this.flowArgsJsonTextBox.Text;
            JsonElement args = TryDeserializeJsonElement(rawArgs);
            RpcResult<RunFlowResponse> result = await rpc.InvokeAsync<RpcResult<RunFlowResponse>>("RunFlow", new RunFlowRequest
            {
                SessionId = sessionId,
                FlowName = this.flowNameComboBox.SelectedItem?.ToString() ?? string.Empty,
                TimeoutMs = 30_000,
                Args = args,
                ArgsJson = string.IsNullOrWhiteSpace(rawArgs) ? null : rawArgs,
            });

            this.AppendRpcResult("RunFlow", result);
        }
        catch (Exception ex)
        {
            this.AppendLog($"RunFlow failed: {ex}");
        }
    }

    private JsonRpc RequireRpc()
    {
        if (this.client.Rpc is null)
        {
            throw new InvalidOperationException("RPC is not connected. Start agent first.");
        }

        return this.client.Rpc;
    }

    private string RequireSessionId()
    {
        string sessionId = this.sessionIdTextBox.Text.Trim();
        if (string.IsNullOrWhiteSpace(sessionId))
        {
            throw new InvalidOperationException("SessionId is empty. Call OpenSession first.");
        }

        return sessionId;
    }

    private ElementRef RequireLastElement()
    {
        if (this.lastElement is null)
        {
            throw new InvalidOperationException("No ElementRef captured. Run FindElement first.");
        }

        return this.lastElement;
    }

    private static ElementSelector DeserializeSelector(string json)
    {
        ElementSelector? selector = JsonSerializer.Deserialize<ElementSelector>(json, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
        });

        return selector ?? new ElementSelector();
    }

    private static JsonElement TryDeserializeJsonElement(string json)
    {
        if (string.IsNullOrWhiteSpace(json))
        {
            return default;
        }

        try
        {
            using JsonDocument doc = JsonDocument.Parse(json);
            return doc.RootElement.Clone();
        }
        catch
        {
            return default;
        }
    }

    private void BrowseAgentExe()
    {
        using var dlg = new OpenFileDialog
        {
            Filter = "Autothink.UiaAgent.exe|Autothink.UiaAgent.exe|Executable (*.exe)|*.exe|All files (*.*)|*.*",
            FileName = "Autothink.UiaAgent.exe",
        };

        if (dlg.ShowDialog(this) == DialogResult.OK)
        {
            this.agentExePathTextBox.Text = dlg.FileName;
        }
    }

    private void BrowseSelectorsFile()
    {
        using var dlg = new OpenFileDialog
        {
            Filter = "Selector JSON (*.json)|*.json|All files (*.*)|*.*",
            FileName = "autothink.v1.base.json",
        };

        if (dlg.ShowDialog(this) == DialogResult.OK)
        {
            this.selectorsFileTextBox.Text = dlg.FileName;
        }
    }

    private void LoadSelectors()
    {
        string path = this.selectorsFileTextBox.Text.Trim();
        if (string.IsNullOrWhiteSpace(path))
        {
            this.AppendLog("Selectors file path is empty.");
            return;
        }

        if (!File.Exists(path))
        {
            this.AppendLog($"Selectors file not found: {path}");
            return;
        }

        try
        {
            string json = File.ReadAllText(path);
            Dictionary<string, ElementSelector> selectors = ParseSelectors(json);

            this.selectorCache.Clear();
            foreach (KeyValuePair<string, ElementSelector> pair in selectors.OrderBy(p => p.Key, StringComparer.OrdinalIgnoreCase))
            {
                this.selectorCache[pair.Key] = pair.Value;
            }

            this.selectorKeyComboBox.Items.Clear();
            foreach (string key in this.selectorCache.Keys.OrderBy(k => k, StringComparer.OrdinalIgnoreCase))
            {
                this.selectorKeyComboBox.Items.Add(key);
            }

            if (this.selectorKeyComboBox.Items.Count > 0)
            {
                this.selectorKeyComboBox.SelectedIndex = 0;
            }

            this.AppendLog($"Loaded selectors: {this.selectorCache.Count} (from {path})");
        }
        catch (Exception ex)
        {
            this.AppendLog($"Load selectors failed: {ex}");
        }
    }

    private void ApplySelectedSelector()
    {
        string? key = this.selectorKeyComboBox.SelectedItem?.ToString();
        if (string.IsNullOrWhiteSpace(key))
        {
            this.AppendLog("Selector key is empty.");
            return;
        }

        if (!this.selectorCache.TryGetValue(key, out ElementSelector? selector))
        {
            this.AppendLog($"Selector key not found: {key}");
            return;
        }

        this.selectorJsonTextBox.Text = JsonSerializer.Serialize(selector, new JsonSerializerOptions { WriteIndented = true });
        this.AppendLog($"Applied selector: {key}");
    }

    private void AppendRpcResult(string title, RpcResult result)
    {
        this.AppendLog($"--- {title} ---");
        this.AppendLog($"Ok: {result.Ok}");
        if (result.Error is not null)
        {
            this.AppendLog($"Error: {result.Error.Kind} - {result.Error.Message}");
        }

        this.AppendStepLog(result.StepLog);
    }

    private void AppendRpcResult<T>(string title, RpcResult<T> result)
    {
        this.AppendLog($"--- {title} ---");
        this.AppendLog($"Ok: {result.Ok}");
        if (result.Error is not null)
        {
            this.AppendLog($"Error: {result.Error.Kind} - {result.Error.Message}");
        }

        if (result.Value is not null)
        {
            this.AppendLog("Value:");
            this.AppendLog(JsonSerializer.Serialize(result.Value, new JsonSerializerOptions { WriteIndented = true }));
        }

        this.AppendStepLog(result.StepLog);
    }

    private void AppendStepLog(StepLog stepLog)
    {
        if (stepLog.Steps.Count == 0)
        {
            return;
        }

        this.AppendLog("StepLog:");
        foreach (StepLogEntry step in stepLog.Steps)
        {
            string extra = step.Error is null ? string.Empty : $" | {step.Error.Kind}: {step.Error.Message}";
            this.AppendLog($"- {step.StepId} [{step.Outcome}] ({step.DurationMs}ms){extra}");
        }
    }

    private void AppendLog(string text)
    {
        if (this.InvokeRequired)
        {
            _ = this.BeginInvoke(() => this.AppendLog(text));
            return;
        }

        this.logTextBox.AppendText(text + Environment.NewLine);
    }

    private static void ApplyButtonStyle(params Button[] buttons)
    {
        foreach (Button button in buttons)
        {
            button.AutoSize = false;
            button.Size = new Size(ButtonWidth, ButtonHeight);
            button.Margin = new Padding(6, 4, 6, 4);
        }
    }

    private static string? TryGuessAgentExePath()
    {
        try
        {
            string baseDir = AppContext.BaseDirectory;
            string? root = Path.GetFullPath(Path.Combine(baseDir, "..", "..", "..", ".."));
            string candidate = Path.Combine(root, "Autothink.UiaAgent", "bin", "Release", "net8.0-windows", "Autothink.UiaAgent.exe");
            return File.Exists(candidate) ? candidate : null;
        }
        catch
        {
            return null;
        }
    }

    private static string? TryGuessSelectorsPath()
    {
        try
        {
            string baseDir = AppContext.BaseDirectory;
            string? root = Path.GetFullPath(Path.Combine(baseDir, "..", "..", "..", ".."));
            string v1 = Path.Combine(root, "Docs", "组态软件自动操作", "Selectors", "autothink.v1.base.json");
            if (File.Exists(v1))
            {
                return v1;
            }

            string demo = Path.Combine(root, "Docs", "组态软件自动操作", "Selectors", "autothink.demo.json");
            return File.Exists(demo) ? demo : null;
        }
        catch
        {
            return null;
        }
    }

    private static Dictionary<string, ElementSelector> ParseSelectors(string json)
    {
        var result = new Dictionary<string, ElementSelector>(StringComparer.Ordinal);
        using JsonDocument doc = JsonDocument.Parse(json);
        JsonElement root = doc.RootElement;

        if (root.ValueKind != JsonValueKind.Object)
        {
            return result;
        }

        JsonElement selectors = root;
        if (root.TryGetProperty("selectors", out JsonElement selectorsProp) && selectorsProp.ValueKind == JsonValueKind.Object)
        {
            selectors = selectorsProp;
        }

        foreach (JsonProperty prop in selectors.EnumerateObject())
        {
            if (prop.Value.ValueKind != JsonValueKind.Object)
            {
                continue;
            }

            try
            {
                ElementSelector? selector = prop.Value.Deserialize<ElementSelector>(new JsonSerializerOptions
                {
                    PropertyNameCaseInsensitive = true,
                });

                if (selector is not null)
                {
                    result[prop.Name] = selector;
                }
            }
            catch
            {
                // ignore invalid selector entries
            }
        }

        return result;
    }
}
