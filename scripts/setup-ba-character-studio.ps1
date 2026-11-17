$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$studio = Join-Path $root "character-studio"
$loot = Join-Path $studio "public\loot-assets"
$assets = Join-Path $root "assets\character"

# Drop non-human packs (male/female only).
foreach ($extra in @("0N1", "tubbycats")) {
  $p = Join-Path $loot $extra
  if (Test-Path $p) {
    Remove-Item -Recurse -Force $p
    Write-Host "Removed $extra"
  }
}

New-Item -ItemType Directory -Force -Path $assets | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $assets "exports") | Out-Null

# Low-level starter gear IDs (Loot Project bottom tiers).
$starterIds = @{
  Body  = $null  # keep all body (male/female only)
  Head  = @("leather_cap", "cap", "hood", "linen_hood")
  Hands = @("leather_gloves", "gloves", "linen_gloves", "wool_gloves")
  Shoes = @("leather_boots", "shoes", "linen_shoes", "wool_shoes")
  Chest = @("shirt", "leather_armor", "linen_robe", "robe")
  Waist = @("leather_belt", "sash", "linen_sash", "wool_sash")
  Neck  = @()  # none at start — craft later
  Weapon = @() # none at start — craft later
}

$starterEquip = @{
  Head  = "leather_cap"
  Hands = "leather_gloves"
  Shoes = "leather_boots"
  Chest = "shirt"
  Waist = "leather_belt"
}

function Filter-Manifest([string]$path, [string]$gender) {
  $json = Get-Content $path -Raw -Encoding utf8 | ConvertFrom-Json
  $json.assetsLocation = "./loot-assets/"
  $json.traitsDirectory = "/anata/$gender/"
  $json.thumbnailsDirectory = "./loot-assets/loot/thumbnails/"
  $json.traitIconsDirectorySvg = "./loot-assets/loot/icons/"
  $json.animationPath = @(
    "./loot-assets/animations/1_T-Pose.fbx",
    "./loot-assets/animations/2_Idle.fbx",
    "./loot-assets/animations/3_Walking.fbx",
    "./loot-assets/animations/4_Waving.fbx"
  )
  # Starter loadout: body + base gear only.
  $json.requiredTraits = @("Body")
  $json.initialTraits = @("Body", "Chest", "Head", "Hands", "Shoes", "Waist")
  $json.randomTraits = @("Body", "Chest", "Head", "Hands", "Shoes", "Waist")

  $newTraits = @()
  foreach ($trait in $json.traits) {
    $name = [string]$trait.trait
    $allow = $starterIds[$name]
    if ($null -eq $allow) {
      # Body: keep as-is
      $newTraits += $trait
      continue
    }
    if ($allow.Count -eq 0) {
      # Empty slot group kept for future craft equip, no options yet.
      $trait.collection = @()
      $newTraits += $trait
      continue
    }
    $filtered = @()
    foreach ($item in $trait.collection) {
      if ($allow -contains [string]$item.id) {
        $filtered += $item
      }
    }
    if ($filtered.Count -eq 0) {
      # Fallback: keep cheapest-looking first 1-2 if names didn't match.
      $filtered = @($trait.collection | Select-Object -First 2)
    }
    $trait.collection = $filtered
    $newTraits += $trait
  }
  $json.traits = $newTraits

  $out = $json | ConvertTo-Json -Depth 40
  # PowerShell ConvertTo-Json can mangle; write UTF8 no BOM via .NET
  [IO.File]::WriteAllText($path, $out)
  Write-Host "Filtered $path"
}

Filter-Manifest (Join-Path $loot "anata\male\manifest.json") "male"
Filter-Manifest (Join-Path $loot "anata\female\manifest.json") "female"

# Character select: male/female Anata only, local manifests.
$select = @{
  characters = @(
    @{
      name = "Anata"
      description = "Female"
      portrait = "./assets/portraitImages/female.jpg"
      manifest = "./loot-assets/anata/female/manifest.json"
      format = "vrm"
    },
    @{
      name = "Anata"
      description = "Male"
      portrait = "./assets/portraitImages/male.jpg"
      manifest = "./loot-assets/anata/male/manifest.json"
      format = "vrm"
    }
  )
  loras = @()
  sprites = @()
  thumbnails = @()
  defaultAnimations = @(
    @{ name = "T-Pose"; description = "T-Pose"; location = "./loot-assets/animations/1_T-Pose.fbx"; icon = "|" },
    @{ name = "Idle"; description = "Idle"; location = "./loot-assets/animations/2_Idle.fbx"; icon = "|" },
    @{ name = "Walking"; description = "Walk"; location = "./loot-assets/animations/3_Walking.fbx"; icon = "|" },
    @{ name = "Waving"; description = "Wave"; location = "./loot-assets/animations/4_Waving.fbx"; icon = "|" }
  )
}
[IO.File]::WriteAllText(
  (Join-Path $studio "public\manifest.json"),
  ($select | ConvertTo-Json -Depth 20)
)
Write-Host "Wrote public/manifest.json (male/female only)"

# Craft catalog: all Anata gear as craftable items (full list from disk, not filtered).
$craft = @{
  version = 1
  note = "Loot-style Anata gear. Starter kit uses base pieces; rest unlock via crafting."
  starter = @{
    level = 1
    traits = $starterEquip
  }
  slots = @{}
}
foreach ($gender in @("male", "female")) {
  $slots = @{}
  $base = Join-Path $loot "anata\$gender"
  Get-ChildItem $base -Directory | ForEach-Object {
    $slot = $_.Name
    $items = @()
    Get-ChildItem $_.FullName -Filter *.vrm | ForEach-Object {
      $id = [IO.Path]::GetFileNameWithoutExtension($_.Name)
      $tier = if ($starterIds.ContainsKey($slot) -and $starterIds[$slot] -contains $id) { 1 }
              elseif ($id -match "divine|holy|ornate|dragon|demon|ancient|great") { 3 }
              elseif ($id -match "plate|chain|studded|hard_leather|war|full") { 2 }
              else { 2 }
      $items += @{
        id = $id
        name = ($id -replace "_", " ")
        slot = $slot
        gender = $gender
        vrm = "anata/$gender/$slot/$($_.Name)"
        craft_level = $tier
        starter = [bool]($tier -eq 1 -and $starterEquip.ContainsKey($slot) -and $starterEquip[$slot] -eq $id)
      }
    }
    $slots[$slot] = $items
  }
  $craft.slots[$gender] = $slots
}
$craftPath = Join-Path $assets "loot-craft-catalog.json"
[IO.File]::WriteAllText($craftPath, ($craft | ConvertTo-Json -Depth 20))
Write-Host "Wrote $craftPath"

# Env: local assets, BA mode flags.
$envText = @"
VITE_ASSET_PATH=./
VITE_BA_MODE=1
VITE_APP_TITLE=Black Anvil Character Forge
"@
[IO.File]::WriteAllText((Join-Path $studio ".env"), $envText)
[IO.File]::WriteAllText((Join-Path $studio ".env.local"), $envText)
Write-Host "Wrote .env"

# Starter selection defaults for BA (picked on create).
$defaults = @{
  male = @{
    Body = "male"
    Chest = "shirt"
    Head = "leather_cap"
    Hands = "leather_gloves"
    Shoes = "leather_boots"
    Waist = "leather_belt"
  }
  female = @{
    Body = "female"
    Chest = "shirt"
    Head = "leather_cap"
    Hands = "leather_gloves"
    Shoes = "leather_boots"
    Waist = "leather_belt"
  }
}
[IO.File]::WriteAllText(
  (Join-Path $assets "starter-loadout.json"),
  ($defaults | ConvertTo-Json -Depth 10)
)
Write-Host "Wrote starter-loadout.json"
Write-Host "SETUP OK"
