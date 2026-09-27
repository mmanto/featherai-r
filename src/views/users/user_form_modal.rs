//! Modal de alta/edición de usuario — shell visual de `NewProjectModal`
//! (`views/projects/project_management.rs`).

use crate::models::{NEW_BUSINESS, ROLES, User, role_label};
use dioxus::prelude::*;

/// Clase de los inputs del formulario (misma que los modales de proyectos).
const INPUT: &str = "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none";

/// Clase de las etiquetas de campo.
const LABEL: &str = "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]";

/// Trim → `None` si queda vacío.
fn opt(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Fila del selector de negocios: un nombre y su checkbox.
#[component]
fn BusinessCheck(name: String, selected: bool, on_toggle: EventHandler<(String, bool)>) -> Element {
    let label = name.clone();
    rsx! {
        label { class: "flex items-center gap-2 text-sm text-[var(--text-primary)]",
            input {
                r#type: "checkbox",
                checked: selected,
                onchange: move |e| on_toggle.call((label.clone(), e.checked())),
            }
            span { "{name}" }
        }
    }
}

/// Modal de alta/edición de usuario.
///
/// - `editing` `Some` = edición: el username es la clave del doc y queda
///   deshabilitado (no se renombra).
/// - `is_self` deshabilita el rol: al propio usuario no se le puede quitar el
///   `super_admin` desde acá.
/// - `options` son los negocios conocidos (ver [`crate::models::business`]); se
///   pueden agregar nuevos escribiéndolos.
/// - `on_submit` recibe el usuario y la contraseña: en alta es obligatoria, en
///   edición `None` conserva el hash actual.
#[component]
pub fn UserFormModal(
    editing: Option<User>,
    is_self: bool,
    options: Vec<String>,
    on_close: EventHandler<()>,
    on_submit: EventHandler<(User, Option<String>)>,
) -> Element {
    let es_edicion = editing.is_some();
    let init_username = editing
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();
    let init_nombre = editing
        .as_ref()
        .and_then(|u| u.nombre.clone())
        .unwrap_or_default();
    let init_apellido = editing
        .as_ref()
        .and_then(|u| u.apellido.clone())
        .unwrap_or_default();
    let init_email = editing
        .as_ref()
        .and_then(|u| u.email.clone())
        .unwrap_or_default();
    let init_avatar = editing
        .as_ref()
        .and_then(|u| u.avatar_url.clone())
        .unwrap_or_default();
    let init_role = editing
        .as_ref()
        .map(|u| u.role.clone())
        .unwrap_or_else(|| "operativo".to_string());
    let init_businesses = editing
        .as_ref()
        .map(|u| u.businesses.clone())
        .unwrap_or_default();

    let mut username = use_signal(|| init_username);
    let mut password = use_signal(String::new);
    let mut nombre = use_signal(|| init_nombre);
    let mut apellido = use_signal(|| init_apellido);
    let mut email = use_signal(|| init_email);
    let mut avatar_url = use_signal(|| init_avatar);
    let mut role = use_signal(|| init_role);
    let mut businesses = use_signal(|| init_businesses);
    let mut new_business = use_signal(String::new);
    let mut form_error = use_signal(|| Option::<String>::None);

    // Opciones mostradas: las conocidas + las seleccionadas que no estén en la
    // lista (p. ej. un negocio escrito a mano o que sólo tiene este usuario).
    let mut shown = options;
    for name in businesses() {
        if !shown.contains(&name) {
            shown.push(name);
        }
    }

    let submit = move |_| {
        let normalized = username().trim().to_lowercase();
        if !es_edicion && normalized.len() < 3 {
            *form_error.write() = Some("El usuario debe tener al menos 3 caracteres".to_string());
            return;
        }
        if !es_edicion && password().trim().is_empty() {
            *form_error.write() = Some("La contraseña es obligatoria".to_string());
            return;
        }
        let negocios = businesses();
        if negocios.is_empty() {
            *form_error.write() = Some("Seleccioná al menos un negocio".to_string());
            return;
        }
        *form_error.write() = None;
        let user = User {
            username: editing
                .as_ref()
                .map(|u| u.username.clone())
                .unwrap_or(normalized),
            email: opt(&email()),
            nombre: opt(&nombre()),
            apellido: opt(&apellido()),
            avatar_url: opt(&avatar_url()),
            role: role(),
            businesses: negocios,
        };
        let pass = {
            let p = password();
            if p.trim().is_empty() { None } else { Some(p) }
        };
        on_submit.call((user, pass));
    };

    rsx! {
        div { class: "fixed inset-0 z-[1050] flex items-center justify-center overflow-y-auto bg-black/50 p-4",
            div { class: "w-full max-w-2xl",
                div { class: "w-full rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-lg)]",
                    div { class: "flex items-center justify-between border-b border-[var(--border-color)] px-4 py-3",
                        h5 { class: "text-[1.0625rem] font-medium text-[var(--text-primary)]",
                            if es_edicion { "Editar Usuario" } else { "Nuevo Usuario" }
                        }
                        button {
                            class: "flex h-8 w-8 items-center justify-center rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]",
                            onclick: move |_| on_close.call(()),
                            i { class: "bi bi-x-lg" }
                        }
                    }
                    div { class: "p-4",
                        if let Some(err) = form_error() {
                            div { class: "mb-3 rounded border px-3 py-2 text-sm", style: "border-color:var(--danger-color);color:var(--danger-color)", "{err}" }
                        }

                        div { class: "grid grid-cols-12 gap-4",
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: LABEL, "Usuario *" }
                                input {
                                    class: INPUT,
                                    value: username(),
                                    disabled: es_edicion,
                                    autocapitalize: "none",
                                    placeholder: "ana.ruiz",
                                    oninput: move |e| username.set(e.value()),
                                }
                            }
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: LABEL,
                                    if es_edicion { "Nueva contraseña (opcional)" } else { "Contraseña *" }
                                }
                                input {
                                    class: INPUT,
                                    r#type: "password",
                                    value: password(),
                                    placeholder: if es_edicion { "Dejar en blanco para no cambiarla" } else { "" },
                                    oninput: move |e| password.set(e.value()),
                                }
                            }
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: LABEL, "Nombre" }
                                input { class: INPUT, value: nombre(), oninput: move |e| nombre.set(e.value()) }
                            }
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: LABEL, "Apellido" }
                                input { class: INPUT, value: apellido(), oninput: move |e| apellido.set(e.value()) }
                            }
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: LABEL, "Email" }
                                input { class: INPUT, r#type: "email", value: email(), oninput: move |e| email.set(e.value()) }
                            }
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: LABEL, "URL de avatar" }
                                input { class: INPUT, value: avatar_url(), placeholder: "https://…", oninput: move |e| avatar_url.set(e.value()) }
                            }
                            div { class: "col-span-12 mb-3",
                                label { class: LABEL, "Rol" }
                                select {
                                    class: INPUT,
                                    value: role(),
                                    disabled: is_self,
                                    onchange: move |e| role.set(e.value()),
                                    for value in ROLES {
                                        option { key: "{value}", value: "{value}", "{role_label(value)}" }
                                    }
                                }
                                if is_self {
                                    p { class: "mt-1 text-xs text-[var(--text-secondary)]", "No podés cambiar tu propio rol." }
                                }
                            }
                        }

                        div { class: "mb-3",
                            label { class: LABEL, "Negocios *" }
                            div { class: "flex flex-wrap gap-x-4 gap-y-1",
                                for name in shown {
                                    BusinessCheck {
                                        key: "{name}",
                                        name: name.clone(),
                                        selected: businesses().contains(&name),
                                        on_toggle: move |(name, checked): (String, bool)| {
                                            let mut list = businesses();
                                            if checked {
                                                if !list.contains(&name) {
                                                    list.push(name);
                                                }
                                            } else {
                                                list.retain(|b| b != &name);
                                            }
                                            businesses.set(list);
                                        },
                                    }
                                }
                            }
                            div { class: "mt-2 flex gap-2",
                                input {
                                    class: INPUT,
                                    value: new_business(),
                                    placeholder: "Nuevo negocio",
                                    oninput: move |e| new_business.set(e.value()),
                                }
                                button {
                                    class: "shrink-0 rounded border border-[var(--border-color)] px-3 py-2 text-sm text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)]",
                                    onclick: move |_| {
                                        let name = new_business().trim().to_string();
                                        if name.is_empty() || name == NEW_BUSINESS {
                                            return;
                                        }
                                        let mut list = businesses();
                                        if !list.contains(&name) {
                                            list.push(name);
                                        }
                                        businesses.set(list);
                                        new_business.set(String::new());
                                    },
                                    "Agregar"
                                }
                            }
                            p { class: "mt-1 text-xs text-[var(--text-secondary)]", "Un usuario pertenece a uno o más negocios." }
                        }
                    }
                    div { class: "flex justify-end gap-2 border-t border-[var(--border-color)] px-4 py-3",
                        button { class: "inline-flex items-center justify-center rounded bg-[var(--bg-tertiary)] text-[var(--text-primary)] hover:opacity-90 px-3 py-1.5 text-sm", onclick: move |_| on_close.call(()), "Cancelar" }
                        button { class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90 px-3 py-1.5 text-sm", onclick: submit, "Guardar" }
                    }
                }
            }
        }
    }
}
