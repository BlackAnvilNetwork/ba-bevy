# Full 2H cut. The locked plant (01_forward, blade straight at the foe) is the
# contact waypoint — the swing passes through it, it is not the start.
# Kimodo has no sword — body prompts only. Both hands stay on one grip.
# 0.5 s clip fits the 0.48 s combat clock (straight-out at frame 8).
# Sequential GPU (GTX 1070). slash_1h stays locked.
#
# Wind: both hands high-right (chamber).
# Plant: locked 01_forward — hands in front, reaching at the foe.
# Follow: through center to the left, still in front (not the dirt).
# After the oneshot, idle 2H wrap returns to center.
param(
    [ValidateSet("poses", "slashes", "all")]
    [string]$Phase = "all"
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
$slashGatePy = Join-Path $root ".dream-loop\qa\two_hand_slash_gate.py"
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

# Right = negative X. Forward toward the foe = positive Z.
# Plant matches 01_forward handle [-0.16, 0.12, 0.40] in chest space (~world Y 1.3, Z 0.4).
$Poses = @(
    @{
        Rel       = "pose-2h-wind-v1-soma.glb"
        Prompt    = "A standing person raises both hands together in front of the right side of the chest, toward the right shoulder, looking forward, both feet planted, in place."
        Seed0     = 1841
        Xmin      = -0.32; Xmax = 0.06
        Ymin      = 1.30;  Ymax = 1.68
        Zmin      = 0.08;  Zmax = 0.36
        LeftXmin  = -0.20; LeftXmax = 0.18
        LeftYmin  = 1.30;  LeftYmax = 1.72
        LeftZmin  = 0.06;  LeftZmax = 0.34
        Role      = "2h-wind"
    }
    @{
        Rel       = "pose-2h-plant-v1-soma.glb"
        Prompt    = "A standing person holds both hands together in front of the chest, slightly to their right, reaching forward, looking forward, both feet planted, in place."
        Seed0     = 1801
        Xmin      = -0.32; Xmax = 0.02
        Ymin      = 1.10;  Ymax = 1.50
        Zmin      = 0.20;  Zmax = 0.65
        LeftXmin  = -0.22; LeftXmax = 0.16
        LeftYmin  = 1.10;  LeftYmax = 1.50
        LeftZmin  = 0.16;  LeftZmax = 0.60
        Role      = "2h-plant"
    }
    @{
        Rel       = "pose-2h-follow-left-v1-soma.glb"
        Prompt    = "A standing person swings both hands across in front of the chest from the center toward their left, reaching forward, looking forward, both feet planted, in place."
        Seed0     = 1811
        Xmin      = 0.04;  Xmax = 0.36
        Ymin      = 1.00;  Ymax = 1.42
        Zmin      = 0.20;  Zmax = 0.70
        LeftXmin  = 0.04;  LeftXmax = 0.42
        LeftYmin  = 1.00;  LeftYmax = 1.42
        LeftZmin  = 0.16;  LeftZmax = 0.66
        Role      = "2h-follow-left"
        StartRel  = "pose-2h-plant-v1-soma.glb"
    }
    @{
        Rel       = "pose-2h-idle-center-v1-soma.glb"
        Prompt    = "A standing person holds both hands together in front of the middle of the chest, elbows bent, looking forward, both feet planted, in place."
        Seed0     = 1821
        Xmin      = -0.14; Xmax = 0.14
        Ymin      = 1.05;  Ymax = 1.50
        Zmin      = 0.18;  Zmax = 0.52
        LeftXmin  = -0.12; LeftXmax = 0.20
        LeftYmin  = 1.05;  LeftYmax = 1.52
        LeftZmin  = 0.16;  LeftZmax = 0.52
        Role      = "2h-idle-center"
        StartRel  = "pose-2h-follow-left-v1-soma.glb"
    }
)

function Test-PoseGate {
    param($Pose)
    $path = Join-Path $work $Pose.Rel
    $right = Invoke-PyOk @(
            $gatePy, $path,
            "$($Pose.Xmin)", "$($Pose.Xmax)",
            "$($Pose.Ymin)", "$($Pose.Ymax)",
            "$($Pose.Zmin)", "$($Pose.Zmax)"
        )
    if (-not $right) { return $false }
    return (Invoke-PyOk @(
            $gatePy, $path,
            "$($Pose.LeftXmin)", "$($Pose.LeftXmax)",
            "$($Pose.LeftYmin)", "$($Pose.LeftYmax)",
            "$($Pose.LeftZmin)", "$($Pose.LeftZmax)",
            "--left"
        ))
}

function New-GatedPose {
    param($Pose)
    $path = Join-Path $work $Pose.Rel
    if ((Test-Path $path) -and (Test-PoseGate $Pose)) {
        Write-BaGate $path @{
            role      = $Pose.Role
            seed      = $Pose.Seed0
            seed0     = $Pose.Seed0
            prompt    = $Pose.Prompt
            startPose = $(if ($Pose.StartRel) { "$($Pose.StartRel)@last" } else { "idle-soma.glb@last" })
            model     = "v1.1"
            steps     = 25
            guidance  = 4
            plant     = "debug/swing_estimates/01_forward.json"
            kind      = "pose"
            reused    = $true
        }
        Write-Host "POSE_GATE_REUSE_OK $($Pose.Rel)"
        return
    }
    $start = $idle
    if ($Pose.StartRel) { $start = Join-Path $work $Pose.StartRel }
    for ($i = 0; $i -lt 5; $i++) {
        $seed = $Pose.Seed0 + $i
        Remove-Item -Force -ErrorAction SilentlyContinue $path, "$path.json"
        Invoke-Tide (@(
                "generate", $Pose.Prompt,
                "--duration", "1.2", "--seed", "$seed", "--guidance", "4",
                "--start-pose", "$start@last",
                "-o", $path
            ) + $gpu)
        if (Test-PoseGate $Pose) {
            Write-BaGate $path @{
                role      = $Pose.Role
                seed      = $seed
                seed0     = $Pose.Seed0
                prompt    = $Pose.Prompt
                startPose = $(if ($Pose.StartRel) { "$($Pose.StartRel)@last" } else { "idle-soma.glb@last" })
                model     = "v1.1"
                steps     = 25
                guidance  = 4
                plant     = "debug/swing_estimates/01_forward.json"
                kind      = "pose"
            }
            Write-Host "POSE_GATE_OK $($Pose.Rel) seed=$seed"
            return
        }
        Write-Host "POSE_GATE_FAIL $($Pose.Rel) seed=$seed — retry"
    }
    throw "Pose $($Pose.Rel) missed the 2H gate after 5 seeds"
}

if ($Phase -eq "poses" -or $Phase -eq "all") {
    foreach ($pose in $Poses) { New-GatedPose $pose }
    Write-Host "POSE_AUTHOR_DONE"
}

if ($Phase -eq "poses") { return }

foreach ($pose in $Poses) {
    $path = Join-Path $work $pose.Rel
    if (!(Test-Path $path)) { throw "Missing pose $path — run with -Phase poses first" }
    if (!(Test-PoseGate $pose)) { throw "Pose $($pose.Rel) failed the 2H gate." }
    Write-Host "POSE_GATE_RECHECK_OK $($pose.Rel)"
}

$wind = Join-Path $work "pose-2h-wind-v1-soma.glb"
$plant = Join-Path $work "pose-2h-plant-v1-soma.glb"
$follow = Join-Path $work "pose-2h-follow-left-v1-soma.glb"
$slashPrompt = "A standing person chops both arms forward from the right shoulder so both hands pass straight out in front of the chest, then swings them across to the left, looking forward, both feet planted, in place."

$path = Join-Path $work "slash-2h-v2-soma.glb"
$reused = (Test-Path $path) -and (Invoke-PyOk @($slashGatePy, $path))
if ($reused) {
    Write-Host "SLASH_GATE_REUSE_OK slash-2h-v2-soma.glb"
} else {
    $ok = $false
    for ($i = 0; $i -lt 3; $i++) {
        $seed = 1851 + $i
        Remove-Item -Force -ErrorAction SilentlyContinue $path, "$path.json"
        Invoke-Tide (@(
                "generate", $slashPrompt,
                "--duration", "0.5",
                "--seed", "$seed",
                "--guidance", "4",
                "--constraint-guidance", "2",
                "--start-pose", "$wind@last",
                "--keyframe", "8", "$plant@last",
                "--end-pose", "$follow@last",
                "--correction-frames", "4",
                "-o", $path
            ) + $gpu)
        if (Invoke-PyOk @($slashGatePy, $path)) {
            Write-BaGate $path @{
                role               = "2h-slash"
                seed               = $seed
                seed0              = 1851
                prompt             = $slashPrompt
                wind               = "pose-2h-wind-v1-soma.glb"
                plant              = "pose-2h-plant-v1-soma.glb"
                follow             = "pose-2h-follow-left-v1-soma.glb"
                saved              = "debug/swing_estimates/01_forward.json"
                model              = "v1.1"
                steps              = 25
                duration           = 0.5
                guidance           = 4
                constraintGuidance = 2
                kind               = "slash"
            }
            Write-Host "SLASH_GATE_OK slash-2h-v2-soma.glb seed=$seed"
            $ok = $true
            break
        }
        Write-Host "SLASH_GATE_FAIL slash-2h-v2-soma.glb seed=$seed — retry"
    }
    if (-not $ok) { throw "Slash slash-2h-v2-soma.glb missed the wind-through-plant-to-left gate after 3 seeds" }
}

$walk = Join-Path $outDir "slash_2h.glb"
Remove-Item -Force -ErrorAction SilentlyContinue $walk, "$walk.json"
Write-Host "Retargeting slash-2h-v1-soma.glb -> slash_2h.glb"
Invoke-Tide @("retarget", $path, "--target", $target, "--map", $map, "-o", $walk)
Write-Host "SLASH_2H_AUTHOR_DONE"
