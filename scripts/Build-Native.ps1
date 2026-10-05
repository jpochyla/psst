param([switch]$Release)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$nativeRoot = $projectRoot
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$cargoPath = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path -LiteralPath $cargoPath)) { throw 'Instala Rust desde https://rust-lang.org/tools/install y las herramientas C++ de Visual Studio.' }
Push-Location $nativeRoot
try {
    if ($Release) { & $cargoPath build --locked --release --bin psst-gui }
    else { & $cargoPath build --locked --bin psst-gui }
    if ($LASTEXITCODE -ne 0) { throw 'La compilacion nativa fallo.' }
    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $nativeRoot 'target' }
    $outputRoot = Join-Path $projectRoot 'dist'
    New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
    $profile = if ($Release) { 'release' } else { 'debug' }
    Copy-Item -LiteralPath (Join-Path $targetRoot "$profile\psst-gui.exe") -Destination (Join-Path $outputRoot 'Xpotify.exe') -Force
    $shortcutShell = New-Object -ComObject WScript.Shell
    $shortcut = $shortcutShell.CreateShortcut((Join-Path $projectRoot 'Xpotify.lnk'))
    $shortcut.TargetPath = Join-Path $outputRoot 'Xpotify.exe'
    $shortcut.WorkingDirectory = $projectRoot
    $shortcut.IconLocation = "$outputRoot\Xpotify.exe,0"
    $shortcut.Description = 'Xpotify - Spotify + Splitify'
    $shortcut.Save()
    Write-Host "Aplicacion compilada: $outputRoot\Xpotify.exe"
} finally { Pop-Location }
