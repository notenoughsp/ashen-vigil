# Runs every check at once: fmt and clippy (both builds), then in parallel the unit tests,
# the long ignored soak tests, and the self-tests (alone, host with client, stress), each
# muted, in its own window on the second monitor, with a throwaway save.
# Logs and screenshots go to the given folder; a summary is printed at the end.
# The stress test's frame rates are lower while everything else runs beside it.
param([string]$OutDir = (Join-Path $PSScriptRoot "..\shots\all"))
$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Set-Location (Join-Path $PSScriptRoot "..")
if (Test-Path $OutDir) { Remove-Item -Recurse -Force $OutDir }
New-Item -ItemType Directory -Force $OutDir | Out-Null
$out = (Resolve-Path $OutDir).Path
$start = Get-Date

Add-Type -AssemblyName System.Windows.Forms
$second = [System.Windows.Forms.Screen]::AllScreens | Where-Object { -not $_.Primary } | Select-Object -First 1
function Set-TestWindow([int]$offsetX) {
    if ($second) { $env:ASHEN_WINDOW_POS = "$($second.Bounds.X + 40 + $offsetX),$($second.Bounds.Y + 40)" }
}

# 1. Everything that needs the build lock, one after another.
function Step([string]$name, [scriptblock]$cmd) {
    Write-Host "== $name"
    & $cmd
    if ($LASTEXITCODE -ne 0) { throw "$name failed" }
}
Step "fmt" { cargo fmt --check }
Step "clippy" { cargo clippy --release --all-targets -- -D warnings }
Step "clippy (synth only)" { cargo clippy --release --all-targets --no-default-features -- -D warnings }
Step "build" { cargo build --release }
Step "build tests" { cargo test --release --no-run }

# 2. Everything else at once.
$env:ASHEN_AUTOTEST_AUDIO = "1"
$env:ASHEN_PORT = "7790"
$jobs = @()
function Launch([string]$name, [string]$exe, [string[]]$argv) {
    $log = Join-Path $out "$name.out.txt"
    $p = if ($argv) {
        Start-Process $exe -ArgumentList $argv -PassThru -NoNewWindow -RedirectStandardOutput $log -RedirectStandardError (Join-Path $out "$name.err.txt")
    } else {
        Start-Process $exe -PassThru -NoNewWindow -RedirectStandardOutput $log -RedirectStandardError (Join-Path $out "$name.err.txt")
    }
    $null = $p.Handle # keeps the exit code readable after the process ends
    $script:jobs += [pscustomobject]@{ Name = $name; Proc = $p; Started = Get-Date }
}

Launch "unit" "cargo" @("test", "--release")
Launch "soak" "cargo" @("test", "--release", "--", "--ignored", "--nocapture", "huge_", "chaos_across_modes")

$game = "target\release\ashen-vigil.exe"
foreach ($role in @(@{ Role = ""; X = 0 }, @{ Role = "host"; X = 900 }, @{ Role = "stress"; X = 2700 })) {
    $dir = Join-Path $out ($(if ($role.Role) { $role.Role } else { "single" }))
    New-Item -ItemType Directory -Force $dir | Out-Null
    $env:ASHEN_AUTOTEST = $dir
    $env:ASHEN_AUTOTEST_ROLE = $role.Role
    Set-TestWindow $role.X
    Launch $(if ($role.Role) { $role.Role } else { "single" }) $game $null
    if ($role.Role -eq "host") {
        # The client joins the host a moment later.
        Start-Sleep -Seconds 2
        $env:ASHEN_AUTOTEST = $dir
        $env:ASHEN_AUTOTEST_ROLE = "client"
        Set-TestWindow 1800
        Launch "client" $game $null
    }
}
Remove-Item Env:ASHEN_AUTOTEST, Env:ASHEN_AUTOTEST_ROLE, Env:ASHEN_WINDOW_POS -ErrorAction SilentlyContinue

# 3. Wait for all and sum up.
$failed = 0
foreach ($j in $jobs) {
    $j.Proc.WaitForExit()
    $secs = [int]($j.Proc.ExitTime - $j.Started).TotalSeconds
    $code = $j.Proc.ExitCode
    $logs = Get-ChildItem $out -Recurse -Include "*log.txt", "$($j.Name).out.txt", "$($j.Name).err.txt" -ErrorAction SilentlyContinue
    $panic = $logs | Where-Object { $_.Name -like "$($j.Name)*" -or $_.Directory.Name -eq $j.Name } | Select-String -Pattern "panicked|test result: FAILED" -CaseSensitive -List
    $ok = ($code -eq 0) -and -not $panic
    if (-not $ok) { $failed++ }
    Write-Host ("{0,-8} {1,-5} exit {2}  {3} s" -f $j.Name, $(if ($ok) { "ok" } else { "FAIL" }), $code, $secs)
}
Write-Host ("all done in {0} s, logs and screenshots in {1}" -f [int]((Get-Date) - $start).TotalSeconds, $out)
if ($failed) { exit 1 }
