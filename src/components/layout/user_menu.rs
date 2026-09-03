use crate::components::layout::state::{exit_app, logout, use_auth, use_current_path};
use crate::components::theme::{use_theme, ThemeId};
use dioxus::prelude::*;

/// Menú desplegable de usuario — port de `UserMenu.tsx` de feathrai-frontend.
///
/// Disparador Bootstrap (`btn dropdown-toggle` con `bi-person-circle`) y menú
/// con cabecera de perfil, toggle claro/oscuro, Ajustes y Cerrar sesión.
/// Cierra al hacer click fuera del menú o al navegar.
#[component]
pub fn UserMenu(collapsed: Option<bool>) -> Element {
    let collapsed = collapsed.unwrap_or(false);
    let auth = use_auth();
    let mut theme = use_theme();
    let navigator = use_navigator();
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

    let user = auth.read().user.clone();
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "Admin".to_string());
    let email = user.as_ref().and_then(|u| u.email.clone());

    let is_dark = theme() == ThemeId::Dark;

    rsx! {
        div {
            "data-feather-usermenu": "true",
            class: "relative",

            button {
                class: "dropdown-toggle inline-flex items-center gap-1 bg-transparent border-0 text-[var(--text-secondary)] hover:text-[var(--text-primary)]",
                style: if collapsed { "padding: 4px 8px; width: 100%;" },
                title: if collapsed { Some(username.clone()) } else { None },
                aria_expanded: menu_open().to_string(),
                onclick: move |_| *menu_open.write() = !menu_open(),
                i { class: if collapsed { "bi bi-person-circle" } else { "bi bi-person-circle me-1" } }
                if !collapsed {
                    span { "{username}" }
                    i { class: "bi bi-chevron-down ms-1", style: "font-size:0.7rem" }
                }
            }

            ul {
                class: format!(
                    "absolute bottom-full right-0 mb-2 z-[1100] min-w-[12rem] rounded-md border border-[var(--card-border)] bg-[var(--card-bg)] py-1 shadow-[var(--shadow)] {}",
                    if menu_open() { "" } else { "hidden" },
                ),
                li {
                    h6 { class: "px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-[var(--text-secondary)]", "Perfil" }
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
                            *menu_open.write() = false;
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
                            *menu_open.write() = false;
                            exit_app();
                        },
                        i { class: "bi bi-power me-2" }
                        "Salir"
                    }
                }
            }
        }
    }
}
