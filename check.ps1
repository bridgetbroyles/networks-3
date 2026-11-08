# check.ps1 - build both programs and score them on a practice egg.
# Usage (from the repo root):
#   .\check.ps1                                   # solved example, egg 21
#   .\check.ps1 -Switch a3_switch -Detect a3_detect_scanner -Egg 23

param(
  [string]$Switch = "examples/broken_link",
  [string]$Detect = "examples/broken_link_detect",
  [int]$Egg = 21
)

function Build-Wasm($dir) {
  Push-Location $dir
  cargo build --release --target wasm32-unknown-unknown | Out-Host
  $ok = ($LASTEXITCODE -eq 0)
  Pop-Location
  if (-not $ok) { Write-Host "Build failed in $dir" -ForegroundColor Red; exit 1 }
  # newest .wasm in the release folder, whatever the crate is called
  $wasm = Get-ChildItem "$dir/target/wasm32-unknown-unknown/release/*.wasm" |
          Sort-Object LastWriteTime -Descending | Select-Object -First 1
  return $wasm.FullName
}

$sw = Build-Wasm $Switch
$dt = Build-Wasm $Detect
Write-Host "Checking egg $Egg with`n  switch: $sw`n  detect: $dt" -ForegroundColor Cyan
python scripts/practice-check.py --switch $sw --detect $dt --egg $Egg