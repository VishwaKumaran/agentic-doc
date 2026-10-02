# Install the agentic-doc binary for the current user, on their PATH.
#
# The binary is built from the checkout this script lives in: nothing is
# downloaded and no release is required.
#
# Usage:  .\install.ps1 [-Prefix DIR]
# Env:    AGENTIC_DOC_PREFIX    same as -Prefix (the parameter wins)
#         AGENTIC_DOC_NO_PATH=1 refuse to touch the user PATH
#
# No administrator rights are required: the destination is under %LOCALAPPDATA%
# and the PATH entry is set for the current user only.
#
# Requires a Rust toolchain (https://rustup.rs).

[CmdletBinding()]
param(
    [string]$Prefix = ""
)

$ErrorActionPreference = "Stop"
$Prog = "agentic-doc"
$DefaultPrefix = Join-Path $env:LOCALAPPDATA "Programs\agentic-doc"

function Fail([string]$Message) {
    Write-Error "Error: $Message"
    exit 1
}

if ([string]::IsNullOrWhiteSpace($Prefix)) {
    if ([string]::IsNullOrWhiteSpace($env:AGENTIC_DOC_PREFIX)) {
        $Prefix = $DefaultPrefix
    } else {
        $Prefix = $env:AGENTIC_DOC_PREFIX
    }
}

# --- locate the checkout -----------------------------------------------------

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ScriptDir

if (-not (Test-Path (Join-Path $ScriptDir "Cargo.toml"))) {
    Fail "Cargo.toml not found in $ScriptDir; run this script from the repository."
}
if (-not (Test-Path (Join-Path $ScriptDir "src"))) {
    Fail "src not found in $ScriptDir; run this script from the repository."
}

# --- locate cargo ------------------------------------------------------------

$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    Fail "cargo not found. Install a Rust toolchain (https://rustup.rs) and re-run."
}

# --- build -------------------------------------------------------------------

Write-Host "Building $Prog (release)..."
& $cargo.Source build --release
$Built = Join-Path $ScriptDir "target\release\$Prog.exe"
if (-not (Test-Path $Built)) {
    Fail "the build did not produce $Built."
}

# --- install -----------------------------------------------------------------

$Bindir = Join-Path $Prefix "bin"
$Binary = Join-Path $Bindir "$Prog.exe"

if (-not (Test-Path $Bindir)) {
    New-Item -ItemType Directory -Path $Bindir -Force | Out-Null
}

$Before = "none"
if (Test-Path $Binary) {
    $Before = (& $Binary --version 2>$null)
    if (-not $Before) { $Before = "unknown" }
}

$same = $false
if (Test-Path $Binary) {
    $same = (Get-FileHash $Binary).Hash -eq (Get-FileHash $Built).Hash
}

if ($same) {
    Write-Host "Already up to date: $Binary"
} else {
    Copy-Item -Path $Built -Destination $Binary -Force
    Write-Host "Installed: $Binary"
}

$After = (& $Binary --version 2>$null)
if (-not $After) {
    Fail "the installed binary at $Binary failed to run."
}

# --- PATH (current user) -----------------------------------------------------

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($null -eq $userPath) { $userPath = "" }
$entries = $userPath.Split(";") | Where-Object { $_ -ne "" }

if ($entries -contains $Bindir) {
    Write-Host "Already in PATH (user): $Bindir"
} elseif ($env:AGENTIC_DOC_NO_PATH -eq "1") {
    Write-Warning "$Bindir is not in PATH; add it yourself."
} else {
    # Prepend so this copy wins over any other one.
    $newPath = (@($Bindir) + $entries) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Added to PATH (user): $Bindir"
    Write-Host "Open a new terminal for the change to take effect."
}

Write-Host "Version:   $After (before: $Before)"