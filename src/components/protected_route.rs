//! Guard de rutas privadas — equivalente de `components/auth/ProtectedRoute.tsx`.
//!
//! - Sin autenticación → redirige a `/login` (reemplazo, como `Navigate replace`).
//! - Con `roles` y rol no autorizado → redirige también a `/login`
//!   (esta app es de uso exclusivo de administración general).
//!
//! Nota: el `isLoading` de React no existe en el port (la verificación de auth
//! es síncrona), por lo que no hay pantalla de "Verificando autenticación...".

use crate::components::layout::use_auth;
use dioxus::prelude::*;

/// Roles permitidos por defecto en las rutas de administración general.
pub const SUPER_ADMIN_ROLES: &[&str] = &["super_admin"];

/// Envuelve contenido protegido — equivalente de `<ProtectedRoute>`.
///
/// ```ignore
/// ProtectedRoute {
///     roles: Some(SUPER_ADMIN_ROLES),
///     AppLayout { "contenido" }
/// }
/// ```
#[component]
pub fn ProtectedRoute(children: Element, roles: Option<&'static [&'static str]>) -> Element {
    let auth = use_auth();
    let navigator = use_navigator();

    // No autenticado → /login
    if !auth.read().is_authenticated {
        navigator.replace("/login");
        return VNode::empty();
    }

    // Rol no autorizado → /login
    if let Some(roles) = roles {
        let user = auth.read().user.clone();
        let authorized = user
            .as_ref()
            .map(|u| roles.iter().any(|r| *r == u.role))
            .unwrap_or(false);
        if !authorized {
            navigator.replace("/login");
            return VNode::empty();
        }
    }

    rsx! { {children} }
}
