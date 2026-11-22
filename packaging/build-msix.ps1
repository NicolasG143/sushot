#requires -Version 5.1
<#
.SYNOPSIS
  Builds the klip release binary and packs it into an MSIX package.

.PARAMETER CertificatePath
  Code signing certificate (.pfx). Without it the package is left unsigned, which
  Windows refuses to install but the Microsoft Store accepts for upload.

.PARAMETER CertificatePassword
  Password of the .pfx file.

.PARAMETER Version
  Four-part package version. Defaults to the version in the manifest.
#>
[CmdletBinding()]
param(
    [string] $CertificatePath,
    [string] $CertificatePassword,
    [string] $Version
)

$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$stage = Join-Path $root 'target\msix\stage'
$package = Join-Path $root 'target\msix\klip.msix'

function Find-SdkTool([string] $name) {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $tool = Get-ChildItem -Path $kits -Recurse -Filter $name -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $tool) { throw "$name was not found; install the Windows 10/11 SDK." }
    return $tool.FullName
}

Push-Location $root
try {
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed.' }
} finally {
    Pop-Location
}

if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force (Join-Path $stage 'Assets') | Out-Null
Copy-Item (Join-Path $root 'target\release\klip.exe') $stage
Copy-Item (Join-Path $root 'packaging\msix\Assets\*.png') (Join-Path $stage 'Assets')

$manifest = Get-Content (Join-Path $root 'packaging\msix\AppxManifest.xml') -Raw
if ($Version) {
    $manifest = $manifest -replace '(<Identity[^>]*Version=")[^"]*(")', "`$1$Version`$2"
}
Set-Content -Path (Join-Path $stage 'AppxManifest.xml') -Value $manifest -Encoding UTF8

$makeappx = Find-SdkTool 'makeappx.exe'
& $makeappx pack /d $stage /p $package /o
if ($LASTEXITCODE -ne 0) { throw 'makeappx failed.' }

if ($CertificatePath) {
    $signtool = Find-SdkTool 'signtool.exe'
    & $signtool sign /fd SHA256 /f $CertificatePath /p $CertificatePassword $package
    if ($LASTEXITCODE -ne 0) { throw 'signtool failed.' }
    Write-Host "Signed package: $package"
} else {
    Write-Host "Unsigned package: $package"
}
