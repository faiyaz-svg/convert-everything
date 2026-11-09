<#
Downloads the conversion engines listed in engines.lock.json into src-tauri/engines.

  powershell -ExecutionPolicy Bypass -File scripts/fetch-engines.ps1
  powershell -ExecutionPolicy Bypass -File scripts/fetch-engines.ps1 -Only ffmpeg,pandoc -Force
  powershell -ExecutionPolicy Bypass -File scripts/fetch-engines.ps1 -UpdateHashes

Downloads are kept in .engine-cache so a second run is fast. Each file is checked
against its SHA-256 from the lock file. -UpdateHashes writes the hashes of the
downloaded files back into the lock file (use it after changing a URL).
Needs Windows 10 or later (curl.exe, msiexec) and Python 3 on PATH for the pip step.
#>
param(
  [switch]$UpdateHashes,
  [string[]]$Only = @(),
  [switch]$Force
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
# With powershell -File, "-Only a,b" arrives as one string.
$Only = @($Only | ForEach-Object { $_ -split ',' } | Where-Object { $_ })

$Root = Split-Path -Parent $PSScriptRoot
$LockPath = Join-Path $PSScriptRoot 'engines.lock.json'
$Cache = Join-Path $Root '.engine-cache'
$Dest = Join-Path $Root 'src-tauri\engines'
$Licenses = Join-Path $Dest 'licenses'
$LockText = [IO.File]::ReadAllText($LockPath)
$Lock = $LockText | ConvertFrom-Json

New-Item -ItemType Directory -Force -Path $Cache, $Dest, $Licenses | Out-Null

function Save-Url([string]$Url, [string]$OutFile) {
  & curl.exe -L --fail --retry 3 --silent --show-error -o "$OutFile.part" $Url
  if ($LASTEXITCODE -ne 0) { throw "Download failed: $Url" }
  Move-Item -Force "$OutFile.part" $OutFile
}

# Get-FileHash is missing when Windows PowerShell is started from PowerShell 7
# (as on GitHub's runners), so hash with .NET directly.
function Get-Sha256([string]$Path) {
  $stream = [IO.File]::OpenRead($Path)
  try { $bytes = [Security.Cryptography.SHA256]::Create().ComputeHash($stream) }
  finally { $stream.Dispose() }
  return -join ($bytes | ForEach-Object { $_.ToString('x2') })
}

function Get-Download($Engine) {
  $name = [IO.Path]::GetFileName(([Uri]$Engine.url).AbsolutePath)
  $file = Join-Path $Cache $name
  if (-not (Test-Path $file)) {
    Write-Host "  downloading $($Engine.url)"
    Save-Url $Engine.url $file
  }
  $hash = Get-Sha256 $file
  if ($Engine.sha256 -and $Engine.sha256 -ne $hash -and -not $UpdateHashes) {
    throw "SHA-256 mismatch for $name`n  expected $($Engine.sha256)`n  got      $hash"
  }
  if (-not $Engine.sha256) { Write-Host "  sha256 $hash (not pinned yet, run with -UpdateHashes)" }
  return @{ Path = $file; Hash = $hash }
}

function Expand-Download($Engine, [string]$File, [string]$Tmp) {
  if ($Engine.type -eq 'msi') {
    $args = @('/a', "`"$File`"", '/qn', "TARGETDIR=`"$Tmp`"")
    $p = Start-Process -FilePath msiexec.exe -ArgumentList $args -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw "msiexec /a failed with code $($p.ExitCode) for $File" }
  }
  elseif ($Engine.type -eq 'archive') {
    $seven = Join-Path $Dest '7zip\7z.exe'
    if (-not (Test-Path $seven)) { throw '7-Zip must be fetched first (it unpacks the other engines).' }
    & $seven x $File "-o$Tmp" -y | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "7z could not unpack $File" }
  }
  else { throw "Unknown type $($Engine.type)" }
}

function Find-Base([string]$Tmp, [string]$Marker) {
  $leaf = Split-Path $Marker -Leaf
  $hit = Get-ChildItem -Path $Tmp -Recurse -File -Filter $leaf |
    Where-Object { ('/' + $_.FullName.Replace('\', '/')).EndsWith('/' + $Marker) } |
    Sort-Object { $_.FullName.Length } |
    Select-Object -First 1
  if (-not $hit) { throw "Could not find $Marker after unpacking" }
  $base = $hit.FullName
  foreach ($part in $Marker.Split('/')) { $base = Split-Path $base -Parent }
  return $base
}

function Set-LockHash([string]$Url, [string]$Hash) {
  $at = $script:LockText.IndexOf('"' + $Url + '"')
  if ($at -lt 0) { return }
  $re = [regex]'"sha256":\s*"[0-9a-f]*"'
  $m = $re.Match($script:LockText, $at)
  if ($m.Success) {
    $script:LockText = $script:LockText.Remove($m.Index, $m.Length).Insert($m.Index, '"sha256": "' + $Hash + '"')
  }
}

$versions = [ordered]@{}
$versionsPath = Join-Path $Dest 'versions.json'
if (Test-Path $versionsPath) {
  $old = Get-Content $versionsPath -Raw | ConvertFrom-Json
  foreach ($p in $old.PSObject.Properties) { $versions[$p.Name] = $p.Value }
}

foreach ($e in $Lock.engines) {
  if ($Only.Count -gt 0 -and $Only -notcontains $e.name) { continue }
  $target = Join-Path $Dest $e.name
  $stamp = Join-Path $target '.version'
  Write-Host "$($e.title) $($e.version)"

  $dl = Get-Download $e
  if ($UpdateHashes) { Set-LockHash $e.url $dl.Hash }

  if (-not $Force -and (Test-Path $stamp) -and ((Get-Content $stamp -Raw).Trim() -eq $e.version)) {
    Write-Host '  already installed'
  }
  else {
    $tmp = Join-Path $Cache ("unpack-" + $e.name)
    if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    Expand-Download $e $dl.Path $tmp

    $base = Find-Base $tmp $e.marker
    if (Test-Path $target) { Remove-Item -Recurse -Force $target }
    Move-Item -Path $base -Destination $target
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue

    # Copy license files before anything is trimmed. They go to engines/licenses/<name>/.
    $licDir = Join-Path $Licenses $e.name
    if (Test-Path $licDir) { Remove-Item -Recurse -Force $licDir }
    New-Item -ItemType Directory -Force -Path $licDir | Out-Null
    foreach ($l in @($e.licenseFiles)) {
      if (-not $l) { continue }
      $src = Join-Path $target $l
      if (Test-Path -LiteralPath $src) { Copy-Item -LiteralPath $src -Destination $licDir -Force }
    }

    # keep: only these top-level names survive (wildcards allowed).
    if ($e.keep) {
      Get-ChildItem -Path $target -Force |
        Where-Object { $n = $_.Name; -not ($e.keep | Where-Object { $n -like $_ }) } |
        Remove-Item -Recurse -Force
    }
    # prune: "path/with*wildcards" removes matches; { path, except } empties a folder
    # apart from the names that match one of the except patterns.
    foreach ($p in @($e.prune)) {
      if (-not $p) { continue }
      if ($p -is [string]) {
        Get-Item -Path (Join-Path $target $p) -Force -ErrorAction SilentlyContinue |
          Remove-Item -Recurse -Force
      }
      else {
        $dir = Join-Path $target $p.path
        if (Test-Path -LiteralPath $dir) {
          Get-ChildItem -LiteralPath $dir -Force |
            Where-Object { $n = $_.Name; -not (@($p.except) | Where-Object { $_ -and $n -like $_ }) } |
            Remove-Item -Recurse -Force
        }
      }
    }

    if ($e.pip) {
      $site = Join-Path $target 'Lib\site-packages'
      $pipArgs = @('-m', 'pip', 'install', '--disable-pip-version-check', '--no-compile',
        '--target', $site, '--platform', 'win_amd64', '--python-version', $e.pythonVersion,
        '--implementation', 'cp', '--only-binary=:all:') + @($e.pip)
      & python @pipArgs | Out-Null
      if ($LASTEXITCODE -ne 0) { throw 'pip install failed' }
      $pth = Get-ChildItem -Path $target -Filter 'python*._pth' | Select-Object -First 1
      if ($pth) {
        $lines = Get-Content $pth.FullName
        if ($lines -notcontains 'Lib\site-packages') {
          Add-Content -Path $pth.FullName -Value 'Lib\site-packages'
        }
      }
    }

    Set-Content -Path $stamp -Value $e.version -Encoding ascii
  }

  $licDir = Join-Path $Licenses $e.name
  New-Item -ItemType Directory -Force -Path $licDir | Out-Null
  if ($e.licenseUrl) {
    $lf = Join-Path $licDir 'LICENSE.txt'
    if (-not (Test-Path $lf)) { Save-Url $e.licenseUrl $lf }
  }
  if ($e.pip) {
    Get-ChildItem -Path (Join-Path $target 'Lib\site-packages') -Directory -Filter '*.dist-info' |
      ForEach-Object {
        Get-ChildItem -Path $_.FullName -Recurse -File |
          Where-Object { $_.Name -match '^(LICEN[CS]E|COPYING)' } |
          ForEach-Object { Copy-Item $_.FullName (Join-Path $licDir ($_.Directory.Name + '-' + $_.Name)) -Force }
      }
  }

  $versions[$e.name] = [ordered]@{ title = $e.title; version = $e.version; license = $e.license; source = $e.source }
}

[IO.File]::WriteAllText($versionsPath, ($versions | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))
if ($UpdateHashes) {
  [IO.File]::WriteAllText($LockPath, $script:LockText, (New-Object Text.UTF8Encoding $false))
  Write-Host 'Updated hashes in engines.lock.json'
}
Write-Host 'Done.'
