use crate::components::layout::state::{use_auth, use_current_path, use_sidebar, SIDEBAR_SECTIONS};
use dioxus::prelude::*;

/// Barra lateral de navegación — port de `Sidebar.tsx` de feathrai-frontend.
///
/// Fija a la izquierda (CSS `.app-sidebar`), colapsable a iconos (estado
/// compartido vía [`use_sidebar`]) y con el menú de usuario en el pie.
#[component]
pub fn Sidebar() -> Element {
    let mut collapsed = use_sidebar();
    let navigator = use_navigator();
    let current = use_current_path();

    if !use_auth().read().is_authenticated {
        return VNode::empty();
    }

    let is_active = move |to: &str| {
        let path = current();
        path == to || path.starts_with(&format!("{to}/"))
    };

    rsx! {
        div {
            class: if collapsed() { "app-sidebar collapsed" } else { "app-sidebar expanded" },

            // ── Header: título + botón de colapso ──
            div { class: "app-sidebar-header",
                div { class: "app-sidebar-title",
                    i { class: "bi bi-kanban me-2" }
                    span { "featherpro" }
                }
                div { class: "app-sidebar-header-buttons",
                    button {
                        class: "app-sidebar-toggle-btn",
                        title: if collapsed() { "Expandir sidebar" } else { "Colapsar sidebar" },
                        aria_label: if collapsed() { "Expandir sidebar" } else { "Colapsar sidebar" },
                        onclick: move |_| *collapsed.write() = !collapsed(),
                        i { class: if collapsed() { "bi bi-chevron-right" } else { "bi bi-chevron-left" } }
                    }
                }
            }

            // ── Secciones de navegación ──
            div { class: "app-sidebar-content",
                for section in SIDEBAR_SECTIONS {
                    div { key: "{section.title}", class: "app-sidebar-section",
                        div { class: "app-sidebar-section-title", span { "{section.title}" } }
                        for link in section.items {
                            button {
                                key: "{link.to}",
                                class: format!(
                                    "app-sidebar-item {}",
                                    if is_active(link.to) { "active" } else { "" },
                                ),
                                title: link.label,
                                onclick: move |_| {
                                    navigator.push(link.to);
                                },
                                i { class: link.icon.class() }
                                span { class: "app-sidebar-item-text", "{link.label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
