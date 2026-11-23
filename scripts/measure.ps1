#requires -Version 5.1
<#
.SYNOPSIS
  Measures the size, cold start, idle memory and idle CPU of a sushot release build.

.DESCRIPTION
  Starts the executable once, waits for its "ready" log line, lets it idle and reports
  the numbers as Markdown rows. A default configuration file is created first, so the
  first-run dialog does not distort the memory numbers, and the original state is
  restored afterwards.

.PARAMETER Exe
  Path to sushot.exe. Defaults to the release build of this repository.

.PARAMETER IdleSeconds
  Seconds over which idle CPU time is sampled.
#>
[CmdletBinding()]
param(
    [string] $Exe,
    [int] $IdleSeconds = 20
)

$ErrorActionPreference = 'Stop'

if (-not $Exe) {
    $Exe = Join-Path $PSScriptRoot '..\target\release\sushot.exe'
}
$Exe = (Resolve-Path $Exe).Path
$config = Join-Path $env:APPDATA 'sushot\config.toml'
$backup = if (Test-Path $config) { Get-Content $config -Raw } else { $null }
$log = Join-Path ([IO.Path]::GetTempPath()) "sushot-measure-$PID.log"
$process = $null

try {
    if ($null -eq $backup) {
        New-Item -ItemType Directory -Force (Split-Path $config) | Out-Null
        Set-Content -Path $config -Value "autostart = false`nshow_toast = false"
    }

    $clock = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $Exe -PassThru -RedirectStandardError $log
    while (-not ((Test-Path $log) -and (Select-String -Path $log -Pattern 'ready' -Quiet))) {
        if ($process.HasExited) { throw "sushot exited early with code $($process.ExitCode)." }
        if ($clock.Elapsed.TotalSeconds -gt 10) { throw 'sushot never reported ready.' }
        Start-Sleep -Milliseconds 5
    }
    $startup = $clock.Elapsed.TotalMilliseconds

    Start-Sleep -Seconds 5
    $process.Refresh()
    $cpuBefore = $process.TotalProcessorTime.TotalSeconds
    Start-Sleep -Seconds $IdleSeconds
    $process.Refresh()
    $cpuAfter = $process.TotalProcessorTime.TotalSeconds
    $cores = [Environment]::ProcessorCount

    $sizeKb = [math]::Round((Get-Item $Exe).Length / 1KB)
    $workingSetMb = [math]::Round($process.WorkingSet64 / 1MB, 1)
    $privateMb = [math]::Round($process.PrivateMemorySize64 / 1MB, 1)
    $cpuPercent = [math]::Round(100 * ($cpuAfter - $cpuBefore) / $IdleSeconds / $cores, 3)

    Write-Output '| Metric | Value |'
    Write-Output '|---|---|'
    Write-Output "| Binary size | $sizeKb KB |"
    Write-Output "| Cold start until ready | $([math]::Round($startup)) ms |"
    Write-Output "| Idle working set | $workingSetMb MB |"
    Write-Output "| Idle private bytes | $privateMb MB |"
    Write-Output "| Idle CPU over $IdleSeconds s ($cores logical cores) | $cpuPercent % |"
} finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit()
    }
    if (Test-Path $log) { Remove-Item $log -Force }
    if ($null -ne $backup) {
        Set-Content -Path $config -Value $backup -NoNewline
    } elseif (Test-Path $config) {
        Remove-Item $config -Force
    }
}
