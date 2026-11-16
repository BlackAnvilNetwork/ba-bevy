$ErrorActionPreference = "Stop"
$root = "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil"
$studioPublic = Join-Path $root "character-studio\public"
$charAssets = Join-Path $studioPublic "character-assets"
$lootAssets = Join-Path $studioPublic "loot-assets"

Write-Host "=== 1) character-assets: neurohacker + drophunter ==="
New-Item -ItemType Directory -Force -Path $charAssets | Out-Null
Set-Location $charAssets
if (-not (Test-Path ".git")) {
  git init
  git remote add origin https://github.com/memelotsqui/character-assets.git
}
git config core.sparseCheckout true
New-Item -ItemType Directory -Force -Path ".git\info" | Out-Null
@(
  "neurohacker/"
  "drophunter/"
  "manifest.json"
  "README.md"
) | Set-Content ".git\info\sparse-checkout" -Encoding ascii
git fetch --depth 1 origin main
git checkout FETCH_HEAD -- neurohacker drophunter manifest.json README.md 2>$null
git sparse-checkout reapply 2>$null
Write-Host "neurohacker files:" (Get-ChildItem neurohacker -Recurse -File -EA SilentlyContinue | Measure-Object).Count
Write-Host "drophunter files:" (Get-ChildItem drophunter -Recurse -File -EA SilentlyContinue | Measure-Object).Count

Write-Host "`n=== 2) loot-assets: anata + 0N1 + tubbycats + animations + loot ==="
if (-not (Test-Path (Join-Path $lootAssets ".git"))) {
  if (Test-Path $lootAssets) {
    Write-Host "loot-assets exists without .git — cloning into temp then merging missing packs"
    $tmp = Join-Path $env:TEMP "ba-loot-assets-full"
    if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
    git clone --depth 1 --filter=blob:none --sparse https://github.com/m3-org/loot-assets.git $tmp
    Set-Location $tmp
    git sparse-checkout set anata 0N1 tubbycats animations loot manifest.json README.md
    foreach ($pack in @("anata", "0N1", "tubbycats", "animations", "loot", "manifest.json", "README.md")) {
      $src = Join-Path $tmp $pack
      $dst = Join-Path $lootAssets $pack
      if (-not (Test-Path $src)) { continue }
      if (Test-Path $dst) {
        Write-Host "keep existing $pack"
      } else {
        Write-Host "copy $pack"
        Copy-Item -Recurse -Force $src $dst
      }
    }
  } else {
    git clone --depth 1 https://github.com/m3-org/loot-assets.git $lootAssets
  }
} else {
  Set-Location $lootAssets
  git fetch --depth 1 origin main
  git checkout FETCH_HEAD -- 0N1 tubbycats anata animations loot manifest.json README.md 2>$null
}

Write-Host "`n=== local pack status ==="
$checks = @{
  "neurohacker" = (Join-Path $charAssets "neurohacker\body")
  "drophunter"  = (Join-Path $charAssets "drophunter\body")
  "anata/male"  = (Join-Path $lootAssets "anata\male\Body")
  "anata/female"= (Join-Path $lootAssets "anata\female\Body")
  "0N1"         = (Join-Path $lootAssets "0N1")
  "tubbycats"   = (Join-Path $lootAssets "tubbycats")
}
foreach ($k in $checks.Keys | Sort-Object) {
  $ok = Test-Path $checks[$k]
  "{0,-14} {1}" -f $k, ($(if ($ok) { "OK" } else { "MISSING" }))
}
