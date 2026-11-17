# Build a self-contained Windows Black Anvil installer.
# The game executable and every required asset are packaged with the Tauri
# launcher. Staging MSI is unsigned; production fails closed without a
# certificate and signs both Bevy EXE and the MSI.
[CmdletBinding()]
param(
    [ValidateSet("staging","production")]
    [string]$Channel = "staging"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$bundleRoot = Join-Path $root "launcher/src-tauri/game"
$gameExe = Join-Path $root "target/release/black_anvil.exe"
$studio = Join-Path $root "character-studio"
$launcher = Join-Path $root "launcher"

function Run-External([string]$Exe,[string[]]$Arguments) {
    & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Exe failed with exit code $LASTEXITCODE"
    }
}

Write-Host "[Black Anvil] Compiling release Bevy..."
Push-Location $root
try { Run-External "cargo" @("build","--release","--bin","black_anvil") }
finally { Pop-Location }

if (-not (Test-Path $gameExe)) {
    throw "Compiled Bevy executable missing: $gameExe"
}
if (-not (Test-Path (Join-Path $root "assets"))) {
    throw "Game assets folder missing"
}

# Never read production signing credentials from source files or build params.
$pfxPath = $null
try {
    if ($Channel -eq "production") {
        if (-not $env:BA_SIGNING_PFX_BASE64 -or -not $env:BA_SIGNING_PFX_PASSWORD) {
            throw "Production requires BA_SIGNING_PFX_BASE64 and BA_SIGNING_PFX_PASSWORD"
        }
        $signtool = (Get-Command signtool.exe -ErrorAction SilentlyContinue)
        if (-not $signtool) { throw "signtool.exe not available for production release" }
        $pfxPath = Join-Path $env:TEMP ("ba-signing-" + [guid]::NewGuid().ToString("N") + ".pfx")
        [IO.File]::WriteAllBytes($pfxPath, [Convert]::FromBase64String($env:BA_SIGNING_PFX_BASE64))
        Run-External "signtool.exe" @("sign","/f",$pfxPath,"/p",$env:BA_SIGNING_PFX_PASSWORD,
            "/fd","SHA256","/tr","http://timestamp.digicert.com","/td","SHA256",$gameExe)
    }

    # Recreate staging directory from scratch. Never include editor caches,
    # node_modules, old binaries or arbitrary user-generated character files.
    if (Test-Path $bundleRoot) { Remove-Item -Recurse -Force $bundleRoot }
    New-Item -ItemType Directory -Path $bundleRoot -Force | Out-Null
    Copy-Item $gameExe (Join-Path $bundleRoot "black_anvil.exe")
    Copy-Item -Path (Join-Path $root "assets") -Destination (Join-Path $bundleRoot "assets") -Recurse -Force

    $manifestFiles = @(
        Get-ChildItem -Path $bundleRoot -Recurse -File |
        Where-Object { $_.Name -ne "manifest.json" } |
        ForEach-Object {
            $rel = $_.FullName.Substring($bundleRoot.Length).TrimStart('\','/') -replace '\\','/'
            [ordered]@{
                path = $rel
                bytes = $_.Length
                sha256 = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            }
        } | Sort-Object { $_.path }
    )
    if ($manifestFiles.Count -lt 2) { throw "Game bundle has no assets" }
    $manifest = [ordered]@{
        schemaVersion = 1
        gameVersion = "0.1.0"
        channel = $Channel
        files = $manifestFiles
    }
    $manifest | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $bundleRoot "manifest.json") -Encoding UTF8
    Write-Host "[Black Anvil] Packaged $($manifestFiles.Count) files."

    # Builds the already-integrated Studio into Tauri's embedded webview.
    Push-Location $studio
    try { Run-External "npm" @("ci","--legacy-peer-deps") }
    finally { Pop-Location }
    Push-Location $launcher
    try {
        Run-External "npm" @("ci")
        Run-External "npm" @("run","studio:build")
        Run-External "npm" @("run","tauri","build","--","--config","src-tauri/tauri.release.conf.json")
    } finally { Pop-Location }

    $output = Join-Path $root "launcher/src-tauri/target/release/bundle/msi"
    $installers = @(Get-ChildItem -Path $output -Filter "*.msi" -File -ErrorAction SilentlyContinue)
    if ($installers.Count -eq 0) { throw "Tauri did not produce a Windows MSI" }

    if ($Channel -eq "production") {
        foreach ($installer in $installers) {
            Run-External "signtool.exe" @("sign","/f",$pfxPath,"/p",$env:BA_SIGNING_PFX_PASSWORD,
                "/fd","SHA256","/tr","http://timestamp.digicert.com","/td","SHA256",$installer.FullName)
        }
    }

    Write-Host "[Black Anvil] MSI ready:"
    $installers | ForEach-Object { Write-Host $_.FullName }
}
finally {
    if ($pfxPath -and (Test-Path $pfxPath)) {
        Remove-Item $pfxPath -Force
    }
}
