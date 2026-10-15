$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$assetRoot = Join-Path (Split-Path -Parent $PSScriptRoot) 'psst-gui\assets'

function New-RoundedRectangle([float]$x, [float]$y, [float]$width, [float]$height, [float]$radius) {
    $path = New-Object Drawing.Drawing2D.GraphicsPath
    $diameter = $radius * 2
    $path.AddArc($x, $y, $diameter, $diameter, 180, 90)
    $path.AddArc($x + $width - $diameter, $y, $diameter, $diameter, 270, 90)
    $path.AddArc($x + $width - $diameter, $y + $height - $diameter, $diameter, $diameter, 0, 90)
    $path.AddArc($x, $y + $height - $diameter, $diameter, $diameter, 90, 90)
    $path.CloseFigure()
    return ,$path
}

$master = New-Object Drawing.Bitmap 512, 512
$canvas = [Drawing.Graphics]::FromImage($master)
$canvas.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
$canvas.Clear([Drawing.Color]::Transparent)
$green = New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb(29, 185, 84))
$white = New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb(245, 255, 249))
$tile = New-RoundedRectangle 32 32 448 448 112
$canvas.FillPath($green, $tile)
$tile.Dispose()
foreach ($bar in @(@(144, 192, 48, 128), @(232, 144, 48, 224), @(320, 192, 48, 128))) {
    $shape = New-RoundedRectangle $bar[0] $bar[1] $bar[2] $bar[3] 24
    $canvas.FillPath($white, $shape)
    $shape.Dispose()
}
$canvas.Dispose()
$green.Dispose()
$white.Dispose()
try {
    foreach ($size in @(16, 32, 64, 128, 256, 512)) {
        $bitmap = New-Object Drawing.Bitmap $size, $size
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.DrawImage($master, 0, 0, $size, $size)
        $bitmap.Save((Join-Path $assetRoot "logo_$size.png"), [Drawing.Imaging.ImageFormat]::Png)
        $graphics.Dispose()
        $bitmap.Dispose()
    }
} finally { $master.Dispose() }
