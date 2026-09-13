# Builds byok-stt, installs it to %LOCALAPPDATA%\byok-stt, and creates a
# Start Menu shortcut. User-level only: no admin, no registry.
#
# Usage:  powershell -ExecutionPolicy Bypass -File scripts\install.ps1

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot

Write-Host '==> Building (cargo build --release)...'
cargo build --release --manifest-path (Join-Path $repoRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { Write-Error 'cargo build failed'; exit 1 }

$installDir = Join-Path $env:LOCALAPPDATA 'byok-stt'
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item (Join-Path $repoRoot 'target\release\byok-stt.exe') $installDir -Force
Copy-Item (Join-Path $repoRoot 'assets\app.ico') $installDir -Force

$exe = Join-Path $installDir 'byok-stt.exe'
$ico = Join-Path $installDir 'app.ico'

Write-Host '==> Creating Start Menu shortcut...'
$startMenuDir = [IO.Path]::Combine(
    [Environment]::GetFolderPath('ApplicationData'),
    'Microsoft', 'Windows', 'Start Menu', 'Programs')
$ws = New-Object -ComObject WScript.Shell
$lnk = $ws.CreateShortcut((Join-Path $startMenuDir 'byok-stt.lnk'))
$lnk.TargetPath = $exe
$lnk.WorkingDirectory = $installDir
$lnk.IconLocation = $ico
$lnk.Description = 'byok-stt - push-to-talk voice typing (Ctrl+Win)'
$lnk.Save()

Write-Host ''
Write-Host 'Installed!'
Write-Host "  exe:      $exe"
Write-Host "  shortcut: Start Menu > byok-stt (searchable)"
Write-Host ''
Write-Host 'Launch it, then right-click the tray icon > Settings to add your API key.'
Write-Host 'Hold Ctrl+Win, speak, release - text lands in the focused app.'
