[CmdletBinding()]
param(
  [Parameter(Mandatory = $true, Position = 0)]
  [ValidateSet("vaexcore studio", "vaexcore pulse", "vaexcore console")]
  [string]$AppName
)

$ErrorActionPreference = "Stop"

function Resolve-VaexcoreApp {
  param([string]$Name)

  $exeName = Resolve-VaexcoreExecutableName $Name
  $knownLocalAppDataPaths = @{
    "vaexcore studio" = @(
      "vaexcore studio\vaexcore-studio.exe",
      "Programs\vaexcore studio\vaexcore-studio.exe"
    )
    "vaexcore pulse" = @(
      "vaexcore pulse\vaexcore-pulse.exe",
      "Programs\vaexcore pulse\vaexcore-pulse.exe"
    )
    "vaexcore console" = @(
      "Programs\vaexcore console\vaexcore-console.exe"
    )
  }
  $candidates = @()

  if ($env:LOCALAPPDATA) {
    foreach ($relativePath in @($knownLocalAppDataPaths[$Name])) {
      $candidates += Join-Path $env:LOCALAPPDATA $relativePath
    }
    $candidates += Join-Path $env:LOCALAPPDATA "Programs\$Name\$exeName"
  }
  if ($env:ProgramFiles) {
    $candidates += Join-Path $env:ProgramFiles "$Name\$exeName"
  }
  if (${env:ProgramFiles(x86)}) {
    $candidates += Join-Path ${env:ProgramFiles(x86)} "$Name\$exeName"
  }

  foreach ($candidate in $candidates) {
    if ($candidate -and (Test-Path $candidate)) {
      return $candidate
    }
  }

  $shortcutRoots = @()
  if ($env:APPDATA) {
    $shortcutRoots += Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
  }
  if ($env:ProgramData) {
    $shortcutRoots += Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs"
  }

  foreach ($root in $shortcutRoots) {
    if (-not (Test-Path $root)) {
      continue
    }

    $shortcut = Get-ChildItem -Path $root -Filter "$Name*.lnk" -File -Recurse -ErrorAction SilentlyContinue |
      Select-Object -First 1
    if ($shortcut) {
      return $shortcut.FullName
    }
  }

  return $null
}

function Resolve-VaexcoreExecutableName {
  param([string]$Name)

  switch ($Name) {
    "vaexcore studio" { return "vaexcore-studio.exe" }
    "vaexcore pulse" { return "vaexcore-pulse.exe" }
    "vaexcore console" { return "vaexcore-console.exe" }
    default { return "$Name.exe" }
  }
}

$appPath = Resolve-VaexcoreApp $AppName
if (-not $appPath) {
  Write-Error "Could not find $AppName. Install it first, then run this launcher again."
  exit 1
}

Write-Host "Launching $AppName from $appPath"
Start-Process -FilePath $appPath

