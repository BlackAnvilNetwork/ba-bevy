$ErrorActionPreference = "Stop"
$root = "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil"
$dest = Join-Path $root "character-studio\public\character-assets"

# Repair: accidental `git init` in BA root during a broken install pointed origin at character-assets.
$baGit = Join-Path $root ".git"
if (Test-Path $baGit) {
  $origin = ""
  try { $origin = (git -C $root remote get-url origin 2>$null) } catch {}
  if ($origin -match "character-assets") {
    Write-Host "Removing broken BA root .git (was pointing at character-assets)"
    Remove-Item -Recurse -Force $baGit
    git -C $root init | Out-Null
    Write-Host "Re-initialized empty BA git repo (no remote)."
  }
}

if (Test-Path (Join-Path $dest "neurohacker\manifest.json")) {
  Write-Host "neurohacker already present at $dest"
  exit 0
}

if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Set-Location $dest
git init
git remote add origin https://github.com/memelotsqui/character-assets.git
git config core.sparseCheckout true
@(
  "neurohacker/"
  "manifest.json"
  "README.md"
) | Set-Content ".git\info\sparse-checkout" -Encoding ascii
git pull --depth 1 origin main
Write-Host "Installed neurohacker into $dest"
Get-ChildItem "neurohacker" -Name
