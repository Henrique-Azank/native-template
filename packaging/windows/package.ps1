<#
.SYNOPSIS
  Build, sign and package TemplateApp for Windows.

.DESCRIPTION
  Produces, in $env:DIST (default: dist/release):
    template-app-<version>-windows-<arch>.msi            per-machine installer (WiX v5)
    template-app-<version>-windows-<arch>-portable.zip   template-desktop.exe + template-cli.exe

  The binaries link the C runtime statically (+crt-static), so neither the MSI nor the portable
  zip needs the Visual C++ redistributable. Signing is delegated to sign.ps1 (skipped with a
  warning when no signing secrets are set).

  Needs: Rust (MSVC toolchain + the target), the Windows SDK (rc.exe, signtool.exe),
  and WiX v5: dotnet tool install --global wix --version 5.0.2

.EXAMPLE
  pwsh packaging/windows/package.ps1 -Arch x64
  pwsh packaging/windows/package.ps1 -Arch x86 -SkipBuild
  pwsh packaging/windows/package.ps1 -Arch arm64
#>
param(
  [ValidateSet('x64', 'x86', 'arm64')] [string] $Arch = 'x64',
  [switch] $SkipBuild
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

$Version = $env:TEMPLATE_VERSION
if (-not $Version) {
  $inPkg = $false
  foreach ($line in Get-Content (Join-Path $Root 'Cargo.toml')) {
    if ($line -match '^\s*\[') { $inPkg = ($line.Trim() -eq '[workspace.package]'); continue }
    if ($inPkg -and $line -match '^\s*version\s*=\s*"([^"]+)"') { $Version = $Matches[1]; break }
  }
}
if (-not $Version) { throw 'could not read [workspace.package] version from Cargo.toml' }
$MsiVersion = ($Version -split '-')[0]

$Target = switch ($Arch) {
  'x64'   { 'x86_64-pc-windows-msvc' }
  'x86'   { 'i686-pc-windows-msvc' }
  'arm64' { 'aarch64-pc-windows-msvc' }
}
$Dist = if ($env:DIST) { $env:DIST } else { Join-Path $Root 'dist\release' }
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
New-Item -ItemType Directory -Force -Path $Dist | Out-Null

if (-not $env:TEMPLATE_BUILD_SHA) { $env:TEMPLATE_BUILD_SHA = (git -C $Root rev-parse HEAD 2>$null) }
if (-not $env:TEMPLATE_BUILD_DATE) { $env:TEMPLATE_BUILD_DATE = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd') }

Write-Output "TemplateApp $Version for Windows $Arch ($Target)"

if (-not $SkipBuild) {
  $flagVar = 'CARGO_TARGET_' + ($Target.ToUpper() -replace '-', '_') + '_RUSTFLAGS'
  [Environment]::SetEnvironmentVariable($flagVar, '-C target-feature=+crt-static')
  $env:TEMPLATE_REQUIRE_WINRES = '1'
  Invoke-Native "cargo build ($Target)" {
    cargo build --release --locked -p template-desktop -p template-cli --target $Target
  }
}

$Bin = Join-Path $TargetDir "$Target\release"

# Check PE headers: Machine (COFF header) must match -Arch; Subsystem: 2 = Windows GUI, 3 = Console.
function Get-PeHeader([string] $Path) {
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  $pe = [BitConverter]::ToInt32($bytes, 0x3C)
  return @{
    Machine   = [BitConverter]::ToUInt16($bytes, $pe + 4)
    Subsystem = [BitConverter]::ToUInt16($bytes, $pe + 0x5C)
  }
}
$Machine = switch ($Arch) { 'x64' { 0x8664 } 'x86' { 0x14C } 'arm64' { 0xAA64 } }
foreach ($check in @(@('template-desktop.exe', 2), @('template-cli.exe', 3))) {
  $h = Get-PeHeader (Join-Path $Bin $check[0])
  if ($h.Machine -ne $Machine) {
    throw "$($check[0]) is for machine 0x$('{0:X}' -f $h.Machine), expected 0x$('{0:X}' -f $Machine) ($Arch)"
  }
  if ($h.Subsystem -ne $check[1]) {
    throw "$($check[0]) has PE subsystem $($h.Subsystem), expected $($check[1])"
  }
  Write-Output "ok $($check[0]): $Arch, PE subsystem $($h.Subsystem)"
}

$Stage = Join-Path $TargetDir "windows-package\$Arch"
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item (Join-Path $Bin 'template-desktop.exe'), (Join-Path $Bin 'template-cli.exe') $Stage

& (Join-Path $PSScriptRoot 'sign.ps1') (Join-Path $Stage 'template-desktop.exe') (Join-Path $Stage 'template-cli.exe')

# ---- MSI ---------------------------------------------------------------------------------------
$Msi = Join-Path $Dist "template-app-$Version-windows-$Arch.msi"
$IconPath = Join-Path $Root 'assets\app-icon\template-app.ico'
Invoke-Native 'wix build' {
  wix build (Join-Path $PSScriptRoot 'template-app.wxs') -arch $Arch `
    -d "Version=$MsiVersion" -d "BinDir=$Stage" -d "IconPath=$IconPath" `
    -o $Msi
}
Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($Msi, '.wixpdb'))
& (Join-Path $PSScriptRoot 'sign.ps1') $Msi

# ---- portable zip ------------------------------------------------------------------------------
$Portable = Join-Path $TargetDir "windows-package\template-app-$Version-windows-$Arch-portable"
Remove-Item -Recurse -Force $Portable -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Portable | Out-Null
Copy-Item (Join-Path $Stage '*.exe') $Portable
foreach ($f in 'README.md', 'LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE') {
  $p = Join-Path $Root $f
  if (Test-Path $p) { Copy-Item $p $Portable }
}
$Zip = Join-Path $Dist "template-app-$Version-windows-$Arch-portable.zip"
Remove-Item -Force $Zip -ErrorAction SilentlyContinue
Compress-Archive -Path $Portable -DestinationPath $Zip

Write-Output "==> done"
Get-Item $Msi, $Zip | Select-Object Name, Length
