# Bygger release-binæren og pakker den i en Inno Setup-installer.
# Kjør fra prosjektroten:  powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1

$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
Set-Location $root

$version = (Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"(.+)"' |
    Select-Object -First 1).Matches.Groups[1].Value
Write-Host "Bygger SyncthingStatus $version" -ForegroundColor Cyan

# MinGW må være i PATH hvis GNU-toolchain brukes (maskiner uten MSVC C++ build tools)
$mingw = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin'
if ((Test-Path $mingw) -and ($env:PATH -notlike "*$mingw*")) {
    $env:PATH = "$mingw;$env:PATH"
}

cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'cargo build feilet' }

$iscc = @(
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
    'C:\Program Files (x86)\Inno Setup 6\ISCC.exe',
    'C:\Program Files\Inno Setup 6\ISCC.exe'
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $iscc) {
    throw 'Fant ikke ISCC.exe. Installer med: winget install JRSoftware.InnoSetup'
}

New-Item -ItemType Directory -Force -Path 'dist' | Out-Null
& $iscc "/DMyAppVersion=$version" 'packaging\windows\syncthing-status.iss'
if ($LASTEXITCODE -ne 0) { throw 'ISCC feilet' }

Write-Host "`nFerdig:" -ForegroundColor Green
Get-ChildItem "dist\syncthing-status-$version-setup.exe" | Format-List Name, Length, FullName
