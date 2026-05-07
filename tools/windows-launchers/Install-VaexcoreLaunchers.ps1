[CmdletBinding()]
param(
  [switch]$NoDesktop
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$SourceDir = Split-Path -Parent $PSCommandPath
$IconPath = Join-Path $SourceDir "assets\vaexcore-suite.ico"
$WScriptExe = Join-Path $env:SystemRoot "System32\wscript.exe"
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\vaexcore"
$DesktopDir = [Environment]::GetFolderPath("DesktopDirectory")

function New-VaexcoreShortcut {
  param(
    [string]$Directory,
    [string]$Name,
    [string]$Launcher
  )

  New-Item -ItemType Directory -Force -Path $Directory | Out-Null

  $shortcutPath = Join-Path $Directory "$Name.lnk"
  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($shortcutPath)
  $shortcut.TargetPath = $WScriptExe
  $shortcut.Arguments = "//B //Nologo `"$Launcher`""
  $shortcut.WorkingDirectory = $SourceDir
  $shortcut.WindowStyle = 7
  if (Test-Path $IconPath) {
    $shortcut.IconLocation = $IconPath
  }
  $shortcut.Save()
  Write-Host "Created $shortcutPath"
}

$shortcuts = @(
  @{ Name = "vaexcore suite"; Launcher = (Join-Path $SourceDir "Start-VaexcoreSuite.vbs") },
  @{ Name = "vaexcore studio"; Launcher = (Join-Path $SourceDir "Start-VaexcoreStudio.vbs") },
  @{ Name = "vaexcore pulse"; Launcher = (Join-Path $SourceDir "Start-VaexcorePulse.vbs") },
  @{ Name = "vaexcore console"; Launcher = (Join-Path $SourceDir "Start-VaexcoreConsole.vbs") }
)

foreach ($entry in $shortcuts) {
  New-VaexcoreShortcut -Directory $StartMenuDir -Name $entry.Name -Launcher $entry.Launcher
  if (-not $NoDesktop -and $entry.Name -eq "vaexcore suite") {
    New-VaexcoreShortcut -Directory $DesktopDir -Name $entry.Name -Launcher $entry.Launcher
  }
}

Write-Host "vaexcore launchers installed." -ForegroundColor Green

