@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Launch-VaexcoreApp.ps1" "vaexcore studio"
if errorlevel 1 pause
