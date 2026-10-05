$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$nativeRoot = $projectRoot
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$cargoPath = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path -LiteralPath $cargoPath)) { throw 'Instala Rust desde https://rust-lang.org/tools/install y las herramientas C++ de Visual Studio.' }
Push-Location $nativeRoot
try {
    & $cargoPath build --locked --bin psst-gui
    if ($LASTEXITCODE -ne 0) { throw 'La compilacion nativa fallo.' }
    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $nativeRoot 'target' }
    $outputRoot = Join-Path $projectRoot 'dist'
    New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
    Copy-Item -LiteralPath (Join-Path $targetRoot 'debug\psst-gui.exe') -Destination (Join-Path $outputRoot 'Xpotify.exe') -Force
    Write-Host "Aplicacion compilada: $outputRoot\Xpotify.exe"
} finally { Pop-Location }
