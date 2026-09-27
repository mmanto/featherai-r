//! Página de Proyectos — port de `ProjectManagement.tsx` de feathrai-frontend
//! y sus componentes relacionados (ProjectsGrid, ProjectDetail,
//! ProjectTasksList, KanbanBoard, GanttChart, TaskDetail). El diálogo de
//! confirmación de borrados vive en `crate::components::confirm_dialog`
//! (reemplaza al `confirm()` de JS, que no muestra diálogo en el webview de
//! desktop).

mod gantt_chart;
mod kanban_board;
mod project_detail;
mod project_management;
mod project_tasks_list;
mod projects_grid;
mod state;
mod task_detail;

pub use project_management::Projects;
