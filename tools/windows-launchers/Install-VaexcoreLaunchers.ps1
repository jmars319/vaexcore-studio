[CmdletBinding()]
param(
  [switch]$NoDesktop
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$SourceDir = Split-Path -Parent $PSCommandPath
$IconPath = Join-Path $SourceDir "assets\vaexcore-suite.ico"
$PowerShellExe = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\powershell.exe"
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\vaexcore"
$DesktopDir = [Environment]::GetFolderPath("DesktopDirectory")

function New-VaexcoreShortcut {
  param(
    [string]$Directory,
    [string]$Name,
    [string]$Script,
    [string]$ScriptArguments = ""
  )

  New-Item -ItemType Directory -Force -Path $Directory | Out-Null

  $shortcutPath = Join-Path $Directory "$Name.lnk"
  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($shortcutPath)
  $shortcut.TargetPath = $PowerShellExe
  $shortcut.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$Script`" $ScriptArguments".Trim()
  $shortcut.WorkingDirectory = $SourceDir
  if (Test-Path $IconPath) {
    $shortcut.IconLocation = $IconPath
  }
  $shortcut.Save()
  Write-Host "Created $shortcutPath"
}

$suiteScript = Join-Path $SourceDir "Launch-VaexcoreSuite.ps1"
$appScript = Join-Path $SourceDir "Launch-VaexcoreApp.ps1"

$shortcuts = @(
  @{ Name = "vaexcore suite"; Script = $suiteScript; Arguments = "" },
  @{ Name = "vaexcore studio"; Script = $appScript; Arguments = '"vaexcore studio"' },
  @{ Name = "vaexcore pulse"; Script = $appScript; Arguments = '"vaexcore pulse"' },
  @{ Name = "vaexcore console"; Script = $appScript; Arguments = '"vaexcore console"' }
)

foreach ($entry in $shortcuts) {
  New-VaexcoreShortcut -Directory $StartMenuDir -Name $entry.Name -Script $entry.Script -ScriptArguments $entry.Arguments
  if (-not $NoDesktop -and $entry.Name -eq "vaexcore suite") {
    New-VaexcoreShortcut -Directory $DesktopDir -Name $entry.Name -Script $entry.Script -ScriptArguments $entry.Arguments
  }
}

Write-Host "vaexcore launchers installed." -ForegroundColor Green

