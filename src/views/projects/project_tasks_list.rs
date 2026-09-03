use crate::components::layout::use_auth;
use crate::views::projects::state::{fmt_date, Task, TaskStatus, ViewMode};
use dioxus::prelude::*;
use std::rc::Rc;

/// Campos ordenables de la tabla de tareas — equivalente de `keyof Task`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskSort {
    Title,
    Status,
    Priority,
    StartDate,
    EndDate,
}

impl TaskSort {
    fn value(self, t: &Task) -> String {
        match self {
            Self::Title => t.title.clone(),
            Self::Status => t.status.as_str().to_string(),
            Self::Priority => {
                let rank = match t.priority {
                    crate::views::projects::state::Priority::Low => 0,
                    crate::views::projects::state::Priority::Medium => 1,
                    crate::views::projects::state::Priority::High => 2,
                };
                rank.to_string()
            }
            Self::StartDate => t.start_date.clone().unwrap_or_default(),
            Self::EndDate => t.end_date.clone().unwrap_or_default(),
        }
    }
}

fn icon_sort(current: TaskSort, field: TaskSort, asc: bool) -> Element {
    if current != field {
        return rsx! { i { class: "bi bi-chevron-expand text-[var(--text-secondary)] ms-1" } };
    }
    if asc {
        rsx! { i { class: "bi bi-chevron-up ms-1" } }
    } else {
        rsx! { i { class: "bi bi-chevron-down ms-1" } }
    }
}

/// Fila de la tabla de tareas — componente propio para aislar closures.
#[component]
fn TaskRow(
    task: Task,
    user_name: String,
    on_select: EventHandler<String>,
    on_update: EventHandler<Task>,
) -> Element {
    let mut hovered = use_signal(|| false);
    let mut editing = use_signal(|| false);
    let mut editing_value = use_signal(|| task.title.clone());

    // Rc para que cada handler capture un clon barato (los closures de rsx
    // son move y 'static; el Task prop no es Copy).
    let task = Rc::new(task);
    let tid = task.id.clone();
    let ttitle = Rc::new(task.title.clone());
    let tdesc = task.description.clone();
    let ttags = task.tags.clone();
    let tstatus_label = task.status.label();
    let tstatus_badge = task.status.badge();
    let tpriority_label = task.priority.label();
    let tpriority_badge = task.priority.badge();
    let tassignee = task.assignee.clone();
    let tstart = task.start_date.clone().unwrap_or_default();
    let tend = task.end_date.clone().unwrap_or_default();
    let tstart_fmt = fmt_date(&tstart);
    let tend_fmt = fmt_date(&tend);
    let thours = task.estimated_hours_text();

    rsx! {
        tr {
            class: "notion-grid-row",
            style: "cursor:pointer",
            onclick: move |_| on_select.call(tid.clone()),
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),

            td {
                div { class: "relative",
                    if editing() {
                        input {
                            class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                            value: editing_value(),
                            oninput: move |e| editing_value.set(e.value()),
                            onblur: {
                let task = task.clone();
                move |_| {
                    let value = editing_value().trim().to_string();
                    if !value.is_empty() {
                        let mut updated = (*task).clone();
                        updated.title = value;
                        on_update.call(updated);
                    }
                    editing.set(false);
                }
            },
                            onkeydown: {
                                let task = task.clone();
                                move |e| {
                                    if e.key() == Key::Enter {
                                        let value = editing_value().trim().to_string();
                                        if !value.is_empty() {
                                            let mut updated = (*task).clone();
                                            updated.title = value;
                                            on_update.call(updated);
                                        }
                                        editing.set(false);
                                    } else if e.key() == Key::Escape {
                                        editing.set(false);
                                    }
                                }
                            },
                            onclick: move |e| e.stop_propagation(),
                        }
                    } else {
                        div {
                            class: "font-medium",
                            ondoubleclick: {
                                let ttitle = ttitle.clone();
                                move |e| {
                                    e.stop_propagation();
                                    editing.set(true);
                                    editing_value.set(ttitle.as_str().to_string());
                                }
                            },
                            "{ttitle}"
                        }
                    }
                    if let Some(desc) = &tdesc {
                        small { class: "text-[var(--text-secondary)] block truncate", style: "max-width:350px", "{desc}" }
                    }
                    if !ttags.is_empty() {
                        div { class: "mt-1",
                            { ttags.iter().enumerate().map(|(idx, tag)| {
                                rsx! { span { key: "{idx}", class: "inline-flex items-center rounded-full bg-[var(--bg-tertiary)] px-2 py-0.5 text-xs text-[var(--text-primary)] border border-[var(--border-color)] me-1", style: "font-size:0.7rem", "{tag}" } }
                            }) }
                        }
                    }
                    if hovered() {
                        div { class: "notion-hover-actions",
                            button {
                                class: "inline-flex items-center justify-center p-1.5 rounded text-[var(--text-secondary)] hover:bg-[var(--table-hover)] hover:text-[var(--text-primary)]",
                                title: "Editar",
                                onclick: {
                                    let ttitle = ttitle.clone();
                                    move |e| {
                                        e.stop_propagation();
                                        editing.set(true);
                                        editing_value.set(ttitle.as_str().to_string());
                                    }
                                },
                                i { class: "bi bi-pencil" }
                            }
                        }
                    }
                }
            }
            td {
                span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {tstatus_badge}"), "{tstatus_label}" }
            }
            td {
                span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {tpriority_badge}"), "{tpriority_label}" }
            }
            td {
                onclick: move |e| e.stop_propagation(),
                if let Some(assignee) = &tassignee {
                    div { class: "flex items-center gap-1",
                        i { class: "bi bi-person-circle text-[var(--text-secondary)]" }
                        small { "{assignee}" }
                        if hovered() && !user_name.is_empty() && *assignee == user_name {
                            button {
                                class: "bg-transparent border-0 p-0 ms-1 text-[var(--text-secondary)] hover:underline",
                                style: "font-size:0.7rem",
                                title: "Desasignarme",
                                onclick: {
                                    let task = task.clone();
                                    move |_| {
                                        let mut updated = (*task).clone();
                                        updated.assignee = None;
                                        on_update.call(updated);
                                    }
                                },
                                i { class: "bi bi-x-circle" }
                            }
                        }
                    }
                } else {
                    div { class: "flex items-center gap-1",
                        small { class: "text-[var(--text-secondary)]", "Sin asignar" }
                        if hovered() && !user_name.is_empty() {
                            button {
                                class: "inline-flex items-center justify-center rounded border border-[var(--primary-color)] text-[var(--primary-color)] hover:bg-[var(--primary-color)] hover:text-[var(--text-inverse)] px-1 py-0 ms-1",
                                style: "font-size:0.7rem",
                                title: "Asignarme esta tarea",
                                onclick: {
                                    let task = task.clone();
                                    let user_name = user_name.clone();
                                    move |_| {
                                        let mut updated = (*task).clone();
                                        updated.assignee = Some(user_name.clone());
                                        on_update.call(updated);
                                    }
                                },
                                i { class: "bi bi-person-check me-1" }
                                "Asignarme"
                            }
                        }
                    }
                }
            }
            td { small { "{tstart_fmt}" } }
            td { small { "{tend_fmt}" } }
            td { small { "{thours}" } }
        }
    }
}

/// Tabla de tareas de un proyecto — port de `ProjectTasksList.tsx`.
///
/// Vista lista con ordenamiento, filtro por estado, control de la vista
/// (lista/kanban/gantt) y "Nueva Tarea"; el detalle por fila vive en
/// [`TaskRow`].
#[component]
pub fn ProjectTasksList(
    project: crate::views::projects::state::Project,
    on_update_task: EventHandler<Task>,
    on_task_select: EventHandler<String>,
    view_mode: ViewMode,
    on_view_mode_change: EventHandler<ViewMode>,
    on_add_task: EventHandler<()>,
) -> Element {
    let mut filter = use_signal(|| Option::<TaskStatus>::None);
    let sort_field = use_signal(|| TaskSort::Title);
    let asc = use_signal(|| true);

    let auth = use_auth();
    let user_name = auth
        .read()
        .user
        .clone()
        .map(|u| u.full_name())
        .unwrap_or_default();

    let tasks = project.tasks.clone();
    let status_filter = filter();
    let filtered: Vec<Task> = tasks
        .iter()
        .filter(|t| status_filter.map(|s| t.status == s).unwrap_or(true))
        .cloned()
        .collect();

    let field = sort_field();
    let asc_flag = asc();
    let mut sorted = filtered;
    sorted.sort_by(|a, b| {
        let ord = field.value(a).cmp(&field.value(b));
        if asc_flag {
            ord
        } else {
            ord.reverse()
        }
    });

    let total_all = tasks.len();
    let total_todo = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Todo)
        .count();
    let total_progress = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::InProgress)
        .count();
    let total_done = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();
    let no_filter = filter() == Option::<TaskStatus>::None;

    fn do_sort(mut sort_field: Signal<TaskSort>, mut asc: Signal<bool>, f: TaskSort) {
        if sort_field() == f {
            *asc.write() = !asc();
        } else {
            sort_field.set(f);
            asc.set(true);
        }
    }

    rsx! {
        div { class: "tasks-grid-container",

            // ── Controles: vista + acciones ──
            div { class: "mb-3 flex items-center justify-between",
                div { class: "inline-flex overflow-hidden rounded-md border border-[var(--border-color)] divide-x divide-[var(--border-color)]", role: "group",
                    button {
                        class: if view_mode == ViewMode::List { "inline-flex items-center justify-center gap-2 px-2.5 py-1 text-sm bg-[var(--primary-color)] text-white" } else { "inline-flex items-center justify-center gap-2 px-2.5 py-1 text-sm text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                        onclick: move |_| on_view_mode_change.call(ViewMode::List),
                        i { class: "bi bi-list-task me-2" }
                        "Lista"
                    }
                    button {
                        class: if view_mode == ViewMode::Kanban { "inline-flex items-center justify-center gap-2 px-2.5 py-1 text-sm bg-[var(--primary-color)] text-white" } else { "inline-flex items-center justify-center gap-2 px-2.5 py-1 text-sm text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                        onclick: move |_| on_view_mode_change.call(ViewMode::Kanban),
                        i { class: "bi bi-columns-gap me-2" }
                        "Kanban"
                    }
                    button {
                        class: if view_mode == ViewMode::Gantt { "inline-flex items-center justify-center gap-2 px-2.5 py-1 text-sm bg-[var(--primary-color)] text-white" } else { "inline-flex items-center justify-center gap-2 px-2.5 py-1 text-sm text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                        onclick: move |_| on_view_mode_change.call(ViewMode::Gantt),
                        i { class: "bi bi-bar-chart-steps me-2" }
                        "Gantt"
                    }
                }
                div { class: "flex items-center gap-2",
                    button {
                        class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90 px-2.5 py-1 text-sm",
                        onclick: move |_| on_add_task.call(()),
                        i { class: "bi bi-plus-lg me-2" }
                        "Nueva Tarea"
                    }
                    div { class: "inline-flex overflow-hidden rounded-md border border-[var(--border-color)] divide-x divide-[var(--border-color)]", role: "group",
                        button {
                            class: if no_filter { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs bg-[var(--primary-color)] text-white" } else { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                            title: "Todas las tareas",
                            onclick: move |_| filter.set(None),
                            "Todas ({total_all})"
                        }
                        button {
                            class: if filter() == Some(TaskStatus::Todo) { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs bg-[var(--bg-tertiary)] text-[var(--text-primary)]" } else { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                            title: "Por hacer",
                            onclick: move |_| filter.set(Some(TaskStatus::Todo)),
                            "Por hacer ({total_todo})"
                        }
                        button {
                            class: if filter() == Some(TaskStatus::InProgress) { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs bg-[var(--primary-color)] text-white" } else { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                            title: "En progreso",
                            onclick: move |_| filter.set(Some(TaskStatus::InProgress)),
                            "En progreso ({total_progress})"
                        }
                        button {
                            class: if filter() == Some(TaskStatus::Done) { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs bg-[var(--success-color)] text-white" } else { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                            title: "Completadas",
                            onclick: move |_| filter.set(Some(TaskStatus::Done)),
                            "Completadas ({total_done})"
                        }
                    }
                }
            }

            // ── Tabla ──
            div { class: "tasks-grid-table rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]",
                div { class: "overflow-x-auto",
                    table { class: "w-full border-collapse text-left",
                        thead { class: "bg-[var(--table-header-bg)]",
                            tr {
                                th { style: "cursor:pointer;width:30%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, TaskSort::Title),
                                    "Tarea "
                                    { icon_sort(sort_field(), TaskSort::Title, asc()) }
                                }
                                th { style: "cursor:pointer;width:12%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, TaskSort::Status),
                                    "Estado "
                                    { icon_sort(sort_field(), TaskSort::Status, asc()) }
                                }
                                th { style: "cursor:pointer;width:12%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, TaskSort::Priority),
                                    "Prioridad "
                                    { icon_sort(sort_field(), TaskSort::Priority, asc()) }
                                }
                                th { style: "width:15%", class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)]", "Asignado" }
                                th { style: "cursor:pointer;width:12%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, TaskSort::StartDate),
                                    "F. Inicio "
                                    { icon_sort(sort_field(), TaskSort::StartDate, asc()) }
                                }
                                th { style: "cursor:pointer;width:12%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, TaskSort::EndDate),
                                    "F. Límite "
                                    { icon_sort(sort_field(), TaskSort::EndDate, asc()) }
                                }
                                th { style: "width:7%", class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)]", "Horas" }
                            }
                        }
                        tbody {
                            if sorted.is_empty() {
                                tr {
                                    td { colspan: "7", class: "text-center text-[var(--text-secondary)] py-12",
                                        if no_filter {
                                            i { class: "bi bi-list-check mb-2 block", style: "font-size:2rem" }
                                            p { class: "mb-0", "No hay tareas en este proyecto" }
                                        } else {
                                            i { class: "bi bi-funnel mb-2 block", style: "font-size:2rem" }
                                            p { class: "mb-0", "No hay tareas con este estado" }
                                        }
                                    }
                                }
                            } else {
                                { sorted.iter().map(|task| {
                                    rsx! {
                                        TaskRow {
                                            key: "{task.id}",
                                            task: task.clone(),
                                            user_name: user_name.clone(),
                                            on_select: on_task_select.clone(),
                                            on_update: on_update_task.clone(),
                                        }
                                    }
                                }) }
                            }
                        }
                    }
                }
            }
        }
    }
}
