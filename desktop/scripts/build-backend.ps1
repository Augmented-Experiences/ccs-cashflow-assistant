# ============================================================
# build-backend.ps1 — Empaqueta el backend FastAPI de SmartCaja
# en un binario único (sidecar de Tauri) con PyInstaller (Windows)
# y lo coloca en desktop/src-tauri/binaries con el sufijo del target.
# ============================================================
$ErrorActionPreference = "Stop"

$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Desktop = Split-Path -Parent $Here
$Root = Split-Path -Parent $Desktop
Set-Location $Root

$Py = "python"
if (Test-Path "venv\Scripts\python.exe") { $Py = "venv\Scripts\python.exe" }

Write-Host "==> Instalando dependencias de build (PyInstaller + requirements)"
& $Py -m pip install --quiet -r requirements.txt pyinstaller

Write-Host "==> Empaquetando backend con PyInstaller"
& $Py -m PyInstaller --clean --noconfirm `
  --distpath desktop/backend/dist --workpath desktop/backend/build `
  desktop/backend/smartcaja-backend.spec

$Triple = ((rustc -vV | Select-String "host: ") -replace "host: ", "").Trim()
New-Item -ItemType Directory -Force -Path desktop/src-tauri/binaries | Out-Null
Copy-Item "desktop/backend/dist/smartcaja-backend.exe" "desktop/src-tauri/binaries/smartcaja-backend-$Triple.exe" -Force
Write-Host "==> Sidecar listo: desktop/src-tauri/binaries/smartcaja-backend-$Triple.exe"
