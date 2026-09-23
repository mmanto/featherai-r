//! Componentes de layout — port a Dioxus del shell de feathrai-frontend:
//!
//! - `Layout.tsx`   → [`app_layout::AppLayout`]
//! - `Sidebar.tsx`  → [`sidebar::Sidebar`]
//! - `UserMenu.tsx` → [`user_menu::UserMenu`] (anclado en [`action_bar::ActionBar`])
//! - `ActionBar` (de xmusic) → [`action_bar::ActionBar`]
//! - Navegación móvil (patrón propio, sin equivalente en React) →
//!   [`mobile_nav::MobileNav`]: tab bar inferior + hoja de cuenta, reemplaza al
//!   sidebar y a la ActionBar en viewports ≤ 768px.
//!
//! Dependencias:
//! - Estado/auth/secciones → [`state`].
//! - Tema claro/oscuro → [`crate::components::theme`].

mod action_bar;
mod app_layout;
mod mobile_nav;
mod sidebar;
mod state;
mod user_menu;

pub use action_bar::ActionBar;
pub use app_layout::AppLayout;
pub use mobile_nav::MobileNav;
pub use state::{use_auth, AuthProvider, SidebarProvider};
