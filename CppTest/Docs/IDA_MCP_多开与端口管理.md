# IDA MCP 多开与端口管理（启动脚本与操作流程）

本文记录多开 IDA + MCP 的标准操作方式，确保每个实例使用不同端口并能稳定连接。

## 1. Codex MCP 配置（多端口）

配置文件：
`C:\Users\DELL\.codex\config.toml`

示例端口映射（按需调整）：
```toml
[mcp_servers.ida-pro-mcp]
url = "http://127.0.0.1:13337/mcp"

[mcp_servers.ida-pro-mcp-1]
url = "http://127.0.0.1:13338/mcp"

[mcp_servers.ida-pro-mcp-2]
url = "http://127.0.0.1:13339/mcp"

[mcp_servers.ida-pro-mcp-3]
url = "http://127.0.0.1:13340/mcp"

[mcp_servers.ida-pro-mcp-4]
url = "http://127.0.0.1:13341/mcp"
```

## 2. IDA MCP 端口优先级（关键）

IDA MCP 插件读取环境变量的优先级如下：
1. `IDA_MCP_URL`（最高优先级，示例：`http://127.0.0.1:13337`）
2. `IDA_MCP_HOST` + `IDA_MCP_PORT`
3. 默认值：`127.0.0.1:13337`

如果所有实例都跑到同一个端口，通常是系统里存在全局 `IDA_MCP_URL/IDA_MCP_PORT` 导致覆盖。

## 3. 启动脚本（推荐）

脚本位置：
`c:\Program Files\Git\code\PLCCodeForge\CppTest\ida-mcp-launch.ps1`

参数说明：
- `-IdaExe`：IDA 可执行文件路径（`ida.exe` 或 `ida64.exe`）
- `-IdbPaths`：要打开的 IDB 列表
- `-Ports`：端口列表（默认 13337-13341）
- `-Instances`：启动实例数（<= 端口数）
- `-McpHost`：默认 `127.0.0.1`
- `-AutoStart`：是否自动启动 MCP（默认 `false`，避免启动/停止循环）

### 启动 5 个实例（端口 13337-13341）
```powershell
.\ida-mcp-launch.ps1 -IdaExe "C:\Program Files\IDA Professional 9.0\ida.exe" -Instances 5
```

### 启动 3 个实例并打开指定 IDB
```powershell
.\ida-mcp-launch.ps1 -IdaExe "C:\Program Files\IDA Professional 9.0\ida.exe" -Instances 3 -IdbPaths `
  "C:\AutoThink\dllDPLogic.dll.i64","C:\AutoThink\dll_DPFrame.dll.i64","C:\AutoThink\dllDPSource.dll.i64"
```

### 自动启动 MCP（不再手动点插件）
```powershell
.\ida-mcp-launch.ps1 -IdaExe "C:\Program Files\IDA Professional 9.0\ida.exe" -Instances 3 -IdbPaths `
  "C:\AutoThink\dllDPLogic.dll.i64","C:\AutoThink\dll_DPFrame.dll.i64","C:\AutoThink\dllDPSource.dll.i64" -AutoStart:$true
```

## 4. 手动启动 MCP（稳定方式）

在每个 IDA 窗口中执行：
- 菜单：`Edit -> Plugins -> MCP`
- 或热键：`Ctrl-Alt-M`

注意：该菜单是“开关式”，点第二次会停止服务。

## 5. 仅关闭脚本启动的 IDA（不影响你自己开的）

按 IDB 路径匹配并关闭（安全关窗 -> 强制结束）：
```powershell
$targets = @(
  "C:\AutoThink\dllDPLogic.dll.i64",
  "C:\AutoThink\dll_DPFrame.dll.i64",
  "C:\AutoThink\dllDPSource.dll.i64"
)

$idaProcs = Get-CimInstance Win32_Process -Filter "Name='ida.exe' OR Name='ida64.exe'" -ErrorAction SilentlyContinue
$toStop = @()
foreach ($p in $idaProcs) {
  $cmd = $p.CommandLine
  if (-not $cmd) { continue }
  foreach ($t in $targets) {
    if ($cmd -like "*$t*") { $toStop += $p; break }
  }
}
$toStop = $toStop | Sort-Object -Property ProcessId -Unique

foreach ($p in $toStop) {
  $proc = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue
  if ($proc) {
    try { $null = $proc.CloseMainWindow() } catch {}
    Start-Sleep -Milliseconds 500
    $stillRunning = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue
    if ($stillRunning) { Stop-Process -Id $p.ProcessId -Force }
  }
}
```

## 6. 验证端口是否生效

检查端口监听：
```powershell
Get-NetTCPConnection -LocalPort 13337,13338,13339,13340,13341 -ErrorAction SilentlyContinue
```

在 IDA 输出窗口中看到：
`Server started: http://127.0.0.1:1333X/mcp` 即表示启动成功。

## 7. 已记录的工程路径

- `C:\AutoThink\dllDPLogic.dll.i64`
- `C:\AutoThink\dll_DPFrame.dll.i64`
- `C:\AutoThink\dllDPSource.dll.i64`
- `C:\AutoThink\AutoThink.exe.i64`
