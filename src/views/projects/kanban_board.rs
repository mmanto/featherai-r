use crate::views::projects::state::{Task, TaskStatus, ViewMode};
use dioxus::prelude::*;

/// Tablero Kanban de tareas — port de `KanbanBoard.tsx`.
///
/// Tres columnas (Por hacer / En progreso / Completado) con drag & drop nativo
/// (eventos HTML5), menú de tarjeta (Editar/Eliminar) y modal de edición.
#[component]
pub fn KanbanBoard(
    project: crate::views::projects::state::Project,
    view_mode: ViewMode,
    on_view_mode_change: EventHandler<ViewMode>,
    on_add_task: EventHandler<()>,
    on_update_task: EventHandler<Task>,
    on_delete_task: EventHandler<String>,
    on_move_task: EventHandler<(String, TaskStatus)>,
) -> Element {
    let dragged = use_signal(|| Option::<String>::None);
    let drag_over = use_signal(|| Option::<TaskStatus>::None);
    let mut editing_task = use_signal(|| Option::<Task>::None);
    let open_menu = use_signal(|| Option::<String>::None);

    let tasks = project.tasks.clone();

    // Columnas precomputadas: (estado, título, cantidad, tareas).
    let columns: Vec<(TaskStatus, &'static str, usize, Vec<Task>)> = TaskStatus::ALL
        .iter()
        .map(|s| {
            let col: Vec<Task> = tasks.iter().filter(|t| t.status == *s).cloned().collect();
            (*s, s.label(), col.len(), col)
        })
        .collect();

    let delete_with_confirm = move |id: String| {
        let eval = document::eval("confirm('¿Estás seguro de que quieres eliminar esta tarea?');");
        let on_delete_task = on_delete_task.clone();
        spawn(async move {
            let mut eval = eval;
            if eval.recv::<bool>().await.unwrap_or(false) {
                on_delete_task.call(id.clone());
            }
        });
    };

    rsx! {
        div {
            // ── Controles ──
            div { class: "mb-3 flex items-center justify-between",
                div {
                    h6 { class: "mb-0", "Tablero Kanban" }
                    p { class: "text-[var(--text-secondary)] text-xs mb-0", "Arrastra las tareas entre columnas para cambiar su estado" }
                }
                div { class: "flex gap-2",
                    button {
                        class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90 px-2.5 py-1 text-sm",
                        onclick: move |_| on_add_task.call(()),
                        i { class: "bi bi-plus-lg me-2" }
                        "Nueva Tarea"
                    }
                    div { class: "inline-flex overflow-hidden rounded-md border border-[var(--border-color)] divide-x divide-[var(--border-color)]", role: "group",
                        button {
                            class: if view_mode == ViewMode::List { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs bg-[var(--primary-color)] text-white" } else { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                            title: "Vista de lista",
                            onclick: move |_| on_view_mode_change.call(ViewMode::List),
                            i { class: "bi bi-list-task" }
                        }
                        button {
                            class: if view_mode == ViewMode::Kanban { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs bg-[var(--primary-color)] text-white" } else { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                            title: "Vista Kanban",
                            onclick: move |_| on_view_mode_change.call(ViewMode::Kanban),
                            i { class: "bi bi-columns-gap" }
                        }
                        button {
                            class: if view_mode == ViewMode::Gantt { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs bg-[var(--primary-color)] text-white" } else { "inline-flex items-center justify-center gap-1 px-2 py-1 text-xs text-[var(--text-secondary)] bg-transparent hover:bg-[var(--bg-tertiary)]" },
                            title: "Vista Gantt",
                            onclick: move |_| on_view_mode_change.call(ViewMode::Gantt),
                            i { class: "bi bi-bar-chart-steps" }
                        }
                    }
                }
            }

            div { class: "grid grid-cols-12 gap-4",
                { columns.into_iter().map(|(status, col_title, col_count, column_tasks)| {
                    let col_drag_over = drag_over() == Some(status);
                    let open_menu = open_menu.clone();
                    let mut dragged = dragged.clone();
                    let mut drag_over_sig = drag_over.clone();
                    let tasks = tasks.clone();
                    let on_move = on_move_task.clone();
                    let on_update = on_update_task.clone();
                    let on_delete = delete_with_confirm.clone();
                    rsx! {
                        div { class: "col-span-12 md:col-span-4 mb-4",
                            div { class: "flex h-full flex-col rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]", style: "background-color:#f8f9fa;border:1px solid #dee2e6",
                                div { class: "flex items-center justify-between border-b border-[var(--border-color)] bg-[var(--bg-secondary)] px-4 py-3", style: "background-color:#ffffff;border-bottom:1px solid #dee2e6",
                                    h6 { class: "mb-0 text-[var(--text-primary)]", "{col_title}" }
                                    span { class: "inline-flex items-center rounded-full bg-[var(--secondary-color)] px-2 py-0.5 text-xs text-white", "{col_count}" }
                                }
                                div {
                                    class: if col_drag_over { "p-4 bg-[var(--bg-secondary)]" } else { "p-4" },
                                    style: "min-height:400px;max-height:600px;overflow-y:auto",
                                    ondragover: move |e| {
                                        e.prevent_default();
                                        drag_over_sig.set(Some(status));
                                    },
                                    ondragleave: move |_| {
                                        if drag_over_sig() == Some(status) {
                                            drag_over_sig.set(None);
                                        }
                                    },
                                    ondrop: move |e| {
                                        e.prevent_default();
                                        drag_over_sig.set(None);
                                        if let Some(id) = dragged() {
                                            on_move.call((id, status));
                                        }
                                        dragged.set(None);
                                    },
                                    if column_tasks.is_empty() {
                                        div { class: "text-center text-[var(--text-secondary)] mt-4", p { "No hay tareas" } }
                                    } else {
                                        { column_tasks.into_iter().map(|task| {
                                            let tid = task.id.clone();
                                            let tid_drag = tid.clone();
                                            let tid_menu = tid.clone();
                                            let tid_edit = tid.clone();
                                            let tid_delete = tid.clone();
                                            let ttitle = task.title.clone();
                                            let tdesc = task.description.clone();
                                            let tpriority_color = task.priority.color();
                                            let tpriority_label = task.priority.label();
                                            let tassignee = task.assignee.clone();
                                            let tend = task.end_date.clone();
                                            let ttags = task.tags.clone();
                                            let menu_is_open = open_menu() == Some(tid.clone());
                                            let mut open_menu = open_menu.clone();
                                            let mut dragged_card = dragged.clone();
                                            let tasks = tasks.clone();
                                            let on_update = on_update.clone();
                                            let on_delete = on_delete.clone();
                                            rsx! {
                                                div {
                                                    key: "{tid}",
                                                    draggable: "true",
                                                    ondragstart: move |_| {
                                                        dragged_card.set(Some(tid_drag.clone()));
                                                        open_menu.set(None);
                                                    },
                                                    ondragend: move |_| dragged_card.set(None),
                                                    div { class: "mb-2 rounded-lg border border-[#e5e7eb] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]", style: "cursor:move;border:1px solid #e5e7eb",
                                                        div { class: "p-3",
                                                            div { class: "flex items-start justify-between mb-2",
                                                                h6 { class: "font-medium text-[var(--text-primary)] mb-0", style: "font-size:0.9rem;font-weight:500", "{ttitle}" }
                                                                div { class: "relative",
                                                                    button {
                                                                        class: "inline-flex items-center justify-center rounded border border-[var(--border-color)] text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] px-2.5 py-1 text-sm",
                                                                        onclick: move |e| {
                                                                            e.stop_propagation();
                                                                            let cur = open_menu();
                                                                            *open_menu.write() = if cur.as_deref() == Some(&tid_menu) { None } else { Some(tid_menu.clone()) };
                                                                        },
                                                                        "···"
                                                                    }
                                                                    ul { class: if menu_is_open { "absolute right-0 z-[1100] mt-1 min-w-[12rem] rounded-md border border-[var(--card-border)] bg-[var(--card-bg)] py-1 shadow-[var(--shadow)]" } else { "absolute right-0 z-[1100] mt-1 min-w-[12rem] rounded-md border border-[var(--card-border)] bg-[var(--card-bg)] py-1 shadow-[var(--shadow)] hidden" },
                                                                        li {
                                                                            button {
                                                                                class: "flex w-full items-center gap-2 px-3 py-1.5 text-sm text-[var(--text-primary)] hover:bg-[var(--bg-secondary)]",
                                                                                onclick: move |_| {
                                                                                    open_menu.set(None);
                                                                                    if let Some(t) = tasks.iter().find(|t| t.id == tid_edit) {
                                                                                        on_update.call(t.clone());
                                                                                    }
                                                                                },
                                                                                "Editar"
                                                                            }
                                                                        }
                                                                        li {
                                                                            button {
                                                                                class: "flex w-full items-center gap-2 px-3 py-1.5 text-sm text-[var(--danger-color)] hover:bg-[var(--bg-secondary)]",
                                                                                onclick: move |_| {
                                                                                    open_menu.set(None);
                                                                                    on_delete(tid_delete.clone());
                                                                                },
                                                                                "Eliminar"
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            if let Some(desc) = &tdesc {
                                                                p { class: "text-[var(--text-secondary)] text-xs mb-2", "{desc}" }
                                                            }
                                                            div { class: "flex items-center justify-between",
                                                                div { class: "flex items-center gap-2",
                                                                    span { class: "inline-flex items-center rounded-full px-2 py-0.5 text-xs", style: format!("background-color:{tpriority_color};font-size:0.7rem"), "{tpriority_label}" }
                                                                    if let Some(assignee) = &tassignee {
                                                                        small { class: "text-[var(--text-secondary)]", "{assignee}" }
                                                                    }
                                                                }
                                                                if let Some(end) = &tend {
                                                                    small { class: "text-[var(--text-secondary)]", "{end}" }
                                                                }
                                                            }
                                                            if !ttags.is_empty() {
                                                                div { class: "mt-2",
                                                                    { ttags.iter().enumerate().map(|(idx, tag)| {
                                                                        rsx! { span { key: "{idx}", class: "inline-flex items-center rounded-full bg-[var(--bg-tertiary)] px-2 py-0.5 text-xs text-[var(--text-primary)] me-1", style: "font-size:0.7rem", "{tag}" } }
                                                                    }) }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }) }
                                    }
                                }
                            }
                        }
                    }
                }) }
            }

            // ── Modal de edición ──
            if let Some(task) = editing_task() {
                EditTaskModal {
                    task: task.clone(),
                    on_close: move |_| editing_task.set(None),
                    on_save: move |updated| {
                        on_update_task.call(updated);
                        editing_task.set(None);
                    },
                }
            }
        }
    }
}

/// Modal de edición de tarea desde el tablero.
#[component]
fn EditTaskModal(task: Task, on_close: EventHandler<()>, on_save: EventHandler<Task>) -> Element {
    let mut title = use_signal(|| task.title.clone());
    let mut description = use_signal(|| task.description.clone().unwrap_or_default());
    let mut priority = use_signal(|| task.priority);
    let mut assignee = use_signal(|| task.assignee.clone().unwrap_or_default());
    let mut end_date = use_signal(|| task.end_date.clone().unwrap_or_default());

    let save = move |_| {
        let title = title().trim().to_string();
        if title.is_empty() {
            return;
        }
        let mut updated = task.clone();
        updated.title = title;
        updated.description = {
            let d = description().trim().to_string();
            if d.is_empty() {
                None
            } else {
                Some(d)
            }
        };
        updated.priority = priority();
        updated.assignee = {
            let a = assignee().trim().to_string();
            if a.is_empty() {
                None
            } else {
                Some(a)
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
        on_save.call(updated);
    };

    rsx! {
        div { class: "fixed inset-0 z-[1050] flex items-center justify-center overflow-y-auto bg-black/50 p-4",
            div { class: "w-full max-w-md",
                div { class: "w-full rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-lg)]",
                    div { class: "flex items-center justify-between border-b border-[var(--border-color)] px-4 py-3",
                        h5 { class: "text-[1.0625rem] font-medium text-[var(--text-primary)]", "Editar Tarea" }
                        button { class: "flex h-8 w-8 items-center justify-center rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]", onclick: move |_| on_close.call(()), i { class: "bi bi-x-lg" } }
                    }
                    div { class: "p-4",
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Título" }
                            input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", value: title(), oninput: move |e| title.set(e.value()) }
                        }
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Descripción" }
                            textarea { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", rows: "3", value: description(), oninput: move |e| description.set(e.value()) }
                        }
                        div { class: "grid grid-cols-12 gap-4",
                            div { class: "col-span-12 md:col-span-6 mb-3",
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
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Asignado a" }
                                input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", value: assignee(), oninput: move |e| assignee.set(e.value()) }
                            }
                        }
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Fecha límite" }
                            input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", r#type: "date", value: end_date(), oninput: move |e| end_date.set(e.value()) }
                        }
                    }
                    div { class: "flex justify-end gap-2 border-t border-[var(--border-color)] px-4 py-3",
                        button { class: "inline-flex items-center justify-center rounded bg-[var(--bg-tertiary)] text-[var(--text-primary)] hover:opacity-90", onclick: move |_| on_close.call(()), "Cancelar" }
                        button { class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90", onclick: save, "Guardar" }
                    }
                }
            }
        }
    }
}
