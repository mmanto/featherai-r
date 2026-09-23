use crate::components::layout::sidebar::Sidebar;
use crate::components::layout::{ActionBar, MobileNav};
use dioxus::prelude::*;

/// Shell de la app — port de `Layout.tsx` de feathrai-frontend.
///
/// Renderiza el [`Sidebar`] como hermano del contenedor para que el selector
/// CSS `.app-sidebar.collapsed ~ .layout-container .layout-main-wrapper`
/// ajuste el margen, y el área principal (scroll) con el contenido como
/// `children`.
///
/// Cierra con [`MobileNav`]: la navegación de móvil (≤ 768px), que el CSS
/// muestra en lugar del sidebar y de la ActionBar.
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
                ActionBar {}
            }
        }
        MobileNav {}
    }
}
