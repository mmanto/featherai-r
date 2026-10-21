//! featherai como servicio del sistema: nodo headless 24/7 (solo nativo).
//!
//! La replicación P2P vive dentro del proceso que abre la base
//! ([`crate::persistence::open`]): sin proceso no hay endpoint Iroh, no hay
//! watcher mDNS y ese equipo no recibe cambios. Este módulo registra el mismo
//! binario como servicio del SO (`--node`) para que el nodo siga vivo con la
//! app cerrada.
//!
//! # Un solo dueño del data dir
//! redb toma un flock **exclusivo** por archivo: el data dir lo tiene un
//! proceso por vez (la GUI o el nodo). El reparto es explícito:
//!
//! - la GUI llama [`stop_node`] antes de abrir la base y [`start_node`] al
//!   salir (ver `main.rs`), así nunca hay dos dueños y la sincronización no se
//!   corta más que el tiempo del handoff;
//! - un nodo que arranca mientras la GUI tiene la base muere con código 3, que
//!   la unidad systemd excluye del reinicio (`RestartPreventExitStatus=3`).
//!
//! No hay guard de segunda instancia genérico: si algo más tiene el lock, se
//! reporta el error con [`lock_hint`] y se sale.
//!
//! # Plataformas
//! - **Linux**: unidad de **usuario** `featherai-node.service` (systemd
//!   `--user`) + `loginctl enable-linger` para que corra sin sesión abierta.
//! - **Windows**: tarea programada `featherai-node` al iniciar sesión
//!   (`schtasks /sc ONLOGON`), per-usuario y sin elevación.
//!
//! El render de la unidad y de la acción de la tarea ([`render_unit`],
//! [`render_task_action`]) es puro y testeable; el I/O de cada SO vive en
//! `service/{linux,windows,noop}_impl.rs`.

use std::path::Path;

#[cfg(target_os = "linux")]
mod linux_impl;
#[cfg(target_os = "linux")]
use linux_impl as imp;

#[cfg(target_os = "windows")]
mod windows_impl;
#[cfg(target_os = "windows")]
use windows_impl as imp;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod noop_impl;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
use noop_impl as imp;

/// Acción pedida por línea de comandos. `main` la resuelve antes de crear la
/// ventana (y antes de tocar el data dir).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// `--node`: nodo headless (sin ventana) hasta SIGTERM/SIGINT.
    Node,
    /// `--install-service`: instala la unidad/tarea del SO.
    Install,
    /// `--uninstall-service`: la quita.
    Uninstall,
    /// `--service-status`: imprime [`status_line`] y sale.
    Status,
}

/// Bandera → acción, buscando en **cualquier** posición y ignorando el resto.
///
/// Los argumentos extra no son hipotéticos: el runtime de un `.AppImage` agrega
/// los suyos (`--no-sandbox`, FUSE, etc.) y `dx serve` pasa los de su CLI.
pub fn action_from_args(mut args: impl Iterator<Item = String>) -> Option<Action> {
    args.find_map(|arg| match arg.as_str() {
        "--node" => Some(Action::Node),
        "--install-service" => Some(Action::Install),
        "--uninstall-service" => Some(Action::Uninstall),
        "--service-status" => Some(Action::Status),
        _ => None,
    })
}

/// Estado del servicio del SO.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Status {
    /// Instalado y corriendo.
    Active,
    /// Instalado y detenido (Linux: la unidad existe y systemd la ve inactiva;
    /// en Windows no hay forma barata de distinguirlo, así que no se emite).
    #[allow(dead_code)]
    Inactive,
    /// No hay unidad/tarea instalada.
    NotInstalled,
    /// El SO (o su gestor de servicios) no está disponible: sin systemd ni bus
    /// de sesión, sin `schtasks`, plataforma sin soporte.
    Unavailable(String),
}

/// `true` en las plataformas con implementación real (Linux y Windows).
///
/// El resto (macOS, Android) compila la API no-op: la tarjeta de Ajustes no se
/// muestra.
pub fn os_supported() -> bool {
    cfg!(any(target_os = "linux", target_os = "windows"))
}

/// Estado actual, preguntándole al SO (bloquea: `systemctl`/`schtasks`).
pub fn status() -> Status {
    imp::status()
}

/// Instala (o reescribe) la unidad/tarea del nodo.
///
/// **No** arranca el nodo: quien instala es la app, que tiene el lock del data
/// dir en ese momento (el nodo arranca al cerrarla, o al iniciar sesión).
pub fn install() -> Result<String, String> {
    imp::install()
}

/// Quita la unidad/tarea del nodo (idempotente).
pub fn uninstall() -> Result<String, String> {
    imp::uninstall()
}

/// Detiene el nodo si la unidad/tarea está instalada (best-effort). La llama
/// la GUI antes de abrir la base.
///
/// `Err` no cancela el arranque de la app: devuelve el motivo para que el
/// llamador lo registre (un handoff que falla en silencio deja el nodo
/// corriendo con la app cerrada, o al revés, sin rastro).
pub fn stop_node() -> Result<(), String> {
    imp::stop_node()
}

/// Arranca el nodo si la unidad/tarea está instalada (best-effort). La llama
/// la GUI al cerrar para devolverle el data dir; `Err` se registra.
pub fn start_node() -> Result<(), String> {
    imp::start_node()
}

/// Una línea con el estado, para `--service-status`.
pub fn status_line() -> String {
    match status() {
        Status::Active => active_line(),
        Status::Inactive => inactive_line(),
        Status::NotInstalled => "servicio: no instalado".to_string(),
        Status::Unavailable(motivo) => format!("servicio: no disponible ({motivo})"),
    }
}

fn active_line() -> String {
    #[cfg(target_os = "linux")]
    let linea = format!("servicio: activo (systemd --user: {})", linux_impl::UNIT_FILE);
    #[cfg(target_os = "windows")]
    let linea = format!(
        "servicio: instalado (tarea programada: {})",
        windows_impl::NAME
    );
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let linea = "servicio: activo".to_string();
    linea
}

fn inactive_line() -> String {
    #[cfg(target_os = "linux")]
    let linea = "servicio: instalado, detenido (systemd --user)".to_string();
    #[cfg(target_os = "windows")]
    let linea = "servicio: instalado (tarea programada)".to_string();
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let linea = "servicio: instalado, detenido".to_string();
    linea
}

/// Ayuda para liberar el data dir cuando el lock redb está tomado.
pub fn lock_hint() -> &'static str {
    LOCK_HINT
}

#[cfg(target_os = "linux")]
const LOCK_HINT: &str =
    "¿Está corriendo otra instancia o el servicio? Probá: systemctl --user stop featherai-node";
#[cfg(target_os = "windows")]
const LOCK_HINT: &str =
    "¿Está corriendo otra instancia o el servicio? Probá: schtasks /end /tn featherai-node";
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
const LOCK_HINT: &str = "¿Está corriendo otra instancia de featherai?";

// ── Render (puro) de la unidad systemd ───────────────────────────────────────

/// Variables de entorno que la unidad replica: las que cambian **qué base**
/// abre el nodo y **con quién** habla.
#[cfg(any(target_os = "linux", test))]
pub const SERVICE_ENV_VARS: &[&str] = &[
    "FEATHRAI_DATA_DIR",
    "FEATHRAI_PEERS",
    "FEATHRAI_LAN_WAIT_MS",
    "FEATHRAI_LOG",
];

/// Las variables de [`SERVICE_ENV_VARS`] que esta corrida tiene definidas y no
/// vacías, en ese orden.
///
/// Es lo que hace que el nodo abra **la misma** base que la app: si el usuario
/// corre con `FEATHRAI_DATA_DIR`, el servicio tiene que heredarlo.
#[cfg(any(target_os = "linux", test))]
pub fn service_env() -> Vec<(String, String)> {
    SERVICE_ENV_VARS
        .iter()
        .filter_map(|clave| {
            let valor = std::env::var(clave).ok()?;
            (!valor.trim().is_empty()).then(|| ((*clave).to_string(), valor))
        })
        .collect()
}

/// Ruta citada para la línea `ExecStart`: entre comillas si tiene espacios.
///
/// `%` se escapa a `%%`: en systemd `%` es un especificador (`%h`, `%i`,
/// `%u`) y una carpeta `/home/x/100% off/` rompería la unidad.
#[cfg(any(target_os = "linux", test))]
fn quote_exec(exe: &Path) -> String {
    let ruta = exe.to_string_lossy().replace('%', "%%");
    if ruta.contains(char::is_whitespace) {
        format!("\"{ruta}\"")
    } else {
        ruta
    }
}

/// Valor de `Environment=`: systemd parte por espacios, así que va entre
/// comillas y con `\` y `"` escapados.
#[cfg(any(target_os = "linux", test))]
fn quote_unit_value(valor: &str) -> String {
    format!(
        "\"{}\"",
        valor.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

/// Contenido de la unidad systemd de usuario que corre `<exe> --node`.
///
/// `env` sale de [`service_env`] (una línea `Environment=` por variable).
/// `RestartPreventExitStatus=3` es el código con el que el nodo muere si el
/// data dir está tomado: sin esa línea systemd reintentaría cada 30 s (ver
/// `main.rs`), con ella espera a que la GUI lo levanté.
#[cfg(any(target_os = "linux", test))]
pub fn render_unit(exe: &Path, env: &[(String, String)]) -> String {
    let mut unidad = String::from(
        "# generado por featherai (Ajustes → Servicio de nodo)\n\
         [Unit]\n\
         Description=featherai — nodo de sincronización (GuardianDB/Iroh)\n\
         \n\
         [Service]\n\
         Type=simple\n",
    );
    unidad.push_str(&format!("ExecStart={} --node\n", quote_exec(exe)));
    unidad.push_str(
        "Restart=on-failure\n\
         RestartSec=30\n\
         RestartPreventExitStatus=3\n\
         KillSignal=SIGTERM\n\
         TimeoutStopSec=6\n",
    );
    for (clave, valor) in env {
        unidad.push_str(&format!("Environment={clave}={}\n", quote_unit_value(valor)));
    }
    unidad.push_str("\n[Install]\nWantedBy=default.target\n");
    unidad
}

// ── Render (puro) de la tarea programada de Windows ─────────────────────────

/// Límite de `schtasks /tr` (la acción de la tarea, en caracteres).
#[cfg(any(target_os = "windows", test))]
pub const MAX_TASK_ACTION: usize = 255;

/// `/tr` de la tarea: el ejecutable **entre comillas** y `--node`.
///
/// Entre comillas también sin espacios: es la forma con la que `schtasks`
/// guarda y devuelve la acción, así el read-back de [`check_task_action`]
/// compara la misma cadena.
#[cfg(any(target_os = "windows", test))]
pub fn render_task_action(exe: &Path) -> String {
    format!("\"{}\" --node", exe.display())
}

/// Valida el límite de [`MAX_TASK_ACTION`] (en caracteres, no bytes: el límite
/// de Windows es de caracteres).
#[cfg(any(target_os = "windows", test))]
pub fn check_task_action(accion: &str) -> Result<(), String> {
    let largo = accion.chars().count();
    if largo > MAX_TASK_ACTION {
        Err(format!(
            "ruta del ejecutable demasiado larga para la tarea programada ({largo} caracteres): mové la app a una ruta más corta"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn accion(raw: &[&str]) -> Option<Action> {
        action_from_args(raw.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn action_from_args_reconoce_las_banderas() {
        assert_eq!(accion(&["--node"]), Some(Action::Node));
        assert_eq!(accion(&["--install-service"]), Some(Action::Install));
        assert_eq!(accion(&["--uninstall-service"]), Some(Action::Uninstall));
        assert_eq!(accion(&["--service-status"]), Some(Action::Status));
    }

    #[test]
    fn action_from_args_ignora_basura_y_argumentos_del_runtime() {
        // Sin banderas: arranque normal de la GUI.
        assert_eq!(accion(&[]), None);
        assert_eq!(accion(&["--serve", "8080"]), None);
        assert_eq!(accion(&["--no-sandbox", "--user-data-dir=/tmp/x"]), None);
        // La bandera vale en cualquier posición (el runtime agrega las suyas).
        assert_eq!(accion(&["--no-sandbox", "--node", "extra"]), Some(Action::Node));
    }

    #[test]
    fn render_unit_cita_ruta_con_espacios_y_escapa_porcentajes() {
        let unit = render_unit(
            Path::new("/opt/fea 100%/featherai"),
            &[("FEATHRAI_DATA_DIR".to_string(), "/tmp/a b".to_string())],
        );
        assert!(
            unit.contains("ExecStart=\"/opt/fea 100%%/featherai\" --node"),
            "{unit}"
        );
        assert!(
            unit.contains("Environment=FEATHRAI_DATA_DIR=\"/tmp/a b\""),
            "{unit}"
        );
        assert!(unit.contains("RestartPreventExitStatus=3"), "{unit}");
        assert!(unit.contains("WantedBy=default.target"), "{unit}");
    }

    #[test]
    fn render_unit_sin_variables_no_emite_environment() {
        let unit = render_unit(Path::new("/usr/bin/featherai"), &[]);
        assert!(unit.contains("ExecStart=/usr/bin/featherai --node"), "{unit}");
        assert!(!unit.contains("Environment="), "{unit}");
    }

    #[test]
    fn render_unit_escapa_comillas_y_barras_del_valor() {
        let unit = render_unit(
            Path::new("/usr/bin/featherai"),
            &[
                ("FEATHRAI_LOG".to_string(), "info".to_string()),
                ("FEATHRAI_PEERS".to_string(), "a\"b\\c".to_string()),
            ],
        );
        assert!(unit.contains("Environment=FEATHRAI_LOG=\"info\""), "{unit}");
        assert!(
            unit.contains("Environment=FEATHRAI_PEERS=\"a\\\"b\\\\c\""),
            "{unit}"
        );
    }

    #[test]
    fn render_task_action_cita_el_exe_y_agrega_node() {
        let accion = render_task_action(Path::new(r"C:\Program Files\featherai\featherai.exe"));
        assert_eq!(
            accion,
            r#""C:\Program Files\featherai\featherai.exe" --node"#
        );
        assert_eq!(check_task_action(&accion), Ok(()));
    }

    #[test]
    fn check_task_action_rechaza_la_ruta_demasiado_larga() {
        let accion = render_task_action(Path::new(&format!(
            "C:\\{}",
            "camino-largo\\".repeat(30)
        )));
        assert!(accion.chars().count() > MAX_TASK_ACTION, "{accion}");
        let err = check_task_action(&accion).unwrap_err();
        assert!(err.contains("demasiado larga"), "{err}");
        assert!(
            err.contains(&format!("({} caracteres)", accion.chars().count())),
            "{err}"
        );
    }

    #[test]
    fn check_task_action_cuenta_caracteres_y_no_bytes() {
        // `C:\ñ` son 4 caracteres (el largo en bytes del `ñ` es mayor): el
        // límite de Windows es de caracteres, no de bytes.
        let exe = format!("C:\\ñ{}", "e".repeat(MAX_TASK_ACTION - 13));
        let accion = render_task_action(Path::new(&exe));
        assert_eq!(accion.chars().count(), MAX_TASK_ACTION);
        assert!(accion.len() > MAX_TASK_ACTION, "{}", accion.len());
        assert_eq!(check_task_action(&accion), Ok(()));
    }
}
