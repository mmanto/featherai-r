use crate::components::layout::use_auth;
use crate::views::projects::state::{fmt_date, Task};
use dioxus::prelude::*;
use std::rc::Rc;

/// Detalle de una tarea — port de `TaskDetail.tsx`.
///
/// Breadcrumb (Proyectos → proyecto → tarea), tarjeta con edición inline
/// (título, descripción, estado, prioridad, asignado, horas, fechas y
/// etiquetas) y accesos rápidos "Asignarme"/"Desasignarme".
#[component]
pub fn TaskDetail(
    task: Task,
    project: crate::views::projects::state::Project,
    on_back: EventHandler<()>,
    on_back_to_projects: EventHandler<()>,
    on_update_task: EventHandler<Task>,
) -> Element {
    let mut editing = use_signal(|| false);
    let mut title = use_signal(|| task.title.clone());
    let mut description = use_signal(|| task.description.clone().unwrap_or_default());
    let mut status = use_signal(|| task.status);
    let mut priority = use_signal(|| task.priority);
    let mut assignee = use_signal(|| task.assignee.clone().unwrap_or_default());
    let mut hours = use_signal(|| {
        task.estimated_hours
            .map(|h| h.to_string())
            .unwrap_or_default()
    });
    let mut start_date = use_signal(|| task.start_date.clone().unwrap_or_default());
    let mut end_date = use_signal(|| task.end_date.clone().unwrap_or_default());
    let mut tags = use_signal(|| task.tags.join(", "));

    // Rc para compartir el Task entre handlers (closures move 'static).
    let task = Rc::new(task);
    let start_fmt = fmt_date(task.start_date.as_deref().unwrap_or(""));
    let end_fmt = fmt_date(task.end_date.as_deref().unwrap_or(""));

    let auth = use_auth();
    let user_name = auth
        .read()
        .user
        .clone()
        .map(|u| u.full_name())
        .unwrap_or_default();

    let build_updated = move |t: &Task| -> Task {
        let mut updated = t.clone();
        updated.title = title().trim().to_string();
        updated.description = {
            let d = description().trim().to_string();
            if d.is_empty() {
                None
            } else {
                Some(d)
            }
        };
        updated.status = status();
        updated.priority = priority();
        updated.assignee = {
            let a = assignee().trim().to_string();
            if a.is_empty() {
                None
            } else {
                Some(a)
            }
        };
        updated.estimated_hours = hours().trim().parse::<f64>().ok();
        updated.start_date = {
            let d = start_date().trim().to_string();
            if d.is_empty() {
                None
            } else {
                Some(d)
            }
        };
        updated.end_date = {
            let d = end_date().trim().to_string();
            if d.is_empty() {
                None
            } else {
                Some(d)
            }
        };
        updated.tags = tags()
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        updated
    };

    let save = {
        let task = task.clone();
        move |_| {
            let updated = build_updated(&*task);
            if updated.title.is_empty() {
                return;
            }
            on_update_task.call(updated);
            editing.set(false);
        }
    };

    let set_assignee_quick = {
        let task = task.clone();
        move |a: Option<String>| {
            let mut updated = (*task).clone();
            updated.assignee = a;
            on_update_task.call(updated);
        }
    };

    let start_edit = {
        let task = task.clone();
        move |_| {
            title.set(task.title.clone());
            description.set(task.description.clone().unwrap_or_default());
            status.set(task.status);
            priority.set(task.priority);
            assignee.set(task.assignee.clone().unwrap_or_default());
            hours.set(
                task.estimated_hours
                    .map(|h| h.to_string())
                    .unwrap_or_default(),
            );
            start_date.set(task.start_date.clone().unwrap_or_default());
            end_date.set(task.end_date.clone().unwrap_or_default());
            tags.set(task.tags.join(", "));
            editing.set(true);
        }
    };
    rsx! {
        div {
            // ── Breadcrumb ──
            div { class: "mb-4",
                nav { aria_label: "breadcrumb",
                    ol { class: "flex items-center gap-2 text-sm",
                        li {
                            button {
                                class: "bg-transparent border-0 p-0 text-[var(--primary-color)] hover:underline",
                                onclick: move |_| on_back_to_projects.call(()),
                                "Proyectos"
                            }
                        }
                        span { class: "text-[var(--text-muted)]", "/" }
                        li {
                            button {
                                class: "bg-transparent border-0 p-0 text-[var(--primary-color)] hover:underline",
                                onclick: move |_| on_back.call(()),
                                i { class: "bi bi-arrow-left me-2" }
                                "{project.name}"
                            }
                        }
                        span { class: "text-[var(--text-muted)]", "/" }
                        li { class: "text-[var(--text-secondary)]", aria_current: "page", "{task.title}" }
                    }
                }
            }

            // ── Tarjeta principal ──
            div { class: "rounded-lg bg-[var(--card-bg)] mb-4",
                div { class: "p-4",
                    div { class: "flex items-start justify-between mb-4",
                        div { class: "flex-1 me-3",
                            if editing() {
                                input {
                                    class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2.5 text-base text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none mb-2",
                                    value: title(),
                                    oninput: move |e| title.set(e.value()),
                                    placeholder: "Título de la tarea",
                                }
                                textarea {
                                    class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                    rows: "3",
                                    value: description(),
                                    oninput: move |e| description.set(e.value()),
                                    placeholder: "Descripción",
                                }
                            } else {
                                h1 { class: "text-[1.75rem] font-medium leading-[1.2] mb-1", style: "font-weight:500", "{task.title}" }
                                if let Some(desc) = &task.description {
                                    p { class: "text-[var(--text-secondary)] mb-0", "{desc}" }
                                }
                            }
                        }
                        div { class: "flex items-start gap-2",
                            if editing() {
                                button {
                                    class: "inline-flex items-center justify-center rounded bg-[var(--success-color)] text-white px-2.5 py-1 text-sm",
                                    onclick: save,
                                    i { class: "bi bi-check-lg me-1" }
                                    "Guardar"
                                }
                                button {
                                    class: "inline-flex items-center justify-center rounded bg-[var(--bg-tertiary)] text-[var(--text-primary)] hover:opacity-90 px-2.5 py-1 text-sm",
                                    onclick: move |_| editing.set(false),
                                    i { class: "bi bi-x-lg me-1" }
                                    "Cancelar"
                                }
                            } else {
                                span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {}", task.status.badge()), "{task.status.label()}" }
                                span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {}", task.priority.badge()), "{task.priority.label()}" }
                                button {
                                    class: "inline-flex items-center justify-center rounded border border-[var(--primary-color)] text-[var(--primary-color)] hover:bg-[var(--primary-color)] hover:text-[var(--text-inverse)] px-2.5 py-1 text-sm",
                                    onclick: start_edit,
                                    i { class: "bi bi-pencil me-1" }
                                    "Editar"
                                }
                            }
                        }
                    }

                    // ── Stats ──
                    div { class: "grid grid-cols-12 gap-4",
                        div { class: "col-span-12 md:col-span-3",
                            div { class: "rounded-[0.375rem] border border-[var(--border-color)] p-3",
                                div { class: "text-[var(--text-secondary)] text-xs mb-2", i { class: "bi bi-list-check me-1" }, "Estado" }
                                if editing() {
                                    select {
                                        class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:outline-none",
                                        value: status().as_str(),
                                        onchange: move |e| status.set(e.value().as_str().into()),
                                        option { value: "todo", "Por hacer" }
                                        option { value: "in_progress", "En progreso" }
                                        option { value: "done", "Completada" }
                                    }
                                } else {
                                    div { class: "font-medium",
                                        span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {}", task.status.badge()), "{task.status.label()}" }
                                    }
                                }
                            }
                        }
                        div { class: "col-span-12 md:col-span-3",
                            div { class: "rounded-[0.375rem] border border-[var(--border-color)] p-3",
                                div { class: "text-[var(--text-secondary)] text-xs mb-2", i { class: "bi bi-exclamation-triangle me-1" }, "Prioridad" }
                                if editing() {
                                    select {
                                        class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:outline-none",
                                        value: priority().as_str(),
                                        onchange: move |e| priority.set(e.value().as_str().into()),
                                        option { value: "low", "Baja" }
                                        option { value: "medium", "Media" }
                                        option { value: "high", "Alta" }
                                    }
                                } else {
                                    div { class: "font-medium",
                                        span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {}", task.priority.badge()), "{task.priority.label()}" }
                                    }
                                }
                            }
                        }
                        div { class: "col-span-12 md:col-span-3",
                            div { class: "rounded-[0.375rem] border border-[var(--border-color)] p-3",
                                div { class: "text-[var(--text-secondary)] text-xs mb-2", i { class: "bi bi-person-circle me-1" }, "Asignado a" }
                                if editing() {
                                    div {
                                        input {
                                            class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none mb-1",
                                            value: assignee(),
                                            oninput: move |e| assignee.set(e.value()),
                                            placeholder: "Nombre",
                                        }
                                        if !user_name.is_empty() && assignee() != user_name {
                                            button {
                                                r#type: "button",
                                                class: "bg-transparent border-0 p-0 text-[var(--primary-color)] hover:underline",
                                                onclick: move |_| assignee.set(user_name.clone()),
                                                i { class: "bi bi-person-check me-1" }
                                                "Usar mi nombre"
                                            }
                                        }
                                    }
                                } else {
                                    div {
                                        div { class: "font-medium mb-1",
                                            if let Some(a) = &task.assignee {
                                                span { i { class: "bi bi-person-fill me-1 text-[var(--text-secondary)]" }, "{a}" }
                                            } else {
                                                span { class: "text-[var(--text-secondary)]", "Sin asignar" }
                                            }
                                        }
                                        if !user_name.is_empty() {
                                            if task.assignee.as_deref() != Some(user_name.as_str()) {
                                                button {
                                                    r#type: "button",
                                                    class: "inline-flex items-center justify-center rounded border border-[var(--primary-color)] text-[var(--primary-color)] hover:bg-[var(--primary-color)] hover:text-[var(--text-inverse)] px-2 py-0",
                                                    style: "font-size:0.75rem",
                                                    onclick: move |_| set_assignee_quick(Some(user_name.clone())),
                                                    i { class: "bi bi-person-check me-1" }
                                                    "Asignarme"
                                                }
                                            } else {
                                                button {
                                                    r#type: "button",
                                                    class: "bg-transparent border-0 p-0 text-[var(--text-secondary)] hover:underline",
                                                    style: "font-size:0.75rem",
                                                    onclick: move |_| set_assignee_quick(None),
                                                    i { class: "bi bi-person-dash me-1" }
                                                    "Desasignarme"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "col-span-12 md:col-span-3",
                            div { class: "rounded-[0.375rem] border border-[var(--border-color)] p-3",
                                div { class: "text-[var(--text-secondary)] text-xs mb-2", i { class: "bi bi-clock me-1" }, "Horas estimadas" }
                                if editing() {
                                    input {
                                        r#type: "number",
                                        class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                        value: hours(),
                                        oninput: move |e| hours.set(e.value()),
                                        placeholder: "0",
                                        min: "0",
                                        step: "0.5",
                                    }
                                } else {
                                    div { class: "font-medium",
                                        if let Some(h) = task.estimated_hours {
                                            "{h}h"
                                        } else {
                                            span { class: "text-[var(--text-secondary)]", "No definido" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Fechas ──
                    div { class: "grid grid-cols-12 gap-4 mt-2",
                        div { class: "col-span-12 md:col-span-6",
                            div { class: "rounded-[0.375rem] border border-[var(--border-color)] p-3",
                                div { class: "text-[var(--text-secondary)] text-xs mb-2", i { class: "bi bi-calendar me-1" }, "Fecha de inicio" }
                                if editing() {
                                    input {
                                        r#type: "date",
                                        class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                        value: start_date(),
                                        oninput: move |e| start_date.set(e.value()),
                                    }
                                } else {
                                    div { class: "font-medium", "{start_fmt}" }
                                }
                            }
                        }
                        div { class: "col-span-12 md:col-span-6",
                            div { class: "rounded-[0.375rem] border border-[var(--border-color)] p-3",
                                div { class: "text-[var(--text-secondary)] text-xs mb-2", i { class: "bi bi-calendar-check me-1" }, "Fecha límite" }
                                if editing() {
                                    input {
                                        r#type: "date",
                                        class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                        value: end_date(),
                                        oninput: move |e| end_date.set(e.value()),
                                    }
                                } else {
                                    div { class: "font-medium", "{end_fmt}" }
                                }
                            }
                        }
                    }

                    // ── Etiquetas ──
                    div { class: "mt-4",
                        div { class: "text-[var(--text-secondary)] text-xs mb-2", i { class: "bi bi-tags me-1" }, "Etiquetas:" }
                        if editing() {
                            input {
                                class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                value: tags(),
                                oninput: move |e| tags.set(e.value()),
                                placeholder: "frontend, backend, bug (separadas por comas)",
                            }
                        } else {
                            div { class: "flex flex-wrap gap-2",
                                if task.tags.is_empty() {
                                    span { class: "text-[var(--text-secondary)]", "Sin etiquetas" }
                                } else {
                                    for (idx, tag) in task.tags.iter().enumerate() {
                                        span { key: "{idx}", class: "inline-flex items-center rounded-full bg-[var(--bg-tertiary)] px-2 py-0.5 text-xs text-[var(--text-primary)] border border-[var(--border-color)]", "{tag}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
