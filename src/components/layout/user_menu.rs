use crate::components::layout::state::{exit_app, logout, use_auth, use_current_path};
use crate::components::theme::{use_theme, ThemeId};
use dioxus::prelude::*;

/// Ítems del menú de usuario — cabecera de perfil (usuario + email), toggle
/// claro/oscuro, Ajustes, Cerrar sesión y Salir.
///
/// Compartidos por el desplegable de escritorio ([`UserMenu`], anclado en la
/// barra de acciones flotante) y por la hoja de cuenta de la navegación móvil
/// (`MobileNav`): el contenedor decide qué hacer cuando se activa un ítem
/// (`on_action`). El toggle de tema no lo dispara — el cambio se ve en el mismo
/// menú abierto.
#[component]
pub fn UserMenuItems(on_action: EventHandler<()>) -> Element {
    let auth = use_auth();
    let mut theme = use_theme();
    let navigator = use_navigator();

    let user = auth.read().user.clone();
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "Admin".to_string());
    let email = user.as_ref().and_then(|u| u.email.clone());

    let is_dark = theme() == ThemeId::Dark;

    rsx! {
        li {
            h6 { class: "px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-[var(--text-secondary)]", "Perfil" }
        }
        li {
            span { class: "px-3 py-1.5 text-sm text-[var(--text-primary)]", "{username}" }
        }
        if let Some(email) = email {
            li {
                span { class: "px-3 py-1.5 text-xs text-[var(--text-secondary)]", "{email}" }
            }
        }
        li {
            hr { class: "my-1 border-t border-[var(--border-color)]" }
        }
        li {
            button {
                class: "dropdown-item flex w-full items-center gap-2 px-3 py-1.5 text-sm text-[var(--text-primary)] hover:bg-[var(--bg-secondary)]",
                onclick: move |_| *theme.write() = theme().toggled(),
                i { class: format!("bi {}", if is_dark { "bi-sun" } else { "bi-moon-stars" }) }
                span { if is_dark { "Cambiar a claro" } else { "Cambiar a oscuro" } }
            }
        }
        li {
            hr { class: "my-1 border-t border-[var(--border-color)]" }
        }
        li {
            button {
                class: "dropdown-item flex w-full items-center gap-2 px-3 py-1.5 text-sm text-[var(--text-primary)] hover:bg-[var(--bg-secondary)]",
                onclick: move |_| {
                    on_action.call(());
                    navigator.push("/settings");
                },
                i { class: "bi bi-gear me-2" }
                "Ajustes"
            }
        }
        li {
            button {
                class: "dropdown-item flex w-full items-center gap-2 px-3 py-1.5 text-sm text-[var(--text-primary)] hover:bg-[var(--bg-secondary)]",
                onclick: move |_| {
                    on_action.call(());
                    logout(auth);
                    navigator.push("/login");
                },
                i { class: "bi bi-box-arrow-right me-2" }
                "Cerrar sesión"
            }
        }
        li {
            hr { class: "my-1 border-t border-[var(--border-color)]" }
        }
        li {
            button {
                class: "dropdown-item flex w-full items-center gap-2 px-3 py-1.5 text-sm text-[var(--danger-color)] hover:bg-[var(--bg-secondary)]",
                onclick: move |_| {
                    on_action.call(());
                    exit_app();
                },
                i { class: "bi bi-power me-2" }
                "Salir"
            }
        }
    }
}

/// Menú desplegable de usuario — port de `UserMenu.tsx`/`UserMenu` de xmusic.
///
/// Ancla en la barra de acciones flotante (esquina inferior derecha). El
/// disparador es un botón cuadrado `.action-btn` con icono de engranaje y el
/// desplegable son los [`UserMenuItems`]. Cierra al hacer click fuera del menú
/// o al navegar.
///
/// Patrón de escritorio: en móvil la ActionBar queda oculta por CSS y esas
/// acciones viven en la hoja de cuenta de `MobileNav`.
#[component]
pub fn UserMenu() -> Element {
    let mut menu_open = use_signal(|| false);
    let current = use_current_path();

    // Cerrar al cambiar de ruta — equivalente del `useEffect(..., [pathname])`.
    use_effect(move || {
        let _ = current();
        *menu_open.write() = false;
    });

    // Cerrar al hacer click fuera — listener global `mousedown` (mismo
    // mecanismo que el port de gestion.ar, reutilizando `dioxus.send`).
    use_effect(move || {
        let eval = document::eval(
            r#"if (!window.__featherUsermenuInstalled) { window.__featherUsermenuInstalled = true; window.__featherUsermenuHandler = (ev) => { const t = ev.target; if (!(t && t.closest) || !t.closest('[data-feather-usermenu]')) dioxus.send('outside'); }; document.addEventListener('mousedown', window.__featherUsermenuHandler); }"#,
        );
        spawn(async move {
            let (mut eval, mut menu_open) = (eval, menu_open);
            while let Ok(msg) = eval.recv::<String>().await {
                if msg == "outside" {
                    *menu_open.write() = false;
                }
            }
        });
    });
    use_drop(|| {
        let _ = document::eval(
            "if (window.__featherUsermenuInstalled) { document.removeEventListener('mousedown', window.__featherUsermenuHandler); window.__featherUsermenuInstalled = false; }",
        );
    });

    rsx! {
        div {
            "data-feather-usermenu": "true",
            class: "relative",

            button {
                class: "action-btn action-btn-menu",
                title: "Sistema y configuración",
                aria_label: "Sistema y configuración",
                aria_expanded: menu_open().to_string(),
                onclick: move |_| *menu_open.write() = !menu_open(),
                i { class: "bi bi-gear" }
            }

            ul {
                class: format!(
                    "absolute bottom-full right-0 mb-4 z-[1100] min-w-[12rem] rounded-md border border-[var(--card-border)] bg-[var(--card-bg)] py-1 shadow-[var(--shadow)] {}",
                    if menu_open() { "" } else { "hidden" },
                ),
                UserMenuItems { on_action: move |_| *menu_open.write() = false }
            }
        }
    }
}
