$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
if (-not $env:BA_INTERNAL_TOKEN) { $env:BA_DEV_ALLOW_UNAUTH = "1"; Write-Host "DEV ONLY: enabling unauthenticated loopback zone" }
Write-Host "Starting ba_zone_server on ws://127.0.0.1:8787"
Write-Host "Client: set BA_ZONE=1 then cargo run"
cargo run -p ba_zone_server
