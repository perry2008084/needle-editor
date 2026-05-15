param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Needle Editor"
)

$ErrorActionPreference = 'Stop'

$ShortcutPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Needle Editor.lnk'

if (Test-Path $ShortcutPath) {
    Remove-Item -Force $ShortcutPath
}

if (Test-Path $InstallDir) {
    Remove-Item -Recurse -Force $InstallDir
}

Write-Host "Needle Editor uninstalled from: $InstallDir"
