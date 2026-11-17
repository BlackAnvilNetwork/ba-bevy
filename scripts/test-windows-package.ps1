# Validate the MSI payload without installing into system Program Files.
# This catches missing native game binaries, assets, and corrupt packaged files.
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$MsiPath)
$ErrorActionPreference = "Stop"
$msi = (Resolve-Path $MsiPath).Path
$temp = Join-Path $env:RUNNER_TEMP ("ba-msi-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $temp -Force | Out-Null
try {
    $args = "/a `"$msi`" /qn TARGETDIR=`"$temp`""
    $p = Start-Process -FilePath "msiexec.exe" -ArgumentList $args -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw "MSI administrative extraction failed: $($p.ExitCode)" }
    $game = @(Get-ChildItem -Path $temp -Recurse -File -Filter "black_anvil.exe")
    if ($game.Count -ne 1) { throw "Expected one bundled black_anvil.exe; found $($game.Count)" }
    $manifestPath = Join-Path $game[0].Directory.FullName "manifest.json"
    if (-not (Test-Path $manifestPath)) { throw "Bundled game manifest missing" }
    $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.schemaVersion -ne 1 -or $manifest.files.Count -lt 2) { throw "Manifest invalid" }
    foreach ($item in $manifest.files) {
        if ($item.path -match '(\.\.|:|^/|^\\)') { throw "Unsafe manifest entry" }
        $path = Join-Path $game[0].Directory.FullName ($item.path -replace '/', '\')
        if (-not (Test-Path $path -PathType Leaf)) { throw "Bundle missing $($item.path)" }
        $f = Get-Item $path
        if ($f.Length -ne $item.bytes) { throw "Incorrect size: $($item.path)" }
        $digest = (Get-FileHash -Path $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($digest -ne $item.sha256.ToLowerInvariant()) { throw "Incorrect hash: $($item.path)" }
    }
    Write-Host "PASS: Windows installer contains verified Bevy game and $($manifest.files.Count) content files."
} finally {
    Remove-Item -Path $temp -Recurse -Force -ErrorAction SilentlyContinue
}
