//! Implementación Linux: unidad de **usuario** de systemd.
//!
//! `systemd --user` (no un servicio de root): el data dir, la identidad iroh y
//! la base son del usuario que corre la app, y un servicio de sistema los
//! abriría como root. `loginctl enable-linger $USER` es lo que hace que la
//! unidad siga viva sin sesión abierta (si no, muere al cerrar sesión).

use super::Status;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Nombre del archivo de unidad.
pub const UNIT_FILE: &str = "featherai-node.service";
/// Nombre con el que lo resuelve `systemctl` (la extensión es implícita).
const UNIT: &str = "featherai-node";

/// Directorio de unidades de usuario: `$XDG_CONFIG_HOME/systemd/user` o
/// `~/.config/systemd/user` (el XDG ignorado si es relativo, como en el resto
/// del repo).
fn unit_dir() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.map(|dir| dir.join("systemd/user")).ok_or_else(|| {
        "sin $HOME ni $XDG_CONFIG_HOME: no sé dónde escribir la unidad".to_string()
    })
}

fn unit_path() -> Result<PathBuf, String> {
    unit_dir().map(|dir| dir.join(UNIT_FILE))
}

/// Ejecutable que arranca la unidad.
///
/// Dentro de un `.AppImage` `current_exe()` es el binario desempaquetado en
/// `/tmp/.mount_*`: cambia en cada corrida y desaparece al cerrar la app, así
/// que ahí manda `$APPIMAGE` (el archivo que el usuario realmente tiene). Si
/// el usuario mueve ese archivo, la unidad queda apuntando a la nada y hay que
/// reinstalar.
fn service_exe() -> Result<PathBuf, String> {
    if let Some(appimage) = std::env::var_os("APPIMAGE").filter(|valor| !valor.is_empty()) {
        return Ok(PathBuf::from(appimage));
    }
    std::env::current_exe().map_err(|e| format!("no pude ubicar el ejecutable: {e}"))
}

/// `systemctl --user <args>`; en error devuelve el código y el stderr.
fn systemctl(args: &[&str]) -> Result<String, String> {
    let salida = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("no pude ejecutar systemctl: {e}"))?;
    if salida.status.success() {
        return Ok(String::from_utf8_lossy(&salida.stdout).trim().to_string());
    }
    Err(format!(
        "systemctl --user {} falló (código {:?}){}",
        args.join(" "),
        salida.status.code(),
        stderr_suffix(&salida)
    ))
}

/// Detalle del stderr para un mensaje de error (vacío si no hay nada).
fn stderr_suffix(salida: &Output) -> String {
    let err = String::from_utf8_lossy(&salida.stderr).trim().to_string();
    if err.is_empty() {
        String::new()
    } else {
        format!(": {err}")
    }
}

/// Habilita linger (best-effort): sin esto el nodo corre sólo mientras haya
/// sesión abierta. Puede requerir autorización de polkit.
fn enable_linger() -> Result<(), String> {
    let usuario = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .map_err(|_| "sin $USER ni $LOGNAME".to_string())?;
    let salida = Command::new("loginctl")
        .args(["enable-linger", &usuario])
        .output()
        .map_err(|e| format!("no pude ejecutar loginctl: {e}"))?;
    if salida.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&salida.stderr).trim().to_string();
    Err(if err.is_empty() {
        format!("loginctl enable-linger salió con código {:?}", salida.status.code())
    } else {
        err
    })
}

pub fn install() -> Result<String, String> {
    let exe = service_exe()?;
    let dir = unit_dir()?;
    let path = dir.join(UNIT_FILE);
    std::fs::create_dir_all(&dir).map_err(|e| format!("creando {dir:?}: {e}"))?;
    let unidad = super::render_unit(&exe, &super::service_env());
    std::fs::write(&path, unidad).map_err(|e| format!("escribiendo {path:?}: {e}"))?;
    // La unidad recién escrita no existe para systemd hasta el reload.
    systemctl(&["daemon-reload"])?;
    // `enable` **sin** `--now`: quien instala es la app, que en este momento
    // tiene el lock del data dir; el nodo arranca al cerrarla, al iniciar
    // sesión, o ya con `systemctl --user start featherai-node`.
    systemctl(&["enable", UNIT])?;
    let mut mensaje = format!(
        "servicio instalado: {UNIT_FILE}\n\
         se inicia al iniciar sesión (y sin sesión abierta si `loginctl enable-linger` quedó habilitado); \
         ahora mismo arranca cuando cierres la app, o ya con: systemctl --user start {UNIT}"
    );
    if let Err(e) = enable_linger() {
        mensaje.push_str(&format!(
            "\n(sin linger: {e}; el nodo correrá sólo con tu sesión abierta)"
        ));
    }
    Ok(mensaje)
}

pub fn uninstall() -> Result<String, String> {
    // Errores ignorados: puede no estar cargada, ni existir, ni correr.
    let _ = systemctl(&["disable", "--now", UNIT]);
    let path = unit_path()?;
    if let Err(e) = std::fs::remove_file(&path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(format!("borrando {path:?}: {e}"));
        }
    }
    let _ = systemctl(&["daemon-reload"]);
    Ok(format!("servicio quitado: {UNIT_FILE}"))
}

pub fn status() -> Status {
    let path = match unit_path() {
        Ok(path) => path,
        Err(e) => return Status::Unavailable(e),
    };
    if !path.exists() {
        return Status::NotInstalled;
    }
    // `is-active`: 0 = corriendo; 3 = instalado y detenido (systemd también
    // devuelve 3 para una unidad que no conoce). Cualquier otro código es
    // systemd no disponible: sin bus de sesión (login por SSH sin
    // `XDG_RUNTIME_DIR`), sin systemd, o el binario no está.
    match Command::new("systemctl")
        .arg("--user")
        .args(["is-active", UNIT])
        .output()
    {
        Ok(salida) if salida.status.success() => Status::Active,
        Ok(salida) if salida.status.code() == Some(3) => Status::Inactive,
        Ok(salida) => Status::Unavailable(format!(
            "systemctl --user is-active {UNIT} falló (código {:?}){}",
            salida.status.code(),
            stderr_suffix(&salida)
        )),
        Err(e) => Status::Unavailable(format!("no pude ejecutar systemctl: {e}")),
    }
}

/// Detiene el nodo si la unidad está instalada.
///
/// `systemctl stop` espera a que el proceso termine (hasta `TimeoutStopSec`),
/// así que al volver el lock del data dir ya está libre: es lo que hace
/// segura la apertura inmediata de la base por parte de la GUI.
pub fn stop_node() -> Result<(), String> {
    let path = unit_path()?;
    if !path.exists() {
        return Ok(());
    }
    systemctl(&["stop", UNIT]).map(|_| ())
}

/// Arranca el nodo si la unidad está instalada. Arrancar una unidad ya activa
/// es inocuo.
pub fn start_node() -> Result<(), String> {
    let path = unit_path()?;
    if !path.exists() {
        return Ok(());
    }
    systemctl(&["start", UNIT]).map(|_| ())
}
