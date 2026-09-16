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

use crate::components::layout::{AuthProvider, SidebarProvider};
use crate::components::theme::ThemeProvider;
use crate::components::toast::{ToastContainer, ToastProvider};
use views::{AdminPanel, Login, Projects, Settings};

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

/// Abre GuardianDB (data dir por defecto o `FEATHRAI_DATA_DIR`), siembra
/// demo/admin si el store está vacío y registra la base + el listado
/// inicial en los globals de `crate::persistence`.
///
/// Un runtime tokio propio (estático) para no depender del runtime de
/// dioxus: GuardianDB necesita su propio loop de I/O.
#[cfg(not(target_arch = "wasm32"))]
fn init_backend() {
    use std::sync::OnceLock;

    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let rt = RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime tokio de featherai")
    });
    // Antes de abrir la base: captura también las trazas de la apertura de
    // stores (importación de espacios, descubrimiento).
    init_network_log();
    let dir = persistence::default_data_dir();
    let db = rt
        .block_on(persistence::open(dir))
        .unwrap_or_else(|e| {
            eprintln!("[featherai] error abriendo GuardianDB: {e}");
            std::process::exit(1);
        });
    let initial = rt
        .block_on(services::project::list_projects(&db))
        .expect("listado inicial de proyectos");
    log_to_file(&format!("main: {}", db.peers_summary()));
    persistence::set_global(db, initial);
    // Opt-in: `FEATHRAI_SENTINEL_PORT=15433` expone el Admin RPC de sentinel
    // para `guardian-sentinel --connect` (inspección en vivo, sin tocar el
    // lock redb).
    persistence::maybe_spawn_admin_rpc();
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
        // Antes de crear la ventana: WebKit lee estas variables al inicializar
        // el webview (sin esto, ventana gris con driver NVIDIA + X11).
        quirk_webkit();
        // Antes de escribir el log: si la base quedó en un data dir heredado
        // (`$HOME/.local/share/featherai` en cualquier SO, o `%APPDATA%\featherai`),
        // se mueve al data dir del SO (ver `persistence::prepare_data_dir`).
        let (data_dir, migracion) = persistence::prepare_data_dir();
        log_to_file(&format!(
            "main: inicio (cwd={:?}, exe={:?})",
            std::env::current_dir(),
            std::env::current_exe()
        ));
        log_to_file(&format!("main: data dir {data_dir:?}"));
        #[cfg(target_os = "linux")]
        log_to_file(&format!(
            "main: webkit (WEBKIT_DISABLE_DMABUF_RENDERER={:?}, __NV_DISABLE_EXPLICIT_SYNC={:?})",
            std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").ok(),
            std::env::var("__NV_DISABLE_EXPLICIT_SYNC").ok()
        ));
        for nota in &migracion {
            log_to_file(nota);
        }
        init_backend();
        log_to_file("main: backend GuardianDB listo; lanzando la ventana");
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

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each component is a function
/// that takes some props and returns an Element. In this case, App takes no props because it is the root of our app.
///
/// Components should be annotated with `#[component]` to support props, better error messages, and autocomplete
#[component]
fn App() -> Element {
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
