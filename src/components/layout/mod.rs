//! Componentes de layout — port a Dioxus del shell de feathrai-frontend:
//!
//! - `Layout.tsx`   → [`app_layout::AppLayout`]
//! - `Sidebar.tsx`  → [`sidebar::Sidebar`]
//! - `UserMenu.tsx` → [`user_menu::UserMenu`]
//!
//! Dependencias:
//! - Estado/auth/secciones → [`state`].
//! - Tema claro/oscuro → [`crate::components::theme`].

mod app_layout;
mod sidebar;
mod state;
mod user_menu;

pub use app_layout::AppLayout;
pub use state::{use_auth, AuthProvider, SidebarProvider, User};
