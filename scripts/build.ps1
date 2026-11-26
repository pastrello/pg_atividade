param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
function Invoke-Checked([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Command falhou (exit $LASTEXITCODE)" }
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Instale Rust stable x64 MSVC (rustup) e Visual Studio Build Tools com Desktop development with C++.'
}
Invoke-Checked rustc @('--version')
if (-not (Test-Path Cargo.lock)) { Invoke-Checked cargo @('generate-lockfile') }
if (-not $SkipTests) { Invoke-Checked cargo @('test','--locked','--target','x86_64-pc-windows-msvc') }
Invoke-Checked cargo @('build','--release','--locked','--target','x86_64-pc-windows-msvc')
New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item 'target/x86_64-pc-windows-msvc/release/pgactivity.exe' 'dist/pgactivity.exe' -Force
$hash = (Get-FileHash 'dist/pgactivity.exe' -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  pgactivity.exe" | Set-Content -Encoding ascii 'dist/pgactivity.exe.sha256'
Copy-Item README.md,CHANGELOG.md -Destination dist -Force
Write-Host 'Executável: dist\pgactivity.exe'
