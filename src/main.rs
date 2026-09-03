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

/// The Route enum is used to define the structure of internal routes in our app. All route enums need to derive
/// the [`Routable`] trait, which provides the necessary methods for the router to work.
///
/// Each variant represents a different URL pattern that can be matched by the router. If that pattern is matched,
/// the components for that route will be rendered.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    // La primera pantalla es el login: "/" y "/login" renderizan la misma vista.
    #[route("/")]
    #[route("/login")]
    Login {},
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

fn main() {
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
