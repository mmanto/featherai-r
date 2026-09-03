use crate::components::layout::AppLayout;
use crate::components::protected_route::{ProtectedRoute, SUPER_ADMIN_ROLES};
use crate::views::projects::project_detail::ProjectDetail;
use crate::views::projects::projects_grid::ProjectsGrid;
use crate::views::projects::state::{
    add_project, add_task, delete_task, next_id, select_project, update_project, update_task,
    use_projects, Priority, Project, ProjectProvider, ProjectStatus, Task, TaskStatus,
};
use dioxus::prelude::*;

/// Negocios de demostración para el selector del modal (en React venían de
/// `businessService.getAll()`).
const BUSINESSES: [&str; 3] = ["AgileTeam Corp", "NovaSoft", "TechCorp"];

/// Página `/admin/projects` — port de `ProjectManagement.tsx`.
///
/// Vista raíz autenticada con el proveedor de proyectos: muestra la grilla de
/// proyectos o el detalle del seleccionado, más los modales de creación.
#[component]
pub fn Projects() -> Element {
    rsx! {
        ProtectedRoute {
            roles: Some(SUPER_ADMIN_ROLES),
            AppLayout {
                ProjectProvider {
                    ProjectManagement {}
                }
            }
        }
    }
}

#[component]
fn ProjectManagement() -> Element {
    let state = use_projects();
    let mut show_new_project = use_signal(|| false);
    let mut show_new_task = use_signal(|| false);

    let selected_id = state.read().selected_project_id.clone();
    let selected = state
        .read()
        .projects
        .iter()
        .find(|p| Some(p.id.as_str()) == selected_id.as_deref())
        .cloned();

    // El id del proyecto seleccionado para los handlers del detalle (el
    // componente cierra sobre `state` y relee el id actual al disparar).

    // ── Handlers del detalle — releen el id desde el estado ──
    let detail_on_update_task = {
        let state = state.clone();
        move |t: Task| {
            if let Some(pid) = state.read().selected_project_id.clone() {
                let t2 = t.clone();
                update_task(state.clone(), &pid, &t2.id, |x| *x = t);
            }
        }
    };
    let detail_on_delete_task = {
        let state = state.clone();
        move |id: String| {
            if let Some(pid) = state.read().selected_project_id.clone() {
                delete_task(state.clone(), &pid, &id);
            }
        }
    };
    let detail_on_update_project = {
        let state = state.clone();
        move |p: Project| {
            if let Some(pid) = state.read().selected_project_id.clone() {
                let p = p.clone();
                update_project(state.clone(), &pid, |x| *x = p);
            }
        }
    };

    rsx! {
        if selected_id.is_some() {
            if let Some(project) = selected {
                ProjectDetail {
                    project: project.clone(),
                    on_back: move |_| select_project(state, None),
                    on_back_to_projects: move |_| select_project(state, None),
                    on_add_task: move |_| *show_new_task.write() = true,
                    on_update_task: detail_on_update_task.clone(),
                    on_delete_task: detail_on_delete_task.clone(),
                    on_update_project: detail_on_update_project.clone(),
                }
            }
        } else {
            ProjectsGrid {
                projects: state.read().projects.clone(),
                on_project_select: move |id| select_project(state, Some(id)),
                on_new_project: move |_| *show_new_project.write() = true,
            }
        }

        if show_new_project() {
            NewProjectModal {
                on_close: move |_| *show_new_project.write() = false,
                on_submit: move |p| {
                    add_project(state, p);
                    *show_new_project.write() = false;
                },
            }
        }

        if show_new_task() {
            NewTaskModal {
                on_close: move |_| *show_new_task.write() = false,
                on_submit: move |t| {
                    if let Some(pid) = state.read().selected_project_id.clone() {
                        add_task(state, &pid, t);
                    }
                    *show_new_task.write() = false;
                },
            }
        }
    }
}

/// Modal de nuevo proyecto — port de `NewProjectModal` de `ProjectManagement.tsx`.
#[component]
fn NewProjectModal(on_close: EventHandler<()>, on_submit: EventHandler<Project>) -> Element {
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut business = use_signal(|| BUSINESSES[0].to_string());
    let mut start_date = use_signal(String::new);
    let mut end_date = use_signal(String::new);
    let mut status = use_signal(|| ProjectStatus::Planning);
    let mut priority = use_signal(|| Priority::Medium);
    let mut color = use_signal(|| "#6c757d".to_string());
    let mut team = use_signal(String::new);

    let submit = move |_| {
        let name = name().trim().to_string();
        if name.is_empty() {
            return;
        }
        on_submit.call(Project {
            id: next_id("p"),
            name,
            description: {
                let d = description().trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            business: business(),
            start_date: {
                let d = start_date().trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            end_date: {
                let d = end_date().trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            status: status(),
            priority: priority(),
            team: team()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            color: color(),
            tasks: Vec::new(),
        });
    };

    rsx! {
        div { class: "fixed inset-0 z-[1050] flex items-center justify-center overflow-y-auto bg-black/50 p-4",
            div { class: "w-full max-w-2xl",
                div { class: "w-full rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-lg)]",
                    div { class: "flex items-center justify-between border-b border-[var(--border-color)] px-4 py-3",
                        h5 { class: "text-[1.0625rem] font-medium text-[var(--text-primary)]", "Nuevo Proyecto" }
                        button { class: "flex h-8 w-8 items-center justify-center rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]", onclick: move |_| on_close.call(()), i { class: "bi bi-x-lg" } }
                    }
                    div { class: "p-4",
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Empresa / Negocio *" }
                            select {
                                class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:outline-none",
                                value: business(),
                                onchange: move |e| business.set(e.value()),
                                option { value: "AgileTeam Corp", "AgileTeam Corp" }
                                option { value: "NovaSoft", "NovaSoft" }
                                option { value: "TechCorp", "TechCorp" }
                            }
                        }
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
                                input { class: "h-9 w-12 cursor-pointer rounded border border-[var(--border-color)] bg-transparent p-1", r#type: "color", value: color(), oninput: move |e| color.set(e.value()) }
                            }
                        }
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Miembros del equipo (separados por comas)" }
                            input {
                                class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                value: team(),
                                oninput: move |e| team.set(e.value()),
                                placeholder: "Juan Pérez, María García, Carlos López",
                            }
                        }
                    }
                    div { class: "flex justify-end gap-2 border-t border-[var(--border-color)] px-4 py-3",
                        button { class: "inline-flex items-center justify-center rounded bg-[var(--bg-tertiary)] text-[var(--text-primary)] hover:opacity-90", onclick: move |_| on_close.call(()), "Cancelar" }
                        button { class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90", onclick: submit, "Crear Proyecto" }
                    }
                }
            }
        }
    }
}

/// Modal de nueva tarea — port de `NewTaskModal` de `ProjectManagement.tsx`.
#[component]
fn NewTaskModal(on_close: EventHandler<()>, on_submit: EventHandler<Task>) -> Element {
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut status = use_signal(|| TaskStatus::Todo);
    let mut priority = use_signal(|| Priority::Medium);
    let mut assignee = use_signal(String::new);
    let mut estimated_hours = use_signal(String::new);
    let mut start_date = use_signal(String::new);
    let mut end_date = use_signal(String::new);
    let mut tags = use_signal(String::new);

    let submit = move |_| {
        let title = title().trim().to_string();
        if title.is_empty() {
            return;
        }
        on_submit.call(Task {
            id: next_id("t"),
            title,
            description: {
                let d = description().trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            status: status(),
            assignee: {
                let a = assignee().trim().to_string();
                if a.is_empty() {
                    None
                } else {
                    Some(a)
                }
            },
            priority: priority(),
            start_date: {
                let d = start_date().trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            end_date: {
                let d = end_date().trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            estimated_hours: estimated_hours().trim().parse::<f64>().ok(),
            tags: tags()
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect(),
        });
    };

    rsx! {
        div { class: "fixed inset-0 z-[1050] flex items-center justify-center overflow-y-auto bg-black/50 p-4",
            div { class: "w-full max-w-2xl",
                div { class: "w-full rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-lg)]",
                    div { class: "flex items-center justify-between border-b border-[var(--border-color)] px-4 py-3",
                        h5 { class: "text-[1.0625rem] font-medium text-[var(--text-primary)]", "Nueva Tarea" }
                        button { class: "flex h-8 w-8 items-center justify-center rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]", onclick: move |_| on_close.call(()), i { class: "bi bi-x-lg" } }
                    }
                    div { class: "p-4",
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Título *" }
                            input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", value: title(), oninput: move |e| title.set(e.value()) }
                        }
                        div { class: "mb-3",
                            label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Descripción" }
                            textarea { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", rows: "3", value: description(), oninput: move |e| description.set(e.value()) }
                        }
                        div { class: "grid grid-cols-12 gap-4",
                            div { class: "col-span-12 md:col-span-4 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Estado" }
                                select {
                                    class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:outline-none",
                                    value: status().as_str(),
                                    onchange: move |e| status.set(e.value().as_str().into()),
                                    option { value: "todo", "Por hacer" }
                                    option { value: "in_progress", "En progreso" }
                                    option { value: "done", "Completado" }
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
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Horas estimadas" }
                                input {
                                    r#type: "number",
                                    class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                    value: estimated_hours(),
                                    oninput: move |e| estimated_hours.set(e.value()),
                                    min: "0",
                                    step: "0.5",
                                }
                            }
                        }
                        div { class: "grid grid-cols-12 gap-4",
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Fecha de inicio" }
                                input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", r#type: "date", value: start_date(), oninput: move |e| start_date.set(e.value()) }
                            }
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Fecha límite" }
                                input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", r#type: "date", value: end_date(), oninput: move |e| end_date.set(e.value()) }
                            }
                        }
                        div { class: "grid grid-cols-12 gap-4",
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Asignado a" }
                                input { class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none", value: assignee(), oninput: move |e| assignee.set(e.value()) }
                            }
                            div { class: "col-span-12 md:col-span-6 mb-3",
                                label { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Etiquetas (separadas por comas)" }
                                input {
                                    class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                    value: tags(),
                                    oninput: move |e| tags.set(e.value()),
                                    placeholder: "frontend, ui, bug",
                                }
                            }
                        }
                    }
                    div { class: "flex justify-end gap-2 border-t border-[var(--border-color)] px-4 py-3",
                        button { class: "inline-flex items-center justify-center rounded bg-[var(--bg-tertiary)] text-[var(--text-primary)] hover:opacity-90", onclick: move |_| on_close.call(()), "Cancelar" }
                        button { class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90", onclick: submit, "Crear Tarea" }
                    }
                }
            }
        }
    }
}
