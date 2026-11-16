# Extract ability/WS catalogs from ba-stick TypeScript kits into JSON for Black Anvil.
$ErrorActionPreference = "Stop"
$root = "C:\Users\raxac\OneDrive\Desktop\apps\ba-stick\packages\combat\src"
$outDir = "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil\assets\combat"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

function Get-ObjectBlocks([string]$text, [string]$constName) {
  $marker = "export const $constName"
  $idx = $text.IndexOf($marker)
  if ($idx -lt 0) { return @() }
  $brace = $text.IndexOf("{", $idx)
  if ($brace -lt 0) { return @() }
  $depth = 0
  $end = -1
  for ($i = $brace; $i -lt $text.Length; $i++) {
    $c = $text[$i]
    if ($c -eq "{") { $depth++ }
    elseif ($c -eq "}") {
      $depth--
      if ($depth -eq 0) { $end = $i; break }
    }
  }
  if ($end -lt 0) { return @() }
  $body = $text.Substring($brace + 1, $end - $brace - 1)
  # Split top-level key blocks: "key: {" ... "},"
  $blocks = @()
  $rx = [regex]::new('(?ms)^\s*([a-z0-9_]+)\s*:\s*\{')
  $matches = $rx.Matches($body)
  for ($m = 0; $m -lt $matches.Count; $m++) {
    $start = $matches[$m].Index + $matches[$m].Length - 1
    $key = $matches[$m].Groups[1].Value
    $d = 0
    $e = -1
    for ($i = $start; $i -lt $body.Length; $i++) {
      if ($body[$i] -eq "{") { $d++ }
      elseif ($body[$i] -eq "}") {
        $d--
        if ($d -eq 0) { $e = $i; break }
      }
    }
    if ($e -lt 0) { continue }
    $blocks += [pscustomobject]@{ key = $key; body = $body.Substring($start, $e - $start + 1) }
  }
  return $blocks
}

function Get-Field([string]$body, [string]$name) {
  $m = [regex]::Match($body, "(?m)^\s*$name\s*:\s*([^,\r\n]+)")
  if (-not $m.Success) { return $null }
  return $m.Groups[1].Value.Trim().TrimEnd(",")
}

function Get-StringField([string]$body, [string]$name) {
  $m = [regex]::Match($body, "(?ms)$name\s*:\s*`"((?:\\.|[^`"])*)`"")
  if (-not $m.Success) { return $null }
  return $m.Groups[1].Value -replace '\\"', '"' -replace '\\n', "`n"
}

function Get-NumField([string]$body, [string]$name) {
  $raw = Get-Field $body $name
  if ($null -eq $raw) { return $null }
  # evaluate simple expressions like 2 * 60 * 60 * 1000 or 30_000
  $expr = $raw -replace "_", "" -replace "true", "1" -replace "false", "0"
  if ($expr -match "^[\d\.\+\-\*/\(\)\s]+$") {
    try { return [int][math]::Round((Invoke-Expression $expr)) } catch { return $null }
  }
  if ($expr -match "^\d+") { return [int]$Matches[0] }
  return $null
}

function Get-BoolField([string]$body, [string]$name) {
  $raw = Get-Field $body $name
  if ($null -eq $raw) { return $null }
  if ($raw -eq "true") { return $true }
  if ($raw -eq "false") { return $false }
  return $null
}

function Get-FloatField([string]$body, [string]$name) {
  $raw = Get-Field $body $name
  if ($null -eq $raw) { return $null }
  $expr = $raw -replace "_", ""
  if ($expr -match "^[\d\.\+\-\*/\(\)\s]+$") {
    try { return [double](Invoke-Expression $expr) } catch { return $null }
  }
  return $null
}

function Get-Blurbs([string]$text, [string]$constName) {
  $map = @{}
  $blocks = Get-ObjectBlocks $text $constName
  foreach ($b in $blocks) {
    # body is { "text" } for blurbs sometimes key: "text"
    $str = Get-StringField $b.body ""
    # blurbs are key: "string" not nested objects — re-parse from parent
  }
  $marker = "export const $constName"
  $idx = $text.IndexOf($marker)
  if ($idx -lt 0) { return $map }
  $brace = $text.IndexOf("{", $idx)
  $depth = 0
  $end = -1
  for ($i = $brace; $i -lt $text.Length; $i++) {
    if ($text[$i] -eq "{") { $depth++ }
    elseif ($text[$i] -eq "}") { $depth--; if ($depth -eq 0) { $end = $i; break } }
  }
  $body = $text.Substring($brace + 1, $end - $brace - 1)
  $rx = [regex]::new('(?ms)^\s*([a-z0-9_]+)\s*:\s*"((?:\\.|[^"])*)"')
  foreach ($m in $rx.Matches($body)) {
    $map[$m.Groups[1].Value] = ($m.Groups[2].Value -replace '\\"', '"' -replace '\\n', "`n")
  }
  return $map
}

function Extract-Kit([string]$file, [string]$job, [string]$abilitiesConst, [string]$blurbsConst) {
  $text = Get-Content -Raw -Path (Join-Path $root $file)
  $blurbs = Get-Blurbs $text $blurbsConst
  $abilities = @()
  foreach ($b in (Get-ObjectBlocks $text $abilitiesConst)) {
    $id = Get-StringField $b.body "id"
    if (-not $id) { $id = $b.key }
    $abilities += [ordered]@{
      id = $id
      job = $job
      label = (Get-StringField $b.body "label")
      glyph = (Get-StringField $b.body "glyph")
      unlockLevel = (Get-NumField $b.body "unlockLevel")
      mp = (Get-NumField $b.body "mp")
      recastMs = (Get-NumField $b.body "recastMs")
      category = (Get-StringField $b.body "category")
      durationMs = (Get-NumField $b.body "durationMs")
      heal = (Get-NumField $b.body "heal")
      potency = (Get-NumField $b.body "potency")
      range = (Get-NumField $b.body "range")
      aoe = (Get-NumField $b.body "aoe")
      needsTarget = (Get-BoolField $b.body "needsTarget")
      allyTarget = (Get-BoolField $b.body "allyTarget")
      physDt = (Get-FloatField $b.body "physDt")
      magDt = (Get-FloatField $b.body "magDt")
      haste = (Get-FloatField $b.body "haste")
      atkMul = $(
        $v = Get-FloatField $b.body "atkMul"
        if ($null -eq $v) { Get-FloatField $b.body "attMul" } else { $v }
      )
      blurb = $(if ($blurbs.ContainsKey($id)) { $blurbs[$id] } else { "" })
    }
  }
  return $abilities
}

$all = @()
$all += Extract-Kit "tim-kit.ts" "time_mage" "TIM_ABILITIES" "TIM_ABILITY_BLURBS"
$all += Extract-Kit "knight-kit.ts" "knight" "KNIGHT_ABILITIES" "KNIGHT_ABILITY_BLURBS"
$all += Extract-Kit "rogue-kit.ts" "rogue" "ROGUE_ABILITIES" "ROGUE_ABILITY_BLURBS"
$all += Extract-Kit "cleric-kit.ts" "cleric" "CLERIC_ABILITIES" "CLERIC_ABILITY_BLURBS"
$all += Extract-Kit "sorcerer-kit.ts" "sorcerer" "SORCERER_ABILITIES" "SORCERER_ABILITY_BLURBS"
$all += Extract-Kit "fighter-kit.ts" "fighter" "FIGHTER_ABILITIES" "FIGHTER_ABILITY_BLURBS"
$all += Extract-Kit "battlemage-kit.ts" "battle_mage" "BATTLEMAGE_ABILITIES" "BATTLEMAGE_ABILITY_BLURBS"
$all += Extract-Kit "druid-kit.ts" "druid" "DRUID_ABILITIES" "DRUID_ABILITY_BLURBS"

# Weapon skills
$wsText = Get-Content -Raw -Path (Join-Path $root "weapon-tp.ts")
$ws = @()
# Parse def("id", "weapon", level, "label", "glyph", hits, ftp, "blurb", ...
$wsRx = [regex]::new('(?ms)def\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*(\d+)\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*(\d+)\s*,\s*([\d.]+)\s*,\s*"((?:\\.|[^"])*)"')
foreach ($m in $wsRx.Matches($wsText)) {
  $ws += [ordered]@{
    id = $m.Groups[1].Value
    weapon = $m.Groups[2].Value
    unlockLevel = [int]$m.Groups[3].Value
    label = $m.Groups[4].Value
    glyph = $m.Groups[5].Value
    hits = [int]$m.Groups[6].Value
    ftp = [double]$m.Groups[7].Value
    blurb = ($m.Groups[8].Value -replace '\\"', '"')
    tpCost = 1000
    kind = "weapon_skill"
  }
}

$hotbarHints = [ordered]@{
  time_mage = @("flux","quicken","slow","haste","cure","temporal_distortion","ws_staff_heavy_swing")
  knight = @("provoke","kn_flash","shield_bash","bulwark","kn_cure","ws_sword_fast_blade")
  rogue = @("ghost_step","backblade","purse_cut","kill_edge","ws_dg_wasp_sting")
  cleric = @("cl_cure","cl_banish","sacred_mercy","cl_protect","cl_flash","ws_club_shiny_strike")
  sorcerer = @("sc_ember_lance","sc_rime_needle","sc_volt_arc","arcane_flood","ws_staff_heavy_swing")
  fighter = @("berserk","warcry","killing_storm","aggressor","ws_gs_hard_slash")
  battle_mage = @("spellblade","bm_flame_edge","bm_fire","bm_cure","ws_sword_fast_blade")
  druid = @("dr_stickbear","dr_stickdeer","dr_sticklion","dr_dustmend","dr_sour_hum","ws_staff_heavy_swing")
}

$jobWeapons = [ordered]@{
  time_mage = "staff"
  knight = "sword"
  rogue = "dagger"
  cleric = "club"
  sorcerer = "staff"
  fighter = "greatsword"
  battle_mage = "sword"
  druid = "staff"
}

$payload = [ordered]@{
  source = "ba-stick/packages/combat"
  abilityCount = $all.Count
  weaponSkillCount = $ws.Count
  jobWeapons = $jobWeapons
  hotbarHints = $hotbarHints
  abilities = $all
  weaponSkills = $ws
}

$jsonPath = Join-Path $outDir "kits.json"
$json = $payload | ConvertTo-Json -Depth 8 -Compress:$false
# UTF-8 without BOM — serde_json rejects a leading BOM.
[System.IO.File]::WriteAllText($jsonPath, $json, (New-Object System.Text.UTF8Encoding $false))
Write-Host "Wrote $jsonPath"
Write-Host "abilities=$($all.Count) weaponSkills=$($ws.Count)"
$all | Group-Object job | ForEach-Object { Write-Host ("  {0}: {1}" -f $_.Name, $_.Count) }
