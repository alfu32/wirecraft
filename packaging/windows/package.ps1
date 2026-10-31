param(
  [Parameter(Mandatory=$true)][ValidateSet('x64','arm64')][string]$Arch,
  [Parameter(Mandatory=$true)][string]$Version
)
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Installers require a numeric X.Y.Z version' }
$Target = if ($Arch -eq 'arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Assets = Join-Path $Root 'target\release-assets'
$Package = "wirecraft-$Version-$Target"
$Archive = Join-Path $Assets "$Package.zip"
$Stage = Join-Path $Root "target\windows-package-$Arch"
if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
Expand-Archive $Archive -DestinationPath $Stage
$Payload = Join-Path $Stage $Package
foreach ($Executable in @('wirecraft-desktop.exe','wirecraft.exe')) {
  if (-not (Test-Path (Join-Path $Payload $Executable))) { throw "$Executable missing from payload" }
}

dotnet tool install --global wix --version 5.0.2
if ($LASTEXITCODE -ne 0) { throw 'WiX installation failed' }
$env:PATH += ";$env:USERPROFILE\.dotnet\tools"
$Msi = Join-Path $Assets "wirecraft-$Version-windows-$Arch.msi"
wix build (Join-Path $PSScriptRoot 'wirecraft.wxs') -arch $Arch -d "Version=$Version" -d "Stage=$Payload" -d "Icon=$PSScriptRoot\wirecraft.ico" -o $Msi
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $Msi)) { throw 'MSI build failed' }
# WiX emits debug symbol files beside the installer; they are build outputs, not release assets.
Get-ChildItem -Path $Assets -Filter '*.wixpdb' -File | Remove-Item -Force

# The MSIX contains the same payload under the reserved Store identity, self-signed.
$MsixStage = Join-Path $Root "target\msix-package-$Arch"
if (Test-Path $MsixStage) { Remove-Item $MsixStage -Recurse -Force }
Copy-Item $Payload $MsixStage -Recurse
$AssetsDir = Join-Path $MsixStage 'Assets'
New-Item $AssetsDir -ItemType Directory -Force | Out-Null
Add-Type -AssemblyName System.Drawing
foreach ($spec in @(@('StoreLogo',50),@('Square44x44Logo',44),@('Square150x150Logo',150))) {
  $bitmap = [System.Drawing.Bitmap]::new($spec[1],$spec[1])
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.Clear([System.Drawing.Color]::Transparent)
  $image = [System.Drawing.Image]::FromFile((Join-Path $Root 'packaging\wirecraft.png'))
  $graphics.DrawImage($image,0,0,$spec[1],$spec[1])
  $bitmap.Save((Join-Path $AssetsDir "$($spec[0]).png"),[System.Drawing.Imaging.ImageFormat]::Png)
  $image.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
}
python (Join-Path $Root 'scripts\release\msix.py') --version $Version --arch $Arch --output (Join-Path $MsixStage 'AppxManifest.xml')
if ($LASTEXITCODE -ne 0) { throw 'MSIX manifest generation failed' }
$MakeAppx = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter makeappx.exe |
  Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $MakeAppx) { throw 'Windows SDK makeappx.exe not found' }
$Msix = Join-Path $Assets "wirecraft-$Version-windows-$Arch.msix"
& $MakeAppx.FullName pack /d $MsixStage /p $Msix /o
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $Msix)) { throw 'MSIX build failed' }
$PublicCertificate = Join-Path $Assets "wirecraft-$Version-windows-$Arch.cer"
$SourceRevision = (Get-Content (Join-Path $Payload 'package-manifest.json') -Raw | ConvertFrom-Json).sourceRevision
& (Join-Path $PSScriptRoot 'sign-msix.ps1') -Msix $Msix -Arch $Arch -Version $Version -SourceRevision $SourceRevision -PublicCertificate $PublicCertificate
foreach ($artifact in @($Msi,$Msix,$PublicCertificate)) {
  $digest = (Get-FileHash $artifact -Algorithm SHA256).Hash.ToLowerInvariant()
  "$digest  $(Split-Path $artifact -Leaf)" | Set-Content "$artifact.sha256"
}
