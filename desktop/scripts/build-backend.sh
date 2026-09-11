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

# --- Seleccionar intérprete de Python (preferir 3.12; soportado 3.10–3.12) ---
if [ -n "${PYTHON:-}" ]; then
  PY="$PYTHON"
elif [ -x "venv/bin/python" ]; then
  PY="venv/bin/python"
elif command -v python3.12 >/dev/null 2>&1; then
  PY="python3.12"
else
  PY="python3"
fi

VER="$("$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])')"
MAJOR="${VER%%.*}"; MINOR="${VER##*.}"
if [ "$MAJOR" != "3" ] || [ "$MINOR" -lt 10 ] || [ "$MINOR" -gt 12 ]; then
  echo "ERROR: Python $VER no es compatible para empaquetar el backend." >&2
  echo "       numpy 1.26.4 (pinneado) solo tiene wheels para Python 3.10-3.12;" >&2
  echo "       con 3.13/3.14 pip compila desde fuente y falla." >&2
  echo "       Crea el venv con Python 3.12:  python3.12 -m venv venv" >&2
  exit 1
fi
echo "==> Usando Python $VER ($PY)"

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
