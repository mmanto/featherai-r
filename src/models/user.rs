//! Usuarios — `User` (movido desde `src/components/layout/state.rs`) y
//! `StoredUser`, la forma persistida en el store `users` de GuardianDB con
//! el hash PHC argon2 de la contraseña. `StoredUser` es solo nativo (lo
//! usan los servicios sobre GuardianDB).

// StoredUser (el único tipo que serializa) vive solo en el camino nativo.
#[cfg(not(target_arch = "wasm32"))]
use serde::{Deserialize, Serialize};

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

/// Usuario persistido — doc `users/{username}` en GuardianDB. El hash es
/// PHC argon2 (`argon2` crate, `Argon2::default()`); nunca se expone a las
/// vistas. Solo nativo: lo usan los servicios de auth sobre GuardianDB.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct StoredUser {
    pub username: String,
    pub role: String,
    pub nombre: Option<String>,
    pub apellido: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub password_hash: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl StoredUser {
    /// Convierte al tipo de vista descartando el hash de contraseña.
    pub fn into_user(self) -> User {
        User {
            username: self.username,
            email: self.email,
            nombre: self.nombre,
            apellido: self.apellido,
            avatar_url: self.avatar_url,
            role: self.role,
        }
    }
}
