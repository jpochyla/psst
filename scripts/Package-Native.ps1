param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $SkipBuild) { & (Join-Path $PSScriptRoot 'Build-Native.ps1') }
$outputRoot = [IO.Path]::GetFullPath((Join-Path $projectRoot 'dist'))
$executable = Join-Path $outputRoot 'Xpotify.exe'
if (-not (Test-Path -LiteralPath $executable)) { throw 'Compila Xpotify antes de empaquetarlo.' }
$stageRoot = [IO.Path]::GetFullPath((Join-Path $outputRoot ('package-' + [guid]::NewGuid().ToString('N'))))
if (-not $stageRoot.StartsWith($outputRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'La carpeta temporal debe permanecer dentro de dist.'
}
New-Item -ItemType Directory -Path $stageRoot | Out-Null
try {
    Copy-Item -LiteralPath $executable -Destination (Join-Path $stageRoot 'Xpotify.exe')
    Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE.md') -Destination $stageRoot
    Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE-librespot.md') -Destination $stageRoot
    Copy-Item -LiteralPath (Join-Path $projectRoot 'SPLITIFY.md') -Destination (Join-Path $stageRoot 'README.md')
    Copy-Item -LiteralPath (Join-Path $projectRoot 'ROADMAP.md') -Destination $stageRoot
    # Use an explicit file list: user credentials, .env.local and logs never enter the package.
    Set-Content -LiteralPath (Join-Path $stageRoot '.env.example') -Encoding ascii -Value @(
        'GEMINI_API_KEY=', 'GEMINI_MODEL=gemini-3.5-flash-lite'
    )
    Set-Content -LiteralPath (Join-Path $stageRoot 'Start-Xpotify.cmd') -Encoding ascii -Value @(
        '@echo off', 'cd /d "%~dp0"', 'start "" "%~dp0Xpotify.exe"'
    )
    $archive = Join-Path $outputRoot 'Xpotify-Windows-x64.zip'
    Compress-Archive -LiteralPath (Get-ChildItem -LiteralPath $stageRoot -Force -File).FullName -DestinationPath $archive -Force
    (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash | Set-Content -LiteralPath ($archive + '.sha256') -Encoding ascii
    Write-Host "Paquete: $archive"
} finally {
    # The absolute staging path was checked above and is unique to this invocation.
    Remove-Item -LiteralPath $stageRoot -Recurse -Force
}
