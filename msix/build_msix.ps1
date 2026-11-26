# Stock TW — Microsoft Store MSIX 打包腳本
# 用法範例：
#   powershell -ExecutionPolicy Bypass -File msix\build_msix.ps1 `
#     -IdentityName "12345tgsastudio.StockTW" `
#     -Publisher "CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX" `
#     -PublisherDisplayName "tgsastudio" `
#     -Version "1.2.1"

param(
    [Parameter(Mandatory = $true)]
    [string]$IdentityName,

    [Parameter(Mandatory = $true)]
    [string]$Publisher,

    [string]$PublisherDisplayName = "tgsastudio",

    [string]$Version = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($Version)) {
    $cargoToml = Get-Content (Join-Path $repoRoot "Cargo.toml") -Raw
    if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        throw "Unable to read version from Cargo.toml"
    }
}

$exePath = Join-Path $repoRoot "target\release\simple_stock.exe"
if (-not (Test-Path $exePath)) {
    throw "Release binary not found at $exePath. Run 'cargo build --release' first."
}

$makeappx = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending |
    Select-Object -First 1 -ExpandProperty FullName

if (-not $makeappx) {
    throw "makeappx.exe not found. Install Windows 10/11 SDK or run the GitHub Actions 'Build Microsoft Store MSIX' workflow."
}

$stageDir = Join-Path $repoRoot "target\msix_stage"
if (Test-Path $stageDir) { Remove-Item $stageDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "Assets") | Out-Null

Copy-Item $exePath (Join-Path $stageDir "simple_stock.exe") -Force
Copy-Item (Join-Path $PSScriptRoot "Assets\StoreLogo.png") (Join-Path $stageDir "Assets\") -Force
Copy-Item (Join-Path $PSScriptRoot "Assets\Square44x44Logo.png") (Join-Path $stageDir "Assets\") -Force
Copy-Item (Join-Path $PSScriptRoot "Assets\Square150x150Logo.png") (Join-Path $stageDir "Assets\") -Force
Copy-Item (Join-Path $PSScriptRoot "Assets\Wide310x150Logo.png") (Join-Path $stageDir "Assets\") -Force

$manifest = Get-Content (Join-Path $PSScriptRoot "AppxManifest.xml") -Raw -Encoding UTF8
$manifest = $manifest.Replace("__PACKAGE_IDENTITY_NAME__", $IdentityName)
$manifest = $manifest.Replace("__PUBLISHER_CN__", $Publisher)
$manifest = $manifest.Replace("__PUBLISHER_DISPLAY_NAME__", $PublisherDisplayName)
$manifest = $manifest.Replace("__APP_VERSION__", $Version)
[System.IO.File]::WriteAllText((Join-Path $stageDir "AppxManifest.xml"), $manifest, (New-Object System.Text.UTF8Encoding($false)))

$distDir = Join-Path $repoRoot "dist"
New-Item -ItemType Directory -Force -Path $distDir | Out-Null
$outMsix = Join-Path $distDir "StockTW-x64-v$Version.msix"

& $makeappx pack /d $stageDir /p $outMsix /o
if ($LASTEXITCODE -ne 0) {
    throw "makeappx failed with exit code $LASTEXITCODE"
}

Write-Host "Successfully built unsigned MSIX for Microsoft Store upload:"
Write-Host "  $outMsix"
