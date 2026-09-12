//! Tipos de dominio de proyectos/tareas — movidos desde
//! `src/views/projects/state.rs` para que la capa de persistencia
//! (`src/services/`, `src/persistence/`) y las vistas compartan el mismo
//! modelo. Los tipos serializan a JSON (docs de GuardianDB); `tasks` no se
//! serializa en el doc de metadatos del proyecto (cada tarea es un doc
//! aparte en el store `tasks`).

use serde::{Deserialize, Serialize};

// ── Enums compartidos ────────────────────────────────────────────────────────

/// Estado de una tarea — equivalente de `TaskStatus` en `types/project.ts`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
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

/// Proyecto — equivalente de `Project` en `types/project.ts`. El negocio es
/// opcional (`None` = proyecto sin negocio) y se guarda por nombre; en React
/// era `businessId` + servicio. No hay entidad propia de negocios: se crean
/// desde el formulario de proyecto y se reutilizan por nombre.
///
/// `created_at` (ms epoch) lo asigna la capa de servicios al persistir y se
/// usa para ordenar el listado; `tasks` nunca viaja en el doc de metadatos
/// (las tareas viven en docs aparte) y por eso se omite al serializar.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Negocio del proyecto (opcional, por nombre). `#[serde(default)]`:
    /// los docs viejos sin el campo (o sin negocio) deserializan a `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: ProjectStatus,
    pub priority: Priority,
    pub team: Vec<String>,
    pub color: String,
    /// Marca de creación (ms epoch) asignada por la persistencia; `None` en
    /// objetos recién construidos por la UI (el servicio la completa).
    pub created_at: Option<u64>,
    /// Tareas del proyecto. `#[serde(skip)]`: no se incluye en el doc de
    /// metadatos del proyecto; al deserializar queda vacío y `list_projects`
    /// lo ensambla desde el store `tasks`.
    #[serde(skip)]
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
