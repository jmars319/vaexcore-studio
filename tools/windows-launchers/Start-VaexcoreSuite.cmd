@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Launch-VaexcoreSuite.ps1"
if errorlevel 1 pause
