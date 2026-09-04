# Builds the release binary and wraps it in an Inno Setup installer.
# Run from the project root:  powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1

$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
Set-Location $root

$version = (Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"(.+)"' |
    Select-Object -First 1).Matches.Groups[1].Value
Write-Host "Building SyncthingStatus $version" -ForegroundColor Cyan

# MinGW must be on PATH when the GNU toolchain is used (machines without the MSVC C++ build tools)
$mingw = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin'
if ((Test-Path $mingw) -and ($env:PATH -notlike "*$mingw*")) {
    $env:PATH = "$mingw;$env:PATH"
}

cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$iscc = @(
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
    'C:\Program Files (x86)\Inno Setup 6\ISCC.exe',
    'C:\Program Files\Inno Setup 6\ISCC.exe'
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $iscc) {
    throw 'ISCC.exe not found. Install it with: winget install JRSoftware.InnoSetup'
}

New-Item -ItemType Directory -Force -Path 'dist' | Out-Null
& $iscc "/DMyAppVersion=$version" 'packaging\windows\syncthing-status.iss'
if ($LASTEXITCODE -ne 0) { throw 'ISCC failed' }

Write-Host "`nDone:" -ForegroundColor Green
Get-ChildItem "dist\syncthing-status-$version-setup.exe" | Format-List Name, Length, FullName
