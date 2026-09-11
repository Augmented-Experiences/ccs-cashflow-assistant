# SmartCaja — App/Instalador de escritorio (Tauri)

Instalador nativo de **SmartCaja** (CCCE) para **Windows**, **macOS** y **Linux**, sin necesidad de Pinokio. Empaqueta la interfaz web y el backend FastAPI en una app de escritorio.

## Arquitectura

```
Ventana Tauri (webview nativo)
        │  al iniciar muestra ui/index.html (pantalla de carga)
        ▼
Rust (src-tauri/src/main.rs)
  1. Elige un puerto libre.
  2. Lanza el backend empaquetado como "sidecar" (smartcaja-backend).
  3. Prepara Ollama (best-effort): serve + pull del modelo según RAM.
  4. Espera a que el backend responda y navega la ventana a
     http://127.0.0.1:<puerto>/ui/index.html (la UI real de SmartCaja).
  5. Al cerrar la app, detiene el backend.
        │
        ▼
Backend FastAPI (server/app.py) empaquetado con PyInstaller
  - Sirve la UI (/ui) y la API (/api), igual que en Pinokio.
  - Recursos (app/, defaults/) desde el bundle; datos del usuario en una
    carpeta escribible por-usuario (ver server/app.py, modo "frozen").
```

Ventajas: el usuario descarga **un instalador** y ejecuta la app; no necesita conocer Pinokio ni la terminal.

## Requisitos de build

- **Python 3.10–3.12 (recomendado 3.12)** + este repo (`requirements.txt`) — para empaquetar el backend. **No uses 3.13/3.14**: `numpy 1.26.4` (pinneado) no publica wheels para esas versiones y pip intentaría compilarlo desde fuente (falla en Windows). Los scripts de build validan la versión y avisan.
- Rust (stable) + Cargo.
- Node 18+ (para la CLI de Tauri).
- Linux: `libwebkit2gtk-4.1-dev`, `librsvg2-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `patchelf`, `build-essential` (ver CI).
- **Ollama** NO se empaqueta: se instala/usa en la máquina del usuario (la app intenta prepararlo automáticamente si está presente).

## Build local

```bash
# 1) Empaquetar el backend (genera el sidecar en src-tauri/binaries/)
bash desktop/scripts/build-backend.sh      # Windows: desktop/scripts/build-backend.ps1

# 2) Generar iconos (una sola vez; requiere red la primera vez)
cd desktop && npm install && npm run icon   # crea src-tauri/icons/

# 3) Construir el instalador para tu SO
npm run build
```

Los instaladores quedan en `desktop/src-tauri/target/release/bundle/` (`.AppImage`/`.deb` en Linux, `.dmg` en macOS, `.msi`/`.exe` en Windows).

## Build multiplataforma (recomendado)

Los instaladores de cada SO deben construirse en su propio SO. Usa el workflow de GitHub Actions incluido: `.github/workflows/desktop-build.yml` (ejecútalo con *workflow_dispatch* o al publicar un tag `v*`). Genera los artefactos para Windows/macOS/Linux y los sube como *artifacts*.

## Troubleshooting

- **`Preparing metadata (pyproject.toml) ... error` al instalar numpy (Windows):** tu Python es demasiado nuevo (3.13/3.14) y `numpy 1.26.4` no tiene wheel; pip intenta compilar desde fuente. Solución: usa Python 3.12.
  ```powershell
  winget install -e --id Python.Python.3.12
  Remove-Item -Recurse -Force venv
  py -3.12 -m venv venv
  venv\Scripts\python -m pip install -r requirements.txt pyinstaller
  powershell -ExecutionPolicy Bypass -File desktop\scripts\build-backend.ps1
  ```

## Cómo funciona con Ollama y los modelos

Igual que la versión Pinokio: Ollama y los modelos viven en la máquina del usuario. Al iniciar, la app (best-effort) arranca `ollama serve` y descarga el modelo según la RAM (`<6 GB` → `llama3.2:1b`, `6–12 GB` → `llama3.2:3b`, `>12 GB` → `llama3.1:8b`). Si Ollama no está instalado, la app abre igual y la UI indica que el motor de IA está desconectado con instrucciones. Requiere internet solo la primera vez; luego funciona 100% local.
