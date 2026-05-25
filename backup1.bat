@echo off
setlocal enabledelayedexpansion

:: Parent directory (one level up from script location)
set "PARENT_DIR=%~dp0.."

:: Generate random 4-letter name
set chars=ABCDEFGHIJKLMNOPQRSTUVWXYZ
set name=
for /L %%i in (1,1,4) do (
    set /a rand=!random! %% 26
    for %%j in (!rand!) do set name=!name!!chars:~%%j,1!
)

set backup_zip=%PARENT_DIR%\backup_!name!.zip

echo Creating backup zip in parent: %backup_zip%
echo.

:: Create file list (tracked + untracked excluding ignored)
git ls-files --cached --others --exclude-standard > filelist.txt

:: Create zip directly
tar -a -c -f "%backup_zip%" -T filelist.txt

del filelist.txt

echo.
echo ✅ Backup completed successfully!
echo Zip created: %backup_zip%
pause