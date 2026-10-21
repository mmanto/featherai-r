//! Plataformas sin servicio de nodo (macOS, Android, …).
//!
//! La API existe para que el resto del código compile y para que la tarjeta de
//! Ajustes pueda preguntar [`super::os_supported`]: acá todo es no-op o
//! `Unavailable`.

use super::Status;

const SIN_SOPORTE: &str = "esta plataforma no tiene servicio de nodo (solo Linux y Windows)";

pub fn status() -> Status {
    Status::Unavailable(SIN_SOPORTE.to_string())
}

pub fn install() -> Result<String, String> {
    Err(SIN_SOPORTE.to_string())
}

pub fn uninstall() -> Result<String, String> {
    Err(SIN_SOPORTE.to_string())
}

pub fn stop_node() -> Result<(), String> {
    Ok(())
}

pub fn start_node() -> Result<(), String> {
    Ok(())
}
