# vaexcore Windows Launchers

These launchers are versioned in Studio because Studio is the suite coordinator.
They start installed vaexcore Windows apps from standard per-user or machine-wide
install locations, and they also fall back to Start Menu shortcuts.

Double-click from Explorer:

```text
Install-VaexcoreLaunchers.cmd
Start-VaexcoreSuite.cmd
Start-VaexcoreStudio.cmd
Start-VaexcorePulse.cmd
Start-VaexcoreConsole.cmd
```

`Install-VaexcoreLaunchers.cmd` creates Start Menu shortcuts plus a desktop
`vaexcore suite` shortcut using `assets\vaexcore-suite.ico`.

PowerShell equivalents:

```powershell
.\Install-VaexcoreLaunchers.ps1
.\Launch-VaexcoreSuite.ps1
.\Launch-VaexcoreApp.ps1 "vaexcore studio"
.\Launch-VaexcoreApp.ps1 "vaexcore pulse"
.\Launch-VaexcoreApp.ps1 "vaexcore console"
```

These are intentionally source launchers, not committed binary `.exe` files.
If we want `.exe` launchers later, build them from source during release and
publish them as generated artifacts.
