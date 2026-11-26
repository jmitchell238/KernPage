<#
.SYNOPSIS
  Placeholder for code signing; does nothing unless a certificate is named.
.EXAMPLE
  .\sign.ps1 -File ..\..\target-win\installer\KernPage-0.1.0-x64.msi -CertThumbprint <sha1>
#>
param(
    [Parameter(Mandatory)][string]$File,
    [Parameter(Mandatory)][string]$CertThumbprint,
    [string]$TimestampUrl = 'http://timestamp.digicert.com'
)
$ErrorActionPreference = 'Stop'
$tool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if (-not $tool) { throw 'signtool.exe (Windows SDK) not found.' }
& $tool.Source sign /sha1 $CertThumbprint /fd SHA256 /tr $TimestampUrl /td SHA256 $File
if ($LASTEXITCODE -ne 0) { throw 'signtool failed.' }
