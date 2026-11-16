$ErrorActionPreference = "Stop"
$root = "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil"
Set-Location $root

if (-not (Test-Path "character-studio\.git")) {
  if (Test-Path "character-studio") { Remove-Item -Recurse -Force "character-studio" }
  git clone --depth 1 https://github.com/M3-org/CharacterStudio.git character-studio
  Write-Host "CharacterStudio cloned"
} else {
  Write-Host "CharacterStudio already present"
}

$loot = Join-Path $root "character-studio\public\loot-assets"
if (-not (Test-Path "$loot\.git")) {
  if (Test-Path $loot) { Remove-Item -Recurse -Force $loot }
  New-Item -ItemType Directory -Force -Path $loot | Out-Null
  Set-Location $loot
  git init
  git remote add origin https://github.com/M3-org/loot-assets.git
  git config core.sparseCheckout true
  @(
    "anata/"
    "animations/"
    "loot/icons/"
    "loot/thumbnails/"
    "manifest.json"
    "README.md"
  ) | Set-Content ".git\info\sparse-checkout" -Encoding ascii
  git pull --depth 1 origin main
  Write-Host "loot-assets sparse pulled"
} else {
  Write-Host "loot-assets already present"
}

Set-Location $root
Write-Host "Listing loot-assets:"
Get-ChildItem "character-studio\public\loot-assets" -Name
