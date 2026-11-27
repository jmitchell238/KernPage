<#
.SYNOPSIS
  Builds the KernPage release exes (editor and launcher) and wraps them in an MSI.
.DESCRIPTION
  Output: target-win\installer\KernPage-<version>-x64.msi (update feed) and
          target-win\installer\KernPage-<version>-Setup.exe (Burn bundle with the KernPage icon)
  Needs the WiX 5 CLI plus the UI extension:
    dotnet tool install --global wix --version 5.0.2
    wix extension add -g WixToolset.UI.wixext/5.0.2
    wix extension add -g WixToolset.Util.wixext/5.0.2
    wix extension add -g WixToolset.BootstrapperApplications.wixext/5.0.2
.PARAMETER SkipBuild
  Reuse an existing target-win\release\kernpage.exe.
.PARAMETER CustomArt
  Use the navy banner/dialog bitmaps from assets\ instead of WiX's stock ones.
.PARAMETER OverrideVersion
  Override the package version (numeric X.Y.Z) without touching Cargo.toml. For update tests only:
  build once as the real version and once as a higher test version, and serve both from a local
  test feed. Release builds never use it. Unless -SkipBuild is given, the build gets this version
  too (KERNPAGE_VERSION, set only while cargo runs), so the installed app's About box and the
  Launcher report the same version as the installer.
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$CustomArt,
    [ValidatePattern('^\d+\.\d+\.\d+$')][string]$OverrideVersion
)
$ErrorActionPreference = 'Stop'

$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here '..\..')).Path
$targetDir = Join-Path $repo 'target-win'
$exe = Join-Path $targetDir 'release\kernpage.exe'
$launcherExe = Join-Path $targetDir 'release\kernpage-launcher.exe'
$outDir = Join-Path $targetDir 'installer'

# Version comes from the workspace manifest; MSI wants numeric X.Y.Z.
$manifest = Get-Content (Join-Path $repo 'Cargo.toml') -Raw
if ($manifest -notmatch '(?ms)\[workspace\.package\].*?^\s*version\s*=\s*"(\d+\.\d+\.\d+)') {
    throw 'Could not read the workspace version from Cargo.toml.'
}
$version = $Matches[1]
if ($OverrideVersion) { $version = $OverrideVersion }

# Find the wix CLI, including the default dotnet global tools folder.
$wixDir = Join-Path $env:USERPROFILE '.dotnet\tools'
if (Test-Path $wixDir) { $env:PATH = "$wixDir;$env:PATH" }
if (-not (Get-Command wix -ErrorAction SilentlyContinue)) {
    throw 'wix.exe not found. Install it with: dotnet tool install --global wix --version 5.0.2'
}

if (-not $SkipBuild) {
    Write-Host "Building kernpage.exe and kernpage-launcher.exe (release)..."
    $env:CARGO_TARGET_DIR = $targetDir
    # The MinGW toolchain must come from the UCRT64 tree, not the 32-bit one.
    $ucrt = 'C:\msys64\ucrt64\bin'
    $env:PATH = "$ucrt;" + (($env:PATH -split ';' | Where-Object { $_ -notlike '*msys64\mingw32*' }) -join ';')
    # The programs report this version (option_env!("KERNPAGE_VERSION") in the build); cargo rebuilds when it changes.
    $env:KERNPAGE_VERSION = $version
    Push-Location $repo
    try {
        & (Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe") build --locked --release -p kernpage -p kernpage-launcher
        if ($LASTEXITCODE -ne 0) { throw 'cargo build failed.' }
    } finally {
        Pop-Location
        Remove-Item Env:KERNPAGE_VERSION -ErrorAction SilentlyContinue
    }
}
if (-not (Test-Path $exe)) { throw "Missing $exe" }
if (-not (Test-Path $launcherExe)) { throw "Missing $launcherExe (build with: cargo build --release -p kernpage -p kernpage-launcher)" }

# Staging area: license text for the license page, plus notice files to ship.
$stage = Join-Path $outDir 'stage'
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'notices') | Out-Null
Copy-Item (Join-Path $repo 'LICENSE') (Join-Path $stage 'notices\LICENSE.txt')
Copy-Item (Join-Path $repo 'THIRD_PARTY_NOTICES.md') (Join-Path $stage 'notices\THIRD_PARTY_NOTICES.txt')
Copy-Item (Join-Path $repo 'LICENSES\MS-RL.txt') (Join-Path $stage 'notices\LICENSE-MS-RL.txt')

# Licence texts of every Rust dependency built into the Windows programs (cargo-about, see about.toml).
# Install once with: cargo install cargo-about --locked --features cli
$about = Join-Path $stage 'notices\THIRD_PARTY_LICENSES.html'
Push-Location $repo
try {
    & (Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe") about generate (Join-Path $repo 'about.hbs') -o $about
    if ($LASTEXITCODE -ne 0) { throw 'cargo about failed (install it with: cargo install cargo-about --locked --features cli).' }
} finally { Pop-Location }
if (-not (Test-Path $about) -or (Get-Item $about).Length -lt 10000) { throw 'THIRD_PARTY_LICENSES.html was not generated.' }
Copy-Item $about (Join-Path $repo 'THIRD_PARTY_LICENSES.html') -Force

function ConvertTo-RtfText([string]$text) {
    $sb = New-Object System.Text.StringBuilder
    foreach ($ch in $text.ToCharArray()) {
        switch -Regex ($ch) {
            '[\\{}]' { [void]$sb.Append('\' + $ch) }
            "`n"     { [void]$sb.Append('\par ') }
            "`r"     { }
            default  { if ([int]$ch -gt 126) { [void]$sb.Append('\u' + [int]$ch + '?') } else { [void]$sb.Append($ch) } }
        }
    }
    $sb.ToString()
}
$intro = ConvertTo-RtfText 'KernPage is licensed under the GNU General Public License, version 3 only (GPL-3.0-only). The full text follows, and is also installed as LICENSE.txt in the install folder. Versions of KernPage up to and including 0.2.0 were released under MIT OR Apache-2.0. Third-party components keep their own licences; see THIRD_PARTY_LICENSES.html in the install folder. KernPage is an independent project, not affiliated with or endorsed by Microsoft. Microsoft and Publisher are trademarks of the Microsoft group of companies.'
$parts = @("\b0 $intro\par \par ")
foreach ($f in @(,@('LICENSE', 'GNU General Public License, version 3'))) {
    $text = ConvertTo-RtfText (Get-Content (Join-Path $repo $f[0]) -Raw)
    $parts += "\b $($f[1])\b0\par \par $text\par \par "
}
$rtf = Join-Path $stage 'license.rtf'
Set-Content -Path $rtf -Encoding ASCII `
    -Value ('{\rtf1\ansi\deff0{\fonttbl{\f0 Segoe UI;}}\fs18 ' + ($parts -join '') + '}')

New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$msi = Join-Path $outDir "KernPage-$version-x64.msi"
$art = Join-Path $here 'assets'
# The bundled fallback interface font (Noto Sans Regular, OFL 1.1) and its licence: installed in INSTALLFOLDER\fonts.
$fontsDir = Join-Path $repo 'assets\fonts\noto-sans'
foreach ($f in 'NotoSans-Regular.ttf', 'OFL.txt') { if (-not (Test-Path (Join-Path $fontsDir $f))) { throw "Missing $fontsDir\$f" } }

$artArgs = @()
if ($CustomArt) {
    $artArgs = @('-d', 'CustomArt=1',
        '-d', "BannerBmp=$(Join-Path $art 'banner.bmp')",
        '-d', "DialogBmp=$(Join-Path $art 'dialog.bmp')")
}

Write-Host "Building $msi ..."
& wix build -arch x64 -culture en-us `
    -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
    -loc (Join-Path $here 'kernpage.en-us.wxl') `
    -d "Version=$version" `
    -d "ExePath=$exe" `
    -d "LauncherExePath=$launcherExe" `
    -d "IconPath=$(Join-Path $art 'kernpage.ico')" `
    -d "DocIconPath=$(Join-Path $art 'kernpage-doc.ico')" `
    @artArgs `
    -d "LicenseRtf=$rtf" `
    -d "NoticesDir=$(Join-Path $stage 'notices')" `
    -d "FontsDir=$fontsDir" `
    -o $msi (Join-Path $here 'kernpage.wxs')
if ($LASTEXITCODE -ne 0) { throw 'wix build failed.' }

# --- Setup.exe: Burn bundle wrapping the MSI, carrying the KernPage icon ---
$setup = Join-Path $outDir "KernPage-$version-Setup.exe"
# The bundle embeds a per-user-only copy of the MSI (same UpgradeCode) so Burn runs without UAC.
$bundleMsi = Join-Path $stage 'KernPage-bundled.msi'
Write-Host "Building $bundleMsi ..."
& wix build -arch x64 -culture en-us `
    -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
    -loc (Join-Path $here 'kernpage.en-us.wxl') `
    -d PkgScope=perUser `
    -d "Version=$version" -d "ExePath=$exe" -d "LauncherExePath=$launcherExe" `
    -d "IconPath=$(Join-Path $art 'kernpage.ico')" `
    -d "DocIconPath=$(Join-Path $art 'kernpage-doc.ico')" `
    @artArgs `
    -d "LicenseRtf=$rtf" -d "NoticesDir=$(Join-Path $stage 'notices')" `
    -d "FontsDir=$fontsDir" `
    -o $bundleMsi (Join-Path $here 'kernpage.wxs')
if ($LASTEXITCODE -ne 0) { throw 'wix build (per-user MSI for bundle) failed.' }

# Logo for the bootstrapper window: the sharp 64 px master render of the icon.
$logo = Join-Path $stage 'logo.png'
Copy-Item (Join-Path $here 'icon\png-master\64.png') $logo

Write-Host "Building $setup ..."
& wix build -arch x64 -culture en-us `
    -ext WixToolset.BootstrapperApplications.wixext `
    -d "Version=$version" `
    -d "MsiPath=$bundleMsi" `
    -d "IconPath=$(Join-Path $art 'kernpage.ico')" `
    -d "LicenseRtf=$rtf" `
    -d "LogoPng=$logo" `
    -d "ThemeFile=$(Join-Path $here 'bundle-theme.xml')" `
    -d "ThemeLoc=$(Join-Path $here 'bundle-theme.wxl')" `
    -o $setup (Join-Path $here 'kernpage-bundle.wxs')
if ($LASTEXITCODE -ne 0) { throw 'wix build (bundle) failed.' }

Remove-Item $stage -Recurse -Force
Write-Host "Done: $msi"
Write-Host "Done: $setup"
