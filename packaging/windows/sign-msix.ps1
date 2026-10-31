param(
  [Parameter(Mandatory=$true)][string]$Msix,
  [Parameter(Mandatory=$true)][ValidateSet('x64','arm64')][string]$Arch,
  [Parameter(Mandatory=$true)][string]$Version,
  [Parameter(Mandatory=$true)][string]$SourceRevision,
  [Parameter(Mandatory=$true)][string]$PublicCertificate
)
# Signs with a certificate created for this run only: Wirecraft has no Store identity
# or signing secrets. The private key never leaves the runner; the public CER ships.
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Identity = python (Join-Path $Root 'scripts\release\msix.py') --identity --version $Version --arch $Arch --output (Join-Path ([IO.Path]::GetTempPath()) 'wirecraft-identity-manifest.xml') | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'MSIX identity could not be read' }
$TemporaryRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$Pfx = Join-Path $TemporaryRoot ("wirecraft-sign-" + [guid]::NewGuid().ToString('N') + '.pfx')
$Bytes = [byte[]]::new(48)
[Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($Bytes)
$Password = [Convert]::ToBase64String($Bytes)
$Certificate = $null
try {
  $Certificate = New-SelfSignedCertificate -Type CodeSigningCert -Subject $Identity.publisher `
    -CertStoreLocation 'Cert:\CurrentUser\My' -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
    -KeyExportPolicy Exportable -NotAfter (Get-Date).AddYears(3)
  if ($Certificate.Subject -cne $Identity.publisher) { throw 'Signing certificate subject differs from the MSIX publisher' }
  Export-PfxCertificate -Cert $Certificate -FilePath $Pfx -Password (ConvertTo-SecureString $Password -AsPlainText -Force) | Out-Null
  $SignTool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter signtool.exe |
    Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName -Descending | Select-Object -First 1
  if (-not $SignTool) { throw 'Windows SDK signtool.exe not found' }
  & $SignTool.FullName sign /fd SHA256 /f $Pfx /p $Password /tr http://timestamp.digicert.com /td SHA256 $Msix
  if ($LASTEXITCODE -ne 0) { throw 'MSIX signing/timestamping failed' }

  # Verify the embedded CMS signature without changing the machine trust store.
  # Native Add-AppxPackage checks package integrity and deployment trust in CI.
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  Add-Type -AssemblyName System.Security.Cryptography.Pkcs
  $Archive = [IO.Compression.ZipFile]::OpenRead($Msix)
  try {
    $Entry = $Archive.GetEntry('AppxSignature.p7x')
    if (-not $Entry) { throw 'Signed MSIX has no AppxSignature.p7x' }
    $Stream = $Entry.Open()
    $Buffer = [IO.MemoryStream]::new()
    try { $Stream.CopyTo($Buffer); $Signature = $Buffer.ToArray() }
    finally { $Stream.Dispose(); $Buffer.Dispose() }
  } finally { $Archive.Dispose() }
  if ([Text.Encoding]::ASCII.GetString($Signature,0,4) -cne 'PKCX') { throw 'Invalid MSIX signature format' }
  $Cms = [Security.Cryptography.Pkcs.SignedCms]::new()
  $Cms.Decode([byte[]]$Signature[4..($Signature.Length-1)])
  $Cms.CheckSignature($true)
  if ($Cms.SignerInfos.Count -ne 1 -or $Cms.SignerInfos[0].Certificate.Thumbprint -cne $Certificate.Thumbprint) {
    throw 'MSIX embedded signature does not match the signing certificate'
  }
  [IO.File]::WriteAllBytes($PublicCertificate, $Certificate.Export([Security.Cryptography.X509Certificates.X509ContentType]::Cert))
  $CertificateHash = (Get-FileHash $PublicCertificate -Algorithm SHA256).Hash.ToLowerInvariant()
  $MsixVersion = "$(([int]$Version.Split('.')[0]) + $Identity.versionMajorOffset).$($Version.Split('.')[1]).$($Version.Split('.')[2]).0"
  $Metadata = @{
    schemaVersion=1; format='msix'; version=$Version; packageVersion=$MsixVersion; architecture=$Arch
    sourceRevision=$SourceRevision; artifact=(Split-Path $Msix -Leaf); signed=$true
    identityName=$Identity.name; publisher=$Identity.publisher; publisherDisplayName=$Identity.publisherDisplayName
    packageFamilyName=$Identity.packageFamilyName
    certificateSha256=$CertificateHash; certificateThumbprint=$Certificate.Thumbprint; certificateType='self-signed'
    sha256=(Get-FileHash $Msix -Algorithm SHA256).Hash.ToLowerInvariant(); size=(Get-Item $Msix).Length
  }
  $Metadata | ConvertTo-Json | Set-Content "$Msix.manifest.json" -Encoding utf8
} finally {
  if ($Certificate) { Remove-Item "Cert:\CurrentUser\My\$($Certificate.Thumbprint)" -DeleteKey -ErrorAction SilentlyContinue }
  if (Test-Path $Pfx) { Remove-Item $Pfx -Force }
}
