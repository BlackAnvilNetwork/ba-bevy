$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$queue = Join-Path $root "debug\inbox\queue.txt"
$statePath = Join-Path $root ".dream-loop\state.json"
New-Item -ItemType Directory -Force -Path (Join-Path $root ".dream-loop") | Out-Null

function Get-QueueTail {
    if (-not (Test-Path $queue)) { return "" }
    Get-Content $queue -Tail 1
}

$last = Get-QueueTail
Write-Output "AGENT_LOOP_WATCH_debug-dream armed last=$last"
while ($true) {
    Start-Sleep -Seconds 2
    $now = Get-QueueTail
    if ($now -and $now -ne $last) {
        $last = $now
        $stamp = Split-Path $now -Leaf
        Write-Output "AGENT_LOOP_WAKE_debug-dream {`"prompt`":`"New debugger note $stamp. Read debug/inbox/$stamp (note.txt, shot.json, frame.png). Dream-loop critic vs .dream-loop/target.png (lock/refine target from the live frame if needed). Prefer D:\\ba-darkmod\\gltf\\models for props. Implement, then wait for the next shot.`"}"
    }
}
