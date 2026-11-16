$files = @(
  "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil\assets\ui\boot\boot-portrait-female.png",
  "C:\Users\raxac\OneDrive\Desktop\apps\black-anvil\assets\ui\boot\boot-portrait-male.png",
  "C:\Users\raxac\OneDrive\Desktop\apps\ba-stick\apps\stick-web\public\ui\boot\boot-portrait-female.png"
)
foreach ($p in $files) {
  $b = [IO.File]::ReadAllBytes($p)[0..7]
  Write-Host ("{0} -> {1}" -f $p, ($b -join ","))
}
