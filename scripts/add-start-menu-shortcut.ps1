# Creates (or updates) a Start Menu shortcut for byok-stt.
# User-level only: no admin rights, no registry edits.
param(
    [string]$ExePath = "$PSScriptRoot\..\target\release\byok-stt.exe",
    [string]$IconPath = "$PSScriptRoot\..\assets\app.ico"
)

if (-not (Test-Path $ExePath)) {
    Write-Error "byok-stt.exe not found at $ExePath. Build first: cargo build --release"
    exit 1
}
if (-not (Test-Path $IconPath)) { $IconPath = $null }

$startMenuDir = [IO.Path]::Combine(
    [Environment]::GetFolderPath('ApplicationData'),
    'Microsoft', 'Windows', 'Start Menu', 'Programs')
$lnkPath = Join-Path $startMenuDir 'byok-stt.lnk'

$ws = New-Object -ComObject WScript.Shell
$lnk = $ws.CreateShortcut($lnkPath)
$lnk.TargetPath = (Resolve-Path $ExePath).Path
$lnk.WorkingDirectory = Split-Path (Resolve-Path $ExePath).Path
if ($IconPath) { $lnk.IconLocation = (Resolve-Path $IconPath).Path }
$lnk.Description = 'byok-stt - push-to-talk voice typing (Ctrl+Win)'
$lnk.Save()

Write-Host "Shortcut created: $lnkPath"
Write-Host 'byok-stt is now findable in the Windows Start Menu / search.'
