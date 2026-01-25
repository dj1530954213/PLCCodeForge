[CmdletBinding()]
param(
    [string]$IdaExe,
    [string[]]$IdbPaths = @(),
    [int[]]$Ports = @(13337, 13338, 13339, 13340, 13341),
    [int]$Instances = 5,
    [string]$McpHost = "127.0.0.1",
    [bool]$AutoStart = $false
)

function Resolve-IdaExe {
    param([string]$IdaExe)

    if ($IdaExe -and (Test-Path $IdaExe)) {
        return (Resolve-Path $IdaExe).Path
    }

    if ($env:IDA_PATH -and (Test-Path $env:IDA_PATH)) {
        return (Resolve-Path $env:IDA_PATH).Path
    }

    if ($env:IDADIR) {
        $candidate = Join-Path $env:IDADIR "ida64.exe"
        if (Test-Path $candidate) {
            return (Resolve-Path $candidate).Path
        }
        $candidate = Join-Path $env:IDADIR "ida.exe"
        if (Test-Path $candidate) {
            return (Resolve-Path $candidate).Path
        }
    }

    $cmd = Get-Command ida64.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Path
    }
    $cmd = Get-Command ida.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Path
    }

    $candidates = @(
        "C:\Program Files\IDA Professional 9.0\ida64.exe",
        "C:\Program Files\IDA Professional 9.0\ida.exe",
        "C:\Program Files\IDA Professional 8.4\ida64.exe",
        "C:\Program Files\IDA Professional 8.4\ida.exe",
        "C:\Program Files\IDA Professional\ida64.exe",
        "C:\Program Files\IDA Professional\ida.exe",
        "C:\Program Files\IDA\ida64.exe",
        "C:\Program Files\IDA\ida.exe"
    )
    foreach ($path in $candidates) {
        if (Test-Path $path) {
            return $path
        }
    }

    throw "Unable to locate ida64.exe. Set -IdaExe or IDADIR/IDA_PATH."
}

$idaExePath = Resolve-IdaExe -IdaExe $IdaExe

if ($Instances -lt 1) {
    throw "Instances must be >= 1."
}
if ($Instances -gt $Ports.Count) {
    throw "Instances ($Instances) exceeds Ports count ($($Ports.Count))."
}

for ($i = 0; $i -lt $Instances; $i++) {
    $port = $Ports[$i]
    $env:IDA_MCP_HOST = $McpHost
    $env:IDA_MCP_PORT = $port
    # IDA reads IDA_MCP_URL first; set it per instance to avoid global overrides.
    $env:IDA_MCP_URL = "http://$McpHost`:$port"
    if ($AutoStart) {
        $env:IDA_MCP_AUTOSTART = "1"
    } else {
        Remove-Item Env:IDA_MCP_AUTOSTART -ErrorAction SilentlyContinue
    }

    $args = @()
    $idb = $null
    if ($i -lt $IdbPaths.Count) {
        $idb = $IdbPaths[$i]
        if ($idb) {
            if (Test-Path $idb) {
                $args += $idb
            } else {
                Write-Warning "IDB not found: $idb"
            }
        }
    }

    Start-Process -FilePath $idaExePath -ArgumentList $args | Out-Null
    if ($idb) {
        Write-Host ("Started IDA port {0} -> {1}" -f $port, $idb)
    } else {
        Write-Host ("Started IDA port {0}" -f $port)
    }
}
