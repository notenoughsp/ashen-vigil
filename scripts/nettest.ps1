# Starts a host and a client on this machine that play together for ~20 seconds
# and write host_*.png / client_*.png screenshots. The real save file is never touched.
param([string]$OutDir = (Join-Path $PSScriptRoot "..\shots"))
$ErrorActionPreference = "Stop"
# Open test windows on a second monitor when there is one, so they stay out of the way.
Add-Type -AssemblyName System.Windows.Forms
$second = [System.Windows.Forms.Screen]::AllScreens | Where-Object { -not $_.Primary } | Select-Object -First 1
function Set-TestWindow([int]$offsetX) {
    if ($second) { $env:ASHEN_WINDOW_POS = "$($second.Bounds.X + 60 + $offsetX),$($second.Bounds.Y + 60)" }
}
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Set-Location (Join-Path $PSScriptRoot "..")
New-Item -ItemType Directory -Force $OutDir | Out-Null
cargo build --release
$out = (Resolve-Path $OutDir).Path
$env:ASHEN_AUTOTEST = $out
$env:ASHEN_AUTOTEST_ROLE = "host"
Set-TestWindow 0
$hostProc = Start-Process "target\release\ashen-vigil.exe" -PassThru -RedirectStandardOutput (Join-Path $out "host.log")
$env:ASHEN_AUTOTEST_ROLE = "client"
Set-TestWindow 1360
& "target\release\ashen-vigil.exe" | Tee-Object (Join-Path $out "client.log")
$hostProc.WaitForExit()
Remove-Item Env:ASHEN_AUTOTEST, Env:ASHEN_AUTOTEST_ROLE
Write-Host "Screenshots and logs in $out"
