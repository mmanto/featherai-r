use crate::views::projects::state::{
    fmt_date, use_projects, Priority, Project, ProjectState, ProjectStatus,
};
use dioxus::prelude::*;
use std::rc::Rc;

/// Campos ordenables de la grilla — equivalente de `keyof Project` de React.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SortField {
    Name,
    Status,
    StartDate,
    EndDate,
    Priority,
}

impl SortField {
    fn value(self, p: &Project) -> String {
        match self {
            Self::Name => p.name.clone(),
            Self::Status => p.status.as_str().to_string(),
            Self::StartDate => p.start_date.clone().unwrap_or_default(),
            Self::EndDate => p.end_date.clone().unwrap_or_default(),
            Self::Priority => {
                let rank = match p.priority {
                    Priority::Low => 0,
                    Priority::Medium => 1,
                    Priority::High => 2,
                };
                rank.to_string()
            }
        }
    }
}

fn icon_sort(field: SortField, current: SortField, asc: bool) -> Element {
    if field != current {
        return rsx! { i { class: "bi bi-chevron-expand text-[var(--text-secondary)] ms-1" } };
    }
    if asc {
        rsx! { i { class: "bi bi-chevron-up ms-1" } }
    } else {
        rsx! { i { class: "bi bi-chevron-down ms-1" } }
    }
}

/// Fila de la grilla — componente propio para aislar los closures de cada fila
/// (cada fila tiene sus propios signals de hover/edición/menú).
#[component]
fn ProjectRow(
    project: Project,
    state: Signal<ProjectState>,
    on_select: EventHandler<String>,
    on_new_project: EventHandler<()>,
) -> Element {
    let mut hovered = use_signal(|| false);
    let mut editing = use_signal(|| false);
    let mut editing_value = use_signal(|| project.name.clone());
    let mut open_menu = use_signal(|| false);

    // Rc para que cada handler capture clones baratos (closures move 'static).
    let project = Rc::new(project);
    let pid = Rc::new(project.id.clone());
    let pname = Rc::new(project.name.clone());
    let pdesc = project.description.clone();
    let pstart = project.start_date.clone().unwrap_or_default();
    let pend = project.end_date.clone().unwrap_or_default();
    let pstart_fmt = fmt_date(&pstart);
    let pend_fmt = fmt_date(&pend);
    let pcolor = project.color.clone();
    let pstatus_label = project.status.label();
    let pstatus_badge = project.status.badge();
    let ppriority_label = project.priority.label();
    let ppriority_badge = project.priority.badge();
    let team_len = project.team.len();
    let team_suffix = if team_len != 1 { "s" } else { "" };
    let progress = project.progress();

    rsx! {
        tr {
            class: "notion-grid-row",
            style: "cursor:pointer",
            onclick: {
                let pid = pid.clone();
                move |_| on_select.call(pid.as_str().to_string())
            },
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),

            td {
                div { class: "flex items-center gap-2 relative",
                    div { style: format!("width:12px;height:12px;background-color:{pcolor};border-radius:2px;flex-shrink:0") }
                    div { class: "flex-1",
                        if editing() {
                            input {
                                class: "block w-full rounded border border-[var(--border-color)] bg-[var(--card-bg)] px-2 py-1 text-xs text-[var(--text-primary)] focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                value: editing_value(),
                                oninput: move |e| editing_value.set(e.value()),
                                onblur: {
                                let mut state = state.clone();
                                let pid = pid.clone();
                                move |_| {
                                    let value = editing_value().trim().to_string();
                                    if !value.is_empty() {
                                        let mut s = state.write();
                                        if let Some(p) = s.projects.iter_mut().find(|p| p.id == *pid) {
                                            p.name = value;
                                        }
                                    }
                                    editing.set(false);
                                }
                            },
                                onkeydown: {
                                    let mut state = state.clone();
                                    let pid = pid.clone();
                                    move |e| {
                                        if e.key() == Key::Enter {
                                            let value = editing_value().trim().to_string();
                                            if !value.is_empty() {
                                                let mut s = state.write();
                                                if let Some(p) = s.projects.iter_mut().find(|p| p.id == *pid) {
                                                    p.name = value;
                                                }
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
                                    let pname = pname.clone();
                                    move |_| {
                                        editing.set(true);
                                        editing_value.set(pname.as_str().to_string());
                                    }
                                },
                                "{pname}"
                            }
                            if let Some(desc) = &pdesc {
                                small { class: "text-[var(--text-secondary)] block truncate", style: "max-width:300px", "{desc}" }
                            }
                        }
                    }
                    if hovered() {
                        div { class: "notion-hover-actions",
                            button {
                                class: "inline-flex items-center justify-center p-1.5 rounded text-[var(--text-secondary)] hover:bg-[var(--table-hover)] hover:text-[var(--text-primary)]",
                                title: "Abrir",
                                onclick: {
                                    let pid = pid.clone();
                                    move |e| {
                                        e.stop_propagation();
                                        on_select.call(pid.as_str().to_string());
                                    }
                                },
                                i { class: "bi bi-box-arrow-up-right" }
                            }
                        }
                    }
                }
            }
            td {
                span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {pstatus_badge}"), "{pstatus_label}" }
            }
            td { small { "{pstart_fmt}" } }
            td { small { "{pend_fmt}" } }
            td {
                span { class: format!("inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {ppriority_badge}"), "{ppriority_label}" }
            }
            td {
                if team_len > 0 {
                    div { class: "flex items-center gap-1",
                        i { class: "bi bi-people-fill text-[var(--text-secondary)]" }
                        small { "{team_len} miembro{team_suffix}" }
                    }
                } else {
                    small { class: "text-[var(--text-secondary)]", "Sin equipo" }
                }
            }
            td {
                div { class: "flex items-center gap-2",
                    div { class: "h-2 w-full flex-1 overflow-hidden rounded bg-[var(--table-row-odd)]", style: "height:8px",
                        div {
                            class: if progress == 100 { "h-full bg-[var(--success-color)]" } else { "h-full bg-[var(--primary-color)]" },
                            role: "progressbar",
                            style: format!("width:{progress}%"),
                        }
                    }
                    small { class: "text-[var(--text-secondary)]", style: "min-width:35px", "{progress}%" }
                }
            }
            td {
                div { class: "relative",
                    button {
                        class: "inline-flex items-center justify-center p-1.5 rounded text-[var(--text-secondary)] hover:bg-[var(--table-hover)] hover:text-[var(--text-primary)]",
                        onclick: move |e| {
                            e.stop_propagation();
                            *open_menu.write() = !open_menu();
                        },
                        i { class: "bi bi-three-dots-vertical" }
                    }
                    ul { class: if open_menu() { "absolute right-0 z-[1100] mt-1 min-w-[12rem] rounded-md border border-[var(--card-border)] bg-[var(--card-bg)] py-1 shadow-[var(--shadow)]" } else { "absolute right-0 z-[1100] mt-1 min-w-[12rem] rounded-md border border-[var(--card-border)] bg-[var(--card-bg)] py-1 shadow-[var(--shadow)] hidden" },
                        li {
                            button {
                                class: "flex w-full items-center gap-2 px-3 py-1.5 text-sm text-[var(--text-primary)] hover:bg-[var(--bg-secondary)]",
                                onclick: {
                                    let pid = pid.clone();
                                    move |e| {
                                        e.stop_propagation();
                                        open_menu.set(false);
                                        on_select.call(pid.as_str().to_string());
                                    }
                                },
                                i { class: "bi bi-eye me-2" }
                                "Ver detalles"
                            }
                        }
                        li { hr { class: "my-1 border-t border-[var(--border-color)]" } }
                        li {
                            button {
                                class: "flex w-full items-center gap-2 px-3 py-1.5 text-sm text-[var(--danger-color)] hover:bg-[var(--bg-secondary)]",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    open_menu.set(false);
                                    {
                                        let state = state.clone();
                                        let pid = pid.clone();
                                        let eval = document::eval("confirm('¿Eliminar este proyecto?');");
                                        spawn(async move {
                                            let mut eval = eval;
                                            if eval.recv::<bool>().await.unwrap_or(false) {
                                                crate::views::projects::state::delete_project(state, &pid);
                                            }
                                        });
                                    }
                                },
                                i { class: "bi bi-trash me-2" }
                                "Eliminar"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Grilla de proyectos — port de `ProjectsGrid.tsx` (vista lista).
///
/// Header con stats y búsqueda, tabla ordenable con columna de progreso y
/// edición inline del nombre (doble click) a través de [`ProjectRow`].
#[component]
pub fn ProjectsGrid(
    projects: Vec<Project>,
    on_project_select: EventHandler<String>,
    on_new_project: EventHandler<()>,
) -> Element {
    let state = use_projects();
    let mut search = use_signal(String::new);
    let sort_field = use_signal(|| SortField::Name);
    let asc = use_signal(|| true);

    let total = projects.len();
    let active = projects
        .iter()
        .filter(|p| p.status == ProjectStatus::Active)
        .count();
    let planning = projects
        .iter()
        .filter(|p| p.status == ProjectStatus::Planning)
        .count();
    let paused = projects
        .iter()
        .filter(|p| p.status == ProjectStatus::Paused)
        .count();
    let completed = projects
        .iter()
        .filter(|p| p.status == ProjectStatus::Completed)
        .count();

    let query = search().to_lowercase();
    let filtered: Vec<Project> = if query.is_empty() {
        projects.clone()
    } else {
        projects
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query)
                    || p.description
                        .as_deref()
                        .map(|d| d.to_lowercase().contains(&query))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    };
    let results = filtered.len();
    let results_suffix = if results != 1 { "s" } else { "" };
    let searching = !search().is_empty();

    let mut sorted = filtered;
    let field = sort_field();
    let asc_flag = asc();
    sorted.sort_by(|a, b| {
        let ord = field.value(a).cmp(&field.value(b));
        if asc_flag {
            ord
        } else {
            ord.reverse()
        }
    });

    fn do_sort(mut sort_field: Signal<SortField>, mut asc: Signal<bool>, f: SortField) {
        if sort_field() == f {
            *asc.write() = !asc();
        } else {
            sort_field.set(f);
            asc.set(true);
        }
    }

    rsx! {
        div { class: "projects-grid-container",

            // ── Header ──
            div { class: "projects-grid-header rounded-lg bg-[var(--card-bg)] mb-4",
                div { class: "p-4",
                    div { class: "flex items-center justify-between mb-2",
                        h1 { class: "text-[1.75rem] font-medium leading-[1.2] mb-0", style: "font-weight:500",
                            i { class: "bi bi-folder me-2" }
                            "Proyectos"
                        }
                    }
                    div { class: "project-meta-row",
                        span { class: "project-meta-item",
                            i { class: "bi bi-stack me-1" }
                            "{total} en total"
                        }
                        span { class: "project-meta-item text-[var(--success-color)]",
                            i { class: "bi bi-circle-fill me-1", style: "font-size:0.5rem" }
                            "{active} activos"
                        }
                        span { class: "project-meta-item text-[var(--text-secondary)]",
                            i { class: "bi bi-hourglass me-1" }
                            "{planning} en planificación"
                        }
                        if paused > 0 {
                            span { class: "project-meta-item", style: "color:var(--warning-color)",
                                i { class: "bi bi-pause-circle me-1" }
                                "{paused} pausados"
                            }
                        }
                        if completed > 0 {
                            span { class: "project-meta-item", style: "color:var(--primary-color)",
                                i { class: "bi bi-check-circle me-1" }
                                "{completed} completados"
                            }
                        }
                    }
                }
            }

            // ── Tabla ──
            div { class: "projects-grid-table rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]",
                // ── Controles de la grilla: búsqueda + crear ──
                div { class: "flex flex-wrap items-center justify-between gap-3 p-4 border-b border-[var(--border-color)]",
                    div { class: "flex items-center gap-3",
                        div { class: "flex projects-grid-search", style: "width:260px",
                            span { class: "flex items-center border border-r-0 border-[var(--border-color)] bg-[var(--bg-secondary)] px-3 text-[var(--text-secondary)] rounded-l", i { class: "bi bi-search" } }
                            input {
                                class: "block w-full border border-[var(--border-color)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)] rounded-r focus:border-[var(--primary-color)] focus:shadow-[0_0_0_3px_rgba(0,163,240,0.12)] focus:outline-none",
                                placeholder: "Buscar proyectos...",
                                value: search(),
                                oninput: move |e| search.set(e.value()),
                            }
                        }
                        if searching {
                            span { class: "text-[var(--text-secondary)]", style: "font-size:0.8125rem",
                                "{results} resultado{results_suffix}"
                            }
                        }
                    }
                    button {
                        class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90 px-2.5 py-1 text-sm",
                        onclick: move |_| on_new_project.call(()),
                        i { class: "bi bi-plus-lg me-1" }
                        "Nuevo Proyecto"
                    }
                }
                div { class: "overflow-x-auto",
                    table { class: "w-full border-collapse text-left",
                        thead { class: "bg-[var(--table-header-bg)]",
                            tr {
                                th { style: "cursor:pointer;width:25%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, SortField::Name),
                                    "Proyecto "
                                    { icon_sort(SortField::Name, sort_field(), asc()) }
                                }
                                th { style: "cursor:pointer;width:12%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, SortField::Status),
                                    "Estado "
                                    { icon_sort(SortField::Status, sort_field(), asc()) }
                                }
                                th { style: "cursor:pointer;width:12%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, SortField::StartDate),
                                    "Fecha Inicio "
                                    { icon_sort(SortField::StartDate, sort_field(), asc()) }
                                }
                                th { style: "cursor:pointer;width:12%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, SortField::EndDate),
                                    "Fecha Fin "
                                    { icon_sort(SortField::EndDate, sort_field(), asc()) }
                                }
                                th { style: "cursor:pointer;width:11%",
                                    class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)] cursor-pointer",
                                    onclick: move |_| do_sort(sort_field, asc, SortField::Priority),
                                    "Prioridad "
                                    { icon_sort(SortField::Priority, sort_field(), asc()) }
                                }
                                th { style: "width:13%", class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)]", "Equipo" }
                                th { style: "width:10%", class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)]", "Progreso" }
                                th { style: "width:5%", class: "px-3 py-3 text-[0.875rem] font-medium text-[var(--text-primary)] border-b border-[var(--border-color)]" }
                            }
                        }
                        tbody {
                            if sorted.is_empty() {
                                tr {
                                    td { colspan: "8", class: "text-center text-[var(--text-secondary)] py-12",
                                        if searching {
                                            i { class: "bi bi-search mb-2 block", style: "font-size:2rem" }
                                            p { class: "mb-0", "No se encontraron proyectos que coincidan con \"{search()}\"" }
                                        } else {
                                            i { class: "bi bi-folder-plus mb-2 block", style: "font-size:2rem" }
                                            p { class: "mb-2", "No hay proyectos aún" }
                                            button {
                                                class: "inline-flex items-center justify-center rounded border border-[var(--primary-color)] text-[var(--primary-color)] hover:bg-[var(--primary-color)] hover:text-[var(--text-inverse)] px-2.5 py-1 text-sm",
                                                onclick: move |_| on_new_project.call(()),
                                                "Crear el primer proyecto"
                                            }
                                        }
                                    }
                                }
                            } else {
                                { sorted.iter().map(|project| {
                                    rsx! {
                                        ProjectRow {
                                            key: "{project.id}",
                                            project: project.clone(),
                                            state: state.clone(),
                                            on_select: on_project_select.clone(),
                                            on_new_project: on_new_project.clone(),
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
