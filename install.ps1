# Install the agentic-doc binary system-wide, so that every user on this
# machine finds it on their PATH.
#
# The binary is built from the checkout this script lives in: nothing is
# downloaded and no release is required.
#
# Usage:  .\install.ps1 [-Prefix DIR]
# Env:    AGENTIC_DOC_PREFIX   same as -Prefix (the parameter wins)
#
# Requires: a Rust toolchain (https://rustup.rs) and an elevated PowerShell
# session when the destination is not writable by the current user.

[CmdletBinding()]
param(
    [string]$Prefix = ""
)

$ErrorActionPreference = "Stop"
$Prog = "agentic-doc"
$DefaultPrefix = Join-Path $env:ProgramFiles "agentic-doc"

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

# --- privileges --------------------------------------------------------------

$Bindir = Join-Path $Prefix "bin"
$Binary = Join-Path $Bindir "$Prog.exe"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    $writable = (Test-Path $Prefix) -and (
        Test-Path $Bindir
    ) -and (
        (Get-Item $Bindir).Attributes -notmatch "ReadOnly"
    )
    if (-not $writable) {
        Fail "cannot write to $Bindir (administrator privileges are required).`nRe-run from an elevated PowerShell:`n  .\install.ps1 -Prefix `"$Prefix`""
    }
}

# --- build -------------------------------------------------------------------

Write-Host "Building $Prog (release)..."
& $cargo.Source build --release
$Built = Join-Path $ScriptDir "target\release\$Prog.exe"
if (-not (Test-Path $Built)) {
    Fail "the build did not produce $Built."
}

# --- install -----------------------------------------------------------------

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

# --- PATH (machine scope) ----------------------------------------------------

$machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($null -eq $machinePath) { $machinePath = "" }
$entries = $machinePath.Split(";") | Where-Object { $_ -ne "" }
if ($entries -notcontains $Bindir) {
    $newPath = (@($entries) + $Bindir) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
    Write-Host "Added to PATH (machine): $Bindir"
    Write-Host "Open a new terminal for the change to take effect."
} else {
    Write-Host "Already in PATH (machine): $Bindir"
}

Write-Host "Version:   $After (before: $Before)"