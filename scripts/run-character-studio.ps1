$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$studio = Join-Path $root "character-studio"

if (-not (Test-Path "$studio\package.json")) {
  Write-Host "Character Studio missing. Run scripts\install-character-studio.ps1 then scripts\setup-ba-character-studio.ps1"
  exit 1
}

Set-Location $studio
if (-not (Test-Path "node_modules")) {
  Write-Host "Installing npm deps..."
  npm install --legacy-peer-deps
}

New-Item -ItemType Directory -Force -Path (Join-Path $root "assets\character\exports") | Out-Null

Write-Host "Black Anvil Character Studio — Anata male/female."
Write-Host "Save page: Save for Black Anvil → assets/character/exports/active.{json,glb}"
Write-Host "Open http://127.0.0.1:5173"
npx vite --host 127.0.0.1 --port 5173 --strictPort
