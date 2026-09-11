// SmartCaja — app de escritorio nativa (Tauri v2)
//
// Arranca el backend FastAPI empaquetado (sidecar `smartcaja-backend`), espera a
// que responda, carga la UI web en la ventana y, al cerrar, detiene el backend.
// Además intenta (best-effort) dejar Ollama listo y descargar el modelo acorde a
// la RAM. Si Ollama no está instalado, la app abre igual y la UI lo indica.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use tauri::{Manager, RunEvent};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// En Windows evita que se abra una ventana de consola al lanzar procesos.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Guarda el proceso hijo del backend para poder terminarlo al salir.
struct BackendState(Mutex<Option<CommandChild>>);

/// Comando `ollama` sin ventana de consola en Windows.
fn ollama_command() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new("ollama");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Carpeta home del usuario.
fn home_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

/// Carpeta de datos por-usuario (coincide con la del backend).
fn user_data_dir() -> PathBuf {
    let app = "SmartCaja";
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir().join("AppData").join("Roaming"));
        base.join(app)
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join(app)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir().join(".local").join("share"));
        base.join(app)
    }
}

/// Carpeta de logs (se crea si no existe).
fn log_dir() -> PathBuf {
    let dir = user_data_dir().join("logs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Añade una línea de estado al log de Ollama.
fn ollama_status(line: &str) {
    let path = log_dir().join("ollama.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{}", line);
    }
}

/// Crea (trunca) un archivo de log dentro de la carpeta de logs.
fn log_file(name: &str) -> Option<std::fs::File> {
    std::fs::File::create(log_dir().join(name)).ok()
}

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
/// descarga el modelo acorde a la RAM. No abre ninguna ventana de consola y
/// registra el estado y la salida en <datos-usuario>/logs/ para poder revisarlo.
fn bootstrap_ollama() {
    std::thread::spawn(|| {
        ollama_status("== SmartCaja: preparando el motor de IA (Ollama) ==");

        let installed = ollama_command()
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !installed {
            ollama_status(
                "Ollama no está instalado. La app abrirá igual y la UI mostrará 'desconectado'. \
                 Instálalo desde https://ollama.com/download y reinicia SmartCaja.",
            );
            return;
        }
        ollama_status("Ollama detectado.");

        // Asegurar el servicio en 127.0.0.1:11434
        if TcpStream::connect(("127.0.0.1", 11434)).is_err() {
            ollama_status("Iniciando el servicio 'ollama serve'...");
            let mut cmd = ollama_command();
            cmd.arg("serve");
            match log_file("ollama-serve.log") {
                Some(f) => {
                    let err = f.try_clone().ok();
                    cmd.stdout(Stdio::from(f));
                    match err {
                        Some(e) => {
                            cmd.stderr(Stdio::from(e));
                        }
                        None => {
                            cmd.stderr(Stdio::null());
                        }
                    }
                }
                None => {
                    cmd.stdout(Stdio::null()).stderr(Stdio::null());
                }
            }
            let _ = cmd.spawn();
            if wait_for_port(11434, 30) {
                ollama_status("Servicio Ollama listo.");
            } else {
                ollama_status("Ollama no respondió a tiempo; se reintentará al usar la app.");
            }
        } else {
            ollama_status("El servicio Ollama ya estaba en ejecución.");
        }

        // Descargar el modelo acorde a la RAM
        let model = model_for_ram();
        ollama_status(&format!(
            "Descargando el modelo '{}' (puede tardar varios minutos la primera vez)...",
            model
        ));
        let mut pull = ollama_command();
        pull.args(["pull", model]);
        match log_file("ollama-pull.log") {
            Some(f) => {
                let err = f.try_clone().ok();
                pull.stdout(Stdio::from(f));
                match err {
                    Some(e) => {
                        pull.stderr(Stdio::from(e));
                    }
                    None => {
                        pull.stderr(Stdio::null());
                    }
                }
            }
            None => {
                pull.stdout(Stdio::null()).stderr(Stdio::null());
            }
        }
        match pull.status() {
            Ok(s) if s.success() => {
                ollama_status(&format!("Modelo '{}' listo.", model));
            }
            _ => {
                ollama_status(&format!(
                    "No se pudo descargar '{}' automáticamente. Puedes hacerlo manualmente con: ollama pull {}",
                    model, model
                ));
            }
        }
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
