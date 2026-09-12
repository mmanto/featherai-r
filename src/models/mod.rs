//! Modelo de dominio compartido entre vistas, servicios y persistencia.
//!
//! - `project`: enums + `Project`/`Task` (serializables, docs GuardianDB).
//! - `user`: `User` (vista) y `StoredUser` (persistido con hash argon2).

pub mod project;
pub mod user;

pub use project::{Priority, Project, ProjectStatus, Task, TaskStatus};
pub use user::User;
#[cfg(not(target_arch = "wasm32"))]
pub use user::StoredUser;
