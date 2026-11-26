<#
.SYNOPSIS
  Read-only checks of a built KernPage MSI (nothing is installed or run).
.PARAMETER Msi
  Path to the MSI; defaults to the newest one in target-win\installer.
#>
param([string]$Msi)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not $Msi) {
    $Msi = Get-ChildItem (Join-Path $repo 'target-win\installer\KernPage-*-x64.msi') |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $Msi -or -not (Test-Path $Msi)) { throw 'No MSI found. Run package.ps1 first.' }
$Msi = (Resolve-Path $Msi).Path

$failures = New-Object System.Collections.Generic.List[string]
function Expect($ok, $what) {
    if ($ok) { Write-Host "  ok    $what" } else { Write-Host "  FAIL  $what"; $failures.Add($what) }
}

$installer = New-Object -ComObject WindowsInstaller.Installer
$db = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($Msi, 0))
function Query($sql) {
    $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @($sql))
    $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
    $rows = @()
    while ($true) {
        $rec = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
        if ($null -eq $rec) { break }
        $n = $rec.GetType().InvokeMember('FieldCount', 'GetProperty', $null, $rec, $null)
        $rows += , @(1..$n | ForEach-Object { $rec.GetType().InvokeMember('StringData', 'GetProperty', $null, $rec, @([int]$_)) })
    }
    $view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null) | Out-Null
    , $rows
}
function Prop($name) {
    $r = Query "SELECT Value FROM Property WHERE Property='$name'"
    if ($r.Count) { $r[0][0] } else { $null }
}

Write-Host "Checking $Msi"
Expect ((Prop 'ProductName') -eq 'KernPage') 'ProductName'
Expect ((Prop 'Manufacturer') -eq 'The KernPage contributors') 'Manufacturer'
Expect ((Prop 'ProductVersion') -match '^\d+\.\d+\.\d+$') 'ProductVersion is numeric X.Y.Z'
$cargoVersion = if ((Get-Content (Join-Path $repo 'Cargo.toml') -Raw) -match '(?ms)\[workspace\.package\].*?^\s*version\s*=\s*"(\d+\.\d+\.\d+)') { $Matches[1] } else { '?' }
Expect ((Prop 'ProductVersion') -eq $cargoVersion) "ProductVersion matches Cargo.toml ($cargoVersion)"
Expect ((Prop 'WIXUI_EXITDIALOGOPTIONALCHECKBOX') -eq '1') 'finish-page launch checkbox ticked by default'
Expect ((Query "SELECT Action FROM CustomAction WHERE Action='LaunchKernPage'").Count -eq 1) 'launch custom action present'
$events = Query 'SELECT `Dialog_`, `Argument`, `Condition` FROM `ControlEvent`'
Expect (@($events | Where-Object { $_[0] -eq 'ExitDialog' -and $_[1] -eq 'LaunchKernPage' -and $_[2] -like '*NOT Installed*' }).Count -eq 1) 'launch fires from ExitDialog only on fresh install'
Expect ((Prop 'UpgradeCode') -eq '{7A952D88-8013-4CAC-85AF-3A3A1722E1DF}') 'UpgradeCode is the stable one'
Expect ((Prop 'ARPHELPLINK') -eq 'https://github.com/jmitchell238/kernpage') 'ARPHELPLINK'
Expect ((Prop 'ARPPRODUCTICON')) 'ARPPRODUCTICON set'
Expect ((Query "SELECT Name FROM Icon").Count -ge 1) 'Icon table populated'

# Dual scope: ALLUSERS must be 2 with MSIINSTALLPERUSER 1 as the per-user default.
Expect ((Prop 'ALLUSERS') -eq '2') 'ALLUSERS=2 (per-user or per-machine)'
Expect ((Prop 'MSIINSTALLPERUSER') -eq '1') 'MSIINSTALLPERUSER=1 (per-user by default)'

# Summary information: the template property carries the platform.
$si = $installer.GetType().InvokeMember('SummaryInformation', 'GetProperty', $null, $installer, @($Msi, 0))
$tmpl = $si.GetType().InvokeMember('Property', 'GetProperty', $null, $si, @([int]7))
Expect ($tmpl -like 'x64*') "Template is x64 ($tmpl)"

# Files, shortcut, upgrade, file association.
$files = (Query "SELECT FileName FROM File") | ForEach-Object { ($_[0] -split '\|')[-1] }
Expect ($files -contains 'kernpage.exe') 'kernpage.exe packaged'
Expect ($files -contains 'kernpage-launcher.exe') 'kernpage-launcher.exe packaged'
Expect ($files -contains 'kernpage.ico') 'kernpage.ico packaged'
Expect ($files -contains 'kernpage-doc.ico') 'kernpage-doc.ico (.kern document icon) packaged'
Expect (@($files | Where-Object { $_ -like 'LICENSE-*' }).Count -eq 3) 'license notices packaged (MIT, Apache, MS-RL)'
Expect ($files -contains 'THIRD_PARTY_LICENSES.html') 'THIRD_PARTY_LICENSES.html (dependency licences) packaged'
Expect ($files -contains 'THIRD_PARTY_NOTICES.txt') 'THIRD_PARTY_NOTICES.txt packaged'
$scRows = Query 'SELECT `Name`, `Directory_` FROM `Shortcut`'
$start = @($scRows | Where-Object { $_[1] -eq 'ProgramMenuFolder' } | ForEach-Object { ($_[0] -split '\|')[-1] })
Expect (($start.Count -eq 2) -and ($start -contains 'KernPage') -and ($start -contains 'KernPage Launcher')) 'two Start menu shortcuts: "KernPage" and "KernPage Launcher"'
$desk = @($scRows | Where-Object { $_[1] -eq 'DesktopFolder' } | ForEach-Object { ($_[0] -split '\|')[-1] })
Expect (($desk.Count -eq 2) -and ($desk -contains 'KernPage') -and ($desk -contains 'KernPage Launcher')) 'two optional desktop shortcuts'
# "KernPage" opens the editor; "KernPage Launcher" opens the launcher with its AUMID.
$sc = Query 'SELECT `Shortcut`, `Name`, `Target`, `Directory_` FROM `Shortcut`'
function FileKey($name) { $rows = Query 'SELECT `File`, `FileName` FROM `File`'; @($rows | Where-Object { ($_[1] -split '\|')[-1] -eq $name } | ForEach-Object { $_[0] })[0] }
$editorKey = FileKey 'kernpage.exe'
$launcherKey = FileKey 'kernpage-launcher.exe'
$main = @($sc | Where-Object { ($_[1] -split '\|')[-1] -eq 'KernPage' -and $_[3] -eq 'ProgramMenuFolder' })
$lnch = @($sc | Where-Object { ($_[1] -split '\|')[-1] -eq 'KernPage Launcher' -and $_[3] -eq 'ProgramMenuFolder' })
Expect (($main.Count -eq 1) -and ($main[0][2] -eq "[#$editorKey]")) '"KernPage" shortcut targets kernpage.exe'
Expect (($lnch.Count -eq 1) -and ($lnch[0][2] -eq "[#$launcherKey]")) '"KernPage Launcher" shortcut targets kernpage-launcher.exe'
$sprops = Query 'SELECT `Shortcut_`, `PropertyKey`, `PropVariantValue` FROM `MsiShortcutProperty`'
Expect (($lnch.Count -eq 1) -and (@($sprops | Where-Object { $_[0] -eq $lnch[0][0] -and $_[1] -like 'System.AppUserModel.ID*' -and $_[2] -eq 'KernPage.Launcher' }).Count -eq 1)) 'launcher shortcut AppUserModelID = KernPage.Launcher'
# Desktop shortcuts: own components, conditioned on public properties, defaults app=1 launcher=0, choice remembered.
$comp = Query 'SELECT `Component`, `Condition` FROM `Component`'
Expect (@($comp | Where-Object { $_[0] -eq 'DesktopShortcutApp' -and $_[1] -eq 'DESKTOPSHORTCUT_APP = 1' }).Count -eq 1) 'editor desktop shortcut component conditioned on DESKTOPSHORTCUT_APP = 1'
Expect (@($comp | Where-Object { $_[0] -eq 'DesktopShortcutLauncher' -and $_[1] -eq 'DESKTOPSHORTCUT_LAUNCHER = 1' }).Count -eq 1) 'launcher desktop shortcut component conditioned on DESKTOPSHORTCUT_LAUNCHER = 1'
$dsc = Query 'SELECT `Shortcut`, `Target`, `Component_`, `Directory_` FROM `Shortcut`'
$dApp = @($dsc | Where-Object { $_[0] -eq 'DesktopLink' })
$dLnc = @($dsc | Where-Object { $_[0] -eq 'DesktopLauncherLink' })
Expect (($dApp.Count -eq 1) -and ($dApp[0][1] -eq "[#$editorKey]") -and ($dApp[0][3] -eq 'DesktopFolder')) 'desktop "KernPage" targets kernpage.exe'
Expect (($dLnc.Count -eq 1) -and ($dLnc[0][1] -eq "[#$launcherKey]") -and ($dLnc[0][3] -eq 'DesktopFolder')) 'desktop "KernPage Launcher" targets kernpage-launcher.exe'
Expect (($dLnc.Count -eq 1) -and (@($sprops | Where-Object { $_[0] -eq $dLnc[0][0] -and $_[1] -like 'System.AppUserModel.ID*' -and $_[2] -eq 'KernPage.Launcher' }).Count -eq 1)) 'desktop launcher shortcut has the same AppUserModelID'
$setp = Query 'SELECT `Action`, `Condition` FROM `InstallExecuteSequence`'
Expect (@($setp | Where-Object { $_[0] -like 'SetDESKTOPSHORTCUT_APP_*' }).Count -ge 3 -and @($setp | Where-Object { $_[0] -like 'SetDESKTOPSHORTCUT_LAUNCHER_*' }).Count -ge 3) 'defaults/remembered values set only when the property is empty (command line wins)'
Expect (@($setp | Where-Object { $_[0] -like 'SetDESKTOPSHORTCUT_*' -and $_[1] -notlike 'DESKTOPSHORTCUT_* = ""*' }).Count -eq 0) 'every shortcut-property default is guarded by "property is empty"'
$sig = Query 'SELECT `Property` FROM `AppSearch`'
Expect (@($sig | Where-Object { $_[0] -like 'REM_APP_*' }).Count -eq 2 -and @($sig | Where-Object { $_[0] -like 'REM_LAUNCHER_*' }).Count -eq 2) 'remembered shortcut choices are searched in the registry (HKCU and HKLM)'
$regNames = Query 'SELECT `Name` FROM `Registry`'
Expect (@($regNames | Where-Object { $_[0] -eq 'DesktopShortcutApp' }).Count -eq 1) 'shortcut choice written to the registry'
Expect ((Prop 'WixShellExecTarget') -eq "[#$editorKey]") 'finish page launches kernpage.exe'
Expect ((Query "SELECT ActionProperty FROM Upgrade").Count -ge 1) 'MajorUpgrade rows present'
Expect (@(Query "SELECT ActionProperty FROM Upgrade" | Where-Object { $_[0] -eq 'WIX_DOWNGRADE_DETECTED' }).Count -eq 0) 'older versions may be installed over a newer one (no downgrade block)'
Expect ((Query "SELECT Action FROM InstallExecuteSequence WHERE Action='RemoveExistingProducts'").Count -eq 1) 'RemoveExistingProducts scheduled'

$reg = Query 'SELECT `Key`, `Name`, `Value` FROM `Registry`'
Expect (@($reg | Where-Object { $_[0] -like '*\.pub\OpenWithProgids' }).Count -eq 1) '.pub listed under Open with'
Expect (@($reg | Where-Object { $_[0] -like '*\.pub' }).Count -eq 0) '.pub default handler untouched'
Expect (@($reg | Where-Object { $_[0] -like '*KernPage.Document\DefaultIcon' -and $_[2] -like '*KernDocIconFile*' }).Count -eq 1) '.kern DefaultIcon points at kernpage-doc.ico'
Expect (@($reg | Where-Object { $_[0] -like '*KernPage.Publication\DefaultIcon' -and $_[2] -like '*[[]#DocIcon]*' }).Count -eq 1) '.pub DefaultIcon keeps the app icon'
Expect (@($reg | Where-Object { $_[0] -like '*KernPage.Publication\shell\open\command' -and $_[2] -like '*"%1"*' }).Count -eq 1) 'open command passes the file as %1'
Expect (@($reg | Where-Object { $_[0] -like '*\.kern' -and $_[2] -eq 'KernPage.Document' }).Count -eq 1) '.kern opens with KernPage by default'
Expect (@($reg | Where-Object { $_[0] -like '*KernPage.Document\shell\open\command' -and $_[2] -like '*"%1"*' }).Count -eq 1) '.kern open command passes the file as %1'

# No personal data baked in.
$bytes = [System.IO.File]::ReadAllBytes($Msi)
# Look for the builder's own profile folder (e.g. C:\Users\<name>) rather than a fixed name.
$personal = '(?i)users.{0,3}' + [regex]::Escape($env:USERNAME) + '\b'
$text = [System.Text.Encoding]::GetEncoding(28591).GetString($bytes)
$uni = [System.Text.Encoding]::Unicode.GetString($bytes)
Expect (($text -notmatch $personal) -and ($uni -notmatch $personal)) 'no personal paths or profile name in the MSI'

# --- Setup.exe (Burn bundle) ---
$setup = Join-Path (Split-Path $Msi) ("KernPage-" + (Prop 'ProductVersion') + "-Setup.exe")
Write-Host "Checking $setup"
Expect (Test-Path $setup) 'Setup.exe exists next to the MSI'
if (Test-Path $setup) {
    $sb = [System.IO.File]::ReadAllBytes($setup)
    $st = [System.Text.Encoding]::GetEncoding(28591).GetString($sb)
    # Icon: the exe must carry its own icon resource (Explorer shows it), not the generic one.
    Add-Type -AssemblyName System.Drawing
    $ic = [System.Drawing.Icon]::ExtractAssociatedIcon($setup)
    $genericExe = [System.Drawing.Icon]::ExtractAssociatedIcon((Join-Path $env:WINDIR 'System32\cmd.exe'))
    $bm = $ic.ToBitmap(); $gm = $genericExe.ToBitmap()
    $diff = $false
    for ($x = 0; $x -lt $bm.Width -and -not $diff; $x += 2) { for ($y = 0; $y -lt $bm.Height; $y += 2) { if ($bm.GetPixel($x, $y).ToArgb() -ne $gm.GetPixel($x, $y).ToArgb()) { $diff = $true; break } } }
    Expect ($diff) 'Setup.exe has its own icon resource (not the default exe icon)'
    $m = [regex]::Match($st, 'requestedExecutionLevel[^/]*level=.(\w+)')
    Expect ($m.Success -and $m.Groups[1].Value -eq 'asInvoker') 'Setup.exe manifest requests asInvoker (no UAC)'
    Expect ($st -notmatch 'requireAdministrator') 'Setup.exe never requires administrator'
    # Contents and scope, read with the WiX CLI.
    $wixDir = Join-Path $env:USERPROFILE '.dotnet\tools'
    if (Test-Path $wixDir) { $env:PATH = "$wixDir;$env:PATH" }
    $tmp = Join-Path $env:TEMP ("pwcheck-" + [guid]::NewGuid())
    try {
        & wix burn extract $setup -o (Join-Path $tmp 'ex') -oba (Join-Path $tmp 'oba') 2>&1 | Out-Null
        Expect (@(Get-ChildItem (Join-Path $tmp 'ex') -Recurse -Filter '*.msi').Count -eq 1) 'Setup.exe contains the MSI'
        $man = Get-Content (Join-Path $tmp 'oba\manifest.xml') -Raw
        Expect ($man -match '<Registration[^>]*PerMachine="no"') 'bundle is per-user (PerMachine="no")'
        Expect ($man -match 'Id="ARPSYSTEMCOMPONENT" Value="1"') 'MSI ARP row hidden when installed by the bundle (one Settings > Apps entry)'
        Expect ($man -match 'Id="MSIINSTALLPERUSER" Value="1"') 'bundle installs the MSI per-user'
        Expect (($man -match 'MsiProperty Id="DESKTOPSHORTCUT_APP" Value="\[DESKTOPSHORTCUT_APP\]"') -and ($man -match 'MsiProperty Id="DESKTOPSHORTCUT_LAUNCHER" Value="\[DESKTOPSHORTCUT_LAUNCHER\]"')) 'bundle passes both desktop-shortcut properties to the MSI'
        $theme = Get-ChildItem (Join-Path $tmp 'oba') -Recurse -Filter '*.xml' | Where-Object { (Get-Content $_.FullName -Raw) -match 'Name="DESKTOPSHORTCUT_APP"' }
        Expect (@($theme).Count -ge 1) 'Setup.exe theme has the two desktop-shortcut checkboxes'
        Expect ($man -notmatch 'PerMachine="yes"') 'no per-machine package in the chain'
    } finally { Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue }
}

# --- Release launcher carries no test-feed machinery (security review M3) ---
$launcherExe = Join-Path $repo 'target-win\release\kernpage-launcher.exe'
if (Test-Path $launcherExe) {
    $lb = [System.Text.Encoding]::GetEncoding(28591).GetString([System.IO.File]::ReadAllBytes($launcherExe))
    $testKey = (Get-Content (Join-Path $repo 'crates\kernpage-launcher\tests\fixtures\test-signing.pub'))[1].Trim()
    Expect (-not $lb.Contains($testKey)) 'release launcher does not contain the TEST-ONLY signing key'
    Expect (-not $lb.Contains('update-now failed')) 'release launcher has no dev-only headless updater'
    Expect ($lb.Contains('--feed-url is not available in this build')) 'release launcher refuses --feed-url'
} else { Write-Host '  skip  release launcher checks (target-win\release\kernpage-launcher.exe not built)' }

if ($failures.Count) { Write-Host ""; Write-Host "$($failures.Count) check(s) failed."; exit 1 }
Write-Host ""; Write-Host "All checks passed."
