# Builds (release) and starts Ashen Vigil.
$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Set-Location (Join-Path $PSScriptRoot "..")
cargo run --release
