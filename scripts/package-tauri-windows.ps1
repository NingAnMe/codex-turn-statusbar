$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$PackageName = "CodexTurnStatusBar-0.2.2-windows-x64"
$Dist = Join-Path $Root "dist-cross"
$PackageDir = Join-Path $Dist $PackageName

Set-Location $Root
node scripts/generate-tauri-icon.mjs | Out-Null
cargo build --release -p codex-turn-statusbar-tauri -p codex-turn-notify

if (Test-Path $PackageDir) {
  Remove-Item -Recurse -Force $PackageDir
}
New-Item -ItemType Directory -Force -Path (Join-Path $PackageDir "scripts") | Out-Null

Copy-Item "target\release\codex-turn-statusbar-tauri.exe" (Join-Path $PackageDir "CodexTurnStatusBar.exe")
Copy-Item "target\release\codex-turn-notify.exe" (Join-Path $PackageDir "codex-turn-notify.exe")
Copy-Item "scripts\install-cross-platform-notify.ps1" (Join-Path $PackageDir "scripts\install-cross-platform-notify.ps1")
Copy-Item "PACKAGE_README.md" (Join-Path $PackageDir "README.md")

$Zip = Join-Path $Dist "$PackageName.zip"
if (Test-Path $Zip) {
  Remove-Item -Force $Zip
}
Compress-Archive -Path $PackageDir -DestinationPath $Zip
Write-Host $Zip
