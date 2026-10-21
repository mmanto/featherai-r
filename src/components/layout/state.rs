//! Estado compartido y proveedores de contexto — equivalente Dioxus de los
//! hooks y contextos de feathrai-frontend:
//!
//! - `useAuth` + `context/AuthContext.tsx`
//! - estado expandido/collapsed del `Sidebar` de `Layout.tsx`
//! - secciones del sidebar (equivalente de las props `sections` de `Sidebar.tsx`)

use crate::components::protected_route::{PROJECT_ROLES, SUPER_ADMIN_ROLES};
use dioxus::prelude::*;

// `User` compartido con la persistencia y la capa de servicios: vive en
// `crate::models` (el `StoredUser` persistido en GuardianDB convierte a este
// tipo de vista) y se re-exporta acá para no tocar a los consumidores.
pub use crate::models::User;

/// Estado de autenticación — equivalente de `AuthContextType`.
///
/// La sesión es en memoria (no se persiste): `Login` la completa validando
/// credenciales contra GuardianDB en nativo (o modo demo en web); los
/// componentes consumen `user`/`is_authenticated` y cierran sesión con
/// [`logout`].
#[derive(Clone, PartialEq, Debug, Default)]
pub struct AuthState {
    pub user: Option<User>,
    pub is_authenticated: bool,
}

/// Cierra la sesión — equivalente de `logout()` del `AuthContext`.
pub fn logout(mut auth: Signal<AuthState>) {
    *auth.write() = AuthState::default();
}

/// Sale de la aplicación terminando el proceso.
///
/// Sin barra de título no hay botón de cierre nativo, así que el cierre se
/// dispara desde la UI. El cierre ordenado (ocultar la ventana, flush de los
/// stores y backend Iroh, devolver el data dir al servicio del nodo) vive en
/// [`crate::shutdown_and_exit`]: acá sólo se lo invoca con la ventana visible.
/// Solo tiene efecto en desktop; en web/server es no-op.
pub fn exit_app() {
    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    crate::shutdown_and_exit(true);
}

/// Ícono de navegación — clase de Bootstrap Icons (mismo contrato que el
/// campo `icon` de los ítems del `Sidebar` de React, ej. `"bi bi-building"`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavIcon {
    Folder,
    Gear,
    People,
}

impl NavIcon {
    /// Clase Bootstrap Icons del ícono.
    pub fn class(self) -> &'static str {
        match self {
            Self::Folder => "bi bi-folder",
            Self::Gear => "bi bi-gear",
            Self::People => "bi bi-people",
        }
    }
}

/// Enlace de navegación — equivalente de `SidebarItem` en React.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavLink {
    pub to: &'static str,
    pub label: &'static str,
    pub icon: NavIcon,
    /// Roles que pueden ver y abrir el ítem (`None` = cualquier usuario
    /// autenticado).
    pub roles: Option<&'static [&'static str]>,
}

impl NavLink {
    /// `true` si `role` puede ver el ítem. Sidebar y navegación móvil filtran
    /// con esto: ofrecer una ruta que `ProtectedRoute` rebota deja al usuario en
    /// un ida y vuelta de redirecciones.
    pub fn visible_for(&self, role: &str) -> bool {
        match self.roles {
            None => true,
            Some(roles) => roles.iter().any(|r| *r == role),
        }
    }
}

/// Elementos del panel de administración.
pub const NAV_LINKS: [NavLink; 3] = [
    NavLink {
        to: "/admin/projects",
        label: "Proyectos",
        icon: NavIcon::Folder,
        roles: Some(PROJECT_ROLES),
    },
    NavLink {
        to: "/admin/users",
        label: "Usuarios",
        icon: NavIcon::People,
        roles: Some(SUPER_ADMIN_ROLES),
    },
    NavLink {
        to: "/settings",
        label: "Ajustes",
        icon: NavIcon::Gear,
        roles: None,
    },
];

/// Sección del sidebar — equivalente de `SidebarSection` en React.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidebarSection {
    pub title: &'static str,
    pub items: &'static [NavLink],
}

/// Secciones del sidebar — equivalente de `sidebarSections` en `Layout.tsx`.
pub const ADMIN_SECTIONS: [NavLink; 2] = [NAV_LINKS[0], NAV_LINKS[1]];

pub const ACCOUNT_SECTIONS: [NavLink; 1] = [NAV_LINKS[2]];

/// Secciones del sidebar — equivalente de `sidebarSections` en `Layout.tsx`.
///
/// Constantes separadas porque el `Index` sobre slices no es const-stable.
pub const SIDEBAR_SECTIONS: [SidebarSection; 2] = [
    SidebarSection {
        title: "Administración",
        items: &ADMIN_SECTIONS,
    },
    SidebarSection {
        title: "Cuenta",
        items: &ACCOUNT_SECTIONS,
    },
];

/// Ruta actual normalizada (sin query ni hash) — equivalente de
/// `useLocation().pathname` de React Router. Es reactiva: los componentes que
/// la leen se re-renderizan al navegar.
pub fn use_current_path() -> Memo<String> {
    use_memo(|| router().current::<crate::Route>().to_string())
}

/// Proveedor de autenticación — equivalente de `AuthProvider`. La sesión
/// arranca vacía; `Login` la completa validando en nativo contra los
/// usuarios persistidos en GuardianDB (siembra `admin`/`admin`).
#[component]
pub fn AuthProvider(children: Element) -> Element {
    let auth = use_signal(AuthState::default);
    use_context_provider(|| auth);
    rsx! { {children} }
}

/// Proveedor del colapso de la sidebar — equivalente del estado
/// `sidebarExpanded` de `Layout.tsx`.
#[component]
pub fn SidebarProvider(children: Element) -> Element {
    let collapsed = use_signal(|| false);
    use_context_provider(|| collapsed);
    rsx! { {children} }
}

/// Acceso al estado de autenticación — equivalente de `useAuth()`.
///
/// # Panics
/// Fuera de un [`AuthProvider`].
pub fn use_auth() -> Signal<AuthState> {
    use_context::<Signal<AuthState>>()
}

/// Alcance por negocio del usuario de la sesión: `None` = ve todo (rol de
/// administración general); `Some(negocios)` = sólo lo de esos negocios
/// (`admin`/`operativo`).
///
/// Reactivo: se recalcula si cambia la sesión (login/logout o edición del
/// propio usuario desde el ABM).
///
/// # Panics
/// Fuera de un [`AuthProvider`].
pub fn use_business_scope() -> Memo<Option<Vec<String>>> {
    let auth = use_auth();
    use_memo(move || {
        let guard = auth.read();
        match guard.user.as_ref() {
            Some(user) if !SUPER_ADMIN_ROLES.contains(&user.role.as_str()) => {
                Some(user.businesses.clone())
            }
            _ => None,
        }
    })
}

/// Acceso al colapso de la sidebar — equivalente de `useSidebar()`.
pub fn use_sidebar() -> Signal<bool> {
    use_context::<Signal<bool>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::protected_route::home_for_role;

    /// Navegación por rol: Proyectos lo ven los tres roles; el ABM de usuarios
    /// sólo administración general; Ajustes, todos.
    #[test]
    fn la_navegacion_depende_del_rol() {
        let item = |to: &str| *NAV_LINKS.iter().find(|l| l.to == to).expect("ítem");

        let proyectos = item("/admin/projects");
        assert!(proyectos.visible_for("super_admin"));
        assert!(proyectos.visible_for("admin"));
        assert!(proyectos.visible_for("operativo"));

        let usuarios = item("/admin/users");
        assert!(usuarios.visible_for("super_admin"));
        assert!(!usuarios.visible_for("admin"));
        assert!(!usuarios.visible_for("operativo"));

        let ajustes = item("/settings");
        for role in crate::models::ROLES {
            assert!(ajustes.visible_for(role), "{role}");
        }
    }

    /// Los tres roles entran al panel por proyectos (cada uno con su alcance);
    /// un rol desconocido cae en Ajustes.
    #[test]
    fn home_por_rol() {
        assert_eq!(home_for_role("super_admin"), "/admin/projects");
        assert_eq!(home_for_role("admin"), "/admin/projects");
        assert_eq!(home_for_role("operativo"), "/admin/projects");
        assert_eq!(home_for_role(""), "/settings");
    }
}
