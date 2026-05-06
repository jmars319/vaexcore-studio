[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$Apps = @(
  "vaexcore studio",
  "vaexcore pulse",
  "vaexcore console"
)

$Launcher = Join-Path (Split-Path -Parent $PSCommandPath) "Launch-VaexcoreApp.ps1"

foreach ($app in $Apps) {
  & $Launcher $app
}
