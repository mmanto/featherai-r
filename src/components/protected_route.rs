//! Guard de rutas privadas — equivalente de `components/auth/ProtectedRoute.tsx`.
//!
//! - Sin autenticación ni usuario de sesión → redirige a `/login` (reemplazo,
//!   como `Navigate replace`).
//! - Con `roles` y rol no autorizado → redirige a la vista que ese rol sí puede
//!   abrir ([`home_for_role`]). Mandarlo a `/login` lo dejaba en un loop sin
//!   salida (el login empujaba a `/admin/projects`, que volvía a rebotarlo y no
//!   hay barra de direcciones con la que escapar).
//!
//! Nota: el `isLoading` de React no existe en el port (la verificación de auth
//! es síncrona), por lo que no hay pantalla de "Verificando autenticación...".

use crate::components::layout::use_auth;
use dioxus::prelude::*;

/// Roles permitidos por defecto en las rutas de administración general.
pub const SUPER_ADMIN_ROLES: &[&str] = &["super_admin"];

/// Roles con acceso al módulo de proyectos. Lo tienen los tres, con alcance
/// distinto: `super_admin` ve todos los proyectos; `admin` y `operativo`, sólo
/// los de los negocios a los que pertenecen (ver
/// [`crate::components::layout::use_business_scope`]).
pub const PROJECT_ROLES: &[&str] = &crate::models::ROLES;

/// Primera vista que puede abrir `role` — destino del login y de los rebotes
/// de [`ProtectedRoute`]. `/settings` no tiene guard de rol (la ven todos los
/// usuarios autenticados).
pub fn home_for_role(role: &str) -> &'static str {
    if PROJECT_ROLES.contains(&role) {
        "/admin/projects"
    } else {
        "/settings"
    }
}

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

    if let Some(roles) = roles {
        match auth.read().user.as_ref().map(|u| u.role.clone()) {
            // Rol autorizado: se renderiza.
            Some(role) if roles.iter().any(|r| *r == role) => {}
            // Autenticado pero sin permiso → a la vista que sí puede abrir.
            Some(role) => {
                navigator.replace(home_for_role(&role));
                return VNode::empty();
            }
            // Sesión sin usuario (no debería pasar): de vuelta al login.
            None => {
                navigator.replace("/login");
                return VNode::empty();
            }
        }
    }

    rsx! { {children} }
}
