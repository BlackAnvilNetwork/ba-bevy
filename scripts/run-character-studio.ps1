$ErrorActionPreference = "Stop"
$studio = "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil\character-studio"

if (-not (Test-Path "$studio\package.json")) {
  Write-Host "Character Studio missing. Run scripts\install-character-studio.ps1 then scripts\setup-ba-character-studio.ps1"
  exit 1
}

Set-Location $studio
if (-not (Test-Path "node_modules")) {
  Write-Host "Installing npm deps..."
  npm install --legacy-peer-deps
}

Write-Host "Black Anvil Character Studio — male/female Anata, body+face kept, starter gear."
Write-Host "Open http://127.0.0.1:5173  (Optimize page merges meshes/atlas per docs)"
# Call vite directly — `npm run dev -- --port 5173` mangles into `vite --host 127.0.0.1 5173`
# (positional root dir) and serves a 404.
npx vite --host 127.0.0.1 --port 5173 --strictPort
