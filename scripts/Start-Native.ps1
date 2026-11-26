$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$executable = Join-Path $projectRoot 'Xpotify.exe'
if (-not (Test-Path -LiteralPath $executable)) { & (Join-Path $PSScriptRoot 'Build-Native.ps1') }
Start-Process -FilePath $executable -WorkingDirectory $projectRoot -WindowStyle Normal
