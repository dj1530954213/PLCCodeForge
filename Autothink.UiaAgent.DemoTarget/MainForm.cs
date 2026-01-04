namespace Autothink.UiaAgent.DemoTarget;

internal sealed class MainForm : Form
{
    private Form? importDialog;

    private static readonly Font TopBarFont = new("Segoe UI", 10F, FontStyle.Regular, GraphicsUnit.Point);
    private const int TopBarButtonHeight = 44;

    private readonly RichTextBox programEditor = new()
    {
        Name = "programEditor",
        AccessibleName = "programEditor",
        Dock = DockStyle.Fill,
    };

    private readonly Button openImportButton = new()
    {
        Name = "openImportButton",
        AccessibleName = "openImportButton",
        Text = "导入变量…",
        Width = 220,
        Height = TopBarButtonHeight,
        Font = TopBarFont,
    };

    private readonly Button importProgramButton = new()
    {
        Name = "importProgramButton",
        AccessibleName = "importProgramButton",
        Text = "导入程序…",
        Width = 220,
        Height = TopBarButtonHeight,
        Font = TopBarFont,
    };

    private readonly Button importProgramOkButton = new()
    {
        Name = "importProgramOkButton",
        AccessibleName = "importProgramOkButton",
        Text = "导入确认",
        Width = 180,
        Height = TopBarButtonHeight,
        Font = TopBarFont,
    };

    private readonly Button buildButton = new()
    {
        Name = "buildButton",
        AccessibleName = "buildButton",
        Text = "编译",
        Width = 160,
        Height = TopBarButtonHeight,
        Font = TopBarFont,
    };

    private readonly Label statusLabel = new()
    {
        Name = "statusLabel",
        Text = "READY",
        AutoSize = true,
        Font = TopBarFont,
    };

    private readonly Label programPasteIndicator = new()
    {
        Name = "programPasteIndicator",
        AccessibleName = "programPasteIndicator",
        Text = "Program pasted",
        AutoSize = true,
        Visible = false,
        Font = TopBarFont,
    };

    public MainForm()
    {
        this.Text = "AUTOTHINK Demo Target (普通型)";
        this.Width = 1000;
        this.Height = 700;

        this.openImportButton.Click += (_, _) =>
        {
            ShowImportDialog();
        };

        this.importProgramButton.Click += (_, _) =>
        {
            this.programEditor.Focus();
            SetStatusText("Program editor ready");
        };

        this.importProgramOkButton.Click += (_, _) =>
        {
            this.programEditor.Focus();
            SetStatusText("Program confirmed");
        };

        this.programEditor.TextChanged += (_, _) =>
        {
            bool hasText = this.programEditor.TextLength > 0;
            this.programPasteIndicator.Visible = hasText;
            if (hasText)
            {
                SetStatusText("Program pasted");
            }
        };

        this.buildButton.Click += async (_, _) =>
        {
            if (!this.buildButton.Enabled)
            {
                return;
            }

            ShowBuildPopup();

            this.buildButton.Enabled = false;
            SetStatusText("Building...");
            await Task.Delay(800);
            SetStatusText("Build OK");
            this.buildButton.Enabled = true;
        };

        var topBar = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 2,
            Padding = new Padding(8, 6, 8, 6),
        };

        topBar.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        topBar.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        var buttonRow = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            AutoSize = true,
            WrapContents = true,
            Margin = new Padding(0),
        };

        var statusRow = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            AutoSize = true,
            WrapContents = true,
            Margin = new Padding(0, 4, 0, 0),
        };

        ApplyTopBarSpacing(this.importProgramButton);
        ApplyTopBarSpacing(this.importProgramOkButton);
        ApplyTopBarSpacing(this.openImportButton);
        ApplyTopBarSpacing(this.buildButton);
        ApplyTopBarSpacing(this.statusLabel);
        ApplyTopBarSpacing(this.programPasteIndicator);

        buttonRow.Controls.Add(this.importProgramButton);
        buttonRow.Controls.Add(this.importProgramOkButton);
        buttonRow.Controls.Add(this.openImportButton);
        buttonRow.Controls.Add(this.buildButton);

        var statusPrefix = new Label
        {
            Text = "Status:",
            AutoSize = true,
            Font = TopBarFont,
        };
        ApplyTopBarSpacing(statusPrefix);
        statusRow.Controls.Add(statusPrefix);
        statusRow.Controls.Add(this.statusLabel);
        statusRow.Controls.Add(this.programPasteIndicator);

        topBar.Controls.Add(buttonRow, 0, 0);
        topBar.Controls.Add(statusRow, 0, 1);

        this.Controls.Add(this.programEditor);
        this.Controls.Add(topBar);

        SetStatusText(this.statusLabel.Text);
        this.Shown += (_, _) =>
        {
            this.programEditor.Focus();
            BeginInvoke(new Action(ShowStartupPopup));
        };
    }

    private void ShowBuildPopup()
    {
        using var dialog = new Form
        {
            Text = "Build Confirm",
            Name = "buildConfirmDialog",
            AccessibleName = "buildConfirmDialog",
            StartPosition = FormStartPosition.CenterParent,
            FormBorderStyle = FormBorderStyle.FixedDialog,
            MinimizeBox = false,
            MaximizeBox = false,
            ShowInTaskbar = false,
            Width = 320,
            Height = 160,
        };

        var message = new Label
        {
            Text = "是否继续编译？",
            AutoSize = true,
            Left = 20,
            Top = 20,
        };

        var okButton = new Button
        {
            Text = "确定",
            Name = "popupOkButton",
            AccessibleName = "popupOkButton",
            DialogResult = DialogResult.OK,
            Width = 80,
            Left = 110,
            Top = 70,
        };

        var cancelButton = new Button
        {
            Text = "取消",
            Name = "popupCancelButton",
            AccessibleName = "popupCancelButton",
            DialogResult = DialogResult.Cancel,
            Width = 80,
            Left = 200,
            Top = 70,
        };

        dialog.AcceptButton = okButton;
        dialog.CancelButton = cancelButton;

        dialog.Controls.Add(message);
        dialog.Controls.Add(okButton);
        dialog.Controls.Add(cancelButton);

        dialog.ShowDialog(this);
    }

    private void ShowImportDialog()
    {
        if (this.importDialog is { IsDisposed: false })
        {
            this.importDialog.BringToFront();
            return;
        }

        var dialog = new Form
        {
            Text = "变量导入",
            Name = "importDialog",
            AccessibleName = "importDialog",
            StartPosition = FormStartPosition.CenterParent,
            FormBorderStyle = FormBorderStyle.FixedDialog,
            MinimizeBox = false,
            MaximizeBox = false,
            ShowInTaskbar = false,
            Width = 520,
            Height = 180,
        };

        var filePathEdit = new TextBox
        {
            Name = "filePathEdit",
            AccessibleName = "filePathEdit",
            Width = 360,
        };

        var okButton = new Button
        {
            Name = "importOkButton",
            AccessibleName = "importOkButton",
            Text = "确定",
            DialogResult = DialogResult.OK,
            Width = 80,
        };

        var cancelButton = new Button
        {
            Name = "importCancelButton",
            AccessibleName = "importCancelButton",
            Text = "取消",
            DialogResult = DialogResult.Cancel,
            Width = 80,
        };

        var layout = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            WrapContents = false,
            AutoScroll = true,
        };

        layout.Controls.Add(new Label { Text = "变量表路径", AutoSize = true, Padding = new Padding(0, 8, 0, 0) });
        layout.Controls.Add(filePathEdit);
        layout.Controls.Add(okButton);
        layout.Controls.Add(cancelButton);

        dialog.Controls.Add(layout);

        okButton.Click += (_, _) =>
        {
            SetStatusText($"Imported: {filePathEdit.Text}");
            dialog.Close();
            this.programEditor.Focus();
        };

        cancelButton.Click += (_, _) => dialog.Close();

        dialog.FormClosed += (_, _) => this.importDialog = null;
        this.importDialog = dialog;
        dialog.Show(this);
    }

    private void SetStatusText(string text)
    {
        this.statusLabel.Text = text;
        this.statusLabel.AccessibleName = text;
    }

    private static void ApplyTopBarSpacing(Control control)
    {
        control.Margin = new Padding(6, 4, 6, 4);
    }

    private void ShowStartupPopup()
    {
        using var dialog = new Form
        {
            Text = "提示",
            Name = "startupDialog",
            AccessibleName = "startupDialog",
            StartPosition = FormStartPosition.CenterParent,
            FormBorderStyle = FormBorderStyle.FixedDialog,
            MinimizeBox = false,
            MaximizeBox = false,
            ShowInTaskbar = false,
            Width = 320,
            Height = 160,
        };

        var message = new Label
        {
            Text = "启动提示：请确认配置已就绪。",
            AutoSize = true,
            Left = 20,
            Top = 20,
        };

        var okButton = new Button
        {
            Text = "确定",
            Name = "popupOkButton",
            AccessibleName = "popupOkButton",
            DialogResult = DialogResult.OK,
            Width = 80,
            Left = 200,
            Top = 70,
        };

        dialog.AcceptButton = okButton;
        dialog.Controls.Add(message);
        dialog.Controls.Add(okButton);
        dialog.ShowDialog(this);
    }
}
