//! The components module contains all shared components for our app. Components are the building blocks of dioxus apps.
//! They can be used to define common UI elements like buttons, forms, and modals.

pub mod layout;
pub mod protected_route;
pub mod theme;
#[allow(dead_code)] // show_toast/ToastVariant se conservan por contrato del port de gestion.ar
pub mod toast;
