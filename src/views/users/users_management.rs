//! Listado y ABM de usuarios — página `/admin/users`.
//!
//! Sólo `super_admin` (igual que `/admin/projects`). El estado vive en
//! [`UsersProvider`]; el alta/edición usan [`UserFormModal`] y la baja el
//! diálogo compartido `ConfirmDialog`.

use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::layout::{AppLayout, use_auth};
use crate::components::protected_route::{ProtectedRoute, SUPER_ADMIN_ROLES};
use crate::components::toast::{show_toast, use_toast, ToastVariant};
use crate::models::{User, business_options, role_label};
use crate::views::users::state::{
    UsersProvider, add_user, delete_user, update_user, use_users,
};
use crate::views::users::user_form_modal::UserFormModal;
use dioxus::prelude::*;

/// Clase de las celdas de encabezado.
const TH: &str = "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)]";

/// Clase de las celdas del cuerpo.
const TD: &str = "px-3 py-3 text-[0.875rem] text-[var(--text-primary)] border-b border-[var(--border-color)]";

/// Página `/admin/users` — ABM de usuarios.
#[component]
pub fn Users() -> Element {
    rsx! {
        ProtectedRoute {
            roles: Some(SUPER_ADMIN_ROLES),
            AppLayout {
                UsersProvider {
                    UsersManagement {}
                }
            }
        }
    }
}

/// Fila de la tabla: avatar + usuario, nombre, rol, negocios y acciones.
#[component]
fn UserRow(
    user: User,
    /// `true` si es el usuario de la sesión (sin botón de borrar).
    es_propio: bool,
    on_edit: EventHandler<User>,
    on_delete_request: EventHandler<String>,
) -> Element {
    let inicial = user
        .username
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());
    let full_name = user.full_name();
    let user_edit = user.clone();
    let username_delete = user.username.clone();

    rsx! {
        tr {
            td { class: TD,
                div { class: "flex items-center gap-2",
                    if let Some(url) = user.avatar_url.as_deref().filter(|s| !s.is_empty()) {
                        img { src: "{url}", alt: "{user.username}", class: "h-8 w-8 rounded-full object-cover" }
                    } else {
                        div {
                            class: "flex h-8 w-8 items-center justify-center rounded-full text-sm font-medium text-white",
                            style: "background-color: var(--primary-color)",
                            "{inicial}"
                        }
                    }
                    div {
                        div { class: "font-medium text-[var(--text-primary)]", "{user.username}" }
                        if let Some(email) = user.email.as_deref().filter(|s| !s.is_empty()) {
                            div { class: "text-[var(--text-secondary)]", style: "font-size:0.8rem", "{email}" }
                        }
                    }
                }
            }
            td { class: TD, "{full_name}" }
            td { class: TD, "{role_label(user.role.as_str())}" }
            td { class: TD,
                if user.businesses.is_empty() {
                    span { class: "text-[var(--text-secondary)]", "—" }
                } else {
                    div { class: "flex flex-wrap gap-1",
                        for b in user.businesses.iter() {
                            span {
                                key: "{b}",
                                class: "inline-flex items-center rounded-full border border-[var(--border-color)] bg-[var(--bg-secondary)] px-2 py-0.5 text-xs text-[var(--text-secondary)]",
                                "{b}"
                            }
                        }
                    }
                }
            }
            td { class: TD,
                div { class: "flex gap-1",
                    button {
                        class: "flex h-8 w-8 items-center justify-center rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]",
                        title: "Editar",
                        onclick: move |_| on_edit.call(user_edit.clone()),
                        i { class: "bi bi-pencil" }
                    }
                    if !es_propio {
                        button {
                            class: "flex h-8 w-8 items-center justify-center rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--danger-color)]",
                            title: "Eliminar",
                            onclick: move |_| on_delete_request.call(username_delete.clone()),
                            i { class: "bi bi-trash" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn UsersManagement() -> Element {
    let state = use_users();
    let toasts = use_toast();
    let mut auth = use_auth();
    let mut search = use_signal(String::new);
    let mut show_form = use_signal(|| false);
    let mut editing = use_signal(|| Option::<User>::None);
    let mut confirm_delete = use_signal(|| Option::<String>::None);

    // Usuario de la sesión: no se borra a sí mismo ni se cambia su propio rol.
    let propio = auth.read().user.as_ref().map(|u| u.username.clone());

    let term = search().to_lowercase();
    let buscando = !term.is_empty();
    let coincide = |value: &str| value.to_lowercase().contains(term.as_str());
    let filtered: Vec<User> = state
        .read()
        .users
        .iter()
        .filter(|u| {
            !buscando
                || coincide(&u.username)
                || u.nombre.as_deref().is_some_and(coincide)
                || u.apellido.as_deref().is_some_and(coincide)
                || u.email.as_deref().is_some_and(coincide)
                || u.businesses.iter().any(|b| coincide(b))
        })
        .cloned()
        .collect();
    let results = filtered.len();

    rsx! {
        div { class: "grid grid-cols-12 gap-4 mb-4",
            div { class: "col-span-12",
                h1 { class: "text-[1.75rem] font-medium leading-[1.2] mb-1", style: "font-weight:500", "Usuarios" }
                p { class: "text-[var(--text-secondary)] mb-0", "Usuarios del panel y los negocios a los que pertenecen" }
            }
        }

        div { class: "rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]",
            div { class: "flex flex-wrap items-center justify-between gap-3 p-4 border-b border-[var(--border-color)]",
                div { class: "flex items-center gap-3",
                    div { class: "flex", style: "width:260px",
                        span { class: "flex items-center border border-r-0 border-[var(--border-color)] bg-[var(--bg-secondary)] px-3 text-[var(--text-secondary)] rounded-l", i { class: "bi bi-search" } }
                        input {
                            class: "block w-full border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] rounded-r focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                            placeholder: "Buscar usuarios...",
                            value: search(),
                            oninput: move |e| search.set(e.value()),
                        }
                    }
                    if buscando {
                        span { class: "text-[var(--text-secondary)]", style: "font-size:0.8125rem",
                            "{results} resultado"
                            if results != 1 { "s" }
                        }
                    }
                }
                button {
                    class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90 px-2.5 py-1 text-sm",
                    onclick: move |_| {
                        editing.set(None);
                        show_form.set(true);
                    },
                    i { class: "bi bi-plus-lg me-1" }
                    "Nuevo Usuario"
                }
            }
            div { class: "overflow-x-auto",
                table { class: "w-full border-collapse text-left",
                    thead { class: "bg-[var(--table-header-bg)]",
                        tr {
                            th { style: "width:25%", class: TH, "Usuario" }
                            th { style: "width:20%", class: TH, "Nombre" }
                            th { style: "width:15%", class: TH, "Rol" }
                            th { style: "width:25%", class: TH, "Negocios" }
                            th { style: "width:15%", class: TH }
                        }
                    }
                    tbody {
                        if filtered.is_empty() {
                            tr {
                                td { colspan: "5", class: "text-center text-[var(--text-secondary)] py-12",
                                    if buscando {
                                        i { class: "bi bi-search mb-2 block", style: "font-size:2rem" }
                                        p { class: "mb-0", "No se encontraron usuarios que coincidan con \"{search()}\"" }
                                    } else {
                                        i { class: "bi bi-people mb-2 block", style: "font-size:2rem" }
                                        p { class: "mb-2", "No hay usuarios aún" }
                                        button {
                                            class: "inline-flex items-center justify-center rounded border border-[var(--primary-color)] text-[var(--primary-color)] hover:bg-[var(--primary-color)] hover:text-[var(--text-inverse)] px-2.5 py-1 text-sm",
                                            onclick: move |_| show_form.set(true),
                                            "Crear el primer usuario"
                                        }
                                    }
                                }
                            }
                        }
                        for user in filtered {
                            UserRow {
                                key: "{user.username}",
                                es_propio: Some(user.username.as_str()) == propio.as_deref(),
                                user: user.clone(),
                                on_edit: move |u: User| {
                                    editing.set(Some(u));
                                    show_form.set(true);
                                },
                                on_delete_request: move |username: String| confirm_delete.set(Some(username)),
                            }
                        }
                    }
                }
            }
        }

        if show_form() {
            UserFormModal {
                editing: editing(),
                is_self: editing()
                    .as_ref()
                    .is_some_and(|u| Some(u.username.as_str()) == propio.as_deref()),
                options: business_options(
                    state.read().users.iter().flat_map(|u| u.businesses.iter().cloned()),
                ),
                on_close: move |_| {
                    show_form.set(false);
                    editing.set(None);
                },
                on_submit: move |(user, password): (User, Option<String>)| {
                    let edit_username = editing().map(|u| u.username);
                    let es_alta = edit_username.is_none();
                    spawn(async move {
                        let result = match edit_username {
                            Some(username) => update_user(state, &username, user, password).await,
                            None => match password {
                                Some(pass) => add_user(state, user, pass).await,
                                None => Err("La contraseña es obligatoria".to_string()),
                            },
                        };
                        match result {
                            Ok(saved) => {
                                // Si se editó el usuario de la sesión, refrescarla
                                // (Ajustes y el menú de usuario leen su copia).
                                let propio_guardado = auth.read().user.as_ref().map(|u| &u.username)
                                    == Some(&saved.username);
                                if propio_guardado {
                                    auth.write().user = Some(saved);
                                }
                                show_form.set(false);
                                editing.set(None);
                            }
                            Err(e) => show_toast(
                                toasts,
                                format!(
                                    "No se pudo {} el usuario: {e}",
                                    if es_alta { "crear" } else { "guardar" },
                                ),
                                ToastVariant::Error,
                            ),
                        }
                    });
                },
            }
        }

        if let Some(username) = confirm_delete() {
            ConfirmDialog {
                title: "Eliminar usuario".to_string(),
                message: format!("¿Eliminar al usuario \"{username}\"? Esta acción no se puede deshacer."),
                confirm_label: "Eliminar".to_string(),
                on_cancel: move |_| confirm_delete.set(None),
                on_confirm: {
                    let username = username.clone();
                    move |_| {
                        confirm_delete.set(None);
                        let username = username.clone();
                        spawn(async move {
                            if let Err(e) = delete_user(state, &username).await {
                                show_toast(
                                    toasts,
                                    format!("No se pudo eliminar el usuario: {e}"),
                                    ToastVariant::Error,
                                );
                            }
                        });
                    }
                },
            }
        }
    }
}
