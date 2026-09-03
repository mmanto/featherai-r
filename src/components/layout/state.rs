//! Estado compartido y proveedores de contexto — equivalente Dioxus de los
//! hooks y contextos de feathrai-frontend:
//!
//! - `useAuth` + `context/AuthContext.tsx`
//! - estado expandido/collapsed del `Sidebar` de `Layout.tsx`
//! - secciones del sidebar (equivalente de las props `sections` de `Sidebar.tsx`)

use dioxus::prelude::*;

/// Usuario autenticado — equivalente de `User` de la app de referencia.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct User {
    pub username: String,
    pub email: Option<String>,
    pub nombre: Option<String>,
    pub apellido: Option<String>,
    pub avatar_url: Option<String>,
    /// Rol (super_admin/admin/operativo).
    #[allow(dead_code)]
    pub role: String,
}

impl User {
    /// `"nombre apellido"` o `username` — equivalente de `full_name` en
    /// `AuthContext` de feathrai-frontend.
    pub fn full_name(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if let Some(n) = self.nombre.as_deref().filter(|s| !s.is_empty()) {
            parts.push(n);
        }
        if let Some(a) = self.apellido.as_deref().filter(|s| !s.is_empty()) {
            parts.push(a);
        }
        if parts.is_empty() {
            self.username.clone()
        } else {
            parts.join(" ")
        }
    }
}

/// Estado de autenticación — equivalente de `AuthContextType`.
///
/// La API de login (`login`/`verifyToken`) queda fuera del port; los
/// componentes solo consumen `user`/`is_authenticated` y cierran sesión con
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

/// Sale de la aplicación cerrando la ventana nativa.
///
/// Sin barra de título no hay botón de cierre nativo, así que el cierre se
/// dispara desde la UI. Solo tiene efecto en desktop; en web/server es no-op.
pub fn exit_app() {
    #[cfg(feature = "desktop")]
    dioxus::desktop::window().close();
}

/// Ícono de navegación — clase de Bootstrap Icons (mismo contrato que el
/// campo `icon` de los ítems del `Sidebar` de React, ej. `"bi bi-building"`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavIcon {
    Folder,
    Gear,
}

impl NavIcon {
    /// Clase Bootstrap Icons del ícono.
    pub fn class(self) -> &'static str {
        match self {
            Self::Folder => "bi bi-folder",
            Self::Gear => "bi bi-gear",
        }
    }
}

/// Enlace de navegación — equivalente de `SidebarItem` en React.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NavLink {
    pub to: &'static str,
    pub label: &'static str,
    pub icon: NavIcon,
}

/// Elementos del panel de administración.
pub const NAV_LINKS: [NavLink; 2] = [
    NavLink {
        to: "/admin/projects",
        label: "Proyectos",
        icon: NavIcon::Folder,
    },
    NavLink {
        to: "/settings",
        label: "Ajustes",
        icon: NavIcon::Gear,
    },
];

/// Sección del sidebar — equivalente de `SidebarSection` en React.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidebarSection {
    pub title: &'static str,
    pub items: &'static [NavLink],
}

/// Secciones del sidebar — equivalente de `sidebarSections` en `Layout.tsx`.
pub const ADMIN_SECTIONS: [NavLink; 1] = [NAV_LINKS[0]];

pub const ACCOUNT_SECTIONS: [NavLink; 1] = [NAV_LINKS[1]];

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

/// Proveedor de autenticación — equivalente de `AuthProvider` (sesión vacía).
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

/// Acceso al colapso de la sidebar — equivalente de `useSidebar()`.
pub fn use_sidebar() -> Signal<bool> {
    use_context::<Signal<bool>>()
}
