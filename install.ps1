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

# --- presentation ------------------------------------------------------------

# A small step-based UI. Write-Host colours simply drop out when the output is
# redirected, so a piped install stays plain and greppable.

$script:TotalSteps = 6
$script:Step = 0

function Write-Banner {
    Write-Host ""
    Write-Host "$Prog installer" -ForegroundColor White
}

function Write-Step([string]$Text) {
    $script:Step++
    Write-Host ""
    Write-Host "[$script:Step/$script:TotalSteps] $Text" -ForegroundColor Cyan
}

function Write-Note([string]$Text) {
    Write-Host "      > $Text" -ForegroundColor DarkGray
}

function Write-Ok([string]$Text) {
    Write-Host "      OK $Text" -ForegroundColor Green
}

# Runs cargo build --release behind a native progress bar, streaming its output.
function Invoke-CargoBuild([string]$Cargo, [string]$Dir) {
    $total = 0
    $lock = Join-Path $Dir "Cargo.lock"
    if (Test-Path $lock) {
        $total = @(Select-String -Path $lock -Pattern '^\[\[package\]\]$').Count
    }

    $done = 0
    $previous = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & $Cargo build --release 2>&1 | ForEach-Object {
        $line = "$_"
        if ($line -match '\bCompiling\b') {
            $done++
            if ($total -gt 0) {
                $percent = [Math]::Min(100, [int][Math]::Floor(($done / $total) * 100))
                $status = "$done/$total crates"
            } else {
                $percent = [Math]::Min(95, $done * 5)
                $status = "$done crates"
            }
            Write-Progress -Activity "Building $Prog (release)" -Status $status -PercentComplete $percent
        }
        Write-Host "    $line"
    }
    $code = $LASTEXITCODE
    $ErrorActionPreference = $previous
    Write-Progress -Activity "Building $Prog (release)" -Completed
    return $code
}

if ([string]::IsNullOrWhiteSpace($Prefix)) {
    if ([string]::IsNullOrWhiteSpace($env:AGENTIC_DOC_PREFIX)) {
        $Prefix = $DefaultPrefix
    } else {
        $Prefix = $env:AGENTIC_DOC_PREFIX
    }
}

# --- locate the checkout -----------------------------------------------------

Write-Banner

Write-Step "Locating source"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ScriptDir

if (-not (Test-Path (Join-Path $ScriptDir "Cargo.toml"))) {
    Fail "Cargo.toml not found in $ScriptDir; run this script from the repository."
}
if (-not (Test-Path (Join-Path $ScriptDir "src"))) {
    Fail "src not found in $ScriptDir; run this script from the repository."
}
Write-Note "checkout $ScriptDir"

# --- locate cargo ------------------------------------------------------------

Write-Step "Locating the Rust toolchain"

$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    Fail "cargo not found. Install a Rust toolchain (https://rustup.rs) and re-run."
}
Write-Note "$(& $cargo.Source --version)"

Write-Step "Checking C compiler"
$cc = Get-Command cl.exe -ErrorAction SilentlyContinue
if (-not $cc) { $cc = Get-Command link.exe -ErrorAction SilentlyContinue }
if (-not $cc) {
    Fail "no C compiler found. Install Visual Studio Build Tools with the C++ workload (cl.exe/link.exe), then re-run; tree-sitter grammars compile C."
}
Write-Note "found $($cc.Source) (required to compile tree-sitter grammars)"

# --- build -------------------------------------------------------------------

Write-Step "Building $Prog (release)"

$buildStatus = Invoke-CargoBuild $cargo.Source $ScriptDir
if ($buildStatus -ne 0) {
    Fail "the build failed (cargo exited with $buildStatus)."
}

$Built = Join-Path $ScriptDir "target\release\$Prog.exe"
if (-not (Test-Path $Built)) {
    Fail "the build did not produce $Built."
}
Write-Ok "compiled the release binary"

# --- install -----------------------------------------------------------------

Write-Step "Installing to $Prefix"

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
    Write-Ok "Already up to date: $Binary"
} else {
    Copy-Item -Path $Built -Destination $Binary -Force
    Write-Ok "Installed: $Binary"
    if ($Before -ne "none" -and $Before -ne "unknown") {
        Write-Note "replaced $Before"
    }
}

$After = (& $Binary --version 2>$null)
if (-not $After) {
    Fail "the installed binary at $Binary failed to run."
}

# --- PATH (current user) -----------------------------------------------------

Write-Step "Configuring PATH"

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($null -eq $userPath) { $userPath = "" }
$entries = $userPath.Split(";") | Where-Object { $_ -ne "" }

if ($entries -contains $Bindir) {
    Write-Ok "Already in PATH (user): $Bindir"
} elseif ($env:AGENTIC_DOC_NO_PATH -eq "1") {
    Write-Warning "$Bindir is not in PATH; add it yourself."
} else {
    # Prepend so this copy wins over any other one.
    $newPath = (@($Bindir) + $entries) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Ok "Added to PATH (user): $Bindir"
    Write-Note "Open a new terminal for the change to take effect."
}

# --- done --------------------------------------------------------------------

Write-Host ""
Write-Host "$After is ready at $Binary" -ForegroundColor Green
