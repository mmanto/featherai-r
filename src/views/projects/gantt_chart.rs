use crate::views::projects::state::{epoch_days, TaskStatus, ViewMode};
use dioxus::prelude::*;

/// Diagrama de Gantt — port de `GanttChart.tsx`.
///
/// Solo muestra tareas con fechas de inicio y fin definidas; las barras se
/// posicionan contra el rango mínimo/máximo de fechas del proyecto.
#[component]
pub fn GanttChart(
    project: crate::views::projects::state::Project,
    view_mode: ViewMode,
    on_view_mode_change: EventHandler<ViewMode>,
    on_add_task: EventHandler<()>,
) -> Element {
    let tasks = project.tasks.clone();

    // Tareas con ambas fechas, con sus epoch days calculados.
    let dated: Vec<(usize, i64, i64)> = tasks
        .iter()
        .enumerate()
        .filter_map(|(idx, t)| {
            let start = t.start_date.as_deref().and_then(epoch_days)?;
            let end = t.end_date.as_deref().and_then(epoch_days)?;
            Some((idx, start, end))
        })
        .collect();

    if dated.is_empty() {
        return rsx! {
            div { class: "text-center text-[var(--text-secondary)] mt-12",
                div { class: "mb-3 flex items-center justify-between",
                    div {
                        h6 { class: "mb-0", "Diagrama de Gantt - {project.name}" }
                        p { class: "text-[var(--text-secondary)] text-xs mb-0", "0 tareas con fechas definidas" }
                    }
                    div { class: "flex items-center gap-2",
                        button { class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90 px-2.5 py-1 text-sm", onclick: move |_| on_add_task.call(()), i { class: "bi bi-plus-lg" } }
                        { view_buttons(view_mode, on_view_mode_change) }
                    }
                }
                h5 { "No hay tareas con fechas definidas" }
                p { "Asigna fechas de inicio y fin a las tareas para ver el diagrama de Gantt" }
            }
        };
    }

    let min_day = dated.iter().map(|(_, s, _)| *s).min().unwrap_or(0) - 1;
    let max_day = dated.iter().map(|(_, _, e)| *e).max().unwrap_or(0) + 1;
    let total_days = (max_day - min_day).max(1);
    let day_width = (800_f64 / total_days as f64).clamp(40.0, 120.0) as i32;
    let row_height = 50;
    let dates: Vec<i64> = (min_day..=max_day).collect();
    let dates_len = dates.len() as i32;

    let total = tasks.len();
    let done = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();
    let in_progress = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::InProgress)
        .count();
    let progress_pct = if total == 0 {
        0
    } else {
        (done as f64 / total as f64 * 100.0).round() as i64
    };

    // Convierte epoch day → fecha ISO para los rótulos de cabecera
    // (inverso del conversor civil guardado en state.rs).
    let iso_of_day = |day: i64| {
        let z = day + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        format!("{y:04}-{m:02}-{d:02}")
    };

    let bar_spec = |start: i64, end: i64| {
        let left = ((start - min_day) as i32) * day_width;
        let duration = (end - start + 1).max(1) as i32;
        let width = duration * day_width;
        (left, width)
    };

    rsx! {
        div { class: "gantt-container",
            div { class: "mb-3 flex items-start justify-between",
                div {
                    h6 { class: "mb-0", "Diagrama de Gantt - {project.name}" }
                    p { class: "text-[var(--text-secondary)] text-xs mb-0", "{dated.len()} tareas con fechas definidas" }
                }
                div { class: "flex items-center gap-2",
                    button { class: "inline-flex items-center justify-center rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90 px-2.5 py-1 text-sm", onclick: move |_| on_add_task.call(()), i { class: "bi bi-plus-lg me-2" }, "Nueva Tarea" }
                    { view_buttons(view_mode, on_view_mode_change) }
                }
            }

            // ── Leyenda ──
            div { class: "mb-2 flex gap-3 text-sm",
                div { class: "flex items-center gap-1",
                    div { style: "width:12px;height:12px;background-color:#6b7280;border-radius:2px" }
                    span { "Por hacer" }
                }
                div { class: "flex items-center gap-1",
                    div { style: "width:12px;height:12px;background-color:#00A3F0;border-radius:2px" }
                    span { "En progreso" }
                }
                div { class: "flex items-center gap-1",
                    div { style: "width:12px;height:12px;background-color:#10b981;border-radius:2px" }
                    span { "Completado" }
                }
            }

            div { class: "rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]",
                div { class: "p-0",
                    div { style: "display:flex;overflow-x:auto;max-width:100%",

                        // ── Columna izquierda: nombres ──
                        div { style: "min-width:200px;background-color:#f8f9fa;border-right:1px solid #dee2e6",
                            div { style: "height:60px;border-bottom:1px solid #dee2e6;display:flex;align-items:center;padding-left:16px;font-weight:600;background-color:#ffffff",
                                "Tarea"
                            }
                            { dated.iter().map(|(idx, _, _)| {
                                let ttitle = tasks[*idx].title.clone();
                                let tassignee = tasks[*idx].assignee.clone().unwrap_or_else(|| "Sin asignar".into());
                                rsx! {
                                    div { key: "{idx}", style: format!("height:{row_height}px;border-bottom:1px solid #dee2e6;display:flex;align-items:center;padding-left:16px;padding-right:16px"),
                                        div {
                                            div { style: "font-weight:500;font-size:0.9rem", "{ttitle}" }
                                            div { style: "font-size:0.75rem;color:#6b7280", "{tassignee}" }
                                        }
                                    }
                                }
                            }) }
                        }

                        // ── Columna derecha: fechas + barras ──
                        div { style: format!("flex:1;min-width:{dates_len}px"),
                            div { style: "height:60px;border-bottom:1px solid #dee2e6;display:flex;background-color:#ffffff",
                                { dates.iter().map(|day| {
                                    let iso = iso_of_day(*day);
                                    let wd = crate::views::projects::state::weekday_short(&iso).to_string();
                                    let dn = iso.rsplit('-').next().unwrap_or("").to_string();
                                    rsx! {
                                        div { style: format!("width:{day_width}px;border-right:1px solid #e5e7eb;display:flex;flex-direction:column;align-items:center;justify-content:center;font-size:0.75rem;color:#6b7280"),
                                            div { "{wd}" }
                                            div { style: "font-weight:600;color:#374151", "{dn}" }
                                        }
                                    }
                                }) }
                            }
                            div { style: "position:relative",
                                { dated.iter().map(|(idx, start, end)| {
                                    let (left, width) = bar_spec(*start, *end);
                                    let task = &tasks[*idx];
                                    let ttitle = task.title.clone();
                                    let bar_color = match task.status {
                                        TaskStatus::Done => "#10b981",
                                        TaskStatus::InProgress => "#00A3F0",
                                        TaskStatus::Todo => "#6b7280",
                                    };
                                    let border_color = task.priority.color();
                                    let bg = if idx % 2 == 0 { "#ffffff" } else { "#f9fafb" };
                                    let tooltip = format!("{} ({}) - {} priority", ttitle, task.status.label(), task.priority.label());
                                    let bar_style = format!(
                                        "position:absolute;left:{left}px;width:{width}px;height:{}px;top:4px;background-color:{bar_color};border-radius:4px;border:2px solid {border_color};opacity:0.8;display:flex;align-items:center;padding-left:8px;color:white;font-size:0.8rem;font-weight:500;overflow:hidden;white-space:nowrap;text-overflow:ellipsis",
                                        row_height - 8,
                                    );
                                    rsx! {
                                        div { key: "{idx}",
                                            div { style: format!("height:{row_height}px;border-bottom:1px solid #dee2e6;position:relative;background-color:{bg}"),
                                                div { style: bar_style, title: tooltip, "{ttitle}" }
                                            }
                                        }
                                    }
                                }) }
                            }
                        }
                    }
                }
            }

            // ── Resumen ──
            div { class: "mt-3",
                div { class: "grid grid-cols-12 gap-4",
                    div { class: "col-span-12 md:col-span-6",
                        div { class: "rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]",
                            div { class: "p-4",
                                h6 { class: "font-medium text-[var(--text-primary)]", "Resumen del Proyecto" }
                                div { class: "grid grid-cols-12 text-center",
                                    div { class: "col-span-4",
                                        div { class: "text-[var(--text-secondary)] text-sm", "Total" }
                                        div { class: "font-bold", "{total}" }
                                    }
                                    div { class: "col-span-4",
                                        div { class: "text-[var(--text-secondary)] text-sm", "Completadas" }
                                        div { class: "font-bold text-[var(--success-color)]", "{done}" }
                                    }
                                    div { class: "col-span-4",
                                        div { class: "text-[var(--text-secondary)] text-sm", "En progreso" }
                                        div { class: "font-bold text-[var(--primary-color)]", "{in_progress}" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "col-span-12 md:col-span-6",
                        div { class: "rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]",
                            div { class: "p-4",
                                h6 { class: "font-medium text-[var(--text-primary)]", "Avance" }
                                div { class: "flex items-center gap-2",
                                    div { class: "h-2 w-full flex-1 overflow-hidden rounded bg-[var(--table-row-odd)]", style: "height:8px",
                                        div {
                                            class: if progress_pct == 100 { "h-full bg-[var(--success-color)]" } else { "h-full bg-[var(--primary-color)]" },
                                            style: format!("width:{progress_pct}%"),
                                        }
                                    }
                                    small { class: "font-medium text-[var(--text-secondary)]", "{progress_pct}%" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn view_buttons(view_mode: ViewMode, on_view_mode_change: EventHandler<ViewMode>) -> Element {
    rsx! {
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
