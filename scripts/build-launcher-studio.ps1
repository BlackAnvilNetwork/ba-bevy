# Production Character Studio build for Tauri distDir (character-studio/build).
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$studio = Join-Path $root "character-studio"

if (-not (Test-Path "$studio\package.json")) {
  throw "Character Studio missing — run scripts\install-character-studio.ps1"
}

Set-Location $studio
if (-not (Test-Path "node_modules")) {
  npm install --legacy-peer-deps
}

$env:VITE_BA_MODE = "1"
npm run build
Write-Host "Studio build ready at character-studio/build (Tauri distDir)."
