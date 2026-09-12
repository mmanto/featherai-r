//! Página de Proyectos — port de `ProjectManagement.tsx` de feathrai-frontend
//! y sus componentes relacionados (ProjectsGrid, ProjectDetail,
//! ProjectTasksList, KanbanBoard, GanttChart, TaskDetail). ConfirmDialog es
//! el diálogo compartido de confirmación de borrados (reemplaza al
//! `confirm()` de JS, que no muestra diálogo en el webview de desktop).

mod confirm_dialog;
mod gantt_chart;
mod kanban_board;
mod project_detail;
mod project_management;
mod project_tasks_list;
mod projects_grid;
mod state;
mod task_detail;

pub use project_management::Projects;
