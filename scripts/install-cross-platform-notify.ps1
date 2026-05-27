$ErrorActionPreference = "Stop"

$PackageDir = Split-Path -Parent $PSScriptRoot
$NotifySource = Join-Path $PackageDir "codex-turn-notify.exe"
$CodexHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $env:USERPROFILE ".codex" }
$NotifyTarget = Join-Path $CodexHome "bin\codex-turn-notify.exe"
$ConfigFile = Join-Path $CodexHome "config.toml"

if (!(Test-Path $NotifySource)) {
  throw "Missing notify executable: $NotifySource"
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $NotifyTarget) | Out-Null
Copy-Item -Force $NotifySource $NotifyTarget

if (Test-Path $ConfigFile) {
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
  Copy-Item $ConfigFile "$ConfigFile.bak.codex-turn-statusbar.$stamp"
} else {
  New-Item -ItemType File -Force -Path $ConfigFile | Out-Null
}

$escaped = $NotifyTarget.Replace("\", "\\").Replace('"', '\"')
$notifyLine = "notify = [`"$escaped`"]"
$content = Get-Content -Raw $ConfigFile

if ($content -match "(?m)^notify = \[[^\r\n]*\]") {
  $content = [regex]::Replace($content, "(?m)^notify = \[[^\r\n]*\]", $notifyLine)
} else {
  $content = "$notifyLine`r`n`r`n$content"
}

Set-Content -NoNewline -Path $ConfigFile -Value $content
Write-Host "Installed notify executable: $NotifyTarget"
Write-Host "Updated Codex config: $ConfigFile"
Write-Host "Restart Codex Desktop or start a new Codex session for notify changes to take effect."
