# Kimodo generate for 1H cuts at an enemy in front, then retarget onto Chrona.
# Sequential GPU only (batch OOMs the GTX 1070). No MotionBricks chop bridges.
# Prompts describe the body, not a prop — Kimodo has no sword.
#
# Spec (character faces the stick monster; +Z is toward it, -X is their right):
#   Slash1H  diagonal: load high-right-front -> cut THROUGH the foe on the
#            center line -> follow left-forward-down. Still in front.
#   Return   rising:   reverse of that (left-forward-low -> high-right-front).
#   Sweep    yokogiri: chest-height cut across the foe, both ends FORWARD.
# Contact is in front of the sternum (z>=0.45), never the far hip or behind
# the back. 0.5 s clip so follow is the last frame inside the 0.48 s combat
# clock (hit at frame 8, ~0.27 s). Contact is a full-body keyframe so the
# elbow opens through the cut; a hand-only effector left it bent (~100°)
# and the shoulder did the reach. Return ends on the open-elbow hit pose.
# Poses start from hip-carry. -Phase slashes re-runs the pose gate and stops
# if any file fails. Seeds and gate ranges are written into each .glb.json.
#
# LOCKED v13: assets/character/motion/slash_1h*.glb are frozen. Dual wield is
# scripts/ba-author-slash-dw.ps1. Pass -Force to overwrite the 1H retargets.
param(
    [ValidateSet("poses", "slashes", "all")]
    [string]$Phase = "all",
    [switch]$Force
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$tm = Join-Path $env:LOCALAPPDATA "TideMotion\bin\tide-motion.exe"
if (!(Test-Path $tm)) { throw "Tide Motion is not installed. Expected $tm" }

$work = Join-Path $root ".dream-loop\motion"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$outDir = Join-Path $root "assets\character\motion"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$map = Join-Path $outDir "anata-exported.json"
$idle = Join-Path $work "idle-soma.glb"
if (!(Test-Path $idle)) { throw "Missing $idle — run ba-author-motion.ps1 first" }
$gatePy = Join-Path $root ".dream-loop\qa\pose_gate.py"
$slashGatePy = Join-Path $root ".dream-loop\qa\slash_gate.py"
$elbowGatePy = Join-Path $root ".dream-loop\qa\elbow_gate.py"
$baGatePy = Join-Path $root ".dream-loop\qa\write_bagate.py"

$target = Join-Path $env:LOCALAPPDATA "BlackAnvil\characters\1fe031bd-99ca-4674-81ef-a06c42463bf0\revisions\r2\character.glb"
if (!(Test-Path $target)) {
    $target = Get-ChildItem (Join-Path $env:LOCALAPPDATA "BlackAnvil\characters") -Recurse -Filter character.glb |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}
if (!(Test-Path $target)) { throw "No exported character.glb to retarget onto" }

function Invoke-Tide {
    param([string[]]$Cli)
    Write-Host ("tide-motion " + ($Cli -join " "))
    & $tm @Cli
    if ($LASTEXITCODE -ne 0) { throw "tide-motion failed: $($Cli -join ' ')" }
}

$gpu = @("--model", "v1.1", "--steps", "25", "--in-place", "--device", "gpu")
$carry = Join-Path $work "hip-carry-v2-soma.glb"
if (!(Test-Path $carry)) {
    Invoke-Tide (@(
            "generate",
            "A person stands still with the right hand in front of the right hip, elbow slightly bent, left arm relaxed.",
            "--duration", "1.6", "--seed", "301", "--guidance", "4",
            "--start-pose", $idle, "-o", $carry
        ) + $gpu)
} else {
    Write-Host "Reusing $carry"
}

# Right = negative X. Forward toward the stick monster = positive Z.
$Poses = @(
    @{
        Rel     = "pose-diag-wind-v11-soma.glb"
        Prompt  = "A standing person reaches the right hand forward and up from beside the right hip to shoulder height, staying on the right, elbow bent, looking forward, both feet planted, in place."
        Seed0   = 882
        Xmin    = -0.32; Xmax = -0.10
        Ymin    = 1.28;  Ymax = 1.66
        Zmin    = 0.28;  Zmax = 0.60
        Role    = "diag-wind"
    }
    @{
        Rel      = "pose-diag-hit-v13-soma.glb"
        Prompt   = "A standing person reaches the right hand as far forward as possible, level with the middle of the chest, looking forward, both feet planted, in place."
        Seed0    = 1022
        Xmin     = -0.12; Xmax = 0.12
        Ymin     = 1.08;  Ymax = 1.34
        Zmin     = 0.50;  Zmax = 0.88
        ElbowMin = 128; ElbowMax = 175
        Role     = "diag-hit"
    }
    @{
        Rel     = "pose-diag-follow-v11-soma.glb"
        Prompt  = "A standing person reaches the right hand forward and slightly down, just left of center, elbow bent, looking forward, both feet planted, in place."
        Seed0   = 821
        Xmin    = 0.04;  Xmax = 0.24
        Ymin    = 0.85;  Ymax = 1.12
        Zmin    = 0.35;  Zmax = 0.68
        Role    = "diag-follow"
    }
    @{
        Rel     = "pose-sweep-wind-v11-soma.glb"
        Prompt  = "A standing person reaches the right hand straight forward, well to the right of the chest, elbow bent, looking forward, both feet planted, in place."
        Seed0   = 831
        Xmin    = -0.30; Xmax = -0.05
        Ymin    = 1.08;  Ymax = 1.32
        Zmin    = 0.40;  Zmax = 0.72
        Role    = "sweep-wind"
    }
)

function Invoke-PyOk {
    param([string[]]$Cli)
    $out = & py -3.13 @Cli
    $code = $LASTEXITCODE
    if ($null -ne $out) { Write-Host ($out | Out-String).Trim() }
    return ($code -eq 0)
}

function Write-BaGate {
    param($Glb, $Meta)
    $json = $Meta | ConvertTo-Json -Compress -Depth 6
    $json | & py -3.13 $baGatePy "$Glb.json"
    if ($LASTEXITCODE -ne 0) { throw "write_bagate failed for $Glb" }
}

function Test-PoseGate {
    param($Pose)
    $path = Join-Path $work $Pose.Rel
    $hand = Invoke-PyOk @(
            $gatePy, $path,
            "$($Pose.Xmin)", "$($Pose.Xmax)",
            "$($Pose.Ymin)", "$($Pose.Ymax)",
            "$($Pose.Zmin)", "$($Pose.Zmax)"
        )
    if (-not $hand) { return $false }
    if ($null -eq $Pose.ElbowMin) { return $true }
    return (Invoke-PyOk @(
            $elbowGatePy, $path, "$($Pose.ElbowMin)", "$($Pose.ElbowMax)"
        ))
}

function New-GatedPose {
    param($Pose)
    $path = Join-Path $work $Pose.Rel
    if ((Test-Path $path) -and (Test-PoseGate $Pose)) {
        Write-BaGate $path @{
            role       = $Pose.Role
            seed       = $Pose.Seed0
            seed0      = $Pose.Seed0
            prompt     = $Pose.Prompt
            startPose  = $(if ($Pose.StartRel) { "$($Pose.StartRel)@last" } else { "hip-carry-v2-soma.glb@last" })
            model      = "v1.1"
            steps      = 25
            guidance   = 4
            x          = @($Pose.Xmin, $Pose.Xmax)
            y          = @($Pose.Ymin, $Pose.Ymax)
            z          = @($Pose.Zmin, $Pose.Zmax)
            elbow      = @($Pose.ElbowMin, $Pose.ElbowMax)
            kind       = "pose"
            reused     = $true
        }
        Write-Host "POSE_GATE_REUSE_OK $($Pose.Rel)"
        return
    }
    for ($i = 0; $i -lt 5; $i++) {
        $seed = $Pose.Seed0 + $i
        Remove-Item -Force -ErrorAction SilentlyContinue $path, "$path.json"
        $start = $carry
        if ($Pose.StartRel) { $start = Join-Path $work $Pose.StartRel }
        Invoke-Tide (@(
                "generate", $Pose.Prompt,
                "--duration", "1.2", "--seed", "$seed", "--guidance", "4",
                "--start-pose", "$start@last",
                "-o", $path
            ) + $gpu)
        if (Test-PoseGate $Pose) {
            Write-BaGate $path @{
                role       = $Pose.Role
                seed       = $seed
                seed0      = $Pose.Seed0
                prompt     = $Pose.Prompt
                startPose  = $(if ($Pose.StartRel) { "$($Pose.StartRel)@last" } else { "hip-carry-v2-soma.glb@last" })
                model      = "v1.1"
                steps      = 25
                guidance   = 4
                x          = @($Pose.Xmin, $Pose.Xmax)
                y          = @($Pose.Ymin, $Pose.Ymax)
                z          = @($Pose.Zmin, $Pose.Zmax)
                elbow      = @($Pose.ElbowMin, $Pose.ElbowMax)
                kind       = "pose"
            }
            Write-Host "POSE_GATE_OK $($Pose.Rel) seed=$seed"
            return
        }
        Write-Host "POSE_GATE_FAIL $($Pose.Rel) seed=$seed — retry"
    }
    throw "Pose $($Pose.Rel) missed the attack gate after 5 seeds"
}

function Assert-GatedPoses {
    foreach ($pose in $Poses) {
        $path = Join-Path $work $pose.Rel
        if (!(Test-Path $path)) {
            throw "Missing pose $path — run with -Phase poses first"
        }
        if (!(Test-PoseGate $pose)) {
            throw "Pose $($pose.Rel) failed the attack gate. Delete it and re-run -Phase poses."
        }
        Write-Host "POSE_GATE_RECHECK_OK $($pose.Rel)"
    }
}

if ($Phase -eq "poses" -or $Phase -eq "all") {
    foreach ($pose in $Poses) { New-GatedPose $pose }
    Write-Host "POSE_AUTHOR_DONE"
}

if ($Phase -eq "poses") { return }

Assert-GatedPoses

$diagWind = Join-Path $work "pose-diag-wind-v11-soma.glb"
$diagHit = Join-Path $work "pose-diag-hit-v13-soma.glb"
$diagFollow = Join-Path $work "pose-diag-follow-v11-soma.glb"
$sweepWind = Join-Path $work "pose-sweep-wind-v11-soma.glb"
$sweepHit = Join-Path $work "pose-diag-hit-v13-soma.glb"

function New-GatedSlash {
    param($Rel, $Prompt, $Wind, $Hit, $Follow, $Seed0, $Role)
    $path = Join-Path $work $Rel
    if ((Test-Path $path) -and (Invoke-PyOk @($slashGatePy, $path))) {
        Write-Host "SLASH_GATE_REUSE_OK $Rel"
        return
    }
    for ($i = 0; $i -lt 3; $i++) {
        $seed = $Seed0 + $i
        Remove-Item -Force -ErrorAction SilentlyContinue $path, "$path.json"
        Invoke-Tide (@(
                "generate", $Prompt,
                "--duration", "0.5",
                "--seed", "$seed",
                "--guidance", "4",
                "--constraint-guidance", "2",
                "--start-pose", "$Wind@last",
                "--keyframe", "8", "$Hit@last",
                "--end-pose", "$Follow@last",
                "--correction-frames", "4",
                "-o", $path
            ) + $gpu)
        if (Invoke-PyOk @($slashGatePy, $path)) {
            Write-BaGate $path @{
                role                = $Role
                seed                = $seed
                seed0               = $Seed0
                prompt              = $Prompt
                wind                = [IO.Path]::GetFileName($Wind)
                hit                 = [IO.Path]::GetFileName($Hit)
                follow              = [IO.Path]::GetFileName($Follow)
                model               = "v1.1"
                steps               = 25
                duration            = 0.5
                guidance            = 4
                constraintGuidance  = 2
                gate                = @{
                    yMax    = 1.70
                    zMaxMin = 0.45
                    endZMin = 0.36
                    xRange       = @(0.16, 0.72)
                    zMin         = 0.30
                    hitElbowMin  = 125
                }
                kind                = "slash"
            }
            Write-Host "SLASH_GATE_OK $Rel seed=$seed"
            return
        }
        Write-Host "SLASH_GATE_FAIL $Rel seed=$seed — retry"
    }
    throw "Slash $Rel missed the forward-cut gate after 3 seeds"
}

New-GatedSlash "slash-1h-diag-v13-soma.glb" `
    "A standing person chops downward in front of themselves with the right arm, the right elbow opening, in place." `
    $diagWind $diagHit $diagFollow 961 "diag"
New-GatedSlash "slash-1h-return-v13-soma.glb" `
    "A standing person chops upward in front of themselves with the right arm, the right elbow opening, in place." `
    $diagFollow $diagHit $diagHit 971 "return"
New-GatedSlash "slash-1h-sweep-v13-soma.glb" `
    "A standing person swings the right arm across in front of the chest, the right elbow opening, in place." `
    $sweepWind $sweepHit $diagFollow 981 "sweep"

$pairs = @(
    @{ Soma = "slash-1h-diag-v13-soma.glb"; Out = "slash_1h.glb" },
    @{ Soma = "slash-1h-return-v13-soma.glb"; Out = "slash_1h_return.glb" },
    @{ Soma = "slash-1h-sweep-v13-soma.glb"; Out = "slash_1h_sweep.glb" }
)
if (-not $Force) {
    Write-Host "SLASH_1H_LOCKED skipping retarget of slash_1h* (pass -Force to overwrite)"
} else {
    foreach ($pair in $pairs) {
        $soma = Join-Path $work $pair.Soma
        $walk = Join-Path $outDir $pair.Out
        $recipe = "$walk.json"
        Remove-Item -Force -ErrorAction SilentlyContinue $walk, $recipe
        Write-Host "Retargeting $($pair.Soma) -> $($pair.Out)"
        Invoke-Tide @("retarget", $soma, "--target", $target, "--map", $map, "-o", $walk)
    }
}
Write-Host "SLASH_AUTHOR_DONE"
