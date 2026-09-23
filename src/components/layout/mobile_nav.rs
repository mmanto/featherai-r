//! Navegación móvil — tab bar inferior + hoja de cuenta.
//!
//! En viewports ≤ 768px el shell abandona el patrón de escritorio (sidebar fija
//! y ActionBar flotante, las dos ocultas por CSS en `assets/styling/main.css`) y
//! navega con el patrón estándar de móvil: una barra de pestañas fija al pie con
//! las mismas secciones de [`NAV_LINKS`] y una pestaña Cuenta que abre una hoja
//! inferior con los ítems de [`UserMenuItems`].
//!
//! El breakpoint es CSS puro: este componente se renderiza siempre y el media
//! query decide si se ve.

use crate::components::layout::state::{use_auth, use_current_path, NAV_LINKS};
use crate::components::layout::user_menu::UserMenuItems;
use dioxus::prelude::*;

/// Barra de navegación inferior de móvil (equivalente móvil del `Sidebar`) con
/// la hoja de acciones de cuenta.
#[component]
pub fn MobileNav() -> Element {
    let navigator = use_navigator();
    let current = use_current_path();
    let mut sheet_open = use_signal(|| false);

    // Cerrar al cambiar de ruta — mismo contrato que el `UserMenu` de
    // escritorio: cubre el botón atrás del sistema (la ruta cambia sin pasar
    // por un ítem de la hoja).
    use_effect(move || {
        let _ = current();
        *sheet_open.write() = false;
    });

    // Igual que el `Sidebar`: sin sesión no hay navegación (la hoja expone
    // cerrar sesión/salir).
    if !use_auth().read().is_authenticated {
        return VNode::empty();
    }

    let is_active = move |to: &str| {
        let path = current();
        path == to || path.starts_with(&format!("{to}/"))
    };

    rsx! {
        // ── Hoja de cuenta: acciones del UserMenu en formato táctil ──
        if sheet_open() {
            div {
                class: "mobile-sheet-backdrop",
                aria_hidden: "true",
                onclick: move |_| *sheet_open.write() = false,
            }
            div {
                class: "mobile-sheet",
                role: "dialog",
                aria_modal: "true",
                aria_label: "Cuenta",
                div { class: "mobile-sheet-handle" }
                ul {
                    UserMenuItems { on_action: move |_| *sheet_open.write() = false }
                }
            }
        }

        // ── Tab bar ──
        nav { class: "mobile-nav", aria_label: "Navegación principal",
            for link in NAV_LINKS {
                button {
                    key: "{link.to}",
                    class: format!(
                        "mobile-nav-item {}",
                        if is_active(link.to) { "active" } else { "" },
                    ),
                    aria_current: if is_active(link.to) { "page" } else { "false" },
                    onclick: move |_| {
                        navigator.push(link.to);
                    },
                    i { class: link.icon.class() }
                    span { class: "mobile-nav-label", "{link.label}" }
                }
            }
            button {
                class: format!(
                    "mobile-nav-item {}",
                    if sheet_open() { "active" } else { "" },
                ),
                aria_haspopup: "dialog",
                aria_expanded: sheet_open().to_string(),
                onclick: move |_| *sheet_open.write() = !sheet_open(),
                i { class: "bi bi-person-circle" }
                span { class: "mobile-nav-label", "Cuenta" }
            }
        }
    }
}
