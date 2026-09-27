use crate::components::layout::state::{
    NavLink, SIDEBAR_SECTIONS, use_auth, use_current_path, use_sidebar,
};
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
    let auth = use_auth();

    if !auth.read().is_authenticated {
        return VNode::empty();
    }

    // Ítems que el rol de la sesión puede abrir (ver `NavLink::roles`); las
    // secciones sin ítems visibles no se dibujan (el título solo quedaría).
    let role = auth
        .read()
        .user
        .as_ref()
        .map(|u| u.role.clone())
        .unwrap_or_default();
    let mut sections: Vec<(&str, Vec<NavLink>)> = Vec::new();
    for section in SIDEBAR_SECTIONS {
        let items: Vec<NavLink> = section
            .items
            .iter()
            .filter(|link| link.visible_for(&role))
            .copied()
            .collect();
        if !items.is_empty() {
            sections.push((section.title, items));
        }
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
                for (title, items) in sections {
                    div { key: "{title}", class: "app-sidebar-section",
                        div { class: "app-sidebar-section-title", span { "{title}" } }
                        for link in items {
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
