//! Estado de proyectos — port a Dioxus de `ProjectContext.tsx` de
//! feathrai-frontend, con datos de demostración en memoria (este port no
//! tiene backend).

use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// Genera ids únicos — equivalente de los ids asignados por el servidor.
pub fn next_id(prefix: &str) -> String {
    format!("{prefix}{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

// ── Enums compartidos ────────────────────────────────────────────────────────

/// Estado de una tarea — equivalente de `TaskStatus` en `types/project.ts`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    pub const ALL: [TaskStatus; 3] = [Self::Todo, Self::InProgress, Self::Done];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Todo => "Por hacer",
            Self::InProgress => "En progreso",
            Self::Done => "Completada",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::Todo => "bg-[var(--secondary-color)] text-white",
            Self::InProgress => "bg-[var(--primary-color)] text-white",
            Self::Done => "bg-[var(--success-color)] text-white",
        }
    }
}

impl From<&str> for TaskStatus {
    fn from(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "done" => Self::Done,
            _ => Self::Todo,
        }
    }
}

/// Estado de un proyecto — equivalente de `Project['status']`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProjectStatus {
    Planning,
    Active,
    Paused,
    Completed,
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Planning => "Planificación",
            Self::Active => "Activo",
            Self::Paused => "Pausado",
            Self::Completed => "Completado",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::Planning => "bg-[var(--secondary-color)] text-white",
            Self::Active => "bg-[var(--success-color)] text-white",
            Self::Paused => "bg-[var(--warning-color)] text-[#212529]",
            Self::Completed => "bg-[var(--primary-color)] text-white",
        }
    }
}

impl From<&str> for ProjectStatus {
    fn from(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            _ => Self::Planning,
        }
    }
}

/// Prioridad — equivalente de `Project['priority']` / `Task['priority']`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Baja",
            Self::Medium => "Media",
            Self::High => "Alta",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::Low => "bg-[var(--success-color)] text-white",
            Self::Medium => "bg-[var(--warning-color)] text-[#212529]",
            Self::High => "bg-[var(--danger-color)] text-white",
        }
    }

    /// Color usado por el borde de la barra del Gantt y cards de Kanban.
    pub fn color(self) -> &'static str {
        match self {
            Self::Low => "#10b981",
            Self::Medium => "#f59e0b",
            Self::High => "#ef4444",
        }
    }
}

impl From<&str> for Priority {
    fn from(s: &str) -> Self {
        match s {
            "medium" => Self::Medium,
            "high" => Self::High,
            _ => Self::Low,
        }
    }
}

// ── Tipos de dominio ──────────────────────────────────────────────────────────

/// Tarea — equivalente de `Task` en `types/project.ts`. Las fechas se
/// guardan como ISO `YYYY-MM-DD` (sin dependencias de datetime).
#[derive(Clone, PartialEq, Debug)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub assignee: Option<String>,
    pub priority: Priority,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub estimated_hours: Option<f64>,
    pub tags: Vec<String>,
}

impl Task {
    pub fn estimated_hours_text(&self) -> String {
        match self.estimated_hours {
            Some(h) => format!("{h}h"),
            None => "-".to_string(),
        }
    }
}

/// Proyecto — equivalente de `Project` en `types/project.ts`. El negocio se
/// guarda por nombre (demo; en React era `businessId` + servicio).
#[derive(Clone, PartialEq, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub business: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: ProjectStatus,
    pub priority: Priority,
    pub team: Vec<String>,
    pub color: String,
    pub tasks: Vec<Task>,
}

impl Project {
    /// Progreso 0-100 — equivalente de `calculateProgress` de React.
    pub fn progress(&self) -> u8 {
        if self.tasks.is_empty() {
            return 0;
        }
        let done = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Done)
            .count();
        ((done as f64 / self.tasks.len() as f64) * 100.0).round() as u8
    }

    pub fn task_count(&self, status: TaskStatus) -> usize {
        self.tasks.iter().filter(|t| t.status == status).count()
    }
}

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

impl ProjectState {
    fn seed() -> Self {
        Self {
            projects: seed_projects(),
            selected_project_id: None,
        }
    }
}

// ── Proveedor y acciones ──────────────────────────────────────────────────────

/// Proveedor del estado de proyectos — equivalente de `ProjectProvider`.
#[component]
pub fn ProjectProvider(children: Element) -> Element {
    let state = use_signal(ProjectState::seed);
    use_context_provider(|| state);
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
pub fn add_project(mut state: Signal<ProjectState>, project: Project) {
    state.write().projects.push(project);
}

/// Actualiza un proyecto — equivalente de `updateProject`.
pub fn update_project(mut state: Signal<ProjectState>, id: &str, f: impl FnOnce(&mut Project)) {
    let mut s = state.write();
    if let Some(p) = s.projects.iter_mut().find(|p| p.id == id) {
        f(p);
    }
}

/// Elimina un proyecto — equivalente de `deleteProject`.
pub fn delete_project(mut state: Signal<ProjectState>, id: &str) {
    let mut s = state.write();
    s.projects.retain(|p| p.id != id);
    if s.selected_project_id.as_deref() == Some(id) {
        s.selected_project_id = None;
    }
}

/// Agrega una tarea a un proyecto — equivalente de `addTask`.
pub fn add_task(mut state: Signal<ProjectState>, project_id: &str, task: Task) {
    let mut s = state.write();
    if let Some(p) = s.projects.iter_mut().find(|p| p.id == project_id) {
        p.tasks.push(task);
    }
}

/// Actualiza una tarea — equivalente de `updateTask`.
#[allow(clippy::too_many_arguments)]
pub fn update_task(
    mut state: Signal<ProjectState>,
    project_id: &str,
    task_id: &str,
    f: impl FnOnce(&mut Task),
) {
    let mut s = state.write();
    if let Some(p) = s.projects.iter_mut().find(|p| p.id == project_id) {
        if let Some(t) = p.tasks.iter_mut().find(|t| t.id == task_id) {
            f(t);
        }
    }
}

/// Elimina una tarea — equivalente de `deleteTask`.
pub fn delete_task(mut state: Signal<ProjectState>, project_id: &str, task_id: &str) {
    let mut s = state.write();
    if let Some(p) = s.projects.iter_mut().find(|p| p.id == project_id) {
        p.tasks.retain(|t| t.id != task_id);
    }
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

// ── Datos de demostración ─────────────────────────────────────────────────────

fn seed_projects() -> Vec<Project> {
    let mut p1 = Project {
        id: "p1".into(),
        name: "Sistema de Gestión de Inventarios".into(),
        description: Some("Plataforma web para control de stock y reposiciones.".into()),
        business: "AgileTeam Corp".into(),
        start_date: Some("2026-08-03".into()),
        end_date: Some("2026-10-30".into()),
        status: ProjectStatus::Active,
        priority: Priority::High,
        team: vec!["Juan Pérez".into(), "María García".into()],
        color: "#00A3F0".into(),
        tasks: vec![
            Task {
                id: "t1".into(),
                title: "Modelo de datos de productos".into(),
                description: Some("Entidades, relaciones y migraciones iniciales.".into()),
                status: TaskStatus::Done,
                assignee: Some("María García".into()),
                priority: Priority::High,
                start_date: Some("2026-08-03".into()),
                end_date: Some("2026-08-14".into()),
                estimated_hours: Some(24.0),
                tags: vec!["backend".into(), "database".into()],
            },
            Task {
                id: "t2".into(),
                title: "API de movimientos de stock".into(),
                description: Some("Alta, baja y ajuste de existencias.".into()),
                status: TaskStatus::InProgress,
                assignee: Some("Juan Pérez".into()),
                priority: Priority::High,
                start_date: Some("2026-08-15".into()),
                end_date: Some("2026-09-12".into()),
                estimated_hours: Some(40.0),
                tags: vec!["backend".into(), "api".into()],
            },
            Task {
                id: "t3".into(),
                title: "Pantalla de inventario".into(),
                description: Some("Tabla con filtros y exportación a CSV.".into()),
                status: TaskStatus::InProgress,
                assignee: None,
                priority: Priority::Medium,
                start_date: Some("2026-09-01".into()),
                end_date: Some("2026-09-30".into()),
                estimated_hours: Some(32.0),
                tags: vec!["frontend".into(), "ui".into()],
            },
            Task {
                id: "t4".into(),
                title: "Alertas de reposición".into(),
                description: Some("Notificaciones por email bajo umbral mínimo.".into()),
                status: TaskStatus::Todo,
                assignee: Some("Juan Pérez".into()),
                priority: Priority::Low,
                start_date: Some("2026-10-01".into()),
                end_date: Some("2026-10-23".into()),
                estimated_hours: Some(16.0),
                tags: vec!["notifications".into()],
            },
        ],
    };

    let mut p2 = Project {
        id: "p2".into(),
        name: "Portal de Ventas E-commerce".into(),
        description: Some("Catálogo, carrito y checkout para tienda online.".into()),
        business: "NovaSoft".into(),
        start_date: Some("2026-09-01".into()),
        end_date: Some("2026-12-15".into()),
        status: ProjectStatus::Planning,
        priority: Priority::Medium,
        team: vec!["Carlos López".into()],
        color: "#8b5cf6".into(),
        tasks: vec![
            Task {
                id: "t5".into(),
                title: "Definición de alcance".into(),
                description: Some("Requerimientos y mapeo de flujos de compra.".into()),
                status: TaskStatus::InProgress,
                assignee: Some("Carlos López".into()),
                priority: Priority::High,
                start_date: Some("2026-09-01".into()),
                end_date: Some("2026-09-15".into()),
                estimated_hours: Some(20.0),
                tags: vec!["planning".into()],
            },
            Task {
                id: "t6".into(),
                title: "Diseño del catálogo".into(),
                description: Some("Wireframes de listado y detalle de producto.".into()),
                status: TaskStatus::Todo,
                assignee: None,
                priority: Priority::Medium,
                start_date: Some("2026-09-16".into()),
                end_date: Some("2026-10-05".into()),
                estimated_hours: Some(30.0),
                tags: vec!["design".into(), "ux".into()],
            },
            Task {
                id: "t7".into(),
                title: "Integración de pagos".into(),
                description: Some("Evaluación de proveedores y sandbox.".into()),
                status: TaskStatus::Todo,
                assignee: None,
                priority: Priority::Medium,
                start_date: Some("2026-10-06".into()),
                end_date: Some("2026-11-20".into()),
                estimated_hours: Some(48.0),
                tags: vec!["payments".into(), "integration".into()],
            },
        ],
    };

    let mut p3 = Project {
        id: "p3".into(),
        name: "App Móvil de Fidelización".into(),
        description: Some("Programa de puntos y beneficios para clientes.".into()),
        business: "TechCorp".into(),
        start_date: Some("2026-03-10".into()),
        end_date: Some("2026-07-20".into()),
        status: ProjectStatus::Completed,
        priority: Priority::Low,
        team: vec!["Ana Ruiz".into(), "Pedro Díaz".into()],
        color: "#10b981".into(),
        tasks: vec![
            Task {
                id: "t8".into(),
                title: "Módulo de puntos".into(),
                description: Some("Acumulación y canje de puntos por compras.".into()),
                status: TaskStatus::Done,
                assignee: Some("Ana Ruiz".into()),
                priority: Priority::High,
                start_date: Some("2026-03-10".into()),
                end_date: Some("2026-04-30".into()),
                estimated_hours: Some(50.0),
                tags: vec!["backend".into()],
            },
            Task {
                id: "t9".into(),
                title: "App nativa iOS/Android".into(),
                description: Some("Cliente móvil con wallet de beneficios.".into()),
                status: TaskStatus::Done,
                assignee: Some("Pedro Díaz".into()),
                priority: Priority::Medium,
                start_date: Some("2026-04-01".into()),
                end_date: Some("2026-06-15".into()),
                estimated_hours: Some(80.0),
                tags: vec!["mobile".into(), "ui".into()],
            },
            Task {
                id: "t10".into(),
                title: "Lanzamiento en tiendas".into(),
                description: Some("Publicación y revisión de releases.".into()),
                status: TaskStatus::Done,
                assignee: Some("Ana Ruiz".into()),
                priority: Priority::Low,
                start_date: Some("2026-06-20".into()),
                end_date: Some("2026-07-20".into()),
                estimated_hours: Some(12.0),
                tags: vec!["release".into()],
            },
        ],
    };

    p1.tasks[1].id = "t2".into();
    p1.tasks[2].id = "t3".into();
    p1.tasks[3].id = "t4".into();
    p2.tasks[0].id = "t5".into();
    p2.tasks[1].id = "t6".into();
    p2.tasks[2].id = "t7".into();
    p3.tasks[0].id = "t8".into();
    p3.tasks[1].id = "t9".into();
    p3.tasks[2].id = "t10".into();

    vec![p1, p2, p3]
}
