//! Persistencia local-first sobre GuardianDB embebida (solo nativo; el
//! módulo se declara con `cfg(not(target_arch = "wasm32"))` en main.rs).
//!
//! Un `Db` abre tres stores KeyValue de GuardianDB (proyectos, tareas y
//! usuarios) sobre un data dir local. El acceso crudo queda acá; toda
//! lectura/escritura con significado de dominio vive en `crate::services`.
//!
//! - Doc `projects/{id}` = metadatos de `Project` (JSON; `tasks` omitida).
//! - Doc `tasks/{task_id}` = `TaskDoc { project_id, position, task }`.
//! - Doc `users/{username}` = `StoredUser` (hash argon2 PHC).
//!
//! Escrituras locales con LWW por doc; las ops sobre un mismo proyecto se
//! serializan con un mutex tokio por id ([`Db::lock_project`]).
//!
//! # Globals
//! La app abre la base una sola vez en `init_backend` (main.rs) y la
//! registra con [`set_global`]; las vistas la leen con [`db`]. En
//! web/wasm este módulo no existe: `available()` es `false` y la UI usa el
//! comportamiento demo en memoria.

use crate::models::Project;
use guardian_db::guardian::core::NewGuardianDBOptions;
use guardian_db::guardian::error::GuardianError;
use guardian_db::guardian::GuardianDB;
use guardian_db::p2p::network::client::IrohClient;
use guardian_db::p2p::network::config::ClientConfig;
use guardian_db::p2p::network::core::IrohBackend;
use guardian_db::traits::KeyValueStore;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Handle a un store KeyValue de GuardianDB (create-or-open idempotente).
pub type Kv = Arc<dyn KeyValueStore<Error = GuardianError>>;

/// Base de datos abierta: los tres stores + contexto del runtime propio.
pub struct Db {
    pub(crate) projects: Kv, // key = project id
    pub(crate) tasks: Kv,    // key = task id
    pub(crate) users: Kv,    // key = username
    /// Handle del runtime tokio de la app (spawn_blocking para hashing).
    pub(crate) rt: tokio::runtime::Handle,
    /// Backend Iroh compartido (endpoint/docs/gossip) — para el cierre
    /// ordenado en [`Db::close`].
    pub(crate) backend: Arc<IrohBackend>,
    /// Fachada GuardianDB abierta (misma base que los tres stores) — la
    /// expone el Admin RPC de sentinel en modo attached.
    pub(crate) gdb: Arc<GuardianDB>,
    /// Cliente Iroh de la instalación (AdminContext de sentinel lo necesita).
    pub(crate) client: IrohClient,
    pub(crate) node_id: String,
    pub(crate) data_dir: PathBuf,
    /// Serializa las ops de escritura por proyecto (hashmap con mutex por id).
    project_locks: std::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

/// Error de la capa de persistencia/servicios.
#[derive(Debug)]
pub enum AppError {
    /// Error del store GuardianDB (redb/iroh-docs, key inexistente al borrar…).
    Store(String),
    /// JSON inválido en un doc persistido.
    Serde(String),
    /// Entidad requerida que no existe (proyecto/tarea).
    NotFound(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(e) => write!(f, "store: {e}"),
            Self::Serde(e) => write!(f, "json: {e}"),
            Self::NotFound(e) => write!(f, "no encontrado: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<GuardianError> for AppError {
    fn from(e: GuardianError) -> Self {
        Self::Store(e.to_string())
    }
}

impl Db {
    /// Id del nodo iroh de esta instalación (para Ajustes → Sincronización).
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Directorio de datos de esta instalación.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Cierre ordenado: cierra los tres stores y hace shutdown del backend
    /// Iroh (endpoint + flush), liberando los locks de archivo redb.
    ///
    /// Al terminar el proceso no hace falta (el OS libera todo); sirve para
    /// reabrir el mismo directorio dentro del proceso (tests) y como cierre
    /// limpio al salir de la app.
    pub async fn close(&self) -> Result<(), AppError> {
        self.projects.close().await?;
        self.tasks.close().await?;
        self.users.close().await?;
        self.backend.shutdown().await.map_err(AppError::from)?;
        Ok(())
    }

    /// [`Db::close`] esperado desde un hilo propio.
    ///
    /// Necesario desde código que ya corre **dentro de un runtime tokio** (los
    /// handlers de la UI de dioxus desktop, que drivea el event loop con
    /// `block_on`): ahí `Handle::block_on` panickea con *"Cannot start a
    /// runtime from within a runtime"*. El hilo nuevo no hereda ese contexto,
    /// así que puede esperar el cierre en el runtime de la base.
    pub fn close_blocking(&self) {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                if let Err(e) = self.rt.block_on(self.close()) {
                    eprintln!("[featherai] cierre de GuardianDB con error: {e}");
                }
            });
        });
    }

    /// Mutex de serialización para las ops sobre un proyecto.
    pub(crate) async fn lock_project(
        &self,
        id: &str,
    ) -> tokio::sync::OwnedMutexGuard<()> {
        let mtx = {
            let mut locks = self
                .project_locks
                .lock()
                .expect("project_locks envenenado");
            locks
                .entry(id.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        mtx.lock_owned().await
    }
}

/// Directorio de datos por defecto de la app.
///
/// `$FEATHRAI_DATA_DIR` (si está seteado y no vacío) gana y desactiva la
/// migración: es una elección explícita. Si no, el data dir del SO
/// ([`os_data_dir`]) y, como último recurso, `./featherai-data`.
pub fn default_data_dir() -> PathBuf {
    if let Some(dir) = env_path("FEATHRAI_DATA_DIR") {
        return dir;
    }
    os_data_dir().unwrap_or_else(|| PathBuf::from("./featherai-data"))
}

/// Data dir convencional del SO donde corre el binario.
///
/// El `cfg` es por SO, no por variable de entorno: hasta v0.0.1 se miraba
/// `HOME` primero y sin distinguir el SO, así que en Windows lanzado desde un
/// shell con `HOME` (Git Bash/MSYS) la base caía en `<HOME>\.local\share\…`
/// mientras que un lanzamiento desde el menú Inicio usaba `%APPDATA%`: dos
/// bases distintas para el mismo usuario.
#[cfg(target_os = "windows")]
fn os_data_dir() -> Option<PathBuf> {
    // `LOCALAPPDATA`, no `APPDATA`: Roaming se sincroniza por red en perfiles
    // móviles y no es lugar para un redb + las claves del nodo iroh.
    env_path("LOCALAPPDATA").or_else(|| env_path("APPDATA"))
}

#[cfg(target_os = "macos")]
fn os_data_dir() -> Option<PathBuf> {
    env_path("HOME").map(|home| home.join("Library/Application Support/featherai"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn os_data_dir() -> Option<PathBuf> {
    env_path("XDG_DATA_HOME")
        // El spec XDG ignora valores relativos.
        .filter(|p| p.is_absolute())
        .map(|p| p.join("featherai"))
        .or_else(|| env_path("HOME").map(|home| home.join(".local/share/featherai")))
}

/// Variable de entorno no vacía, como `PathBuf` (`None` si falta o es solo
/// espacios: `APPDATA=""` daría un path relativo al CWD, no uno del SO).
fn env_path(name: &str) -> Option<PathBuf> {
    let raw = std::env::var(name).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

/// Prepara el data dir de esta corrida: resuelve el path y, si el layout nuevo
/// todavía no tiene base, mueve la de un data dir heredado (hasta v0.0.1 la
/// base podía estar en `$HOME/.local/share/featherai` — incluso en Windows — o
/// en `%APPDATA%\featherai`).
///
/// Devuelve el path y las notas de la migración (vacío si no hubo nada que
/// reportar) para que `main` las escriba en el log: el log vive dentro del
/// data dir, que recién se crea acá.
///
/// Llamar una sola vez, al arrancar y antes de escribir cualquier cosa en el
/// data dir.
pub fn prepare_data_dir() -> (PathBuf, Vec<String>) {
    let dir = default_data_dir();
    // Con `FEATHRAI_DATA_DIR` el usuario eligió el directorio: no se le muda la
    // base del SO a ese path.
    if env_path("FEATHRAI_DATA_DIR").is_some() {
        return (dir, Vec::new());
    }
    let notes = migrate_from_candidates(&dir, legacy_data_dirs());
    (dir, notes)
}

/// Data dirs que usaron las versiones previas, en el orden de precedencia del
/// código viejo (`HOME` primero, después `APPDATA`).
fn legacy_data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = env_path("HOME") {
        dirs.push(home.join(".local/share/featherai"));
    }
    if let Some(appdata) = env_path("APPDATA") {
        dirs.push(appdata.join("featherai"));
    }
    dirs
}

/// Mueve la base del primer candidato con base a `new_dir`, si ahí todavía no
/// hay ninguna.
///
/// Entrada por entrada y no el directorio entero: el data dir nuevo puede
/// existir ya de una corrida previa con solo el log, y en Windows `rename` de
/// un directorio falla si el destino existe.
fn migrate_from_candidates(new_dir: &Path, candidates: Vec<PathBuf>) -> Vec<String> {
    let mut notes = Vec::new();
    // Ya hay una base en el layout nuevo: no se toca nada.
    if new_dir.join("guardian").exists() {
        return notes;
    }
    let found: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|dir| dir != new_dir && dir.join("guardian").exists())
        .collect();
    let Some((from, rest)) = found.split_first() else {
        return notes;
    };
    if !rest.is_empty() {
        notes.push(format!(
            "migración: {} data dirs heredados con base ({found:?}); migro el primero",
            found.len()
        ));
    }

    let entries = match std::fs::read_dir(from) {
        Ok(entries) => entries,
        Err(e) => return vec![format!("migración: no pude leer {from:?}: {e}")],
    };
    if let Err(e) = std::fs::create_dir_all(new_dir) {
        return vec![format!("migración: no pude crear {new_dir:?}: {e}")];
    }

    let (mut moved, mut skipped, mut failed) = (0usize, Vec::new(), Vec::new());
    for entry in entries.flatten() {
        let to = new_dir.join(entry.file_name());
        if to.exists() {
            // Nunca se pisa lo que ya está en el layout nuevo (p. ej. el log de
            // esta corrida, creado antes de abrir la base).
            skipped.push(entry.file_name().to_string_lossy().into_owned());
            continue;
        }
        match std::fs::rename(entry.path(), &to) {
            Ok(()) => moved += 1,
            Err(e) => failed.push(format!("{:?}: {e}", entry.file_name())),
        }
    }

    if !skipped.is_empty() {
        notes.push(format!(
            "migración: ya existían en {new_dir:?}, no se pisaron ({})",
            skipped.join(", ")
        ));
    }
    if failed.is_empty() {
        // Solo borra el data dir viejo si quedó vacío.
        let _ = std::fs::remove_dir(from);
        notes.push(format!(
            "migración: {moved} entradas movidas de {from:?} a {new_dir:?}"
        ));
    } else {
        notes.push(format!(
            "migración: quedaron entradas en {from:?} sin mover ({})",
            failed.join(", ")
        ));
    }
    notes
}

/// Abre (o crea) la base GuardianDB en `data_dir` y devuelve el `Db` con
/// sus tres stores. Si el store de proyectos está vacío siembra los datos
/// demo; si el de usuarios está vacío crea el admin por defecto
/// (`admin`/`admin`, rol `super_admin`).
///
/// Debe llamarse dentro de un runtime tokio (la app usa el suyo propio en
/// `init_backend`; los tests usan `#[tokio::test]`).
pub async fn open(data_dir: PathBuf) -> Result<Db, AppError> {
    // ClientConfig::development(): mDNS on (descubrimiento local), n0 off,
    // puerto aleatorio. El preset por defecto apunta a ./tmp/iroh_dev; se
    // redirige al data dir de la instalación en open_with_config.
    open_with_config(data_dir, ClientConfig::development()).await
}

/// Variante de [`open`] con configuración de red explícita (los tests usan
/// el preset `offline` para no tocar mDNS/redes).
pub(crate) async fn open_with_config(
    data_dir: PathBuf,
    config: ClientConfig,
) -> Result<Db, AppError> {
    let guardian_dir = data_dir.join("guardian");
    let iroh_dir = data_dir.join("iroh");
    std::fs::create_dir_all(&guardian_dir)
        .map_err(|e| AppError::Store(format!("creando {guardian_dir:?}: {e}")))?;
    std::fs::create_dir_all(&iroh_dir)
        .map_err(|e| AppError::Store(format!("creando {iroh_dir:?}: {e}")))?;

    // guardian-db 0.20.26 (patrón de p2p_chat_tui): el cliente Iroh lleva el
    // data path (`iroh/` de la instalación); el backend se crea junto al
    // cliente y se pasa en las opciones de `GuardianDB` (fachada) junto con
    // el directorio de la base (`guardian/`).
    let config = config.with_data_path(&iroh_dir);
    let client = IrohClient::new(config).await.map_err(AppError::from)?;
    let backend = client.backend().clone();
    let node_id = client.node_id().to_string();

    let gdb = Arc::new(
        GuardianDB::new(
            client.clone(),
            Some(NewGuardianDBOptions {
                directory: Some(guardian_dir),
                backend: Some(backend.clone()),
                ..Default::default()
            }),
        )
        .await
        .map_err(AppError::from)?,
    );

    let projects = gdb
        .key_value("projects", None)
        .await
        .map_err(AppError::from)?;
    let tasks = gdb.key_value("tasks", None).await.map_err(AppError::from)?;
    let users = gdb.key_value("users", None).await.map_err(AppError::from)?;

    let db = Db {
        projects,
        tasks,
        users,
        rt: tokio::runtime::Handle::current(),
        backend,
        gdb,
        client,
        node_id,
        data_dir,
        project_locks: std::sync::Mutex::new(HashMap::new()),
    };

    // Primera corrida: sembrar demo + admin. Idempotente (solo si está vacío).
    if db.projects.all().is_empty() {
        crate::services::project::seed_demo(&db).await?;
    }
    if db.users.all().is_empty() {
        crate::services::auth::ensure_admin(&db).await?;
    }

    Ok(db)
}

// ── Globals de la app (una sola apertura por proceso) ────────────────────────

static GLOBAL_DB: OnceLock<Db> = OnceLock::new();
static INITIAL_PROJECTS: OnceLock<Vec<Project>> = OnceLock::new();

/// Registra la base abierta en `init_backend` (una vez por proceso).
pub fn set_global(db: Db, initial: Vec<Project>) {
    let _ = GLOBAL_DB.set(db);
    let _ = INITIAL_PROJECTS.set(initial);
}

/// Base global de la app.
///
/// # Panics
/// Fuera del camino nativo (`available() == false`) o antes de
/// [`set_global`].
pub fn db() -> &'static Db {
    GLOBAL_DB.get().expect("persistencia no inicializada")
}

/// Base global si ya fue inicializada (`None` antes de [`set_global`]) —
/// para el cierre ordenado en `exit_app`.
pub fn try_db() -> Option<&'static Db> {
    GLOBAL_DB.get()
}

/// Listado inicial de proyectos (el que se mostró al arrancar, tras la
/// siembra si correspondía) — para el estado de la UI.
pub fn initial_projects() -> Vec<Project> {
    INITIAL_PROJECTS.get().cloned().unwrap_or_default()
}

/// `true` cuando la persistencia GuardianDB está disponible (nativo).
#[allow(dead_code)]
pub fn available() -> bool {
    cfg!(not(target_arch = "wasm32"))
}

/// Expone el Admin RPC de sentinel sobre la base global ya abierta, para que
/// `guardian-sentinel --connect 127.0.0.1:PORT` inspeccione la base en vivo
/// (modo attached: el panel no toca el lock redb, lo sigue teniendo la app).
///
/// Opt-in por env: solo actúa si `FEATHRAI_SENTINEL_PORT` está seteado.
/// Loopback, sin token (herramienta de desarrollo local).
pub fn maybe_spawn_admin_rpc() {
    let Ok(port) = std::env::var("FEATHRAI_SENTINEL_PORT") else {
        return;
    };
    let Ok(port) = port.trim().parse::<u16>() else {
        eprintln!("[featherai] FEATHRAI_SENTINEL_PORT inválido: {port:?}");
        return;
    };
    let Some(db) = try_db() else {
        return;
    };

    use guardian_db::sentinel::{AdminContext, AdminSource, EmbeddedSource, serve};
    let addr = format!("127.0.0.1:{port}");
    eprintln!("[featherai] sentinel RPC activo en {addr}");
    let source: Arc<dyn AdminSource> =
        Arc::new(EmbeddedSource::new(AdminContext::new(db.gdb.clone(), db.client.clone())));
    let rt = db.rt.clone();
    rt.spawn(async move {
        if let Err(e) = serve(&addr, source, None).await {
            eprintln!("[featherai] sentinel RPC en {addr} terminó con error: {e}");
        }
    });
}

#[cfg(test)]
mod tests;
