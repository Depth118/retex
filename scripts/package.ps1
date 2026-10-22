# Build every component and bundle a release zip in dist/.
#
#   powershell -ExecutionPolicy Bypass -File scripts\package.ps1
#
# Needs stable Rust with the i686-pc-windows-msvc target and the MSVC build
# tools. The app and CLI are 64-bit; the hook and injector must be 32-bit to
# load into 32-bit D3D9 games.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Invoke-Cargo {
    & cargo @args
    if ($LASTEXITCODE -ne 0) { throw "cargo $args failed" }
}

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$name = "retex-$version-windows"
$stage = Join-Path $root "dist\$name"

Invoke-Cargo build --release -p retex-app -p retex-cli
Invoke-Cargo build --release -p retex-hook -p retex-inject --target i686-pc-windows-msvc

if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force $stage | Out-Null

$files = @(
    'target\release\retex-app.exe',
    'target\release\retex.exe',
    'target\i686-pc-windows-msvc\release\retex_hook.dll',
    'target\i686-pc-windows-msvc\release\retex-inject.exe',
    'README.md',
    'LICENSE-MIT',
    'LICENSE-APACHE',
    'THIRD-PARTY-LICENSES.txt'
)
foreach ($file in $files) { Copy-Item $file $stage }

$zip = Join-Path $root "dist\$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path "$stage\*" -DestinationPath $zip
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
# LF line ending so `sha256sum -c` works on the file.
[IO.File]::WriteAllText("$zip.sha256", "$hash  $name.zip`n")

Write-Host "created dist\$name.zip"
Write-Host "sha256 $hash"
