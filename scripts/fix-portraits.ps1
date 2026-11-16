$dir = "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil\assets\ui\boot"
foreach ($n in @("boot-portrait-male", "boot-portrait-female", "boot-portrait-pepeka")) {
  $png = Join-Path $dir "$n.png"
  $jpg = Join-Path $dir "$n.jpg"
  if (Test-Path $png) {
    Move-Item -Force $png $jpg
    Write-Host "renamed $n.png -> $n.jpg"
  } elseif (Test-Path $jpg) {
    Write-Host "already $n.jpg"
  } else {
    Write-Host "MISSING $n"
  }
}
