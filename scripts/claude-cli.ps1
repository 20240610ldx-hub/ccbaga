param(
    [string]$action = "read"
)
$ErrorActionPreference = "Stop"

if ($action -notin @("read", "uninstall", "reinstall")) {
    exit 2
}

$root = $env:CC_CLI_USER_DIR
if (-not $root) {
    $root = $env:USERPROFILE
}

$package = "@anthropic-ai/claude-code"

function Step([string]$msg) {
    Write-Output "CC_CLI_STEP=$msg"
}

function Fail([string]$msg) {
    [Console]::Error.WriteLine($msg)
    exit 1
}

$methods = [System.Collections.Generic.List[string]]::new()
$npmCmds = [System.Collections.Generic.List[string]]::new()

# 1. 检查原生安装
$nativePaths = @(
    (Join-Path $root ".local\share\claude"),
    (Join-Path $env:LOCALAPPDATA "Programs\claude"),
    (Join-Path $env:LOCALAPPDATA "Claude\bin")
)
$nativeFound = $false
foreach ($np in $nativePaths) {
    if (Test-Path $np) {
        $nativeFound = $true
        break
    }
}
if ($nativeFound) {
    $methods.Add("native")
}

# 2. 检查 npm 全局安装
$npmExe = (Get-Command npm.cmd -ErrorAction SilentlyContinue).Source
if (-not $npmExe) {
    $npmExe = (Get-Command npm -ErrorAction SilentlyContinue).Source
}
if ($npmExe) {
    try {
        $npmRoot = (& $npmExe root -g 2>$null).Trim()
        if ($npmRoot -and (Test-Path (Join-Path $npmRoot $package))) {
            $methods.Add("npm")
            $npmCmds.Add($npmExe)
        }
    } catch {}
}

# 3. 检查 bun 全局安装
$bunExe = (Get-Command bun.exe -ErrorAction SilentlyContinue).Source
if (-not $bunExe -and (Test-Path (Join-Path $root ".bun\bin\bun.exe"))) {
    $bunExe = Join-Path $root ".bun\bin\bun.exe"
}
if ($bunExe) {
    try {
        $bunGlobal = Join-Path $root ".bun\install\global\node_modules\$package"
        if (Test-Path $bunGlobal) {
            $methods.Add("bun")
        }
    } catch {}
}

# 4. 定位 claude 可执行程序
$cli = ""
$cliCmd = (Get-Command claude.exe -ErrorAction SilentlyContinue).Source
if (-not $cliCmd) {
    $cliCmd = (Get-Command claude.cmd -ErrorAction SilentlyContinue).Source
}
if (-not $cliCmd) {
    $cliCmd = (Get-Command claude -ErrorAction SilentlyContinue).Source
}
if ($cliCmd) {
    $cli = $cliCmd
} else {
    foreach ($cand in @(
        (Join-Path $root ".local\bin\claude.exe"),
        (Join-Path $root ".local\bin\claude.cmd"),
        (Join-Path $env:LOCALAPPDATA "Programs\claude\claude.exe"),
        (Join-Path $root ".bun\bin\claude.exe")
    )) {
        if (Test-Path $cand) {
            $cli = $cand
            break
        }
    }
}

if ($cli -and $methods.Count -eq 0) {
    $methods.Add("native")
}

$version = ""
if ($cli) {
    $env:DISABLE_AUTOUPDATER = "1"
    try {
        $version = (& $cli --version 2>$null).Trim()
    } catch {
        $version = ""
    }
}

if ($action -eq "read") {
    $methodStr = ($methods | Select-Object -Unique) -join " / "
    Write-Output "CC_CLI_METHOD=$methodStr"
    Write-Output "CC_CLI_PATH=$cli"
    Write-Output "CC_CLI_VERSION=$version"
    Write-Output "CC_CLI_DONE=1"
    exit 0
}

if ($action -eq "uninstall") {
    Step "正在卸载 Claude Code CLI。"
    if ($npmCmds.Count -gt 0) {
        foreach ($n in $npmCmds) {
            & $n uninstall -g $package 2>$null | Out-Null
        }
    }
    if ($bunExe) {
        & $bunExe remove -g $package 2>$null | Out-Null
    }
    foreach ($np in $nativePaths) {
        if (Test-Path $np) {
            Remove-Item -Recurse -Force $np -ErrorAction SilentlyContinue
        }
    }
    Write-Output "CC_CLI_METHOD="
    Write-Output "CC_CLI_PATH="
    Write-Output "CC_CLI_VERSION="
    Write-Output "CC_CLI_DONE=1"
    exit 0
}

if ($action -eq "reinstall") {
    Step "正在卸载旧版 Claude Code CLI。"
    if ($npmCmds.Count -gt 0) {
        foreach ($n in $npmCmds) {
            & $n uninstall -g $package 2>$null | Out-Null
        }
    }
    if ($bunExe) {
        & $bunExe remove -g $package 2>$null | Out-Null
    }
    Step "正在重装 Claude Code CLI。"
    $reinstallMethod = "npm"
    if ($npmExe) {
        & $npmExe install -g "$package@latest" 2>$null | Out-Null
        $reinstallMethod = "npm"
    } elseif ($bunExe) {
        & $bunExe install -g "$package@latest" 2>$null | Out-Null
        $reinstallMethod = "bun"
    } else {
        Step "正在使用官方 Windows 安装脚本。"
        try {
            irm https://claude.ai/install.ps1 | iex
            $reinstallMethod = "native"
        } catch {
            Fail "Claude Code 官方安装器执行失败，请检查网络后重试。"
        }
    }
    Step "正在核验 Claude Code CLI 版本。"
    $cli = (Get-Command claude.exe -ErrorAction SilentlyContinue).Source
    if (-not $cli) {
        $cli = (Get-Command claude.cmd -ErrorAction SilentlyContinue).Source
    }
    if (-not $cli) {
        Fail "Claude Code 重装后未找到可执行程序。"
    }
    $env:DISABLE_AUTOUPDATER = "1"
    $version = (& $cli --version 2>$null).Trim()
    if (-not $version -or ($version -notlike "*Claude*")) {
        Fail "Claude Code 重装后的版本核验失败。"
    }
    Write-Output "CC_CLI_METHOD=$reinstallMethod"
    Write-Output "CC_CLI_PATH=$cli"
    Write-Output "CC_CLI_VERSION=$version"
    Write-Output "CC_CLI_DONE=1"
    exit 0
}

