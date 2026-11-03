# Builds a release and puts the game into dist/AshenVigil.exe: one file, nothing else needed.
# Font, sounds and license texts are embedded; the save goes to %APPDATA%\AshenVigil.
$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$root = Join-Path $PSScriptRoot ".."
Set-Location $root
cargo build --release
$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force $dist | Out-Null
# Older builds shipped a folder and a zip; they are replaced by the single exe.
foreach ($old in @((Join-Path $dist "AshenVigil"), (Join-Path $dist "AshenVigil.zip"))) {
    if (Test-Path $old) { Remove-Item $old -Recurse -Force }
}
$exe = Join-Path $dist "AshenVigil.exe"
Copy-Item "target\release\ashen-vigil.exe" $exe -Force
Write-Host "Packaged to $exe"
