# Offline SOMA walk -> Anata exported skeleton -> assets/character/motion/walk.glb
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$tm = Join-Path $env:LOCALAPPDATA "TideMotion\bin\tide-motion.exe"
if (!(Test-Path $tm)) { throw "Tide Motion is not installed. Expected $tm" }

$work = Join-Path $root ".dream-loop\motion"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$outDir = Join-Path $root "assets\character\motion"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$map = Join-Path $outDir "anata-exported.json"
$soma = Join-Path $work "walk-soma.glb"
$target = Join-Path $env:LOCALAPPDATA "BlackAnvil\characters\1fe031bd-99ca-4674-81ef-a06c42463bf0\revisions\r2\character.glb"
if (!(Test-Path $target)) {
    $target = Get-ChildItem (Join-Path $env:LOCALAPPDATA "BlackAnvil\characters") -Recurse -Filter character.glb |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}
if (!(Test-Path $target)) { throw "No exported character.glb to retarget onto" }

if (!(Test-Path $soma)) {
    Write-Host "Generating in-place walk on SOMA..."
    & $tm generate "A person walks forward." --duration 2.5 --seed 4 --in-place --footsteps 2 --model v1.1 --steps 25 -o $soma
    if ($LASTEXITCODE -ne 0) { throw "tide-motion generate failed" }
} else {
    Write-Host "Reusing $soma"
}

$walk = Join-Path $outDir "walk.glb"
Write-Host "Retargeting onto $target"
& $tm retarget $soma --target $target --map $map -o $walk
if ($LASTEXITCODE -ne 0) { throw "tide-motion retarget failed" }
Write-Host "Wrote $walk"
