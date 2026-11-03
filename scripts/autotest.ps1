# Lets the game play itself for ~10 seconds and writes screenshots to the given folder.
# Uses a throwaway save and no sound; the real save file is never touched.
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
Set-TestWindow 0
$env:ASHEN_AUTOTEST = (Resolve-Path $OutDir).Path
& "target\release\ashen-vigil.exe"
Remove-Item Env:ASHEN_AUTOTEST
Write-Host "Screenshots in $OutDir"
