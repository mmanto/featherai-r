//! Servicios de proyectos/tareas sobre GuardianDB.
//!
//! Codificación de docs (JSON, serde):
//! - `projects/{id}` → `Project` (sin `tasks`; `created_at` para el orden).
//! - `tasks/{task_id}` → [`TaskDoc`] (`project_id` + `position` para
//!   preservar el orden de la UI dentro del proyecto).
//!
//! Convenciones:
//! - Los ids de proyecto/tarea los asigna este servicio (uuid v4); la UI
//!   puede mandar ids provisionales (los modales usan `next_id`) y se
//!   reemplazan acá al crear.
//! - El `delete` del KV GuardianDB devuelve error si la key no existe: todo
//!   borrado se antecede con una lectura local de existencia.
//! - Las ops sobre un mismo proyecto se serializan con
//!   `Db::lock_project` (evita carreras position/meta).

use crate::models::{Project, Task};
use crate::persistence::{AppError, Db};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Doc de tarea en el store `tasks`.
#[derive(Serialize, Deserialize)]
struct TaskDoc {
    project_id: String,
    /// Orden dentro del proyecto (0-based); lo conserva `replace_task`.
    position: u64,
    task: Task,
}

/// ms epoch actual (solo nativo — este módulo no compila en wasm).
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn encode<T: Serialize>(value: &T, what: &str) -> Result<Vec<u8>, AppError> {
    serde_json::to_vec(value).map_err(|e| AppError::Serde(format!("{what}: {e}")))
}

fn decode<T: DeserializeOwned>(bytes: &[u8], what: &str) -> Result<T, AppError> {
    serde_json::from_slice(bytes).map_err(|e| AppError::Serde(format!("{what}: {e}")))
}

/// Lee el doc de metadatos de un proyecto.
async fn get_project(db: &Db, id: &str) -> Result<Option<Project>, AppError> {
    let Some(bytes) = db.projects.get(id).await? else {
        return Ok(None);
    };
    decode(&bytes, &format!("projects/{id}")).map(Some)
}

/// Lista todos los proyectos con sus tareas ensambladas.
///
/// Orden de proyectos: `created_at` ascendente (desempate por id). Orden de
/// tareas dentro de cada proyecto: `position` (desempate por id).
pub async fn list_projects(db: &Db) -> Result<Vec<Project>, AppError> {
    // Scan único de tareas: agrupar TaskDoc por project_id.
    let mut by_project: std::collections::HashMap<String, Vec<(u64, Task)>> =
        std::collections::HashMap::new();
    for (task_id, bytes) in db.tasks.all() {
        let doc: TaskDoc = decode(&bytes, &format!("tasks/{task_id}"))?;
        by_project
            .entry(doc.project_id)
            .or_default()
            .push((doc.position, doc.task));
    }
    for tasks in by_project.values_mut() {
        tasks.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
    }

    let mut projects = Vec::new();
    for (project_id, bytes) in db.projects.all() {
        let mut project: Project = decode(&bytes, &format!("projects/{project_id}"))?;
        project.tasks = by_project
            .remove(&project_id)
            .map(|t| t.into_iter().map(|(_, task)| task).collect())
            .unwrap_or_default();
        projects.push(project);
    }
    projects.sort_by(|a, b| {
        (a.created_at.unwrap_or(0), &a.id).cmp(&(b.created_at.unwrap_or(0), &b.id))
    });
    Ok(projects)
}

/// Refresco completo del listado — re-sincroniza el estado de la UI con la
/// base (lo usa `ProjectProvider` al montar en nativo).
pub async fn reload_all(db: &Db) -> Result<Vec<Project>, AppError> {
    list_projects(db).await
}

/// Siembra los proyectos demo (`created_at` 1..n; tareas con `position` =
/// índice). Solo corre cuando el store de proyectos está vacío (ver `open`).
pub async fn seed_demo(db: &Db) -> Result<(), AppError> {
    for (i, mut project) in crate::demo::demo_projects().into_iter().enumerate() {
        project.created_at = Some(i as u64 + 1);
        let project_id = project.id.clone();
        let tasks = std::mem::take(&mut project.tasks);
        let meta = encode(&project, &format!("projects/{project_id}"))?;
        db.projects.put(&project_id, meta).await?;
        for (position, task) in tasks.into_iter().enumerate() {
            put_task(
                db,
                TaskDoc {
                    project_id: project_id.clone(),
                    position: position as u64,
                    task,
                },
            )
            .await?;
        }
    }
    Ok(())
}

/// Crea un proyecto: asigna id uuid v4 + `created_at`, arranca sin tareas.
pub async fn create_project(db: &Db, mut project: Project) -> Result<Project, AppError> {
    project.id = uuid::Uuid::new_v4().to_string();
    project.created_at = Some(now_ms());
    project.tasks = Vec::new();
    let meta = encode(&project, &format!("projects/{}", project.id))?;
    db.projects.put(&project.id, meta).await?;
    Ok(project)
}

/// Sobrescribe los metadatos de un proyecto existente (o lo crea si el doc
/// no existe — idempotente). Conserva el `created_at` original si la UI
/// manda `None` (los literales nuevos no lo conocen y el orden del listado
/// no debe cambiar con una edición).
pub async fn replace_project(db: &Db, project: &Project) -> Result<(), AppError> {
    let mut project = project.clone();
    if project.created_at.is_none() {
        if let Some(existing) = get_project(db, &project.id).await? {
            project.created_at = existing.created_at;
        }
    }
    project.tasks = Vec::new();
    let meta = encode(&project, &format!("projects/{}", project.id))?;
    db.projects.put(&project.id, meta).await?;
    Ok(())
}

/// Elimina un proyecto y todos sus TaskDoc. Proyecto inexistente → no-op.
pub async fn delete_project(db: &Db, id: &str) -> Result<(), AppError> {
    let _guard = db.lock_project(id).await;
    if db.projects.get(id).await?.is_none() {
        return Ok(());
    }
    for (task_id, bytes) in db.tasks.all() {
        let Ok(doc) = decode::<TaskDoc>(&bytes, &format!("tasks/{task_id}")) else {
            // Doc corrupto: no bloquea el borrado del proyecto.
            eprintln!("[featherai] TaskDoc corrupto omitido al borrar {id}: {task_id}");
            continue;
        };
        if doc.project_id == id {
            db.tasks.delete(&task_id).await?;
        }
    }
    db.projects.delete(id).await?;
    Ok(())
}

/// Agrega una tarea a un proyecto: asigna id uuid v4 y `position` = max+1.
/// Proyecto inexistente → [`AppError::NotFound`].
pub async fn add_task(db: &Db, project_id: &str, mut task: Task) -> Result<Task, AppError> {
    let _guard = db.lock_project(project_id).await;
    if get_project(db, project_id).await?.is_none() {
        return Err(AppError::NotFound(format!("proyecto {project_id}")));
    }
    let mut max_position: u64 = 0;
    for (task_id, bytes) in db.tasks.all() {
        let doc: TaskDoc = decode(&bytes, &format!("tasks/{task_id}"))?;
        if doc.project_id == project_id {
            max_position = max_position.max(doc.position);
        }
    }
    task.id = uuid::Uuid::new_v4().to_string();
    let task_id = task.id.clone();
    let doc = TaskDoc {
        project_id: project_id.to_string(),
        position: max_position + 1,
        task,
    };
    put_task(db, doc).await?;
    get_task(db, &task_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("tarea {task_id}")))
}

/// Sobrescribe una tarea existente conservando `position` y `project_id`
/// del doc actual. Tarea inexistente → [`AppError::NotFound`].
pub async fn replace_task(db: &Db, project_id: &str, task: &Task) -> Result<(), AppError> {
    let _guard = db.lock_project(project_id).await;
    let Some(bytes) = db.tasks.get(&task.id).await? else {
        return Err(AppError::NotFound(format!("tarea {}", task.id)));
    };
    let doc: TaskDoc = decode(&bytes, &format!("tasks/{}", task.id))?;
    if doc.project_id != project_id {
        return Err(AppError::NotFound(format!(
            "tarea {} en proyecto {project_id}",
            task.id
        )));
    }
    let doc = TaskDoc {
        project_id: doc.project_id,
        position: doc.position,
        task: task.clone(),
    };
    put_task(db, doc).await?;
    Ok(())
}

/// Elimina una tarea. Si el TaskDoc no existe o pertenece a otro proyecto →
/// no-op (borrado idempotente, mismo contrato que el state en memoria).
///
/// Los no-op se registran en stderr (terminal de `dx serve`/binario) porque
/// desde la UI parecen "no pasó nada".
pub async fn delete_task(db: &Db, project_id: &str, task_id: &str) -> Result<(), AppError> {
    let _guard = db.lock_project(project_id).await;
    let Some(bytes) = db.tasks.get(task_id).await? else {
        eprintln!("[featherai] delete_task: {task_id} no existe (no-op)");
        return Ok(());
    };
    let doc: TaskDoc = decode(&bytes, &format!("tasks/{task_id}"))?;
    if doc.project_id != project_id {
        eprintln!("[featherai] delete_task: {task_id} no pertenece a {project_id} (no-op)");
        return Ok(());
    }
    db.tasks.delete(task_id).await?;
    Ok(())
}

async fn put_task(db: &Db, doc: TaskDoc) -> Result<(), AppError> {
    let task_id = doc.task.id.clone();
    let bytes = encode(&doc, &format!("tasks/{task_id}"))?;
    db.tasks.put(&task_id, bytes).await?;
    Ok(())
}

async fn get_task(db: &Db, task_id: &str) -> Result<Option<Task>, AppError> {
    let Some(bytes) = db.tasks.get(task_id).await? else {
        return Ok(None);
    };
    let doc: TaskDoc = decode(&bytes, &format!("tasks/{task_id}"))?;
    Ok(Some(doc.task))
}
