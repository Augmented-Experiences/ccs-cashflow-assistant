#!/usr/bin/env bash
# ============================================================
# build-backend.sh — Empaqueta el backend FastAPI de SmartCaja
# en un binario único (sidecar de Tauri) con PyInstaller y lo
# coloca en desktop/src-tauri/binaries con el sufijo del target.
# ============================================================
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
DESKTOP="$(dirname "$HERE")"
ROOT="$(dirname "$DESKTOP")"
cd "$ROOT"

PY="${PYTHON:-python3}"
if [ -x "venv/bin/python" ]; then PY="venv/bin/python"; fi

echo "==> Instalando dependencias de build (PyInstaller + requirements)"
"$PY" -m pip install --quiet -r requirements.txt pyinstaller

echo "==> Empaquetando backend con PyInstaller"
"$PY" -m PyInstaller --clean --noconfirm \
  --distpath desktop/backend/dist --workpath desktop/backend/build \
  desktop/backend/smartcaja-backend.spec

TRIPLE="$(rustc -vV | sed -n 's/host: //p')"
mkdir -p desktop/src-tauri/binaries
cp "desktop/backend/dist/smartcaja-backend" "desktop/src-tauri/binaries/smartcaja-backend-${TRIPLE}"
chmod +x "desktop/src-tauri/binaries/smartcaja-backend-${TRIPLE}"
echo "==> Sidecar listo: desktop/src-tauri/binaries/smartcaja-backend-${TRIPLE}"
