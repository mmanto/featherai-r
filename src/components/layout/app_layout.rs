use crate::components::layout::sidebar::Sidebar;
use crate::components::layout::state::exit_app;
use dioxus::prelude::*;

/// Shell de la app — port de `Layout.tsx` de feathrai-frontend.
///
/// Renderiza el [`Sidebar`] como hermano del contenedor para que el selector
/// CSS `.app-sidebar.collapsed ~ .layout-container .layout-main-wrapper`
/// ajuste el margen, y el área principal (scroll) con el contenido como
/// `children`.
#[component]
pub fn AppLayout(children: Element) -> Element {
    rsx! {
        Sidebar {}
        div { class: "layout-container",
            div { class: "layout-main-wrapper",
                main { class: "main-content container-fluid px-4 pt-4",
                    div { class: "mx-3 mt-3",
                        {children}
                    }
                }
                button {
                    class: "inline-flex items-center justify-center rounded-full bg-[var(--danger-color)] text-white border-0 hover:opacity-90",
                    style: "position:fixed;bottom:1.25rem;right:1.25rem;width:3.25rem;height:3.25rem;z-index:1000;box-shadow:0 2px 8px rgba(0,0,0,0.2);cursor:pointer;",
                    title: "Salir de la aplicación",
                    onclick: move |_| exit_app(),
                    i { class: "bi bi-power", style: "font-size:1.35rem" }
                }
            }
        }
    }
}
