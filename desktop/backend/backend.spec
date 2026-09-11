# -*- mode: python ; coding: utf-8 -*-
#
# PyInstaller spec para empaquetar el backend FastAPI de SmartCaja como un
# binario único (sidecar de Tauri). Los recursos de solo lectura (app/, defaults/)
# se incluyen en el bundle; el backend los resuelve vía sys._MEIPASS cuando está
# congelado (ver server/app.py).
#
from pathlib import Path

ROOT = Path(SPECPATH).resolve().parents[1]  # desktop/backend -> raíz del repo
SERVER = ROOT / "server"

datas = [
    (str(ROOT / "app"), "app"),
    (str(ROOT / "defaults"), "defaults"),
]

hiddenimports = [
    "uvicorn.logging",
    "uvicorn.loops",
    "uvicorn.loops.auto",
    "uvicorn.protocols",
    "uvicorn.protocols.http",
    "uvicorn.protocols.http.auto",
    "uvicorn.protocols.http.h11_impl",
    "uvicorn.protocols.http.httptools_impl",
    "uvicorn.protocols.websockets",
    "uvicorn.protocols.websockets.auto",
    "uvicorn.protocols.websockets.websockets_impl",
    "uvicorn.lifespan",
    "uvicorn.lifespan.on",
    "uvicorn.lifespan.off",
]

a = Analysis(
    [str(SERVER / "app.py")],
    pathex=[str(SERVER)],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["tkinter", "matplotlib", "pytest"],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name="backend",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
