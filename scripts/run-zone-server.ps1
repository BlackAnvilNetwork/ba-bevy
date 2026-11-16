$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
Write-Host "Starting ba_zone_server on ws://127.0.0.1:8787"
Write-Host "Client: set BA_ZONE=1 then cargo run"
cargo run -p ba_zone_server
