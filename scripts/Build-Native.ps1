param([switch]$Release)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$nativeRoot = [IO.Path]::GetFullPath($projectRoot)
$targetRoot = Join-Path $nativeRoot 'target'
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$cargoPath = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path -LiteralPath $cargoPath)) { throw 'Instala Rust desde https://rust-lang.org/tools/install y las herramientas C++ de Visual Studio.' }
Push-Location $nativeRoot
try {
    if ($Release) { & $cargoPath build --locked --release --bin psst-gui --target-dir $targetRoot }
    else { & $cargoPath build --locked --bin psst-gui --target-dir $targetRoot }
    if ($LASTEXITCODE -ne 0) { throw 'La compilacion nativa fallo.' }
    $outputRoot = $nativeRoot
    New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
    $buildProfile = if ($Release) { 'release' } else { 'debug' }
    Copy-Item -LiteralPath (Join-Path $targetRoot "$buildProfile\psst-gui.exe") -Destination (Join-Path $outputRoot 'Xpotify.exe') -Force
    $shortcutShell = New-Object -ComObject WScript.Shell
    $shortcut = $shortcutShell.CreateShortcut((Join-Path $projectRoot 'Xpotify.lnk'))
    $shortcut.TargetPath = Join-Path $outputRoot 'Xpotify.exe'
    $shortcut.WorkingDirectory = $projectRoot
    $shortcut.IconLocation = "$outputRoot\Xpotify.exe,0"
    $shortcut.Description = 'Xpotify - Spotify + Splitify'
    $shortcut.Save()
    foreach ($name in @('target', 'dist', 'build')) {
        $cleanupPath = [IO.Path]::GetFullPath((Join-Path $nativeRoot $name))
        if ((Split-Path -Parent $cleanupPath) -ne $nativeRoot) { throw "Ruta insegura: $cleanupPath" }
        if (Test-Path -LiteralPath $cleanupPath) {
            $item = Get-Item -LiteralPath $cleanupPath -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "No se elimina un junction: $cleanupPath" }
            Remove-Item -LiteralPath $cleanupPath -Recurse -Force
        }
    }
    Write-Host "Aplicacion compilada: $outputRoot\Xpotify.exe. Artefactos temporales eliminados."
} finally { Pop-Location }
