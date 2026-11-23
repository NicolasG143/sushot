#requires -Version 5.1
<#
.SYNOPSIS
  Renders the MSIX logo assets: a rounded square with viewfinder corners.
#>
Add-Type -AssemblyName System.Drawing

$output = Join-Path $PSScriptRoot 'msix\Assets'
New-Item -ItemType Directory -Force $output | Out-Null

function New-Logo([int] $width, [int] $height, [string] $name) {
    $bitmap = New-Object System.Drawing.Bitmap $width, $height
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.Clear([System.Drawing.Color]::Transparent)

    $side = [Math]::Min($width, $height)
    $left = ($width - $side) / 2
    $top = ($height - $side) / 2
    $radius = $side / 5

    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc($left, $top, 2 * $radius, 2 * $radius, 180, 90)
    $path.AddArc($left + $side - 2 * $radius, $top, 2 * $radius, 2 * $radius, 270, 90)
    $path.AddArc($left + $side - 2 * $radius, $top + $side - 2 * $radius, 2 * $radius, 2 * $radius, 0, 90)
    $path.AddArc($left, $top + $side - 2 * $radius, 2 * $radius, 2 * $radius, 90, 90)
    $path.CloseFigure()
    $graphics.FillPath((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(0x2D, 0x7F, 0xF9))), $path)

    $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::White), ([Math]::Max(1, $side / 10))
    $inset = $side / 4
    $reach = $side / 6
    $near = $left + $inset
    $far = $left + $side - $inset
    $nearY = $top + $inset
    $farY = $top + $side - $inset
    foreach ($corner in @(
        @($near, $nearY, 1, 1), @($far, $nearY, -1, 1),
        @($near, $farY, 1, -1), @($far, $farY, -1, -1))) {
        $graphics.DrawLine($pen, $corner[0], $corner[1], $corner[0] + $corner[2] * $reach, $corner[1])
        $graphics.DrawLine($pen, $corner[0], $corner[1], $corner[0], $corner[1] + $corner[3] * $reach)
    }

    $bitmap.Save((Join-Path $output $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
}

New-Logo 50 50 'StoreLogo.png'
New-Logo 44 44 'Square44x44Logo.png'
New-Logo 150 150 'Square150x150Logo.png'
