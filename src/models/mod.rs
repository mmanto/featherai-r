//! Modelo de dominio compartido entre vistas, servicios y persistencia.
//!
//! - `business`: negocios por nombre (sin entidad) — opciones del selector.
//! - `project`: enums + `Project`/`Task` (serializables, docs GuardianDB).
//! - `user`: `User` (vista) y `StoredUser` (persistido con hash argon2).

pub mod business;
pub mod project;
pub mod user;

pub use business::{
    DEMO_BUSINESSES, NEW_BUSINESS, business_options, project_business_options, resolve_business,
};
pub use project::{Priority, Project, ProjectStatus, Task, TaskStatus};
pub use user::{ROLES, User, role_label};
#[cfg(not(target_arch = "wasm32"))]
pub use user::StoredUser;
