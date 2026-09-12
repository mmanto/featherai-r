//! Capa de servicios — funciones async tipadas sobre el modelo de dominio
//! (`crate::models`) que son la única fuente de verdad para leer/escribir
//! persistencia. Las vistas no tocan GuardianDB directamente: las acciones
//! de `state.rs` disparan estas funciones y reconcilian el estado local con
//! la entidad canónica devuelta; [`project::reload_all`] re-sincroniza el
//! listado completo (lo usa `ProjectProvider` al montar).
//!
//! Solo nativo (declarado con `cfg(not(target_arch = "wasm32"))` en main.rs):
//! dependen de `crate::persistence` y de la base global [`db()`].

pub mod auth;
pub mod project;

#[cfg(test)]
mod tests;
