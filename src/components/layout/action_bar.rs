//! Mini barra de acciones flotante — esquina inferior derecha del shell.
//!
//! Mismo patrón que la `ActionBar` de xmusic: un contenedor `position: fixed`
//! con botones cuadrados `.action-btn` (estilos en `assets/styling/main.css`).
//! Hoy aloja el [`UserMenu`] (sistema y configuración); cada acción nueva se
//! agrega como hermana dentro de `div.action-bar`.

use crate::components::layout::user_menu::UserMenu;
use dioxus::prelude::*;

/// Barra flotante de acciones del shell.
#[component]
pub fn ActionBar() -> Element {
    rsx! {
        div { class: "action-bar", aria_label: "Acciones de la aplicación",
            UserMenu {}
        }
    }
}
