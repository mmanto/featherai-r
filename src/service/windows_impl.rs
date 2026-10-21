//! Implementación Windows: tarea programada del usuario.
//!
//! `schtasks /create /sc ONLOGON /rl LIMITED`: corre en la sesión del usuario
//! al iniciar sesión, sin guardar la contraseña (eso lo exige "ejecutar aunque
//! el usuario no haya iniciado sesión", que acá no se usa) y sin elevación.
//! Así el nodo abre exactamente el mismo `%LOCALAPPDATA%\featherai` que la GUI.

use super::Status;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Nombre de la tarea programada.
pub const NAME: &str = "featherai-node";

fn service_exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("no pude ubicar el ejecutable: {e}"))
}

/// `schtasks <args>`; en error devuelve el código y el stderr.
fn schtasks(args: &[&str]) -> Result<String, String> {
    let salida = Command::new("schtasks")
        .args(args)
        .output()
        .map_err(|e| format!("no pude ejecutar schtasks: {e}"))?;
    if salida.status.success() {
        return Ok(String::from_utf8_lossy(&salida.stdout).trim().to_string());
    }
    Err(format!(
        "schtasks {} falló (código {:?}){}",
        args.join(" "),
        salida.status.code(),
        stderr_suffix(&salida)
    ))
}

fn stderr_suffix(salida: &Output) -> String {
    let err = String::from_utf8_lossy(&salida.stderr).trim().to_string();
    if err.is_empty() {
        String::new()
    } else {
        format!(": {err}")
    }
}

/// Sin elevación, `schtasks /create` puede negar el alta: agrega la salida
/// manual en vez de dejar un "Access is denied" pelado.
fn con_ayuda_de_permisos(error: String) -> String {
    let bajo = error.to_ascii_lowercase();
    if bajo.contains("denied") || bajo.contains("denegado") {
        format!("{error}\nprobá crearlo una vez desde una consola de administrador")
    } else {
        error
    }
}

/// Valor de la línea `Task To Run` de `schtasks /query /fo list /v`.
///
/// Es la única lectura del módulo dependiente del idioma del sistema (el
/// rótulo está localizado): fuera de un Windows en inglés la verificación del
/// read-back no encuentra la etiqueta y el alta falla con un mensaje que
/// incluye la salida completa.
fn task_to_run(salida: &str) -> Option<String> {
    salida.lines().find_map(|linea| {
        let (clave, valor) = linea.split_once(':')?;
        (clave.trim() == "Task To Run").then(|| valor.trim().to_string())
    })
}

pub fn install() -> Result<String, String> {
    let accion = super::render_task_action(&service_exe()?);
    super::check_task_action(&accion)?;
    // La acción va como **un** argumento de `schtasks` (`/tr "\"C:\...\" --node"`).
    schtasks(&[
        "/create",
        "/tn",
        NAME,
        "/tr",
        &accion,
        "/sc",
        "ONLOGON",
        "/rl",
        "LIMITED",
        "/f",
    ])
    .map_err(con_ayuda_de_permisos)?;
    // Read-back: `schtasks` es laxo con `/tr` (puede guardar la acción sin las
    // comillas o truncarla), así que se verifica lo que quedó registrado.
    let tarea = schtasks(&["/query", "/tn", NAME, "/fo", "list", "/v"])?;
    let actual = task_to_run(&tarea)
        .ok_or_else(|| format!("no encontré la acción (Task To Run) en schtasks /query: {tarea}"))?;
    if actual != accion {
        return Err(format!(
            "la tarea quedó con otra acción ({actual:?}); esperaba {accion:?}"
        ));
    }
    Ok(format!(
        "servicio instalado: tarea programada {NAME} (arranca al iniciar sesión; \
         ahora mismo arranca cuando cierres la app)"
    ))
}

pub fn uninstall() -> Result<String, String> {
    schtasks(&["/delete", "/tn", NAME, "/f"])?;
    Ok(format!("servicio quitado: tarea programada {NAME}"))
}

pub fn status() -> Status {
    // En Windows no hay un `is-active` barato (los estados de `/fo list` están
    // localizados y los de `/query` no dicen si el proceso corre): el estado
    // que se reporta es "instalada o no".
    match Command::new("schtasks").args(["/query", "/tn", NAME]).output() {
        Ok(salida) if salida.status.success() => Status::Active,
        Ok(_) => Status::NotInstalled,
        Err(e) => Status::Unavailable(format!("no pude ejecutar schtasks: {e}")),
    }
}

/// Termina el proceso de la tarea si está corriendo.
///
/// Un código distinto de cero es el caso normal (la tarea no está corriendo),
/// así que no se reporta como error; sí se reporta no poder ejecutar
/// `schtasks`.
pub fn stop_node() -> Result<(), String> {
    match Command::new("schtasks").args(["/end", "/tn", NAME]).output() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("no pude ejecutar schtasks: {e}")),
    }
}

/// Arranca la tarea.
pub fn start_node() -> Result<(), String> {
    schtasks(&["/run", "/tn", NAME]).map(|_| ())
}
