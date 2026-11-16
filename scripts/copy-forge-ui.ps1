$ErrorActionPreference = "Stop"
$src = "C:\Users\raxac\OneDrive\Desktop\apps\ba-stick\apps\stick-web\public\ui"
$dst = "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil\assets\ui"
New-Item -ItemType Directory -Force -Path "$dst\boot", "$dst\black-anvil\slices" | Out-Null

$files = @(
  @{ From = "boot\forge-backdrop.png"; To = "boot\forge-backdrop.png" },
  @{ From = "boot\boot-job-scroll.png"; To = "boot\boot-job-scroll.png" },
  @{ From = "boot\boot-portrait-male.png"; To = "boot\boot-portrait-male.png" },
  @{ From = "boot\boot-portrait-female.png"; To = "boot\boot-portrait-female.png" },
  @{ From = "boot\boot-portrait-pepeka.png"; To = "boot\boot-portrait-pepeka.png" },
  @{ From = "black-anvil\slices\a1-title-banner.png"; To = "black-anvil\slices\a1-title-banner.png" },
  @{ From = "black-anvil\slices\i2-iron.png"; To = "black-anvil\slices\i2-iron.png" },
  @{ From = "black-anvil\slices\i3-bronze-filigree.png"; To = "black-anvil\slices\i3-bronze-filigree.png" }
)

foreach ($f in $files) {
  $from = Join-Path $src $f.From
  $to = Join-Path $dst $f.To
  if (Test-Path $from) {
    Copy-Item -Force $from $to
    Write-Host "copied $($f.From)"
  } else {
    Write-Host "MISSING $($f.From)"
  }
}
