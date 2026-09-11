# ============================================================
# build-backend.ps1 — Empaqueta el backend FastAPI de SmartCaja
# en un binario único (sidecar de Tauri) con PyInstaller (Windows)
# y lo coloca en desktop/src-tauri/binaries con el sufijo del target.
#
# Requiere Python 3.10–3.12 (recomendado 3.12). Con 3.13/3.14 las
# dependencias pinneadas (numpy 1.26.4) no tienen wheels y pip intentaría
# compilarlas desde el código fuente, lo que falla en Windows.
# ============================================================
$ErrorActionPreference = "Stop"

$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Desktop = Split-Path -Parent $Here
$Root = Split-Path -Parent $Desktop
Set-Location $Root

# --- Seleccionar intérprete de Python ---
$Py = $null
if (Test-Path "venv\Scripts\python.exe") {
    $Py = "venv\Scripts\python.exe"
} elseif (Get-Command py -ErrorAction SilentlyContinue) {
    # Preferir explícitamente 3.12 vía el Python launcher
    try { & py -3.12 --version *> $null; if ($LASTEXITCODE -eq 0) { $Py = "py -3.12" } } catch {}
    if (-not $Py) { $Py = "py" }
} elseif (Get-Command python -ErrorAction SilentlyContinue) {
    $Py = "python"
} else {
    throw "No se encontró Python. Instala Python 3.12 (https://www.python.org/downloads/)."
}

# --- Validar versión soportada (3.10–3.12) ---
$verRaw = & cmd /c "$Py -c ""import sys;print('%d.%d'%sys.version_info[:2])"""
$parts = $verRaw.Trim().Split('.')
$major = [int]$parts[0]; $minor = [int]$parts[1]
if ($major -ne 3 -or $minor -lt 10 -or $minor -gt 12) {
    Write-Host ""
    Write-Host "ERROR: Python $verRaw no es compatible para empaquetar el backend." -ForegroundColor Red
    Write-Host "       Las dependencias pinneadas (numpy 1.26.4) solo tienen wheels" -ForegroundColor Yellow
    Write-Host "       para Python 3.10-3.12. Con 3.13/3.14 pip compila desde fuente y falla." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  Solución:" -ForegroundColor Cyan
    Write-Host "    winget install -e --id Python.Python.3.12" -ForegroundColor Cyan
    Write-Host "    Remove-Item -Recurse -Force venv" -ForegroundColor Cyan
    Write-Host "    py -3.12 -m venv venv" -ForegroundColor Cyan
    Write-Host "    venv\Scripts\python -m pip install -r requirements.txt pyinstaller" -ForegroundColor Cyan
    Write-Host "    powershell -ExecutionPolicy Bypass -File desktop\scripts\build-backend.ps1" -ForegroundColor Cyan
    throw "Versión de Python no soportada: $verRaw (usa 3.12)."
}
Write-Host "==> Usando Python $verRaw ($Py)"

Write-Host "==> Instalando dependencias de build (PyInstaller + requirements)"
& cmd /c "$Py -m pip install --quiet -r requirements.txt pyinstaller"

Write-Host "==> Empaquetando backend con PyInstaller"
& cmd /c "$Py -m PyInstaller --clean --noconfirm --distpath desktop/backend/dist --workpath desktop/backend/build desktop/backend/smartcaja-backend.spec"

$Triple = ((rustc -vV | Select-String "host: ") -replace "host: ", "").Trim()
New-Item -ItemType Directory -Force -Path desktop/src-tauri/binaries | Out-Null
Copy-Item "desktop/backend/dist/smartcaja-backend.exe" "desktop/src-tauri/binaries/smartcaja-backend-$Triple.exe" -Force
Write-Host "==> Sidecar listo: desktop/src-tauri/binaries/smartcaja-backend-$Triple.exe"
