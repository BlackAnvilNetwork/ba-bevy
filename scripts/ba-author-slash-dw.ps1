# Dual-wield chops: high chamber (forearm up, tip up) then a diagonal cut to the hip.
# Same cut on each hand. The other hand stays at hip-carry so combo % 2 recovers
# while it cuts. slash_1h stays locked. Sequential GPU (GTX 1070). Body prompts only.
param(
    [ValidateSet("poses", "slashes", "all")]
    [string]$Phase = "all"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$tm = Join-Path $env:LOCALAPPDATA "TideMotion\bin\tide-motion.exe"
if (!(Test-Path $tm)) { throw "Tide Motion is not installed. Expected $tm" }

$work = Join-Path $root ".dream-loop\motion"
$outDir = Join-Path $root "assets\character\motion"
$map = Join-Path $outDir "anata-exported.json"
$lockDir = Join-Path $root "archive\sword-slash\locked-v13"
New-Item -ItemType Directory -Force -Path $work, $outDir, $lockDir | Out-Null
$idle = Join-Path $work "idle-soma.glb"
if (!(Test-Path $idle)) { throw "Missing $idle — run ba-author-motion.ps1 first" }
$gatePy = Join-Path $root ".dream-loop\qa\pose_gate.py"
$armGatePy = Join-Path $root ".dream-loop\qa\arm_pose_gate.py"
$dwGatePy = Join-Path $root ".dream-loop\qa\dw_slash_gate.py"
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

foreach ($name in @("slash_1h.glb", "slash_1h_return.glb", "slash_1h_sweep.glb")) {
    $src = Join-Path $outDir $name
    if (Test-Path $src) {
        Copy-Item -Force $src (Join-Path $lockDir $name)
        if (Test-Path "$src.json") {
            Copy-Item -Force "$src.json" (Join-Path $lockDir "$name.json")
        }
    }
}
Write-Host "SLASH_1H_LOCKED copies in $lockDir"

$carry = Join-Path $work "hip-carry-v2-soma.glb"
if (!(Test-Path $carry)) { throw "Missing $carry — locked 1H hip-carry is required" }

$RightPoses = @(
    @{
        Rel          = "pose-dw-high-wind-v7-soma.glb"
        Prompt       = "A standing person raises the right elbow and right hand upward beside the right side of the head, the right hand above the elbow, looking forward, both feet planted, in place."
        Seed0        = 1650
        Xmin         = -0.32; Xmax = -0.10
        Ymin         = 1.40;  Ymax = 1.74
        Zmin         = 0.00;  Zmax = 0.45
        ElbowMin     = 55; ElbowMax = 115
        ForearmYMin  = 0.42
        ElbowXmin    = -0.30; ElbowXmax = -0.04
        ElbowYmin    = 1.18
        HandAbove    = 0.03
        Role         = "dw-high-wind"
    }
    @{
        Rel         = "pose-dw-chop-hit-v7-soma.glb"
        Prompt      = "A standing person holds the right hand in front of the chest, just left of center, looking forward, both feet planted, in place."
        Seed0       = 1610
        Xmin        = -0.05; Xmax = 0.24
        Ymin        = 1.05;  Ymax = 1.32
        Zmin        = 0.28;  Zmax = 0.62
        ElbowMin    = 75; ElbowMax = 165
        ForearmYMax = 0.15
        Role        = "dw-chop-hit"
    }
    @{
        Rel      = "pose-dw-chop-follow-v7-soma.glb"
        LockRel  = "pose-diag-follow-v11-soma.glb"
        Prompt   = "A standing person reaches the right hand forward and down by the left hip, looking forward, both feet planted, in place."
        Seed0    = 1620
        Xmin     = 0.04;  Xmax = 0.30
        Ymin     = 0.82;  Ymax = 1.12
        Zmin     = 0.26;  Zmax = 0.58
        Role     = "dw-chop-follow"
    }
)

$LeftPoses = @(
    @{
        Rel          = "pose-dw-high-wind-left-v7-soma.glb"
        Src          = "pose-dw-high-wind-v7-soma.glb"
        Xmin         = 0.10; Xmax = 0.32
        Ymin         = 1.40; Ymax = 1.74
        Zmin         = 0.00; Zmax = 0.45
        ElbowMin     = 55; ElbowMax = 115
        ForearmYMin  = 0.42
        ElbowXmin    = 0.04; ElbowXmax = 0.30
        ElbowYmin    = 1.18
        HandAbove    = 0.03
        Role         = "dw-high-wind-left"
    }
    @{
        Rel         = "pose-dw-chop-hit-left-v7-soma.glb"
        Src         = "pose-dw-chop-hit-v7-soma.glb"
        Xmin        = -0.24; Xmax = 0.05
        Ymin        = 1.05;  Ymax = 1.32
        Zmin        = 0.28;  Zmax = 0.62
        ElbowMin    = 75; ElbowMax = 165
        ForearmYMax = 0.15
        Role        = "dw-chop-hit-left"
    }
    @{
        Rel  = "pose-dw-chop-follow-left-v7-soma.glb"
        Src  = "pose-dw-chop-follow-v7-soma.glb"
        Xmin = -0.32; Xmax = -0.04
        Ymin = 0.82;  Ymax = 1.12
        Zmin = 0.26;  Zmax = 0.58
        Role = "dw-chop-follow-left"
    }
)

function Add-ArmGateArgs {
    param($Pose, [System.Collections.Generic.List[string]]$Cli)
    if ($null -ne $Pose.ForearmYMin) { [void]$Cli.Add("--forearm-y-min"); [void]$Cli.Add("$($Pose.ForearmYMin)") }
    if ($null -ne $Pose.ForearmYMax) { [void]$Cli.Add("--forearm-y-max"); [void]$Cli.Add("$($Pose.ForearmYMax)") }
    if ($null -ne $Pose.ElbowXmin) { [void]$Cli.Add("--elbow-x-min"); [void]$Cli.Add("$($Pose.ElbowXmin)") }
    if ($null -ne $Pose.ElbowXmax) { [void]$Cli.Add("--elbow-x-max"); [void]$Cli.Add("$($Pose.ElbowXmax)") }
    if ($null -ne $Pose.ElbowYmin) { [void]$Cli.Add("--elbow-y-min"); [void]$Cli.Add("$($Pose.ElbowYmin)") }
    if ($null -ne $Pose.HandAbove) { [void]$Cli.Add("--hand-above"); [void]$Cli.Add("$($Pose.HandAbove)") }
}

function Test-PoseGate {
    param($Pose, [switch]$Left)
    $path = Join-Path $work $Pose.Rel
    $handCli = [System.Collections.Generic.List[string]]@(
        $gatePy, $path,
        "$($Pose.Xmin)", "$($Pose.Xmax)",
        "$($Pose.Ymin)", "$($Pose.Ymax)",
        "$($Pose.Zmin)", "$($Pose.Zmax)"
    )
    if ($Left) { $handCli.Add("--left") }
    if (-not (Invoke-PyOk $handCli)) { return $false }
    if ($null -ne $Pose.ElbowMin) {
        $elbowCli = [System.Collections.Generic.List[string]]@(
            $elbowGatePy, $path, "$($Pose.ElbowMin)", "$($Pose.ElbowMax)"
        )
        if ($Left) { $elbowCli.Add("--left") }
        if (-not (Invoke-PyOk $elbowCli)) { return $false }
    }
    $armCli = [System.Collections.Generic.List[string]]@($armGatePy, $path)
    if ($Left) { $armCli.Add("--left") }
    Add-ArmGateArgs $Pose $armCli
    if ($armCli.Count -le 2 -and -not $Left) { return $true }
    if ($armCli.Count -le 3 -and $Left) { return $true }
    return (Invoke-PyOk $armCli)
}

function New-GatedPose {
    param($Pose)
    $path = Join-Path $work $Pose.Rel
    if ($Pose.LockRel) {
        $lock = Join-Path $work $Pose.LockRel
        if (Test-Path $lock) {
            Copy-Item -Force $lock $path
            if (Test-Path "$lock.json") { Copy-Item -Force "$lock.json" "$path.json" }
            if (Test-PoseGate $Pose) {
                Write-BaGate $path @{
                    role     = $Pose.Role
                    lockRel  = $Pose.LockRel
                    kind     = "pose"
                    reused   = $true
                }
                Write-Host "POSE_GATE_LOCK_OK $($Pose.Rel) from $($Pose.LockRel)"
                return
            }
        }
    }
    if ((Test-Path $path) -and (Test-PoseGate $Pose)) {
        Write-BaGate $path @{
            role      = $Pose.Role
            seed      = $Pose.Seed0
            prompt    = $Pose.Prompt
            startPose = $(if ($Pose.StartRel) { "$($Pose.StartRel)@last" } else { "hip-carry-v2-soma.glb@last" })
            kind      = "pose"
            reused    = $true
        }
        Write-Host "POSE_GATE_REUSE_OK $($Pose.Rel)"
        return
    }
    $startPose = if ($Pose.StartRel) { Join-Path $work $Pose.StartRel } else { $carry }
    for ($i = 0; $i -lt 5; $i++) {
        $seed = $Pose.Seed0 + $i
        Remove-Item -Force -ErrorAction SilentlyContinue $path, "$path.json"
        Invoke-Tide (@(
                "generate", $Pose.Prompt,
                "--duration", "1.2", "--seed", "$seed", "--guidance", "4",
                "--start-pose", "$startPose@last",
                "-o", $path
            ) + $gpu)
        if (Test-PoseGate $Pose) {
            Write-BaGate $path @{
                role      = $Pose.Role
                seed      = $seed
                seed0     = $Pose.Seed0
                prompt    = $Pose.Prompt
                startPose = $(if ($Pose.StartRel) { "$($Pose.StartRel)@last" } else { "hip-carry-v2-soma.glb@last" })
                kind      = "pose"
            }
            Write-Host "POSE_GATE_OK $($Pose.Rel) seed=$seed"
            return
        }
        Write-Host "POSE_GATE_FAIL $($Pose.Rel) seed=$seed — retry"
    }
    throw "Pose $($Pose.Rel) missed the high-chamber gate after 5 seeds"
}

function New-MirroredPose {
    param($Pose)
    $path = Join-Path $work $Pose.Rel
    $src = Join-Path $work $Pose.Src
    if ((Test-Path $path) -and (Test-PoseGate $Pose -Left)) {
        Write-BaGate $path @{
            role         = $Pose.Role
            mirroredFrom = $Pose.Src
            kind         = "pose"
            reused       = $true
        }
        Write-Host "POSE_GATE_REUSE_OK $($Pose.Rel)"
        return
    }
    Remove-Item -Force -ErrorAction SilentlyContinue $path, "$path.json"
    Invoke-Tide @("mirror", $src, "-o", $path)
    if (-not (Test-PoseGate $Pose -Left)) {
        throw "Mirrored pose $($Pose.Rel) missed the left-hand gate"
    }
    Write-BaGate $path @{
        role         = $Pose.Role
        mirroredFrom = $Pose.Src
        kind         = "pose"
    }
    Write-Host "POSE_GATE_OK $($Pose.Rel)"
}

if ($Phase -eq "poses" -or $Phase -eq "all") {
    foreach ($pose in $RightPoses) { New-GatedPose $pose }
    foreach ($pose in $LeftPoses) { New-MirroredPose $pose }
    Write-Host "POSE_AUTHOR_DONE"
}

if ($Phase -eq "poses") { return }

foreach ($pose in $RightPoses) {
    if (-not (Test-PoseGate $pose)) { throw "Pose $($pose.Rel) failed the high-chamber gate." }
    Write-Host "POSE_GATE_RECHECK_OK $($pose.Rel)"
}
foreach ($pose in $LeftPoses) {
    if (-not (Test-PoseGate $pose -Left)) { throw "Pose $($pose.Rel) failed the left-hand gate." }
    Write-Host "POSE_GATE_RECHECK_OK $($pose.Rel)"
}

function New-GatedChop {
    param($Rel, $Prompt, $Wind, $Hit, $Follow, $Seed0, $Role, [switch]$Left)
    $path = Join-Path $work $Rel
    $gateCli = @($dwGatePy, $path)
    if ($Left) { $gateCli += "--left" }
    if ((Test-Path $path) -and (Invoke-PyOk $gateCli)) {
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
        if (Invoke-PyOk $gateCli) {
            Write-BaGate $path @{
                role               = $Role
                seed               = $seed
                seed0              = $Seed0
                prompt             = $Prompt
                wind               = [IO.Path]::GetFileName($Wind)
                hit                = [IO.Path]::GetFileName($Hit)
                follow             = [IO.Path]::GetFileName($Follow)
                model              = "v1.1"
                steps              = 25
                duration           = 0.5
                guidance           = 4
                constraintGuidance = 2
                left               = [bool]$Left
                gate               = "dw_slash_gate"
                kind               = "slash"
            }
            Write-Host "SLASH_GATE_OK $Rel seed=$seed"
            return
        }
        Write-Host "SLASH_GATE_FAIL $Rel seed=$seed — retry"
    }
    throw "Chop $Rel missed the high-chamber chop gate after 3 seeds"
}

$rightWind = Join-Path $work "pose-dw-high-wind-v7-soma.glb"
$rightHit = Join-Path $work "pose-dw-chop-hit-v7-soma.glb"
$rightFollow = Join-Path $work "pose-dw-chop-follow-v7-soma.glb"
$leftWind = Join-Path $work "pose-dw-high-wind-left-v7-soma.glb"
$leftHit = Join-Path $work "pose-dw-chop-hit-left-v7-soma.glb"
$leftFollow = Join-Path $work "pose-dw-chop-follow-left-v7-soma.glb"

New-GatedChop "slash-dw-right-v7-soma.glb" `
    "A standing person chops down and across the body from high on the right toward the left hip, looking forward, in place." `
    $rightWind $rightHit $rightFollow 1630 "dw-chop-right"
New-GatedChop "slash-dw-left-v7-soma.glb" `
    "A standing person chops down and across the body from high on the left toward the right hip, looking forward, in place." `
    $leftWind $leftHit $leftFollow 1640 "dw-chop-left" -Left

$rightSoma = Join-Path $work "slash-dw-right-v7-soma.glb"
$leftSoma = Join-Path $work "slash-dw-left-v7-soma.glb"
$dwRight = Join-Path $outDir "slash_dw.glb"
$dwLeft = Join-Path $outDir "slash_dw_left.glb"
Remove-Item -Force -ErrorAction SilentlyContinue $dwRight, "$dwRight.json", $dwLeft, "$dwLeft.json"
Invoke-Tide @("retarget", $rightSoma, "--target", $target, "--map", $map, "-o", $dwRight)
Invoke-Tide @("retarget", $leftSoma, "--target", $target, "--map", $map, "-o", $dwLeft)
Write-Host "SLASH_DW_AUTHOR_DONE"
