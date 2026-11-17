# E2E: isolated gateway + authenticated zone (two synthetic Discord accounts).
# Never runs against production state; no credentials are committed.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$testDir = Join-Path $env:RUNNER_TEMP ("ba-auth-smoke-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $testDir -Force | Out-Null
$env:BA_GATEWAY_DB = Join-Path $testDir "accounts.sqlite"
$env:BA_AVATAR_DIR = Join-Path $testDir "avatars"
$env:BA_GATEWAY_BIND = "127.0.0.1:8790"
$env:BA_GATEWAY_PUBLIC_URL = "http://127.0.0.1:8790"
$env:BA_GATEWAY_INTERNAL_URL = "http://127.0.0.1:8790"
$env:BA_ZONE_BIND = "127.0.0.1:8787"
Remove-Item Env:BA_DEV_ALLOW_UNAUTH -ErrorAction SilentlyContinue

$rng = [byte[]]::new(48)
[System.Security.Cryptography.RandomNumberGenerator]::Fill($rng)
$env:BA_INTERNAL_TOKEN = [Convert]::ToBase64String($rng)
$secrets = Join-Path $testDir "sessions.json"
$gatewayProcess = $null
$zoneProcess = $null

function Wait-Port([int]$Port, [string]$Label) {
    for ($i = 0; $i -lt 90; $i++) {
        try {
            $connection = New-Object System.Net.Sockets.TcpClient
            $connection.Connect("127.0.0.1", $Port)
            $connection.Close()
            return
        } catch {
            Start-Sleep -Milliseconds 500
        }
    }
    throw "Timed out waiting for $Label on localhost:$Port"
}

try {
    Push-Location $root
    try {
        $gatewayProcess = Start-Process -FilePath (Join-Path $root "target/debug/ba_gateway.exe") -NoNewWindow -PassThru
        Wait-Port 8790 "Discord identity gateway"
        python scripts/seed-auth-smoke.py $env:BA_GATEWAY_DB $secrets
        if ($LASTEXITCODE -ne 0) { throw "Failed to seed isolated authenticated accounts" }
        $zoneProcess = Start-Process -FilePath (Join-Path $root "target/debug/ba_zone_server.exe") -NoNewWindow -PassThru
        Wait-Port 8787 "authenticated game zone"
        $env:BA_SMOKE_SESSION_FILE = $secrets
        node scripts/smoke-authenticated-zone.mjs
        if ($LASTEXITCODE -ne 0) { throw "Authenticated MMO smoke failed" }
    } finally {
        Pop-Location
    }
} finally {
    if ($zoneProcess) { Stop-Process -Id $zoneProcess.Id -Force -ErrorAction SilentlyContinue }
    if ($gatewayProcess) { Stop-Process -Id $gatewayProcess.Id -Force -ErrorAction SilentlyContinue }
    Remove-Item Env:BA_SMOKE_SESSION_FILE -ErrorAction SilentlyContinue
    Remove-Item Env:BA_INTERNAL_TOKEN -ErrorAction SilentlyContinue
    Remove-Item -Path $testDir -Force -Recurse -ErrorAction SilentlyContinue
}
