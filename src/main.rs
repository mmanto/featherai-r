// En Windows, el ejecutable empaquetado es una app GUI: sin esto se abre una
// consola negra detrás de la ventana. Solo en release para conservar los logs
// de `dx serve`/`cargo run` durante el desarrollo.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

// The dioxus prelude contains a ton of common items used in dioxus apps. It's a good idea to import wherever you
// need dioxus
use dioxus::prelude::*;

use crate::components::layout::{AuthProvider, Exiting, SidebarProvider};
use crate::components::theme::ThemeProvider;
use crate::components::splash::{ShutdownScreen, SplashScreen};
use crate::components::toast::{ToastContainer, ToastProvider};
use views::{AdminPanel, Login, Projects, Settings, Users};

/// Define a components module that contains all shared components for our app.
mod components;
/// Define a views module that contains the UI for all Layouts and Routes for our app.
mod views;
/// Modelo de dominio compartido (serializable; docs de GuardianDB).
mod models;
/// Datos de demostración (target-neutral: siembra nativa + demo web).
mod demo;
/// Persistencia GuardianDB embebida + capa de servicios — solo nativo.
/// En web/wasm la UI conserva el comportamiento demo en memoria.
#[cfg(not(target_arch = "wasm32"))]
mod persistence;
/// Red de pares: mDNS en la red interna + conexión explícita, previa a la
/// apertura de los stores — solo nativo.
#[cfg(not(target_arch = "wasm32"))]
mod net;
#[cfg(not(target_arch = "wasm32"))]
mod services;
/// Nodo headless (`--node`) y alta como servicio del SO (`--install-service`,
/// `--uninstall-service`, `--service-status`) — solo nativo.
#[cfg(not(target_arch = "wasm32"))]
mod service;

/// Abre GuardianDB (data dir por defecto o `FEATHRAI_DATA_DIR`), siembra
/// demo/admin si el store está vacío y registra la base + el listado
/// inicial en los globals de `crate::persistence`.
///
/// Un runtime tokio propio (estático) para no depender del runtime de
/// dioxus: GuardianDB necesita su propio loop de I/O.
///
/// `log_failure` duplica el error de apertura en `<data dir>/featherai.log`:
/// lo pide el modo `--node` (en un Windows release no hay consola, y en Linux
/// el journal de la unidad no lo ve el usuario).
#[cfg(not(target_arch = "wasm32"))]
fn init_backend(log_failure: bool) -> Result<(), persistence::AppError> {
    init_backend_with_progress(log_failure, |_| {})
}

/// Igual que [`init_backend`], pero informa el avance (0–100) por `progress`.
///
/// La GUI usa esta variante desde un hilo aparte: la ventana ya está creada y
/// muestra el splash mientras la base se abre. El nodo headless (`--node`) usa
/// la variante sin callback.
#[cfg(not(target_arch = "wasm32"))]
fn init_backend_with_progress(
    log_failure: bool,
    mut progress: impl FnMut(u8),
) -> Result<(), persistence::AppError> {
    /// Runtime propio de la base (GuardianDB necesita su loop de I/O,
    /// independiente del de dioxus).
    static RT: std::sync::LazyLock<tokio::runtime::Runtime> = std::sync::LazyLock::new(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime tokio de featherai")
    });
    let rt: &'static tokio::runtime::Runtime = &RT;
    progress(2);
    // Antes de abrir la base: captura también las trazas de la apertura de
    // stores (importación de espacios, descubrimiento).
    init_network_log();
    let dir = persistence::default_data_dir();
    progress(5);
    let db = match open_store(rt, dir) {
        Ok(db) => db,
        Err(e) => {
            if log_failure {
                log_to_file(&format!("main: error abriendo GuardianDB: {e}"));
            }
            return Err(e);
        }
    };
    // `open_store` es la parte lenta (Iroh + stores): el salto de 5 a 55 es el
    // tramo donde el splash se queda quieto.
    progress(55);
    let initial = rt
        .block_on(services::project::list_projects(&db))
        .expect("listado inicial de proyectos");
    progress(75);
    let initial_users = rt
        .block_on(services::auth::list_users(&db))
        .expect("listado inicial de usuarios");
    progress(90);
    log_to_file(&format!("main: {}", db.peers_summary()));
    persistence::set_global(db, initial_users, initial);
    progress(97);
    // Opt-in: `FEATHRAI_SENTINEL_PORT=15433` expone el Admin RPC de sentinel
    // para `guardian-sentinel --connect` (inspección en vivo, sin tocar el
    // lock redb).
    persistence::maybe_spawn_admin_rpc();
    progress(100);
    Ok(())
}

/// Abre la base tolerando que el data dir esté tomado un instante.
///
/// El nodo del sistema y esta GUI son dueños del mismo data dir **por turnos**
/// ([`service::stop_node`]): si el lock todavía no se soltó, se reintenta ~2 s,
/// pero **sólo** si el error es de lock (ningún otro error se arregla
/// esperando).
#[cfg(not(target_arch = "wasm32"))]
fn open_store(
    rt: &tokio::runtime::Runtime,
    dir: std::path::PathBuf,
) -> Result<persistence::Db, persistence::AppError> {
    const INTENTOS: u32 = 6;
    const ESPERA: std::time::Duration = std::time::Duration::from_millis(400);

    let mut ultimo = None;
    for intento in 1..=INTENTOS {
        match rt.block_on(persistence::open(dir.clone())) {
            Ok(db) => return Ok(db),
            Err(e) => {
                if !is_lock_error(&e) {
                    return Err(e);
                }
                if intento < INTENTOS {
                    std::thread::sleep(ESPERA);
                }
                ultimo = Some(e);
            }
        }
    }
    Err(ultimo.expect("el loop de reintentos deja un error de lock"))
}

/// `true` si el error es "el data dir ya está abierto": los tres textos que
/// emite la cadena redb → guardian-db (`Cannot acquire lock.` es el de redb
/// 4.2, `already open` el de los stores, `os error 32` el *sharing violation*
/// de Windows).
#[cfg(not(target_arch = "wasm32"))]
fn is_lock_error(e: &persistence::AppError) -> bool {
    let texto = e.to_string();
    ["Cannot acquire lock", "already open", "os error 32"]
        .iter()
        .any(|marca| texto.contains(marca))
}

/// Cierra una apertura fallida con su código de salida: 3 si el data dir está
/// tomado (otra instancia o el servicio), 1 en cualquier otro caso.
///
/// El 3 no es decorativo: `RestartPreventExitStatus=3` en la unidad systemd
/// evita que el nodo entre en bucle de reinicio mientras la GUI tiene la base.
#[cfg(not(target_arch = "wasm32"))]
fn exit_on_open_failure(e: &persistence::AppError) -> ! {
    let mensaje = format!("[featherai] error abriendo GuardianDB: {e}");
    eprintln!("{mensaje}");
    log_to_file(&format!("main: {mensaje}"));
    if is_lock_error(e) {
        let pista = service::lock_hint();
        eprintln!("[featherai] {pista}");
        log_to_file(&format!("main: {pista}"));
        std::process::exit(3);
    }
    std::process::exit(1);
}

/// Imprime el resultado de `--install-service`/`--uninstall-service` y sale
/// con el código fijo (0 ok, 1 error).
#[cfg(not(target_arch = "wasm32"))]
fn report(resultado: Result<String, String>, accion: &str) -> ! {
    match resultado {
        Ok(mensaje) => {
            println!("{mensaje}");
            std::process::exit(0);
        }
        Err(e) => {
            let mensaje = format!("[featherai] no se pudo {accion} el servicio: {e}");
            eprintln!("{mensaje}");
            log_to_file(&format!("main: {mensaje}"));
            std::process::exit(1);
        }
    }
}

/// Modo `--node`: nodo headless (sin ventana) hasta SIGTERM/SIGINT.
///
/// Es el proceso que arranca el servicio del SO: abre **la misma** base y la
/// misma identidad iroh que la GUI y se queda vivo para replicar (endpoint
/// Iroh + mDNS + stores) sin crear ventana ni tocar dioxus/webkit.
#[cfg(not(target_arch = "wasm32"))]
fn node_main() -> ! {
    init_network_log();
    let (data_dir, migracion) = persistence::prepare_data_dir();
    log_to_file(&format!(
        "main: nodo headless (pid={}, exe={:?})",
        std::process::id(),
        std::env::current_exe()
    ));
    log_to_file(&format!("main: data dir {data_dir:?}"));
    for nota in &migracion {
        log_to_file(nota);
    }
    if let Err(e) = init_backend(true) {
        exit_on_open_failure(&e);
    }
    let db = persistence::db();
    log_to_file(&format!(
        "main: nodo headless listo (data dir {})",
        db.data_dir().display()
    ));
    db.rt.block_on(wait_for_shutdown_signal());
    log_to_file("main: nodo detenido por señal");
    // Presupuesto acotado, menor que el `TimeoutStopSec=6` de la unidad: el
    // shutdown de Iroh tarda ~15 s y el arranque de la GUI no puede esperar
    // todo eso. redb es ACID y el SO libera los locks al salir, así que cortar
    // acá no pierde nada (el que reabra reconverge por iroh-docs).
    //
    // El `timeout` se construye **dentro** del futuro a propósito:
    // `Handle::block_on` establece el contexto de runtime al *pollear*, no
    // antes, y `Sleep::new_timeout` panickea sin él ("there is no reactor
    // running").
    let _ = db.rt.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(3), db.close()).await
    });
    std::process::exit(0);
}

/// Cierre ordenado: cierra la base (flush + liberación de los locks), le
/// devuelve el data dir al nodo y termina el proceso.
///
/// Bloquea varios segundos (shutdown de Iroh): desde la UI se llama en un hilo
/// aparte, con el splash de cierre visible; desde la señal del SO se llama
/// directo.
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn close_and_exit() -> ! {
    log_to_file("main: cierre ordenado");
    if let Some(db) = persistence::try_db() {
        db.close_blocking();
        log_to_file("main: base cerrada; locks liberados");
    }
    if let Err(e) = service::start_node() {
        let mensaje = format!("main: no pude volver a arrancar el nodo del servicio: {e}");
        log_to_file(&mensaje);
        eprintln!("[featherai] {e}");
    }
    std::process::exit(0);
}

/// Inicia el cierre desde la UI: muestra el splash de cierre y, después de un
/// frame, cierra la base en un hilo aparte para no congelar la ventana.
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub fn begin_shutdown(mut exiting: Signal<bool>) {
    if exiting() {
        return;
    }
    exiting.set(true);
    // Hilo propio, no `spawn` de Dioxus: `exiting` reemplaza el router por el
    // `ShutdownScreen`, y con eso se desmonta el `UserMenuItems` desde donde se
    // disparó “Salir”. Una tarea de Dioxus atada a ese scope se cancelaría antes
    // de llegar a cerrar. Con un hilo propio, el cierre sobrevive al desmontaje.
    std::thread::spawn(|| {
        // Dejar que el webview pinte el `ShutdownScreen` antes de arrancar el
        // cierre, que bloquea varios segundos.
        std::thread::sleep(std::time::Duration::from_millis(80));
        close_and_exit();
    });
}

/// Instala el cierre ordenado de la GUI una vez que la base ya está abierta.
///
/// Se llama desde `App` **después** de `init_backend`: el handler usa
/// `persistence::db()`, que panickea antes de `set_global`. Al moverlo acá, la
/// ventana puede abrirse y mostrar el splash sin esperar a GuardianDB.
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn install_shutdown_handler() {
    persistence::db().rt.spawn(async {
        wait_for_shutdown_signal().await;
        log_to_file("main: señal de cierre recibida");
        close_and_exit();
    });
}

/// Espera la señal de cierre del SO.
///
/// En la GUI es lo que garantiza que el nodo vuelva aunque el proceso no salga
/// por el menú `Salir` (SIGTERM del gestor de sesión o de `systemctl`, Ctrl+C);
/// en el modo `--node` es la única forma de salir.
#[cfg(not(target_arch = "wasm32"))]
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut terminar = match signal(SignalKind::terminate()) {
            Ok(signal) => signal,
            // Sin SIGTERM (raro): al menos Ctrl+C.
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        match signal(SignalKind::interrupt()) {
            Ok(mut interrumpir) => tokio::select! {
                _ = terminar.recv() => {}
                _ = interrumpir.recv() => {}
                _ = tokio::signal::ctrl_c() => {}
            },
            Err(_) => tokio::select! {
                _ = terminar.recv() => {}
                _ = tokio::signal::ctrl_c() => {}
            },
        }
    }
    #[cfg(windows)]
    {
        // Windows no tiene SIGTERM.
        let _ = tokio::signal::ctrl_c().await;
    }
    #[cfg(not(any(unix, windows)))]
    {
        std::future::pending::<()>().await;
    }
}

/// The Route enum is used to define the structure of internal routes in our app. All route enums need to derive
/// the [`Routable`] trait, which provides the necessary methods for the router to work.
///
/// Each variant represents a different URL pattern that can be matched by the router. If that pattern is matched,
/// the components for that route will be rendered.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    // La primera pantalla es el login: "/" y "/login" renderizan la misma
    // vista. Atención: dioxus-router 0.7 solo registra el PRIMER `#[route]`
    // de cada variante (los demás se ignoran en silencio), así que cada ruta
    // es una variante; "/" usa `comp_name` para apuntar a `Login`.
    #[route("/login")]
    Login {},
    #[route("/", Login)]
    LoginIndex {},
    #[route("/settings")]
    Settings {},

    // Panel de administración. La raíz admin redirige a /admin/projects.
    #[route("/admin")]
    AdminPanel {},
    #[route("/admin/projects")]
    Projects {},
    #[route("/admin/users")]
    Users {},

}

// We can import assets in dioxus with the `asset!` macro. This macro takes a path to an asset relative to the crate root.
// The macro returns an `Asset` type that will display as the path to the asset in the browser or a local path in desktop bundles.
const FAVICON: Asset = asset!("/assets/favicon.ico");
// Bootstrap Icons + el CSS del port de feathrai-frontend
const BOOTSTRAP_ICONS_CSS: Asset = asset!("/assets/styling/bootstrap-icons.min.css");
// El asset macro también minifica algunos assets como CSS y JS para hacer el bundle más chico
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
// Fuentes de iconos (url() relativa dentro de bootstrap-icons.min.css)
const ICONS_WOFF2: Asset = asset!("/assets/styling/fonts/bootstrap-icons.woff2");
const ICONS_WOFF: Asset = asset!("/assets/styling/fonts/bootstrap-icons.woff");

/// Agrega una línea a `<data dir>/featherai.log`.
///
/// Un lanzamiento desde el gestor de archivos (doble click) no tiene terminal:
/// stdout/stderr van a un socket que nadie lee, así que un fallo ahí no deja
/// rastro visible. Este archivo (más el hook de panic de
/// [`install_diagnostics`]) es lo que permite diagnosticarlo.
#[cfg(not(target_arch = "wasm32"))]
fn log_to_file(msg: &str) {
    use std::io::Write;

    let path = crate::persistence::default_data_dir().join("featherai.log");
    if let Some(parent) = path.parent() {
        // En el primer arranque el data dir todavía no existe (lo crea
        // GuardianDB), y sin él `open` falla y se perdería la línea.
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = writeln!(file, "[{secs}] {msg}");
}

/// Aplica el workaround de renderizado de WebKitGTK con el driver propietario
/// de NVIDIA: en X11 (`WEBKIT_DISABLE_DMABUF_RENDERER=1`) y en Wayland sin
/// `egl-wayland2` (`__NV_DISABLE_EXPLICIT_SYNC=1`).
///
/// Sin esto la app lanzada desde el menú del sistema o desde el `.AppImage`
/// abre con la ventana en gris y sin componentes: esas vías no heredan lo que
/// el usuario exporta en su shell (el clásico `WEBKIT_DISABLE_DMABUF_RENDERER=1`
/// en `.zshrc`), y el renderer DMA-BUF de WebKit falla con ese driver.
/// Referencias: tauri-apps/tauri#9304, bugs.webkit.org #280210.
///
/// Debe correr **antes** de crear la ventana (WebKit lee las variables al
/// inicializar el webview) y no toca nada si el usuario ya definió alguna de
/// las dos: esa configuración manda.
#[cfg(target_os = "linux")]
fn quirk_webkit() {
    let definido_por_el_usuario = [
        "WEBKIT_DISABLE_DMABUF_RENDERER",
        "__NV_DISABLE_EXPLICIT_SYNC",
    ]
    .iter()
    .any(|var| std::env::var(var).is_ok_and(|valor| !valor.trim().is_empty()));
    if !definido_por_el_usuario {
        webkit2gtk_nvidia_quirk::apply_workaround_with_options(Default::default());
    }
}

/// Log de red opt-in: `FEATHRAI_LOG=info|debug|trace` (o `1`) agrega las
/// trazas de iroh/guardian-db a `<data dir>/featherai.log`.
///
/// Es lo que permite diagnosticar la red de pares sin recompilar: qué pares se
/// descubren/conectan y si este nodo importó un espacio compartido o creó uno
/// propio ("possible split-brain" en el log significa que no encontró pares al
/// abrir). Sin la variable no se instala ningún subscriber: cero costo.
#[cfg(not(target_arch = "wasm32"))]
fn init_network_log() {
    use std::str::FromStr;
    use tracing_subscriber::filter::LevelFilter;

    let Some(valor) = std::env::var("FEATHRAI_LOG").ok().filter(|v| !v.trim().is_empty()) else {
        return;
    };
    let nivel = match valor.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "info" => LevelFilter::INFO,
        "debug" => LevelFilter::DEBUG,
        "trace" => LevelFilter::TRACE,
        // Valor no reconocido: no se toca nada (ni siquiera `warn`).
        _ => LevelFilter::from_str(&valor).unwrap_or(LevelFilter::INFO),
    };

    let path = crate::persistence::default_data_dir().join("featherai.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let writer = move || {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap_or_else(|_| std::fs::File::create("/dev/null").expect("null"))
    };
    // `try_init`: si otro subscriber ya está instalado (tests, DX), se ignora.
    let _ = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(writer)
        .with_max_level(nivel)
        .try_init();
}

#[cfg(all(not(target_os = "linux"), not(target_arch = "wasm32")))]
fn quirk_webkit() {}

/// Duplica los panics en `<data dir>/featherai.log` (stderr puede irse a un
/// socket que nadie lee, como en el doble click de un `.AppImage`).
#[cfg(not(target_arch = "wasm32"))]
fn install_diagnostics() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_hook(info);
        log_to_file(&format!("PANIC: {info}"));
    }));
}

fn main() {
    // Backend nativo (GuardianDB embebida). En web/wasm no existe y la UI
    // corre con el estado demo en memoria.
    #[cfg(not(target_arch = "wasm32"))]
    {
        install_diagnostics();
        // Subcomandos de nodo/servicio: se resuelven acá, antes de crear la
        // ventana y antes de tocar el data dir (el modo `--node` no crea
        // ventana ninguna; los otros terminan con `exit`).
        match service::action_from_args(std::env::args().skip(1)) {
            Some(service::Action::Status) => {
                println!("{}", service::status_line());
                std::process::exit(0);
            }
            Some(service::Action::Install) => report(service::install(), "instalar"),
            Some(service::Action::Uninstall) => report(service::uninstall(), "quitar"),
            Some(service::Action::Node) => node_main(),
            None => {}
        }
        // Antes de crear la ventana: WebKit lee estas variables al inicializar
        // el webview (sin esto, ventana gris con driver NVIDIA + X11).
        quirk_webkit();
        // El arranque pesado (stop_node, preparación del data dir y apertura
        // de GuardianDB) corre en `App`, en un hilo aparte: así la ventana
        // aparece primero y muestra el splash. Acá sólo se registran los datos
        // de la corrida y se lanza la ventana.
        log_to_file(&format!(
            "main: inicio (cwd={:?}, exe={:?})",
            std::env::current_dir(),
            std::env::current_exe()
        ));
        #[cfg(target_os = "linux")]
        log_to_file(&format!(
            "main: webkit (WEBKIT_DISABLE_DMABUF_RENDERER={:?}, __NV_DISABLE_EXPLICIT_SYNC={:?})",
            std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok(),
            std::env::var("__NV_DISABLE_EXPLICIT_SYNC").ok()
        ));
    }

    // Ventana maximizada y sin barra de título/menú. Al no haber botón de
    // cierre nativo, el cierre se dispara desde la UI (menú de usuario y
    // botón flotante).
    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("featherai")
                    .with_decorations(false)
                    .with_maximized(true),
            )
        })
        .launch(App);
}

/// Eventos que el hilo de arranque manda al splash.
///
/// Los `Signal` de Dioxus no son `Send`, así que el hilo no los toca: sólo
/// manda eventos por un canal y el listener de la UI actualiza las señales.
#[cfg(not(target_arch = "wasm32"))]
enum StartupEvent {
    Progress(u8),
    Ready,
    Error(String),
}

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each component is a function
/// that takes some props and returns an Element. In this case, App takes no props because it is the root of our app.
///
/// Components should be annotated with `#[component]` to support props, better error messages, and autocomplete
#[component]
fn App() -> Element {
    // Estado del arranque. En nativo, la base y el nodo se abren en un hilo
    // aparte mientras la ventana ya está creada y muestra el splash. En web no
    // hay backend: se pasa directo al router.
    let mut ready = use_signal(|| false);
    let mut progress = use_signal(|| 0u8);
    let mut error = use_signal(|| None::<String>);
    // `Salir` (UserMenu) activa esta señal: `App` pinta el splash de cierre y
    // `begin_shutdown` cierra la base en un hilo aparte.
    let exiting = use_signal(|| false);
    use_context_provider(|| Exiting(exiting));

    #[cfg(not(target_arch = "wasm32"))]
    {
        use_effect(move || {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StartupEvent>();
            std::thread::spawn(move || {
                // La GUI pasa a ser el nodo: si el servicio está instalado,
                // suelta el data dir y espera a que el proceso termine.
                if let Err(e) = service::stop_node() {
                    let mensaje = format!("main: no pude detener el nodo del servicio: {e}");
                    log_to_file(&mensaje);
                    eprintln!("[featherai] {e}");
                }
                let (data_dir, migracion) = persistence::prepare_data_dir();
                log_to_file(&format!("main: data dir {data_dir:?}"));
                for nota in &migracion {
                    log_to_file(nota);
                }
                let tx_progress = tx.clone();
                match init_backend_with_progress(false, move |p| {
                    let _ = tx_progress.send(StartupEvent::Progress(p));
                }) {
                    Ok(()) => {
                        log_to_file("main: backend GuardianDB listo; lanzando la ventana");
                        // Con la base abierta, un cierre que no pasa por el
                        // menú `Salir` (SIGTERM del gestor de sesión o de
                        // systemctl) tiene que cerrar la base y devolverle el
                        // data dir al nodo.
                        //
                        // Ojo: en builds **debug** `dioxus-desktop` instala su
                        // propio handler de SIGTERM/SIGINT y termina el proceso
                        // en ~30 ms, así que ahí este cierre puede quedar
                        // cortado; en release no existe ese handler.
                        #[cfg(feature = "desktop")]
                        install_shutdown_handler();
                        let _ = tx.send(StartupEvent::Ready);
                    }
                    Err(e) => {
                        let mensaje = format!("[featherai] error abriendo GuardianDB: {e}");
                        log_to_file(&format!("main: {mensaje}"));
                        eprintln!("{mensaje}");
                        let _ = tx.send(StartupEvent::Error(mensaje));
                    }
                }
            });
            // El hilo manda el avance por el canal; acá se refleja en la señal
            // que lee el splash. Los `Signal` de Dioxus no son `Send`: por eso
            // se actualizan en este listener (hilo de la UI) y no en el hilo.
            spawn(async move {
                while let Some(event) = rx.recv().await {
                    match event {
                        StartupEvent::Progress(p) => progress.set(p),
                        StartupEvent::Ready => ready.set(true),
                        StartupEvent::Error(mensaje) => error.set(Some(mensaje)),
                    }
                }
            });
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        use_effect(move || {
            ready.set(true);
        });
    }

    rsx! {
        // In addition to element and text (which we will see later), rsx can contain other components. In this case,
        // we are using the `document::Link` component to add a link to our favicon and CSS files into the head of our app.
        document::Link { rel: "icon", href: FAVICON }

        // Bootstrap Icons (mismo sistema visual que feathrai-frontend)
        document::Link { rel: "stylesheet", href: BOOTSTRAP_ICONS_CSS }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        // Preload de las fuentes de iconos (además las registra en el bundle desktop)
        document::Link { rel: "preload", href: ICONS_WOFF2, r#as: "font", crossorigin: "anonymous", r#type: "font/woff2" }
        document::Link { rel: "preload", href: ICONS_WOFF, r#as: "font", crossorigin: "anonymous", r#type: "font/woff" }

        // Tipografías del port (mismas URLs que index.html de feathrai-frontend)
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Cormorant+Garamond:wght@300;400&family=DM+Sans:ital,opsz,wght@0,9..40,300;0,9..40,400;0,9..40,500&display=swap",
        }

        if exiting() {
            ShutdownScreen {}
        } else if let Some(mensaje) = error() {
            div {
                style: "position:fixed; inset:0; display:flex; align-items:center; justify-content:center; background-color:var(--bg-primary,#fff); color:var(--text-primary,#212529); z-index:9999; padding:1rem;",
                div {
                    style: "display:flex; flex-direction:column; align-items:center; gap:0.75rem; width:min(32rem, calc(100% - 2rem)); padding:2rem; text-align:center;",
                    h1 {
                        style: "margin:0; color:var(--danger-color,#dc3545); font-size:1.25rem; font-weight:600;",
                        "No se pudo iniciar la aplicación"
                    }
                    p {
                        style: "margin:0; color:var(--text-secondary,#6c757d); font-size:0.9rem; overflow-wrap:anywhere;",
                        "{mensaje}"
                    }
                }
            }
        } else if !ready() {
            SplashScreen { progress: progress() }
        } else {
            // Los proveedores de estado (equivalentes de AuthProvider,
            // ThemeProvider de feathrai-frontend) envuelven al router para que
            // el Sidebar y el UserMenu los consuman.
            ToastProvider {
                ThemeProvider {
                    AuthProvider {
                        SidebarProvider {
                            // The router component renders the route enum we defined above. It will handle synchronization of the URL and render
                            // the layouts and components for the active route.
                            Router::<Route> {}
                        }
                    }
                }
                // Contenedor de notificaciones (equiv. de ToastContainer de React).
                ToastContainer {}
            }
        }
    }
}
