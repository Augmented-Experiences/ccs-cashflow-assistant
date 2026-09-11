// SmartCaja — app de escritorio nativa (Tauri v2)
//
// Arranca el backend FastAPI empaquetado (sidecar `smartcaja-backend`), espera a
// que responda, carga la UI web en la ventana y, al cerrar, detiene el backend.
// Además intenta (best-effort) dejar Ollama listo y descargar el modelo acorde a
// la RAM. Si Ollama no está instalado, la app abre igual y la UI lo indica.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{Manager, RunEvent};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// Guarda el proceso hijo del backend para poder terminarlo al salir.
struct BackendState(Mutex<Option<CommandChild>>);

/// Elige un puerto libre para el backend (prefiere 7860, luego efímero).
fn pick_port() -> u16 {
    for p in [7860u16, 7861, 7862, 7863] {
        if TcpListener::bind(("127.0.0.1", p)).is_ok() {
            return p;
        }
    }
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(7860)
}

/// Espera hasta que el puerto acepte conexiones (o se agoten los intentos).
fn wait_for_port(port: u16, attempts: u32) -> bool {
    for _ in 0..attempts {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

/// Modelo de Ollama recomendado según la RAM total del equipo.
fn model_for_ram() -> &'static str {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    if gb < 6.0 {
        "llama3.2:1b"
    } else if gb < 12.0 {
        "llama3.2:3b"
    } else {
        "llama3.1:8b"
    }
}

/// Deja Ollama listo (best-effort): si está instalado, asegura el servicio y
/// descarga el modelo acorde a la RAM. No es fatal si falla o no está presente.
fn bootstrap_ollama() {
    std::thread::spawn(|| {
        let installed = Command::new("ollama")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !installed {
            return;
        }
        if TcpStream::connect(("127.0.0.1", 11434)).is_err() {
            let _ = Command::new("ollama")
                .arg("serve")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            wait_for_port(11434, 30);
        }
        let _ = Command::new("ollama")
            .args(["pull", model_for_ram()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(BackendState(Mutex::new(None)))
        .setup(|app| {
            let handle = app.handle().clone();
            let port = pick_port();

            // 1) Lanzar el backend empaquetado (sidecar).
            let (mut rx, child) = app
                .shell()
                .sidecar("smartcaja-backend")
                .expect("no se encontró el sidecar smartcaja-backend")
                .args(["--port", &port.to_string()])
                .spawn()
                .expect("no se pudo iniciar el backend");

            app.state::<BackendState>()
                .0
                .lock()
                .unwrap()
                .replace(child);

            // Drenar la salida del backend (evita bloqueos del buffer).
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    if let CommandEvent::Stderr(bytes) | CommandEvent::Stdout(bytes) = event {
                        let _ = String::from_utf8_lossy(&bytes);
                    }
                }
            });

            // 2) Preparar Ollama en segundo plano (best-effort).
            bootstrap_ollama();

            // 3) Cuando el backend responda, cargar la UI real en la ventana.
            std::thread::spawn(move || {
                if wait_for_port(port, 240) {
                    if let Some(win) = handle.get_webview_window("main") {
                        let url = format!("http://127.0.0.1:{}/ui/index.html", port);
                        let _ = win.eval(&format!("window.location.replace('{}')", url));
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error al construir la app de SmartCaja")
        .run(|app_handle, event| {
            if let RunEvent::Exit = event {
                if let Some(child) = app_handle.state::<BackendState>().0.lock().unwrap().take() {
                    let _ = child.kill();
                }
            }
        });
}
