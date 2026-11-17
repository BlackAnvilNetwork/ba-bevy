# Install the produced staging MSI on an isolated Windows runner, verify
# the actual installed game/resources and uninstall. This is stronger than
# administrative MSI extraction but is not a visual/manual GUI acceptance test.
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$MsiPath)
$ErrorActionPreference = "Stop"
$msi = (Resolve-Path $MsiPath).Path
$base = Join-Path $env:RUNNER_TEMP ("ba-install-" + [guid]::NewGuid().ToString("N"))
$install = Join-Path $base "installed"
$log = Join-Path $base "install.log"
New-Item -ItemType Directory -Path $base -Force | Out-Null

function Run-Msi([string]$Action, [string]$Additional) {
    $arguments = "/$Action `"$msi`" /qn /norestart $Additional /L*v `"$log`""
    $process = Start-Process -FilePath "msiexec.exe" -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0 -and $process.ExitCode -ne 3010) {
        if (Test-Path $log) {
            Get-Content $log -Tail 60 | Write-Host
        }
        throw "MSI $Action failed with $($process.ExitCode)"
    }
}
try {
    Run-Msi "i" ("INSTALLDIR=`"$install`"")
    $candidates = @($install, (Join-Path $env:ProgramFiles "Black Anvil Launcher"))
    $game = @()
    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            $game += @(Get-ChildItem $candidate -Recurse -File -Filter "black_anvil.exe" -ErrorAction SilentlyContinue)
        }
    }
    $game = @($game | Sort-Object FullName -Unique)
    if ($game.Count -ne 1) {
        throw "Clean MSI installation did not contain exactly one game executable"
    }
    $dir = $game[0].Directory.FullName
    $manifest = Join-Path $dir "manifest.json"
    if (-not (Test-Path $manifest)) {
        throw "Installed game content manifest missing"
    }
    $data = Get-Content $manifest -Raw | ConvertFrom-Json
    if ($data.schemaVersion -ne 1 -or $data.files.Count -lt 2) {
        throw "Installed manifest is invalid"
    }
    foreach ($item in $data.files) {
        $rel = $item.path -replace '/', '\\'
        if ($rel.Contains("..") -or $rel.Contains(":") -or $rel.StartsWith("\\")) {
            throw "Unsafe installed manifest entry"
        }
        $path = Join-Path $dir $rel
        if (-not (Test-Path $path -PathType Leaf)) {
            throw "Installed game missing $rel"
        }
        if ((Get-Item $path).Length -ne $item.bytes) {
            throw "Installed file size mismatch: $rel"
        }
        $hash = (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($hash -ne $item.sha256.ToLowerInvariant()) {
            throw "Installed file SHA256 mismatch: $rel"
        }
    }
    Write-Host "PASS clean MSI install: Bevy executable and $($data.files.Count) authenticated content files present"
}
finally {
    try { Run-Msi "x" "" }
    catch { Write-Warning "MSI uninstall failed: $_" }
    Remove-Item -Path $base -Recurse -Force -ErrorAction SilentlyContinue
}
