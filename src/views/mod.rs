//! The views module contains the components for all Layouts and Routes for our app. Each layout and route in our [`Route`]
//! enum will render one of these components.
//!
//! Las vistas del panel se autoconfienen: cada una envuelve su contenido en
//! [`ProtectedRoute`](crate::components::protected_route::ProtectedRoute) y
//! [`AppLayout`](crate::components::layout::AppLayout). Por eso el enum de rutas
//! no declara layouts de panel (equivalente a como cada página de React monta
//! su propio `<ProtectedRoute><AppLayout>`).

mod admin;
mod login;
mod projects;
mod settings;

pub use admin::AdminPanel;
pub use login::Login;
pub use projects::Projects;
pub use settings::Settings;
