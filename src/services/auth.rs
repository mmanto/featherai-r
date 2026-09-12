//! Servicios de autenticación sobre GuardianDB.
//!
//! Usuarios persistidos en `users/{username}` como [`StoredUser`] con hash
//! PHC argon2 (`Argon2::default()`). El hashing/verificación son
//! CPU-bound y corren vía `Db::rt.spawn_blocking`.
//!
//! Sesión NO persistente: `login` solo valida credenciales; la sesión vive
//! en memoria como hoy.

use crate::models::{StoredUser, User};
use crate::persistence::{AppError, Db};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Store(format!("argon2 hash: {e}")))
}

fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed =
        PasswordHash::new(hash).map_err(|e| AppError::Store(format!("argon2 parse: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Crea el usuario por defecto (`admin`/`admin`, rol `super_admin`) si el
/// store de usuarios está vacío. Idempotente.
pub async fn ensure_admin(db: &Db) -> Result<(), AppError> {
    if !db.users.all().is_empty() {
        return Ok(());
    }
    let username = "admin".to_string();
    let password = "admin".to_string();
    let rt = db.rt.clone();
    let hash = rt
        .spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| AppError::Store(format!("hashing en runtime: {e}")))??;
    let stored = StoredUser {
        username: username.clone(),
        role: "super_admin".to_string(),
        nombre: Some("Admin".to_string()),
        apellido: None,
        email: None,
        avatar_url: None,
        password_hash: hash,
    };
    let bytes = serde_json::to_vec(&stored)
        .map_err(|e| AppError::Serde(format!("users/{username}: {e}")))?;
    db.users.put(&username, bytes).await?;
    Ok(())
}

/// Valida credenciales contra el store. Devuelve `Ok(Some(user))` solo con
/// credenciales correctas; usuario inexistente o contraseña mala → `Ok(None)`
/// (errores de store/JSON sí son `Err`).
pub async fn login(db: &Db, username: &str, password: &str) -> Result<Option<User>, AppError> {
    let key = username.trim().to_lowercase();
    let Some(bytes) = db.users.get(&key).await? else {
        return Ok(None);
    };
    let stored: StoredUser =
        serde_json::from_slice(&bytes).map_err(|e| AppError::Serde(format!("users/{key}: {e}")))?;
    let hash = stored.password_hash.clone();
    let password = password.to_string();
    let rt = db.rt.clone();
    let ok = rt
        .spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|e| AppError::Store(format!("verificación en runtime: {e}")))??;
    if ok {
        Ok(Some(stored.into_user()))
    } else {
        Ok(None)
    }
}
