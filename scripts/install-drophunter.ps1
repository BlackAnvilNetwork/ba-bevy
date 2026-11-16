$ErrorActionPreference = "Stop"
$root = "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil"
$dest = Join-Path $root "character-studio\public\character-assets"
$body = Join-Path $dest "drophunter\body\drophunter.vrm"

if (Test-Path $body) {
  Write-Host "drophunter assets already present"
  exit 0
}

New-Item -ItemType Directory -Force -Path $dest | Out-Null
Set-Location $dest

if (-not (Test-Path (Join-Path $dest ".git"))) {
  git init
  git remote add origin https://github.com/memelotsqui/character-assets.git
  git config core.sparseCheckout true
}

# Keep neurohacker + add drophunter.
$sparse = @(
  "neurohacker/"
  "drophunter/"
  "manifest.json"
  "README.md"
)
$sparseFile = Join-Path $dest ".git\info\sparse-checkout"
New-Item -ItemType Directory -Force -Path (Split-Path $sparseFile) | Out-Null
$sparse | Set-Content $sparseFile -Encoding ascii

git pull --depth 1 origin main
if (-not (Test-Path $body)) {
  throw "drophunter body missing after pull: $body"
}
Write-Host "Installed drophunter into $dest"
Get-ChildItem "drophunter" -Name
