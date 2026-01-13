// 说明:
// - WinForms 手工测试台主界面，用于验证选择器、原子操作与 RunFlow 的端到端效果。
// - 主要面向现场联调：加载 selectors、发起 RPC 调用、查看 StepLog 证据链。
using System.Linq;
using System.Text.Json;
using Autothink.UiaAgent.Rpc.Contracts;
using StreamJsonRpc;

namespace Autothink.UiaAgent.WinFormsHarness;

/// <summary>
/// WinFormsHarness 主窗体：承载 UIA 调试面板与日志视图。
/// </summary>
internal sealed class MainForm : Form
{
    /// <summary>
    /// 下拉框显示项：同时保存显示文本与真实值。
    /// </summary>
    private sealed class ComboItem
    {
        public ComboItem(string display, string value)
        {
            this.Display = display;
            this.Value = value;
        }

        public string Display { get; }

        public string Value { get; }

        public override string ToString() => this.Display;
    }

    private static readonly Font UiFont = new("Microsoft YaHei UI", 11F, FontStyle.Regular, GraphicsUnit.Point);
    private static readonly Font MonoFont = new("Consolas", 10F, FontStyle.Regular, GraphicsUnit.Point);
    private const int ButtonWidth = 180;
    private const int ButtonHeight = 48;
    private const int MinWideTextWidth = 720;
    private const int MinMediumTextWidth = 420;

    private readonly AgentRpcClient client = new();
    private readonly TextBox agentExePathTextBox = new() { Width = MinWideTextWidth };
    private readonly Button browseAgentButton = new() { Text = "浏览..." };
    private readonly Button startAgentButton = new() { Text = "启动Agent" };
    private readonly Button stopAgentButton = new() { Text = "停止Agent", Enabled = false };
    private readonly Button pingButton = new() { Text = "连通测试", Enabled = false };

    private readonly TextBox processNameTextBox = new() { Text = "Autothink", Width = 220 };
    private readonly TextBox mainTitleContainsTextBox = new() { Text = "AUTOTHINK", Width = 320 };
    private readonly NumericUpDown sessionTimeoutMs = new() { Minimum = 100, Maximum = 600_000, Value = 10_000, Width = 140 };
    private readonly CheckBox bringToForegroundCheckBox = new() { Text = "置前窗口", Checked = true, AutoSize = true };
    private readonly Button openSessionButton = new() { Text = "打开会话", Enabled = false };
    private readonly Button closeSessionButton = new() { Text = "关闭会话", Enabled = false };
    private readonly TextBox sessionIdTextBox = new() { Width = 520 };

    private readonly TextBox selectorJsonTextBox = new()
    {
        Multiline = true,
        ScrollBars = ScrollBars.Vertical,
        WordWrap = false,
        Height = 240,
        Width = MinWideTextWidth,
        Font = MonoFont,
    };
    private readonly Button findElementButton = new() { Text = "查找元素", Enabled = false };
    private readonly Button clickButton = new() { Text = "单击", Enabled = false };
    private readonly Button doubleClickButton = new() { Text = "双击", Enabled = false };
    private readonly Button rightClickButton = new() { Text = "右击", Enabled = false };
    private readonly Button setTextButton = new() { Text = "设置文本", Enabled = false };
    private readonly TextBox setTextValueTextBox = new() { Width = MinMediumTextWidth };
    private readonly ComboBox setTextModeComboBox = new() { Width = 220, DropDownStyle = ComboBoxStyle.DropDownList };

    private readonly Button sendKeysButton = new() { Text = "发送按键", Enabled = false };
    private readonly TextBox sendKeysTextBox = new() { Text = "CTRL+V", Width = 240 };

    private readonly ComboBox waitKindComboBox = new() { Width = 240, DropDownStyle = ComboBoxStyle.DropDownList };
    private readonly NumericUpDown waitTimeoutMs = new() { Minimum = 100, Maximum = 600_000, Value = 5_000, Width = 140 };
    private readonly Button waitUntilButton = new() { Text = "等待条件", Enabled = false };
    private readonly Button runFlowButton = new() { Text = "执行流程", Enabled = false };
    private readonly ComboBox flowNameComboBox = new() { Width = 360, DropDownStyle = ComboBoxStyle.DropDownList };
    private readonly TextBox flowArgsJsonTextBox = new()
    {
        Multiline = true,
        ScrollBars = ScrollBars.Vertical,
        WordWrap = false,
        Height = 200,
        Width = MinWideTextWidth,
        Text = "{}",
        Font = MonoFont,
    };

    private readonly TextBox selectorsFileTextBox = new() { Width = MinWideTextWidth };
    private readonly Button browseSelectorsButton = new() { Text = "浏览..." };
    private readonly Button loadSelectorsButton = new() { Text = "加载" };
    private readonly ComboBox selectorKeyComboBox = new() { Width = 320, DropDownStyle = ComboBoxStyle.DropDownList };
    private readonly Button applySelectorButton = new() { Text = "应用选择器" };

    private readonly TextBox logTextBox = new()
    {
        Multiline = true,
        ScrollBars = ScrollBars.Both,
        WordWrap = false,
        Dock = DockStyle.Fill,
        ReadOnly = true,
        Font = MonoFont,
    };

    private readonly Panel controlsPanel = new() { Dock = DockStyle.Fill, AutoScroll = true };
    private readonly TableLayoutPanel controlsLayout = new()
    {
        ColumnCount = 1,
        Dock = DockStyle.Top,
        AutoSize = true,
        AutoSizeMode = AutoSizeMode.GrowAndShrink,
        Padding = new Padding(12, 8, 12, 16),
        GrowStyle = TableLayoutPanelGrowStyle.AddRows,
    };

    private GroupBox? agentGroup;
    private GroupBox? sessionGroup;
    private GroupBox? selectorGroup;
    private GroupBox? atomicGroup;
    private GroupBox? flowGroup;

    private ElementRef? lastElement;
    private readonly Dictionary<string, ElementSelector> selectorCache = new(StringComparer.Ordinal);

    public MainForm()
    {
        this.Text = "Autothink.UiaAgent WinForms 测试台";
        this.Width = 1700;
        this.Height = 1100;
        this.MinimumSize = new Size(1400, 900);
        this.Font = UiFont;
        this.AutoScaleMode = AutoScaleMode.Font;

        this.client.StderrLine += line => this.AppendLog($"[stderr] {line}");
        this.setTextModeComboBox.Items.AddRange(
        [
            new ComboItem("替换", SetTextModes.Replace),
            new ComboItem("追加", SetTextModes.Append),
            new ComboItem("Ctrl+A 替换", SetTextModes.CtrlAReplace),
        ]);
        this.setTextModeComboBox.SelectedIndex = 0;

        this.waitKindComboBox.Items.AddRange(
        [
            new ComboItem("元素存在", WaitConditionKinds.ElementExists),
            new ComboItem("元素消失", WaitConditionKinds.ElementNotExists),
            new ComboItem("元素可用", WaitConditionKinds.ElementEnabled),
        ]);
        this.waitKindComboBox.SelectedIndex = 0;

        this.flowNameComboBox.Items.AddRange(
        [
            new ComboItem("autothink.attach", "autothink.attach"),
            new ComboItem("autothink.importVariables", "autothink.importVariables"),
            new ComboItem("autothink.importProgram.textPaste", "autothink.importProgram.textPaste"),
            new ComboItem("autothink.build", "autothink.build"),
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

        ApplyInputStyle(
            this.agentExePathTextBox,
            this.processNameTextBox,
            this.mainTitleContainsTextBox,
            this.sessionIdTextBox,
            this.selectorsFileTextBox,
            this.selectorJsonTextBox,
            this.setTextValueTextBox,
            this.sendKeysTextBox,
            this.flowArgsJsonTextBox);

        ApplyComboStyle(this.setTextModeComboBox, this.waitKindComboBox, this.flowNameComboBox, this.selectorKeyComboBox);
        ApplyNumericStyle(this.sessionTimeoutMs, this.waitTimeoutMs);

        BuildLayout();

        this.agentExePathTextBox.Text = TryGuessAgentExePath() ?? string.Empty;
        this.selectorsFileTextBox.Text = TryGuessSelectorsPath() ?? string.Empty;
    }
    private void BuildLayout()
    {
        var tabs = new TabControl { Dock = DockStyle.Fill };
        var controlTab = new TabPage("控制台");
        var logTab = new TabPage("日志");

        controlTab.Controls.Add(this.controlsPanel);
        logTab.Controls.Add(this.logTextBox);

        tabs.TabPages.Add(controlTab);
        tabs.TabPages.Add(logTab);
        this.Controls.Add(tabs);

        this.controlsPanel.Controls.Add(this.controlsLayout);

        this.agentGroup = this.BuildAgentPanel();
        this.sessionGroup = this.BuildSessionPanel();
        this.selectorGroup = this.BuildSelectorPanel();
        this.atomicGroup = this.BuildAtomicPanel();
        this.flowGroup = this.BuildFlowPanel();

        AddGroup(this.agentGroup);
        AddGroup(this.sessionGroup);
        AddGroup(this.selectorGroup);
        AddGroup(this.atomicGroup);
        AddGroup(this.flowGroup);

        this.controlsPanel.SizeChanged += (_, _) => this.AdjustWideLayout();
        this.Shown += (_, _) => this.AdjustWideLayout();
    }

    private void AddGroup(Control group)
    {
        int row = this.controlsLayout.RowCount++;
        this.controlsLayout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        group.Margin = new Padding(0, 0, 0, 14);
        this.controlsLayout.Controls.Add(group, 0, row);
    }

    private GroupBox BuildAgentPanel()
    {
        var group = CreateGroupBox("Agent 进程");
        var layout = CreateThreeColumnLayout();

        layout.RowCount = 2;
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        layout.Controls.Add(CreateLabel("Agent路径"), 0, 0);
        layout.Controls.Add(this.agentExePathTextBox, 1, 0);
        layout.Controls.Add(this.browseAgentButton, 2, 0);

        var rowButtons = CreateFlowRow();
        rowButtons.Controls.Add(this.startAgentButton);
        rowButtons.Controls.Add(this.stopAgentButton);
        rowButtons.Controls.Add(this.pingButton);
        layout.Controls.Add(rowButtons, 0, 1);
        layout.SetColumnSpan(rowButtons, 3);

        group.Controls.Add(layout);
        return group;
    }
    private GroupBox BuildSessionPanel()
    {
        var group = CreateGroupBox("会话");
        var layout = CreateSingleColumnLayout();

        var row1 = CreateFlowRow();
        row1.Controls.Add(CreateLabel("进程名"));
        row1.Controls.Add(this.processNameTextBox);
        row1.Controls.Add(CreateLabel("标题包含"));
        row1.Controls.Add(this.mainTitleContainsTextBox);
        row1.Controls.Add(CreateLabel("超时(ms)"));
        row1.Controls.Add(this.sessionTimeoutMs);
        row1.Controls.Add(this.bringToForegroundCheckBox);

        var row2 = CreateFlowRow();
        row2.Controls.Add(this.openSessionButton);
        row2.Controls.Add(this.closeSessionButton);
        row2.Controls.Add(CreateLabel("会话ID"));
        row2.Controls.Add(this.sessionIdTextBox);

        layout.Controls.Add(row1, 0, 0);
        layout.Controls.Add(row2, 0, 1);

        group.Controls.Add(layout);
        return group;
    }
    private GroupBox BuildSelectorPanel()
    {
        var group = CreateGroupBox("选择器配置");
        var layout = CreateThreeColumnLayout();

        layout.RowCount = 2;
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        layout.Controls.Add(CreateLabel("选择器文件"), 0, 0);
        layout.Controls.Add(this.selectorsFileTextBox, 1, 0);
        layout.Controls.Add(this.browseSelectorsButton, 2, 0);

        var row2 = CreateFlowRow();
        row2.Controls.Add(this.loadSelectorsButton);
        row2.Controls.Add(CreateLabel("选择器Key"));
        row2.Controls.Add(this.selectorKeyComboBox);
        row2.Controls.Add(this.applySelectorButton);

        layout.Controls.Add(row2, 0, 1);
        layout.SetColumnSpan(row2, 3);

        group.Controls.Add(layout);
        return group;
    }
    private GroupBox BuildAtomicPanel()
    {
        var group = CreateGroupBox("原子操作");
        var layout = CreateSingleColumnLayout();

        var selectorLabel = CreateLabel("选择器JSON（ElementSelector）");
        selectorLabel.Margin = new Padding(6, 4, 6, 2);

        layout.RowCount = 6;
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        layout.Controls.Add(selectorLabel, 0, 0);
        layout.Controls.Add(this.selectorJsonTextBox, 0, 1);

        var rowButtons = CreateFlowRow();
        rowButtons.Controls.Add(this.findElementButton);
        rowButtons.Controls.Add(this.clickButton);
        rowButtons.Controls.Add(this.doubleClickButton);
        rowButtons.Controls.Add(this.rightClickButton);
        layout.Controls.Add(rowButtons, 0, 2);

        var rowSetText = CreateFlowRow();
        rowSetText.Controls.Add(CreateLabel("文本"));
        rowSetText.Controls.Add(this.setTextValueTextBox);
        rowSetText.Controls.Add(CreateLabel("模式"));
        rowSetText.Controls.Add(this.setTextModeComboBox);
        rowSetText.Controls.Add(this.setTextButton);
        layout.Controls.Add(rowSetText, 0, 3);

        var rowSendKeys = CreateFlowRow();
        rowSendKeys.Controls.Add(CreateLabel("按键"));
        rowSendKeys.Controls.Add(this.sendKeysTextBox);
        rowSendKeys.Controls.Add(this.sendKeysButton);
        layout.Controls.Add(rowSendKeys, 0, 4);

        var rowWait = CreateFlowRow();
        rowWait.Controls.Add(CreateLabel("等待类型"));
        rowWait.Controls.Add(this.waitKindComboBox);
        rowWait.Controls.Add(CreateLabel("超时(ms)"));
        rowWait.Controls.Add(this.waitTimeoutMs);
        rowWait.Controls.Add(this.waitUntilButton);
        layout.Controls.Add(rowWait, 0, 5);

        group.Controls.Add(layout);
        return group;
    }
    private GroupBox BuildFlowPanel()
    {
        var group = CreateGroupBox("流程执行");
        var layout = CreateSingleColumnLayout();

        var row1 = CreateFlowRow();
        row1.Controls.Add(CreateLabel("流程名"));
        row1.Controls.Add(this.flowNameComboBox);
        row1.Controls.Add(this.runFlowButton);

        var argsLabel = CreateLabel("参数JSON（可选）");
        argsLabel.Margin = new Padding(6, 4, 6, 2);

        layout.RowCount = 3;
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        layout.Controls.Add(row1, 0, 0);
        layout.Controls.Add(argsLabel, 0, 1);
        layout.Controls.Add(this.flowArgsJsonTextBox, 0, 2);

        group.Controls.Add(layout);
        return group;
    }
    private void AdjustWideLayout()
    {
        if (this.agentGroup is null || this.sessionGroup is null || this.selectorGroup is null || this.atomicGroup is null || this.flowGroup is null)
        {
            return;
        }

        int width = Math.Max(this.controlsPanel.ClientSize.Width - SystemInformation.VerticalScrollBarWidth - 24, 900);

        SetGroupWidth(this.agentGroup, width);
        SetGroupWidth(this.sessionGroup, width);
        SetGroupWidth(this.selectorGroup, width);
        SetGroupWidth(this.atomicGroup, width);
        SetGroupWidth(this.flowGroup, width);

        int wideWidth = Math.Max(width - 260, MinWideTextWidth);
        int jsonWidth = Math.Max(width - 80, MinWideTextWidth);
        int mediumWidth = Math.Max(width - 520, MinMediumTextWidth);

        this.agentExePathTextBox.Width = wideWidth;
        this.selectorsFileTextBox.Width = wideWidth;
        this.sessionIdTextBox.Width = Math.Max(width - 320, 520);
        this.selectorJsonTextBox.Width = jsonWidth;
        this.flowArgsJsonTextBox.Width = jsonWidth;
        this.setTextValueTextBox.Width = mediumWidth;

        int flowNameWidth = Math.Max(width - 360, 360);
        this.flowNameComboBox.Width = flowNameWidth;
    }

    private async Task StartAgentAsync()
    {
        try
        {
            this.AppendLog($"启动Agent: {this.agentExePathTextBox.Text}");
            await this.client.StartAsync(this.agentExePathTextBox.Text, CancellationToken.None);
            this.AppendLog("Agent READY + JSON-RPC 已连接。");

            this.startAgentButton.Enabled = false;
            this.stopAgentButton.Enabled = true;
            this.pingButton.Enabled = true;
            this.openSessionButton.Enabled = true;
            this.runFlowButton.Enabled = true;
        }
        catch (Exception ex)
        {
            this.AppendLog($"启动失败: {ex}");
        }
    }

    private async Task StopAgentAsync()
    {
        try
        {
            this.AppendLog("停止Agent...");
            await this.client.StopAsync(CancellationToken.None);
            this.AppendLog("Agent 已停止。");
        }
        catch (Exception ex)
        {
            this.AppendLog($"停止失败: {ex}");
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
            this.AppendLog($"Ping 失败: {ex}");
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
            this.AppendLog($"打开会话失败: {ex}");
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
            this.AppendLog($"关闭会话失败: {ex}");
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
                this.AppendLog("元素引用已捕获，可继续点击/设置文本。");
            }
        }
        catch (Exception ex)
        {
            this.AppendLog($"查找元素失败: {ex}");
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
            this.AppendLog($"单击失败: {ex}");
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
            this.AppendLog($"双击失败: {ex}");
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
            this.AppendLog($"右击失败: {ex}");
        }
    }

    private async Task SetTextAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            ElementRef element = this.RequireLastElement();
            string mode = this.setTextModeComboBox.SelectedItem is ComboItem item ? item.Value : SetTextModes.Replace;
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
            this.AppendLog($"设置文本失败: {ex}");
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
            this.AppendLog($"发送按键失败: {ex}");
        }
    }

    private async Task WaitUntilAsync()
    {
        try
        {
            JsonRpc rpc = this.RequireRpc();
            string sessionId = this.RequireSessionId();
            string kind = this.waitKindComboBox.SelectedItem is ComboItem item ? item.Value : WaitConditionKinds.ElementExists;

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
            this.AppendLog($"等待条件失败: {ex}");
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
                FlowName = this.flowNameComboBox.SelectedItem is ComboItem item ? item.Value : string.Empty,
                TimeoutMs = 30_000,
                Args = args,
                ArgsJson = string.IsNullOrWhiteSpace(rawArgs) ? null : rawArgs,
            });

            this.AppendRpcResult("RunFlow", result);
        }
        catch (Exception ex)
        {
            this.AppendLog($"执行流程失败: {ex}");
        }
    }
    private JsonRpc RequireRpc()
    {
        if (this.client.Rpc is null)
        {
            throw new InvalidOperationException("RPC 未连接，请先启动 Agent。");
        }

        return this.client.Rpc;
    }

    private string RequireSessionId()
    {
        string sessionId = this.sessionIdTextBox.Text.Trim();
        if (string.IsNullOrWhiteSpace(sessionId))
        {
            throw new InvalidOperationException("会话ID为空，请先打开会话。");
        }

        return sessionId;
    }

    private ElementRef RequireLastElement()
    {
        if (this.lastElement is null)
        {
            throw new InvalidOperationException("尚未捕获元素引用，请先执行查找元素。");
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
            this.AppendLog("选择器文件路径为空。");
            return;
        }

        if (!File.Exists(path))
        {
            this.AppendLog($"选择器文件不存在: {path}");
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

            this.AppendLog($"已加载选择器: {this.selectorCache.Count} (来自 {path})");
        }
        catch (Exception ex)
        {
            this.AppendLog($"加载选择器失败: {ex}");
        }
    }

    private void ApplySelectedSelector()
    {
        string? key = this.selectorKeyComboBox.SelectedItem?.ToString();
        if (string.IsNullOrWhiteSpace(key))
        {
            this.AppendLog("请选择选择器Key。");
            return;
        }

        if (!this.selectorCache.TryGetValue(key, out ElementSelector? selector))
        {
            this.AppendLog($"选择器Key不存在: {key}");
            return;
        }

        this.selectorJsonTextBox.Text = JsonSerializer.Serialize(selector, new JsonSerializerOptions { WriteIndented = true });
        this.AppendLog($"已应用选择器: {key}");
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
            this.AppendLog(SafeSerializeValue(result.Value));
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

    private static string SafeSerializeValue<T>(T value)
    {
        try
        {
            if (value is RunFlowResponse flowResponse)
            {
                return SerializeRunFlowResponse(flowResponse);
            }

            return JsonSerializer.Serialize(value, new JsonSerializerOptions { WriteIndented = true });
        }
        catch (Exception ex)
        {
            return $"<value serialization failed: {ex.GetType().Name}: {ex.Message}>";
        }
    }

    private static string SerializeRunFlowResponse(RunFlowResponse response)
    {
        if (response.Data is null)
        {
            return "{\n  \"Data\": null\n}";
        }

        try
        {
            JsonElement element = response.Data.Value;
            string payload;
            try
            {
                payload = element.GetRawText();
            }
            catch
            {
                payload = element.ToString() ?? "null";
            }

            return "{\n  \"Data\": " + payload + "\n}";
        }
        catch (Exception ex)
        {
            return $"<flow response serialization failed: {ex.GetType().Name}: {ex.Message}>";
        }
    }
    private static GroupBox CreateGroupBox(string title)
    {
        return new GroupBox
        {
            Text = title,
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = new Padding(10, 20, 10, 12),
        };
    }

    private static TableLayoutPanel CreateThreeColumnLayout()
    {
        var layout = new TableLayoutPanel
        {
            ColumnCount = 3,
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
        };

        layout.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        layout.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        layout.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        return layout;
    }

    private static TableLayoutPanel CreateSingleColumnLayout()
    {
        var layout = new TableLayoutPanel
        {
            ColumnCount = 1,
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
        };

        layout.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        return layout;
    }

    private static FlowLayoutPanel CreateFlowRow()
    {
        return new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            WrapContents = true,
            FlowDirection = FlowDirection.LeftToRight,
            Margin = new Padding(0, 0, 0, 6),
        };
    }

    private static Label CreateLabel(string text)
    {
        return new Label { Text = text, AutoSize = true, Padding = new Padding(0, 8, 0, 0), Margin = new Padding(6, 2, 6, 2) };
    }

    private static void SetGroupWidth(Control group, int width)
    {
        group.Width = width;
        foreach (Control child in group.Controls)
        {
            child.Width = width - 20;
        }
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

    private static void ApplyInputStyle(params TextBox[] textBoxes)
    {
        foreach (TextBox box in textBoxes)
        {
            box.Margin = new Padding(6, 4, 6, 4);
            box.MinimumSize = new Size(160, 32);
        }
    }

    private static void ApplyComboStyle(params ComboBox[] comboBoxes)
    {
        foreach (ComboBox combo in comboBoxes)
        {
            combo.Margin = new Padding(6, 4, 6, 4);
            combo.MinimumSize = new Size(160, 32);
        }
    }

    private static void ApplyNumericStyle(params NumericUpDown[] numericUpDowns)
    {
        foreach (NumericUpDown numeric in numericUpDowns)
        {
            numeric.Margin = new Padding(6, 6, 6, 6);
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
