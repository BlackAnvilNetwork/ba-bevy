# Offline Tide clips -> Anata exported skeleton -> assets/character/motion/*.glb
# Sequential generate (not batch): batch keeps the text encoder resident and OOMs
# the GTX 1070. Single generate --device gpu matches the walk clip path.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$tm = Join-Path $env:LOCALAPPDATA "TideMotion\bin\tide-motion.exe"
if (!(Test-Path $tm)) { throw "Tide Motion is not installed. Expected $tm" }

$work = Join-Path $root ".dream-loop\motion"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$outDir = Join-Path $root "assets\character\motion"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$map = Join-Path $outDir "anata-exported.json"

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

function Ensure-Generate {
    param($Rel, [string[]]$Cli)
    $path = Join-Path $work $Rel
    if (Test-Path $path) {
        Write-Host "Reusing $path"
        return
    }
    Invoke-Tide ($Cli + @("-o", $path))
}

$gpu = @("--model", "v1.1", "--steps", "25", "--in-place", "--device", "gpu")
# Travelling sources for gait QC: --in-place zeroes soma-travelling-footfalls, so
# --footsteps 2 never matches on an in-place clip.
$gpuTravel = @("--model", "v1.1", "--steps", "25", "--device", "gpu")
Ensure-Generate "idle-soma.glb" (@("generate", "A person stands still with arms relaxed at their sides.", "--duration", "2.5", "--seed", "23") + $gpu)
Ensure-Generate "rest-soma.glb" (@(
        "generate", "A person kneels on both knees and sits still with hands in their lap.",
        "--duration", "2.5", "--seed", "41", "--guidance", "3"
    ) + $gpu)
# Debugger 20776-070915: both-knees rest folded into a deep bow. Sit with one
# knee up and the other leg straight, torso upright.
Ensure-Generate "rest-sit-soma.glb" (@(
        "generate", "A person sits on the ground with the right knee up and the left leg stretched straight, back upright.",
        "--duration", "2.5", "--seed", "91", "--guidance", "4"
    ) + $gpu)
$restSit = Join-Path $work "rest-sit-soma.glb"
# @50 of the sit-down was still reclined (lookY +0.35). Frame 73 is seated,
# looking forward, torso up. Debugger 20776-074528: rest should be more idle.
Ensure-Generate "rest-sit-cycle-soma.glb" (@(
        "generate", "A person sits still on the ground in a relaxed idle rest, right knee up, left leg stretched straight, back upright, looking forward, breathing quietly, not leaning back.",
        "--duration", "1.8", "--seed", "93", "--guidance", "4",
        "--start-pose", "$restSit@73",
        "--end-pose", "$restSit@73"
    ) + $gpu)
$walkSoma = Join-Path $work "walk-soma.glb"
# --footsteps N is an exact count. A 2.5s walk produces 4–7 landings, so
# --footsteps 2 rejects real gait. Keep the travelling source and loop from it.
Ensure-Generate "walk-travel-soma.glb" (@(
        "generate", "A person walks forward at a constant speed.",
        "--duration", "2.5", "--seed", "81", "--guidance", "3"
    ) + $gpuTravel)
$walkTravel = Join-Path $work "walk-travel-soma.glb"
# Same seed as walk-travel, with --in-place. Identical start=end pose cycles
# collapsed the stride (~1° legs). The travelling clip has 5 alternating
# L/R footfalls; stripping root travel keeps that gait on the Bevy actor.
Ensure-Generate "walk-cycle-v5-soma.glb" (@(
        "generate", "A person walks forward at a constant speed.",
        "--duration", "2.5", "--seed", "81", "--guidance", "3"
    ) + $gpu)
$runSoma = Join-Path $work "run-soma.glb"
Ensure-Generate "run-soma.glb" (@("generate", "A person runs forward.", "--duration", "2.0", "--seed", "11") + $gpu)
Ensure-Generate "run-travel-soma.glb" (@(
        "generate", "A person runs forward at a constant speed.",
        "--duration", "2.0", "--seed", "83", "--guidance", "3"
    ) + $gpuTravel)
$runTravel = Join-Path $work "run-travel-soma.glb"
Ensure-Generate "run-cycle-v5-soma.glb" (@(
        "generate", "A person runs forward at a constant speed.",
        "--duration", "2.0", "--seed", "83", "--guidance", "3"
    ) + $gpu)
$walkCyclePose = Join-Path $work "walk-cycle-v5-soma.glb"
# Last pass planted the free foot mid-air. Start from a right-foot stride plant.
Ensure-Generate "jump-soma.glb" (@(
        "generate", "From a walking stride the right foot plants and the person springs straight up off that one foot, left knee driving up, body rising, no pause.",
        "--duration", "0.40",
        "--then", "The person hangs in the air at the peak of a jump, neither foot touching the ground, left knee tucked.",
        "--duration", "0.40",
        "--then", "The person drops from the jump and lands on both feet.",
        "--duration", "0.35",
        "--transition", "4", "--seed", "147", "--guidance", "5",
        "--start-pose", "$walkCyclePose@14"
    ) + $gpu)
Ensure-Generate "crouch-soma.glb" (@("generate", "A person crouches down low to the ground and stays there.", "--duration", "2.0", "--seed", "13") + $gpu)
$crouchSoma = Join-Path $work "crouch-soma.glb"
Ensure-Generate "crouch-cycle-soma.glb" (@(
        "generate", "A person stays crouched low to the ground.",
        "--duration", "1.5", "--seed", "43", "--guidance", "3",
        "--start-pose", "$crouchSoma@30",
        "--end-pose", "$crouchSoma@30"
    ) + $gpu)
Ensure-Generate "crouch-walk-soma.glb" (@("generate", "A person walks forward while crouched.", "--duration", "2.5", "--seed", "14") + $gpu)
$crouchWalkSoma = Join-Path $work "crouch-walk-soma.glb"
Ensure-Generate "crouch-walk-cycle-soma.glb" (@(
        "generate", "A person walks forward while crouched, at a constant speed without starting or stopping.",
        "--duration", "1.3", "--seed", "44", "--guidance", "3",
        "--start-pose", "$crouchWalkSoma@30",
        "--end-pose", "$crouchWalkSoma@30"
    ) + $gpu)
Ensure-Generate "walk-back-soma.glb" (@("generate", "A person walks backward at a constant speed.", "--duration", "2.0", "--seed", "51", "--guidance", "3") + $gpu)
$walkBackSoma = Join-Path $work "walk-back-soma.glb"
# @20 of the raw clip was a near-still pose and the first cycle collapsed.
Ensure-Generate "walk-back-cycle-v2-soma.glb" (@(
        "generate", "A person walks backward, stepping one foot then the other, facing forward, at a constant speed without starting or stopping.",
        "--duration", "1.1", "--seed", "56", "--guidance", "3",
        "--start-pose", "$walkBackSoma@35",
        "--end-pose", "$walkBackSoma@35"
    ) + $gpu)
Ensure-Generate "strafe-left-soma.glb" (@(
        "generate", "A person sidesteps to the left, facing forward, at a constant speed.",
        "--duration", "2.0", "--seed", "53", "--guidance", "3"
    ) + $gpu)
$strafeLeftSoma = Join-Path $work "strafe-left-soma.glb"
Ensure-Generate "strafe-left-cycle-soma.glb" (@(
        "generate", "A person sidesteps to the left, facing forward, at a constant speed.",
        "--duration", "1.1", "--seed", "54", "--guidance", "3",
        "--start-pose", "$strafeLeftSoma@20",
        "--end-pose", "$strafeLeftSoma@20"
    ) + $gpu)
# 1H cuts live in ba-author-slash.ps1. Do not generate slash-1h / walk-slash here.
Ensure-Generate "punch-soma.glb" (@(
        "generate", "A person throws a right jab.", "--duration", "0.8",
        "--then", "A person throws a left cross.", "--duration", "0.9",
        "--then", "A person throws a right hook.", "--duration", "1.0",
        "--transition", "6", "--seed", "15", "--guidance", "3"
    ) + $gpu)
Ensure-Generate "slash-2h-soma.glb" (@(
        "generate", "A person raises both arms overhead.", "--duration", "0.8",
        "--then", "A person chops both arms down in front.", "--duration", "1.1",
        "--transition", "6", "--seed", "17", "--guidance", "3"
    ) + $gpu)
Ensure-Generate "roll-soma.glb" (@("generate", "A person rolls forward on the ground then stands up.", "--duration", "1.6", "--seed", "18", "--guidance", "3") + $gpu)
$walkPose = Join-Path $work "walk-soma.glb"
Ensure-Generate "walk-punch-soma.glb" (@(
        "generate", "A person walks forward.", "--duration", "0.5",
        "--then", "A person throws a right jab while walking.", "--duration", "0.8",
        "--then", "A person throws a left cross while walking.", "--duration", "0.9",
        "--then", "A person throws a right hook while walking.", "--duration", "1.0",
        "--transition", "6", "--seed", "21", "--guidance", "3",
        "--start-pose", $walkPose
    ) + $gpu)
Ensure-Generate "walk-jump-soma.glb" (@(
        "generate", "A person walks forward.", "--duration", "0.6",
        "--then", "A person jumps up with both feet.", "--duration", "1.2",
        "--transition", "8", "--seed", "22", "--guidance", "3",
        "--start-pose", $walkPose
    ) + $gpu)

function File-Sha {
    param($Path)
    if (!(Test-Path $Path)) { return "missing" }
    (Get-FileHash -Algorithm SHA256 -Path $Path).Hash
}

function Stamp-Matches {
    param($Stamp, $Key)
    (Test-Path $Stamp) -and ((Get-Content -Raw $Stamp).Trim() -eq $Key)
}

# MotionBricks connect. Stamp on source hashes + token settings so retouching
# clip A only rebuilds A’s edges. Default 8–14 (~1.1 s) for settling; --tokens 6
# (~0.57 s) for small gait edges. Do not connect idle→walk/strafe/jump: those
# start the destination clip with a short Bevy blend.
function Ensure-Connect {
    param($From, $To, $Out, [string]$Tokens)
    $fromPath = Join-Path $work $From
    $toPath = Join-Path $work $To
    $path = Join-Path $work $Out
    $recipe = "$path.json"
    $stamp = "$path.connect-key"
    $tokenKey = if ($Tokens) { $Tokens } else { "default" }
    $key = "from=$(File-Sha $fromPath) to=$(File-Sha $toPath) tokens=$tokenKey"
    if ((Test-Path $path) -and (Test-Path $recipe) -and (Stamp-Matches $stamp $key)) {
        Write-Host "Reusing $path"
        return
    }
    Remove-Item -Force -ErrorAction SilentlyContinue $path, $recipe, $stamp
    $cli = @("connect", $fromPath, $toPath, "-o", $path)
    if ($Tokens) { $cli += @("--tokens", $Tokens) }
    Invoke-Tide $cli
    Set-Content -NoNewline -Path $stamp -Value $key
}

# Short gait edges: walk↔run and crouch↔crouch-walk. Controller speed is
# instant (3.5 walk / 6.5 sprint), so a 1.1 s bridge slides; 6 tokens ~0.57 s.
Ensure-Connect "walk-cycle-v5-soma.glb" "run-cycle-v5-soma.glb" "walk-to-run-soma.glb" "6"
Ensure-Connect "run-cycle-v5-soma.glb" "walk-cycle-v5-soma.glb" "run-to-walk-soma.glb" "6"
Ensure-Connect "crouch-cycle-soma.glb" "crouch-walk-cycle-soma.glb" "crouch-to-crouch-walk-soma.glb" "6"
Ensure-Connect "crouch-walk-cycle-soma.glb" "crouch-cycle-soma.glb" "crouch-walk-to-crouch-soma.glb" "6"

# Settling: slowing down or sitting/crouching can take a moment (~1.1 s).
Ensure-Connect "walk-cycle-v5-soma.glb" "idle-soma.glb" "walk-to-idle-soma.glb"
Ensure-Connect "idle-soma.glb" "rest-sit-cycle-soma.glb" "idle-to-rest-soma.glb"
Ensure-Connect "rest-sit-cycle-soma.glb" "idle-soma.glb" "rest-to-idle-soma.glb"
Ensure-Connect "idle-soma.glb" "crouch-cycle-soma.glb" "idle-to-crouch-soma.glb"
Ensure-Connect "crouch-cycle-soma.glb" "idle-soma.glb" "crouch-to-idle-soma.glb"
Ensure-Connect "walk-back-cycle-v2-soma.glb" "idle-soma.glb" "walk-back-to-idle-soma.glb"
Ensure-Connect "strafe-left-cycle-soma.glb" "idle-soma.glb" "strafe-left-to-idle-soma.glb"
$strafeRight = Join-Path $work "strafe-right-cycle-soma.glb"
if (Test-Path $strafeRight) {
    Write-Host "Reusing $strafeRight"
} else {
    Invoke-Tide @("mirror", (Join-Path $work "strafe-left-cycle-soma.glb"), "-o", $strafeRight)
}
$strafeRightToIdle = Join-Path $work "strafe-right-to-idle-soma.glb"
$strafeRightToIdleStamp = "$strafeRightToIdle.connect-key"
$strafeRightToIdleKey = "mirror-of=$(File-Sha (Join-Path $work 'strafe-left-to-idle-soma.glb'))"
if ((Test-Path $strafeRightToIdle) -and (Stamp-Matches $strafeRightToIdleStamp $strafeRightToIdleKey)) {
    Write-Host "Reusing $strafeRightToIdle"
} else {
    Remove-Item -Force -ErrorAction SilentlyContinue $strafeRightToIdle, "$strafeRightToIdle.json", $strafeRightToIdleStamp
    Invoke-Tide @("mirror", (Join-Path $work "strafe-left-to-idle-soma.glb"), "-o", $strafeRightToIdle)
    Set-Content -NoNewline -Path $strafeRightToIdleStamp -Value $strafeRightToIdleKey
}
Ensure-Connect "jump-soma.glb" "walk-cycle-v5-soma.glb" "jump-to-walk-soma.glb"
Ensure-Connect "jump-soma.glb" "run-cycle-v5-soma.glb" "jump-to-run-soma.glb"

$pairs = @(
    @{ Soma = "idle-soma.glb"; Out = "idle.glb" },
    @{ Soma = "rest-sit-cycle-soma.glb"; Out = "rest.glb" },
    @{ Soma = "walk-cycle-v5-soma.glb"; Out = "walk.glb" },
    @{ Soma = "run-cycle-v5-soma.glb"; Out = "run.glb" },
    @{ Soma = "jump-soma.glb"; Out = "jump.glb" },
    @{ Soma = "crouch-cycle-soma.glb"; Out = "crouch.glb" },
    @{ Soma = "crouch-walk-cycle-soma.glb"; Out = "crouch_walk.glb" },
    @{ Soma = "punch-soma.glb"; Out = "punch.glb" },
    @{ Soma = "slash-2h-soma.glb"; Out = "slash_2h.glb" },
    @{ Soma = "roll-soma.glb"; Out = "roll.glb" },
    @{ Soma = "walk-to-run-soma.glb"; Out = "walk_to_run.glb" },
    @{ Soma = "run-to-walk-soma.glb"; Out = "run_to_walk.glb" },
    @{ Soma = "walk-to-jump-soma.glb"; Out = "walk_to_jump.glb" },
    @{ Soma = "run-to-jump-soma.glb"; Out = "run_to_jump.glb" },
    @{ Soma = "jump-to-walk-soma.glb"; Out = "jump_to_walk.glb" },
    @{ Soma = "jump-to-run-soma.glb"; Out = "jump_to_run.glb" },
    @{ Soma = "walk-punch-soma.glb"; Out = "walk_punch.glb" },
    @{ Soma = "walk-jump-soma.glb"; Out = "walk_jump.glb" },
    @{ Soma = "idle-to-walk-soma.glb"; Out = "idle_to_walk.glb" },
    @{ Soma = "walk-to-idle-soma.glb"; Out = "walk_to_idle.glb" },
    @{ Soma = "idle-to-rest-soma.glb"; Out = "idle_to_rest.glb" },
    @{ Soma = "rest-to-idle-soma.glb"; Out = "rest_to_idle.glb" },
    @{ Soma = "walk-back-cycle-v2-soma.glb"; Out = "walk_back.glb" },
    @{ Soma = "strafe-left-cycle-soma.glb"; Out = "strafe_left.glb" },
    @{ Soma = "strafe-right-cycle-soma.glb"; Out = "strafe_right.glb" },
    @{ Soma = "idle-to-crouch-soma.glb"; Out = "idle_to_crouch.glb" },
    @{ Soma = "crouch-to-idle-soma.glb"; Out = "crouch_to_idle.glb" },
    @{ Soma = "crouch-to-crouch-walk-soma.glb"; Out = "crouch_to_crouch_walk.glb" },
    @{ Soma = "crouch-walk-to-crouch-soma.glb"; Out = "crouch_walk_to_crouch.glb" },
    @{ Soma = "idle-to-walk-back-soma.glb"; Out = "idle_to_walk_back.glb" },
    @{ Soma = "walk-back-to-idle-soma.glb"; Out = "walk_back_to_idle.glb" },
    @{ Soma = "idle-to-strafe-left-soma.glb"; Out = "idle_to_strafe_left.glb" },
    @{ Soma = "strafe-left-to-idle-soma.glb"; Out = "strafe_left_to_idle.glb" },
    @{ Soma = "idle-to-strafe-right-soma.glb"; Out = "idle_to_strafe_right.glb" },
    @{ Soma = "strafe-right-to-idle-soma.glb"; Out = "strafe_right_to_idle.glb" },
)
foreach ($pair in $pairs) {
    $soma = Join-Path $work $pair.Soma
    $walk = Join-Path $outDir $pair.Out
    $recipe = "$walk.json"
    $stamp = Join-Path $work "$($pair.Out).retarget-key"
    if (!(Test-Path $soma)) {
        Write-Host "Skip retarget $($pair.Out) — missing $($pair.Soma)"
        continue
    }
    $key = "soma=$(File-Sha $soma) target=$(File-Sha $target) map=$(File-Sha $map)"
    if ((Test-Path $walk) -and (Test-Path $recipe) -and (Stamp-Matches $stamp $key)) {
        Write-Host "Reusing $walk"
        continue
    }
    Remove-Item -Force -ErrorAction SilentlyContinue $walk, $recipe, $stamp
    Write-Host "Retargeting $($pair.Soma) -> $($pair.Out)"
    Invoke-Tide @("retarget", $soma, "--target", $target, "--map", $map, "-o", $walk)
    Set-Content -NoNewline -Path $stamp -Value $key
}
Write-Host "Authored motion clips in $outDir"
