# Rebuilds the app icon (dark teal rounded tile with a big white K: a rolled-spine stem and two page arms) from kernpage.svg and the
# hinted kernpage@16/20/24.svg: SVGs -> PNGs (resvg example) -> png/ -> kernpage.ico.
$root = Resolve-Path "$PSScriptRoot\..\..\.."
Set-Location $root
$py = (Get-Command python).Source
$env:CARGO_TARGET_DIR = 'target-win'
$env:PATH = 'C:\msys64\ucrt64\bin;' + "$env:USERPROFILE\.cargo\bin;" + (($env:PATH -split ';' | ? { $_ -notmatch 'msys64\mingw32' }) -join ';')
cargo run -q -p kernicons --example render_svg -j 2
& $py -I packaging/windows/icon/make_icons.py
& $py -I packaging/windows/icon/make_ico.py packaging/windows/icon/png packaging/windows/assets/kernpage.ico

# .kern document icon (white page, folded corner, teal K): hinted 16/20/24/32 + master 40/48/64/256 -> kernpage-doc.ico
& $py -I packaging/windows/icon/make_doc_svgs.py
$icon = 'packaging/windows/icon'
New-Item -ItemType Directory -Force "$icon/png-doc-hinted", "$icon/png-doc-master" | Out-Null
foreach ($px in 16, 20, 24, 32) { cargo run -q --manifest-path ../kernicons/Cargo.toml --example render_svg -j 2 -- "$icon/kernpage-doc@$px.svg" $px "$icon/png-doc-hinted/$px.png" }
foreach ($px in 256) { cargo run -q --manifest-path ../kernicons/Cargo.toml --example render_svg -j 2 -- "$icon/kernpage-doc.svg" $px "$icon/png-doc-master/$px.png" }
& $py -I packaging/windows/icon/make_doc_ico.py
