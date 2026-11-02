param(
  [Parameter(Mandatory=$true)][ValidateSet('x64','arm64')][string]$Arch,
  [Parameter(Mandatory=$true)][string]$Version
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Identity = python (Join-Path $Root 'scripts\release\msix.py') --identity --version $Version --arch $Arch --output (Join-Path ([IO.Path]::GetTempPath()) 'wirecraft-identity-manifest.xml') | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'MSIX identity could not be read' }
$Assets = Join-Path $Root 'target\release-assets'
$Public = Join-Path $Assets "wirecraft-$Version-windows-$Arch.cer"
$Msix = Join-Path $Assets "wirecraft-$Version-windows-$Arch.msix"
$Certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new($Public)
$Store = [Security.Cryptography.X509Certificates.X509Store]::new('TrustedPeople','LocalMachine')
$Installed = $null
$AddedCertificate = $false
try {
  if ($Certificate.Subject -cne $Identity.publisher) { throw 'Public certificate publisher mismatch' }
  $Store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
  if ($Store.Certificates.Find([Security.Cryptography.X509Certificates.X509FindType]::FindByThumbprint,$Certificate.Thumbprint,$false).Count -eq 0) {
    $Store.Add($Certificate)
    $AddedCertificate = $true
  }
  if (Get-AppxPackage -Name $Identity.name) { throw 'Unexpected pre-existing package on CI runner' }
  Add-AppxPackage -Path $Msix -ErrorAction Stop
  $Installed = Get-AppxPackage -Name $Identity.name
  $Metadata = Get-Content "$Msix.manifest.json" -Raw | ConvertFrom-Json
  if (-not $Installed -or $Installed.PackageFamilyName -cne $Identity.packageFamilyName -or
      $Installed.Publisher -cne $Identity.publisher -or $Installed.Version.ToString() -ne $Metadata.packageVersion -or
      $Installed.Architecture.ToString() -ine $Arch) { throw 'Installed MSIX identity/version/architecture mismatch' }
  # The application starts from its install location and the command line through the
  # execution alias Windows creates for the installing user, as from a terminal. The GUI
  # application has no console, but writes to these pipes as it does to the desktop's.
  $Desktop = Join-Path $Installed.InstallLocation 'wirecraft-desktop.exe'
  $Command = Join-Path $env:LOCALAPPDATA 'Microsoft\WindowsApps\wirecraft.exe'
  foreach ($Executable in @($Desktop, $Command)) {
    if (-not (Test-Path $Executable)) { throw "Installed MSIX command missing: $Executable" }
    python -c "import subprocess,sys; actual=subprocess.check_output([sys.argv[1],'--version'],text=True,timeout=60).strip(); assert actual=='Wirecraft '+sys.argv[2],actual; print(sys.argv[1], actual)" $Executable $Version
    if ($LASTEXITCODE -ne 0) { throw "Installed MSIX launch failed: $Executable" }
  }
  python -c "import subprocess,sys; actual=subprocess.check_output([sys.argv[1],'help'],text=True,timeout=60); assert 'wirecraft compile' in actual,actual; print(sys.argv[1], 'help: command line')" $Command
  if ($LASTEXITCODE -ne 0) { throw 'The wirecraft alias did not start the command line' }
  # The desktop reads its backend's readiness from a pipe; without a console it must stay up.
  python -c "import json,subprocess,sys,time; p=subprocess.Popen([sys.argv[1],'--backend','serve','--token','check'],stdout=subprocess.PIPE,text=True); ready=json.loads(p.stdout.readline()); assert ready['status']=='ready',ready; time.sleep(3); assert p.poll() is None,'backend exited'; p.kill(); print(sys.argv[1], 'backend: ready')" $Desktop
  if ($LASTEXITCODE -ne 0) { throw 'The console-less backend did not stay up' }
} finally {
  try {
    if ($Installed) { Remove-AppxPackage -Package $Installed.PackageFullName -ErrorAction Stop }
  } finally {
    if ($AddedCertificate) { $Store.Remove($Certificate) }
    $Store.Close()
    $Certificate.Dispose()
  }
}
