@echo off
chcp 65001 >nul 2>&1
doskey j=for /f "usebackq tokens=* delims=" %%i in (`j.exe $*`) do @if exist "%%i\*" (cd /d "%%i") else (echo %%i)
