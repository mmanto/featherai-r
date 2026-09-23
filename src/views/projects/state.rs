//! Estado de proyectos — port a Dioxus de `ProjectContext.tsx` de
//! feathrai-frontend.
//!
//! # Modelo compartido
//! El modelo de dominio ([`Project`], [`Task`] y los enums) vive en
//! `crate::models` — lo comparten la UI, la persistencia y la capa de
//! servicios — y se re-exporta acá para que las vistas sigan importando
//! desde `state`. Este archivo conserva lo específico de la vista:
//! [`ViewMode`], [`ProjectState`], el proveedor, las acciones y las fechas.
//!
//! # Persistencia (nativo) vs demo (web)
//! En desktop, [`ProjectProvider`] arranca con el snapshot persistido que
//! `init_backend` (main.rs) tomó de GuardianDB y lo refresca al montar y cada
//! 3 s mientras la vista está montada. Las
//! acciones (`add_project`, [`update_project`], …) son `async`: mutan el
//! estado local (optimista) y persisten vía `crate::services`, que devuelve
//! la entidad canónica (id uuid v4 asignado por el servicio, `created_at`,
//! `position`) con la que se reconcilia el estado. Devuelven `Err(String)`
//! cuando la escritura falla para que la vista muestre un toast.
//!
//! En web/wasm no hay capa de persistencia: las acciones mutan el estado en
//! memoria con ids provisionales ([`next_id`]) y la siembra inicial es
//! `demo::demo_projects`.

pub use crate::models::{Priority, Project, ProjectStatus, Task, TaskStatus};
use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// Genera ids provisionales únicos para los modales (el modelo demo de React
/// los asignaba en el servidor). En nativo la capa de servicios los
/// reemplaza por uuid v4 al persistir; en web quedan como id definitivo.
pub fn next_id(prefix: &str) -> String {
    format!("{prefix}{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

/// Intervalo del refresco automático del listado en nativo (mismo valor que
/// la lista de pares en Ajustes).
#[cfg(not(target_arch = "wasm32"))]
const PROJECT_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);

/// Modo de vista de las tareas — equivalente de `ViewMode` de React.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ViewMode {
    #[default]
    List,
    Kanban,
    Gantt,
}

/// Estado global del módulo de proyectos — equivalente de los useStates de
/// `ProjectContext`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ProjectState {
    pub projects: Vec<Project>,
    pub selected_project_id: Option<String>,
}

// ── Proveedor y acciones ──────────────────────────────────────────────────────

/// Listado inicial de proyectos:
/// - nativo: el snapshot persistido tomado en `init_backend` (main.rs);
/// - web/wasm: la siembra demo en memoria.
#[cfg(not(target_arch = "wasm32"))]
fn initial_projects() -> Vec<Project> {
    crate::persistence::initial_projects()
}

#[cfg(target_arch = "wasm32")]
fn initial_projects() -> Vec<Project> {
    crate::demo::demo_projects()
}

/// Proveedor del estado de proyectos — equivalente de `ProjectProvider`.
///
/// En nativo refresca el listado desde GuardianDB al montar y luego cada 3 s
/// mientras la vista está montada, para que los cambios hechos por otro nodo
/// (sync Iroh) o por un escritor externo aparezcan sin reiniciar la app. Los
/// cambios quedan persistidos aunque la vista se desmonte al navegar.
#[component]
pub fn ProjectProvider(children: Element) -> Element {
    let initial = initial_projects();
    let state = use_signal(|| ProjectState {
        projects: initial,
        selected_project_id: None,
    });
    use_context_provider(|| state);

    #[cfg(not(target_arch = "wasm32"))]
    use_effect(move || {
        let state = state.clone();
        spawn(async move {
            let mut state = state;
            loop {
                match crate::services::project::reload_all(crate::persistence::db()).await {
                    Ok(projects) => {
                        let mut s = state.write();
                        if s.projects != projects {
                            // Si el proyecto seleccionado ya no existe (lo borró
                            // otro nodo), limpiar la selección: ProjectManagement
                            // no renderiza nada con `selected_id` colgado.
                            let seleccionado_ausente = s
                                .selected_project_id
                                .as_deref()
                                .is_some_and(|sel| !projects.iter().any(|p| p.id == sel));
                            if seleccionado_ausente {
                                s.selected_project_id = None;
                            }
                            s.projects = projects;
                        }
                    }
                    Err(e) => eprintln!("[featherai] refrescando proyectos: {e}"),
                }
                tokio::time::sleep(PROJECT_REFRESH_INTERVAL).await;
            }
        });
    });

    rsx! { {children} }
}

/// Acceso al estado de proyectos — equivalente de `useProject()`.
///
/// # Panics
/// Fuera de un [`ProjectProvider`].
pub fn use_projects() -> Signal<ProjectState> {
    use_context::<Signal<ProjectState>>()
}

/// Selecciona o deselecciona un proyecto — equivalente de `selectProject`.
pub fn select_project(mut state: Signal<ProjectState>, id: Option<String>) {
    state.write().selected_project_id = id;
}

/// Agrega un proyecto — equivalente de `addProject`.
///
/// Nativo: persiste vía `services::project::create_project` (asigna uuid v4
/// y `created_at`) y agrega al estado el proyecto canónico devuelto. Web:
/// lo agrega tal cual (id provisional).
pub async fn add_project(mut state: Signal<ProjectState>, project: Project) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let created = crate::services::project::create_project(crate::persistence::db(), project)
            .await
            .map_err(|e| e.to_string())?;
        state.write().projects.push(created);
    }
    #[cfg(target_arch = "wasm32")]
    {
        state.write().projects.push(project);
    }
    Ok(())
}

/// Actualiza un proyecto — equivalente de `updateProject`.
///
/// Aplica `f` al proyecto en el estado (optimista) y, en nativo, persiste el
/// resultado vía `services::project::replace_project` (conserva
/// `created_at`). Proyecto inexistente → no-op.
pub async fn update_project(
    mut state: Signal<ProjectState>,
    id: &str,
    f: impl FnOnce(&mut Project),
) -> Result<(), String> {
    let mut updated = None;
    {
        let mut s = state.write();
        if let Some(p) = s.projects.iter_mut().find(|p| p.id == id) {
            f(p);
            updated = Some(p.clone());
        }
    }
    let Some(project) = updated else {
        return Ok(());
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::services::project::replace_project(crate::persistence::db(), &project)
            .await
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_arch = "wasm32")]
    let _ = &project; // ya aplicado al estado en memoria
    Ok(())
}

/// Elimina un proyecto — equivalente de `deleteProject`.
///
/// Nativo: borra de GuardianDB (proyecto + sus TaskDoc) y recién después lo
/// quita del estado local. Web: solo estado local.
pub async fn delete_project(mut state: Signal<ProjectState>, id: String) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    crate::services::project::delete_project(crate::persistence::db(), &id)
        .await
        .map_err(|e| e.to_string())?;
    let mut s = state.write();
    s.projects.retain(|p| p.id != id);
    if s.selected_project_id.as_deref() == Some(id.as_str()) {
        s.selected_project_id = None;
    }
    Ok(())
}

/// Agrega una tarea a un proyecto — equivalente de `addTask`.
///
/// Nativo: persiste vía `services::project::add_task` (asigna uuid v4 y
/// `position` = max+1) y agrega la tarea canónica al proyecto del estado.
/// Proyecto inexistente → `Err` (toast en la vista).
pub async fn add_task(
    mut state: Signal<ProjectState>,
    project_id: &str,
    task: Task,
) -> Result<(), String> {
    let project_id = project_id.to_string();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let created =
            crate::services::project::add_task(crate::persistence::db(), &project_id, task)
                .await
                .map_err(|e| e.to_string())?;
        let mut s = state.write();
        if let Some(p) = s.projects.iter_mut().find(|p| p.id == project_id) {
            p.tasks.push(created);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut s = state.write();
        if let Some(p) = s.projects.iter_mut().find(|p| p.id == project_id) {
            p.tasks.push(task);
        }
    }
    Ok(())
}

/// Actualiza una tarea — equivalente de `updateTask`.
///
/// Aplica `f` a la tarea en el estado (optimista) y, en nativo, persiste el
/// resultado vía `services::project::replace_task` (conserva `position`).
/// Tarea inexistente → no-op.
#[allow(clippy::too_many_arguments)]
pub async fn update_task(
    mut state: Signal<ProjectState>,
    project_id: &str,
    task_id: &str,
    f: impl FnOnce(&mut Task),
) -> Result<(), String> {
    let project_id = project_id.to_string();
    let task_id = task_id.to_string();
    let mut updated = None;
    {
        let mut s = state.write();
        if let Some(p) = s.projects.iter_mut().find(|p| p.id == project_id) {
            if let Some(t) = p.tasks.iter_mut().find(|t| t.id == task_id) {
                f(t);
                updated = Some(t.clone());
            }
        }
    }
    let Some(task) = updated else {
        return Ok(());
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::services::project::replace_task(crate::persistence::db(), &project_id, &task)
            .await
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_arch = "wasm32")]
    let _ = &task; // ya aplicado al estado en memoria
    Ok(())
}

/// Elimina una tarea — equivalente de `deleteTask`.
///
/// Nativo: borra de GuardianDB y recién después del estado local. Si el
/// TaskDoc no existe o pertenece a otro proyecto → no-op (contrato
/// idempotente del servicio).
pub async fn delete_task(
    mut state: Signal<ProjectState>,
    project_id: &str,
    task_id: &str,
) -> Result<(), String> {
    let project_id = project_id.to_string();
    let task_id = task_id.to_string();
    #[cfg(not(target_arch = "wasm32"))]
    crate::services::project::delete_task(crate::persistence::db(), &project_id, &task_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut s = state.write();
    if let Some(p) = s.projects.iter_mut().find(|p| p.id == project_id) {
        p.tasks.retain(|t| t.id != task_id);
    }
    Ok(())
}

// ── Fechas (ISO YYYY-MM-DD) ──────────────────────────────────────────────────

const MONTHS_SHORT: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
];
const WEEKDAYS_SHORT: [&str; 7] = ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"];

pub fn parse_ymd(iso: &str) -> Option<(i32, u32, u32)> {
    let mut it = iso.split('-');
    let year = it.next()?.parse().ok()?;
    let month = it.next()?.parse().ok()?;
    let day = it.next()?.parse().ok()?;
    Some((year, month, day))
}

/// Días desde 1970-01-01 (algoritmo de Howard Hinnant, sin dependencias).
pub fn epoch_days(iso: &str) -> Option<i64> {
    let (y, m, d) = parse_ymd(iso)?;
    let adj_y = if m <= 2 { y - 1 } else { y };
    let adj_m = if m <= 2 { m as i64 + 12 } else { m as i64 };
    let era = if adj_y >= 0 { adj_y } else { adj_y - 399 } / 400;
    let yoe = (adj_y - era * 400) as i64;
    let doy = (153 * (adj_m - 3) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era as i64 * 146_097 + doe - 719_468)
}

/// `"30 ago 2026"` — equivalente de `toLocaleDateString('es-ES', ...)`.
pub fn fmt_date(iso: &str) -> String {
    let Some((y, m, d)) = parse_ymd(iso) else {
        return "-".to_string();
    };
    format!("{d} {} {y}", MONTHS_SHORT[(m - 1) as usize])
}

/// Día de la semana corto en español — para los encabezados del Gantt.
pub fn weekday_short(iso: &str) -> &'static str {
    let day = epoch_days(iso).unwrap_or(0);
    WEEKDAYS_SHORT[(day + 4).rem_euclid(7) as usize]
}
