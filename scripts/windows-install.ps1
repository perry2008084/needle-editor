param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Needle Editor"
)

$ErrorActionPreference = 'Stop'

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ExeSource = Join-Path $ScriptRoot 'needle-desktop.exe'
$ExeTarget = Join-Path $InstallDir 'needle-desktop.exe'
$ShortcutPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Needle Editor.lnk'

if (-not (Test-Path $ExeSource)) {
    throw "needle-desktop.exe not found next to install.ps1"
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item -Force $ExeSource $ExeTarget

$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = $ExeTarget
$Shortcut.WorkingDirectory = $InstallDir
$Shortcut.IconLocation = $ExeTarget
$Shortcut.Save()

Write-Host "Needle Editor installed to: $InstallDir"
Write-Host "Desktop shortcut created: $ShortcutPath"
Write-Host "To uninstall, run uninstall.ps1 from this package."
