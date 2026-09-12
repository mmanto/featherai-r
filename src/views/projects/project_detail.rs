use crate::views::projects::gantt_chart::GanttChart;
use crate::views::projects::kanban_board::KanbanBoard;
use crate::views::projects::project_tasks_list::ProjectTasksList;
use crate::views::projects::state::{fmt_date, Project, Task, ViewMode};
use crate::views::projects::task_detail::TaskDetail;
use dioxus::prelude::*;

/// Detalle de proyecto — port de `ProjectDetail.tsx`.
///
/// Header con badges y progreso, sección colapsable de tareas con vistas
/// lista/kanban/gantt y modal de edición de datos básicos. Si hay una tarea
/// seleccionada, muestra [`TaskDetail`] en su lugar.
#[component]
pub fn ProjectDetail(
    project: Project,
    on_back: EventHandler<()>,
    on_back_to_projects: EventHandler<()>,
    on_add_task: EventHandler<()>,
    on_update_task: EventHandler<Task>,
    on_delete_task: EventHandler<String>,
    on_update_project: EventHandler<Project>,
) -> Element {
    let mut view_mode = use_signal(|| ViewMode::List);
    let mut selected_task = use_signal(|| Option::<String>::None);
    let mut show_edit = use_signal(|| false);

    // ── Tarea seleccionada → TaskDetail ──
    if let Some(task) = selected_task()
        .iter()
        .flat_map(|id| project.tasks.iter().find(|t| t.id == *id))
        .cloned()
        .next()
    {
        return rsx! {
            TaskDetail {
                task: task.clone(),
                project: project.clone(),
                on_back: move |_| selected_task.set(None),
                on_back_to_projects: move |_| {
                    selected_task.set(None);
                    on_back_to_projects.call(());
                },
                on_update_task: move |t| on_update_task.call(t),
                on_delete_task: move |id| on_delete_task.call(id),
            }
        };
    }

    let progress = project.progress();
    let tasks_len = project.tasks.len();
    let team_len = project.team.len();
    let team_suffix = if team_len != 1 { "s" } else { "" };
    let tasks_suffix = if tasks_len != 1 { "s" } else { "" };
    let todo_n = project.task_count(crate::views::projects::state::TaskStatus::Todo);
    let progress_n = project.task_count(crate::views::projects::state::TaskStatus::InProgress);
    let done_n = project.task_count(crate::views::projects::state::TaskStatus::Done);
    let start_fmt = fmt_date(project.start_date.as_deref().unwrap_or(""));
    let end_fmt = fmt_date(project.end_date.as_deref().unwrap_or(""));
    let name = project.name.clone();
    let description = project.description.clone();
    let status_badge = project.status.badge();
    let status_label = project.status.label();
    let priority_badge = project.priority.badge();
    let priority_label = project.priority.label();

    rsx! {
        div { class: "project-detail-layout",

            // ── Botón de regreso ──
            div { class: "mb-3",
                button {
                    class: "btn-ghost-back",
                    onclick: move |_| on_back.call(()),
                    i { class: "bi bi-arrow-left me-2" }
                    "Proyectos"
                }
            }

            // ── Header ──
            div { class: "project-detail-header rounded-lg bg-[var(--card-bg)] mb-3",
                div { class: "p-4",
                    div { class: "flex items-start justify-between mb-2",
                        div {
                            h1 { class: "text-[1.75rem] font-medium leading-[1.2] mb-1", style: "font-weight:500", "{name}" }
                            if let Some(desc) = &description {
                                p { class: "text-[var(--text-secondary)] mb-0", style: "font-size:0.9rem", "{desc}" }
                            }
                        }
                        div { class: "flex items-center gap-2 shrink-0 ms-3",
                            span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {status_badge}"), "{status_label}" }
                            span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {priority_badge}"), "{priority_label}" }
                            button {
                                class: "inline-flex items-center justify-center rounded border border-[var(--border-color)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] px-2.5 py-1 text-sm",
                                title: "Editar datos del proyecto",
                                onclick: move |_| *show_edit.write() = true,
                                i { class: "bi bi-pencil me-1" }
                                "Editar"
                            }
                        }
                    }
                    div { class: "project-meta-row",
                        span { class: "project-meta-item",
                            i { class: "bi bi-calendar-event me-1" }
                            span { "{start_fmt}" }
                            span { class: "mx-1 text-[var(--text-secondary)]", "→" }
                            span { "{end_fmt}" }
                        }
                        if team_len > 0 {
                            span { class: "project-meta-item",
                                i { class: "bi bi-people-fill me-1" }
                                "{team_len} miembro{team_suffix}"
                            }
                        }
                        span { class: "project-meta-item",
                            i { class: "bi bi-list-check me-1" }
                            "{tasks_len} tarea{tasks_suffix}"
                        }
                        span { class: "project-meta-item text-[var(--text-secondary)]", title: "Por hacer",
                            i { class: "bi bi-circle me-1" }
                            "{todo_n}"
                        }
                        span { class: "project-meta-item", style: "color:var(--primary-color)", title: "En progreso",
                            i { class: "bi bi-arrow-repeat me-1" }
                            "{progress_n}"
                        }
                        span { class: "project-meta-item text-[var(--success-color)]", title: "Completadas",
                            i { class: "bi bi-check-circle me-1" }
                            "{done_n}"
                        }
                        span { class: "project-meta-item",
                            "{progress}%"
                        }
                    }
                }
            }

            // ── Sección: Tareas ──
            h5 { class: "mb-3",
                i { class: "bi bi-list-task me-2" }
                "Tareas"
                span { class: "inline-flex items-center rounded-full bg-[var(--secondary-color)] px-2 py-0.5 text-xs text-white ms-2", style: "font-size:0.75rem;font-weight:400", "{tasks_len}" }
            }
            if view_mode() == ViewMode::List {
                ProjectTasksList {
                    project: project.clone(),
                    view_mode: view_mode(),
                    on_view_mode_change: move |m| view_mode.set(m),
                    on_add_task: move |_| on_add_task.call(()),
                    on_task_select: move |id| selected_task.set(Some(id)),
                    on_update_task: move |t| on_update_task.call(t),
                    on_delete_task: move |id| on_delete_task.call(id),
                }
            } else if view_mode() == ViewMode::Kanban {
                KanbanBoard {
                    project: project.clone(),
                    view_mode: view_mode(),
                    on_view_mode_change: move |m| view_mode.set(m),
                    on_add_task: move |_| on_add_task.call(()),
                    on_update_task: move |t| on_update_task.call(t),
                    on_delete_task: move |id| on_delete_task.call(id),
                    on_move_task: {
                        let project = project.clone();
                        let on_update_task = on_update_task.clone();
                        move |(id, status)| {
                            // Mover la tarea: se delega vía on_update_task con el
                            // status nuevo (equivale a moveTask de React).
                            if let Some(t) = project.tasks.iter().find(|t| t.id == id) {
                                let mut updated = t.clone();
                                updated.status = status;
                                on_update_task.call(updated);
                            }
                        }
                    },
                }
            } else if view_mode() == ViewMode::Gantt {
                GanttChart {
                    project: project.clone(),
                    view_mode: view_mode(),
                    on_view_mode_change: move |m| view_mode.set(m),
                    on_add_task: move |_| on_add_task.call(()),
                }
            }

            // ── Modal de edición ──
            if show_edit() {
                EditProjectModal {
                    project: project.clone(),
                    on_close: move |_| *show_edit.write() = false,
                    on_save: move |updated| {
                        on_update_project.call(updated);
                        *show_edit.write() = false;
                    },
                }
            }
        }
    }
}

/// Modal de edición de datos básicos del proyecto (mismo formulario que
/// `NewProjectModal` pero precargado).
#[component]
fn EditProjectModal(
    project: Project,
    on_close: EventHandler<()>,
    on_save: EventHandler<Project>,
) -> Element {
    let mut name = use_signal(|| project.name.clone());
    let mut description = use_signal(|| project.description.clone().unwrap_or_default());
    let mut start_date = use_signal(|| project.start_date.clone().unwrap_or_default());
    let mut end_date = use_signal(|| project.end_date.clone().unwrap_or_default());
    let mut status = use_signal(|| project.status);
    let mut priority = use_signal(|| project.priority);
    let mut color = use_signal(|| project.color.clone());
    let mut team = use_signal(|| project.team.join(", "));

    let save = move |_| {
        let name = name().trim().to_string();
        if name.is_empty() {
            return;
        }
        let mut updated = project.clone();
        updated.name = name;
        updated.description = {
            let d = description().trim().to_string();
            if d.is_empty() {
                None
            } else {
                Some(d)
            }
        };
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
        updated.status = status();
        updated.priority = priority();
        updated.color = color();
        updated.team = team()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        on_save.call(updated);
    };

    rsx! {
        div { class: "fixed inset-0 z-[1050] flex items-center justify-center overflow-y-auto bg-black/50 p-4",
            div { class: "w-full max-w-2xl",
                div { class: "w-full rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-lg)]",
                    div { class: "flex items-center justify-between border-b border-[var(--border-color)] px-4 py-3",
                        h5 { class: "text-[1.0625rem] font-medium text-[var(--text-primary)] flex items-center gap-2", i { class: "bi bi-pencil-square" }, "Editar Proyecto" }
                        button { class: "flex h-8 w-8 items-center justify-center rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]", onclick: move |_| on_close.call(()), i { class: "bi bi-x-lg" } }
                    }
                    div { class: "p-4",
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Nombre *" }
                            input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", value: name(), oninput: move |e| name.set(e.value()) }
                        }
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Descripción" }
                            textarea { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", rows: "3", value: description(), oninput: move |e| description.set(e.value()) }
                        }
                        div { class: "grid grid-cols-12 gap-4",
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Fecha de inicio" }
                                input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", r#type: "date", value: start_date(), oninput: move |e| start_date.set(e.value()) }
                            }
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Fecha de fin" }
                                input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", r#type: "date", value: end_date(), oninput: move |e| end_date.set(e.value()) }
                            }
                        }
                        div { class: "grid grid-cols-12 gap-4",
                            div { class: "col-span-12 md:col-span-4 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Estado" }
                                select {
                                    class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:outline-none",
                                    value: status().as_str(),
                                    onchange: move |e| status.set(e.value().as_str().into()),
                                    option { value: "planning", "Planificación" }
                                    option { value: "active", "Activo" }
                                    option { value: "paused", "Pausado" }
                                    option { value: "completed", "Completado" }
                                }
                            }
                            div { class: "col-span-12 md:col-span-4 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Prioridad" }
                                select {
                                    class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:outline-none",
                                    value: priority().as_str(),
                                    onchange: move |e| priority.set(e.value().as_str().into()),
                                    option { value: "low", "Baja" }
                                    option { value: "medium", "Media" }
                                    option { value: "high", "Alta" }
                                }
                            }
                            div { class: "col-span-12 md:col-span-4 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Color" }
                                input { class: "h-9 w-full cursor-pointer rounded border border-[var(--border-color)] bg-transparent p-1", r#type: "color", value: color(), oninput: move |e| color.set(e.value()) }
                            }
                        }
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Miembros del equipo" }
                            input {
                                class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                value: team(),
                                oninput: move |e| team.set(e.value()),
                                placeholder: "Juan Pérez, María García, Carlos López",
                            }
                            div { class: "mt-1 text-xs text-[var(--text-secondary)]", "Separados por coma" }
                        }
                    }
                    div { class: "flex justify-end gap-2 border-t border-[var(--border-color)] px-4 py-3",
                        button { class: "inline-flex items-center justify-center rounded bg-[var(--bg-tertiary)] text-[var(--text-primary)] hover:opacity-90", onclick: move |_| on_close.call(()), "Cancelar" }
                        button { class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90", onclick: save, i { class: "bi bi-check-lg me-1" }, "Guardar cambios" }
                    }
                }
            }
        }
    }
}
