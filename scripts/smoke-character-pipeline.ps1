$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$exports = Join-Path $root "assets\character\exports"
New-Item -ItemType Directory -Force -Path $exports | Out-Null

$char = @{
  schemaVersion   = 1
  revision        = 1
  characterId     = "smoke-$(Get-Date -Format 'yyyyMMddHHmmss')"
  name            = "Chrona"
  startingJob     = "TimeMage"
  pack            = "anata"
  manifestId      = "anata/male"
  catalogVersion  = "anata-2026-11"
  bodyTraits      = @{ Body = @{ id = "male"; name = "male" } }
  starterEquipment = @{ Chest = @{ id = "peasant_shirt"; name = "peasant_shirt" } }
  runtime         = @{ format = "glb"; relativePath = "character/exports/active.glb" }
} | ConvertTo-Json -Depth 6

$jsonPath = Join-Path $exports "active.json"
$glbPath = Join-Path $exports "active.glb"
Set-Content -Path $jsonPath -Value $char -Encoding utf8

$srcGlb = Join-Path $root "assets\kits\castle-kit\Models\GLB format\tower-square-base.glb"
if (-not (Test-Path $srcGlb)) { throw "Missing stand-in GLB: $srcGlb" }
Copy-Item $srcGlb $glbPath -Force

Write-Host "Wrote $jsonPath"
Write-Host "Wrote $glbPath ($((Get-Item $glbPath).Length) bytes)"

# Hit Vite bridge if Studio is up
try {
  $boundary = [guid]::NewGuid().ToString("N")
  $enc = [Text.Encoding]::UTF8
  $ms = New-Object System.IO.MemoryStream
  function W([string]$s) { $b = $enc.GetBytes($s); $ms.Write($b, 0, $b.Length) }
  $glbBytes = [IO.File]::ReadAllBytes($glbPath)
  W "--$boundary`r`n"
  W "Content-Disposition: form-data; name=`"character`"`r`n`r`n"
  W "$char`r`n"
  W "--$boundary`r`n"
  W "Content-Disposition: form-data; name=`"glb`"; filename=`"active.glb`"`r`n"
  W "Content-Type: model/gltf-binary`r`n`r`n"
  $ms.Write($glbBytes, 0, $glbBytes.Length)
  W "`r`n--$boundary--`r`n"
  $tokenResp = Invoke-WebRequest -Uri "http://127.0.0.1:5173/ba/save-token" -Method GET -UseBasicParsing -TimeoutSec 5
  $token = ($tokenResp.Content | ConvertFrom-Json).token
  if (-not $token) { throw "save-token missing" }
  $resp = Invoke-WebRequest -Uri "http://127.0.0.1:5173/ba/save-draft" -Method POST `
    -Headers @{ "X-BA-Save-Token" = $token } `
    -ContentType "multipart/form-data; boundary=$boundary" -Body $ms.ToArray() -UseBasicParsing -TimeoutSec 5
  Write-Host "save-draft OK: $($resp.Content)"
} catch {
  Write-Host "save-draft skipped (Studio not up): $($_.Exception.Message)"
}

# Parse draft with a tiny rust check via existing binary log
cargo build --bin black_anvil -q
$errLog = Join-Path $root "target\smoke_character_err.txt"
Get-Process black_anvil -EA SilentlyContinue | Stop-Process -Force
$p = Start-Process -FilePath (Join-Path $root "target\debug\black_anvil.exe") `
  -WorkingDirectory $root -RedirectStandardError $errLog -RedirectStandardOutput (Join-Path $root "target\smoke_character_out.txt") -PassThru
Start-Sleep -Seconds 10
Stop-Process -Id $p.Id -Force -EA SilentlyContinue
Get-Process black_anvil -EA SilentlyContinue | Stop-Process -Force

$loaded = Select-String -Path $errLog -Pattern "Loaded BaCharacterV1" -SimpleMatch
$queued = Select-String -Path $errLog -Pattern "Queued Studio avatar load" -SimpleMatch
$ready = Select-String -Path $errLog -Pattern "Studio avatar ready" -SimpleMatch
$missingActive = Select-String -Path $errLog -Pattern "character\\exports\\active.glb|character/exports/active.glb" |
  Where-Object { $_.Line -match "Path not found" }

if (-not $loaded) { throw "FAIL: BaCharacterV1 not loaded" }
if (-not $queued) { throw "FAIL: avatar load not queued" }
# AssetEvent may land after the short smoke window on cold caches — require queue at minimum.
if ($missingActive) { throw "FAIL: active.glb missing from asset server" }

if (-not $ready) {
  Write-Host "PROTOTYPE CHECK: draft loaded + avatar queued (AssetEvent ready not observed in window)"
  Write-Host "NOTE: tower GLB stand-in is not M1 acceptance evidence — keep milestone issues open."
} else {
  Write-Host "PROTOTYPE CHECK: draft loaded + avatar queued + ready"
}
