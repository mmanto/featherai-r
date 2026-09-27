//! Servicios de autenticación y ABM de usuarios sobre GuardianDB.
//!
//! Usuarios persistidos en `users/{username}` como [`StoredUser`] con hash
//! PHC argon2 (`Argon2::default()`). El hashing/verificación son
//! CPU-bound y corren vía `Db::rt.spawn_blocking`.
//!
//! El ABM ([`list_users`], [`create_user`], [`update_user`], [`delete_user`])
//! es la única puerta de escritura del store `users`: normaliza el username
//! (trim + lowercase, que además es la clave del doc), valida el rol y los
//! negocios (un usuario pertenece a **al menos uno**) y protege al último
//! `super_admin`.
//!
//! Sesión NO persistente: `login` solo valida credenciales; la sesión vive
//! en memoria como hoy.

use crate::models::{DEMO_BUSINESSES, ROLES, StoredUser, User};
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

/// Clave del doc de un usuario: `users/{username}` con trim + lowercase.
fn user_key(username: &str) -> String {
    username.trim().to_lowercase()
}

/// Trim de un opcional vacío → `None`.
fn normalize_opt(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Nombres de negocio sin vacíos, sin el centinela del selector y sin
/// duplicados (orden de aparición).
fn normalize_businesses(names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in names {
        let name = name.trim();
        if !name.is_empty() && name != crate::models::NEW_BUSINESS && !out.iter().any(|o| o == name)
        {
            out.push(name.to_string());
        }
    }
    out
}

/// Decodifica un doc de `users` (error de JSON con el nombre del doc).
fn decode_user(bytes: &[u8], key: &str) -> Result<StoredUser, AppError> {
    serde_json::from_slice(bytes).map_err(|e| AppError::Serde(format!("users/{key}: {e}")))
}

/// Como [`decode_user`], pero para listados y guardas: un doc ilegible (JSON de
/// otro formato escrito por fuera de la app, p. ej. desde el panel de sentinel)
/// no puede autenticarse ni ser `super_admin`, así que se omite con un aviso en
/// lugar de romper el listado completo — y con él el arranque, que lista los
/// usuarios.
fn decode_user_lenient(bytes: &[u8], key: &str) -> Option<StoredUser> {
    match decode_user(bytes, key) {
        Ok(stored) => Some(stored),
        Err(e) => {
            eprintln!("[featherai] doc users/{key} ilegible, omitido: {e}");
            None
        }
    }
}

/// Hashea una contraseña en el hilo bloqueante del runtime de la app.
async fn hash_async(db: &Db, password: String) -> Result<String, AppError> {
    let rt = db.rt.clone();
    rt.spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| AppError::Store(format!("hashing en runtime: {e}")))?
}

/// Serializa y escribe el doc `users/{username}`.
async fn put_user(db: &Db, stored: &StoredUser) -> Result<(), AppError> {
    let bytes = serde_json::to_vec(stored)
        .map_err(|e| AppError::Serde(format!("users/{}: {e}", stored.username)))?;
    db.users.put(&stored.username, bytes).await?;
    Ok(())
}

/// Valida las reglas de negocio comunes al alta y la modificación.
///
/// Devuelve los negocios normalizados.
fn validate(user: &User) -> Result<Vec<String>, AppError> {
    if !ROLES.contains(&user.role.as_str()) {
        return Err(AppError::Invalid(format!("rol inválido: {}", user.role)));
    }
    let businesses = normalize_businesses(&user.businesses);
    if businesses.is_empty() {
        return Err(AppError::Invalid(
            "el usuario debe pertenecer a al menos un negocio".to_string(),
        ));
    }
    Ok(businesses)
}

/// `true` si hay algún `super_admin` además de `excepto`.
async fn hay_otros_super_admin(db: &Db, excepto: &str) -> bool {
    for (key, bytes) in db.users.all() {
        if key == excepto {
            continue;
        }
        if decode_user_lenient(&bytes, &key).is_some_and(|stored| stored.role == "super_admin") {
            return true;
        }
    }
    false
}

/// Crea el usuario por defecto (`admin`/`admin`, rol `super_admin`, los
/// negocios demo) si el store de usuarios está vacío. Idempotente.
pub async fn ensure_admin(db: &Db) -> Result<(), AppError> {
    if !db.users.all().is_empty() {
        return Ok(());
    }
    let stored = StoredUser {
        username: "admin".to_string(),
        role: "super_admin".to_string(),
        nombre: Some("Admin".to_string()),
        apellido: None,
        email: None,
        avatar_url: None,
        businesses: DEMO_BUSINESSES.iter().map(|b| (*b).to_string()).collect(),
        password_hash: hash_async(db, "admin".to_string()).await?,
    };
    put_user(db, &stored).await
}

/// Valida credenciales contra el store. Devuelve `Ok(Some(user))` solo con
/// credenciales correctas; usuario inexistente o contraseña mala → `Ok(None)`
/// (errores de store/JSON sí son `Err`).
pub async fn login(db: &Db, username: &str, password: &str) -> Result<Option<User>, AppError> {
    let key = user_key(username);
    let Some(bytes) = db.users.get(&key).await? else {
        return Ok(None);
    };
    let stored = decode_user(&bytes, &key)?;
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

/// Lista los usuarios (sin hash de contraseña), ordenados por `username`. Los
/// docs con un JSON ajeno al formato de la app se omiten (ver
/// [`decode_user_lenient`]).
pub async fn list_users(db: &Db) -> Result<Vec<User>, AppError> {
    let mut users = Vec::new();
    for (key, bytes) in db.users.all() {
        if let Some(stored) = decode_user_lenient(&bytes, &key) {
            users.push(stored.into_user());
        }
    }
    users.sort_by(|a, b| a.username.cmp(&b.username));
    Ok(users)
}

/// Alta de usuario: normaliza el username (trim + lowercase), valida rol y
/// negocios y hashea la contraseña. Usuario existente o entrada inválida →
/// [`AppError::Invalid`].
pub async fn create_user(db: &Db, user: &User, password: &str) -> Result<User, AppError> {
    let key = user_key(&user.username);
    if key.len() < 3 {
        return Err(AppError::Invalid(
            "el usuario debe tener al menos 3 caracteres".to_string(),
        ));
    }
    if password.trim().is_empty() {
        return Err(AppError::Invalid("la contraseña es obligatoria".to_string()));
    }
    let businesses = validate(user)?;
    if db.users.get(&key).await?.is_some() {
        return Err(AppError::Invalid(format!("ya existe el usuario {key}")));
    }
    let stored = StoredUser {
        username: key,
        role: user.role.clone(),
        nombre: normalize_opt(&user.nombre),
        apellido: normalize_opt(&user.apellido),
        email: normalize_opt(&user.email),
        avatar_url: normalize_opt(&user.avatar_url),
        businesses,
        password_hash: hash_async(db, password.to_string()).await?,
    };
    put_user(db, &stored).await?;
    Ok(stored.into_user())
}

/// Modificación de usuario. `username` es la clave del doc (el username es
/// inmutable: se ignora `user.username`). `new_password` `None` o vacío
/// conserva el hash actual. Doc inexistente → [`AppError::NotFound`].
pub async fn update_user(
    db: &Db,
    username: &str,
    user: &User,
    new_password: Option<&str>,
) -> Result<User, AppError> {
    let key = user_key(username);
    let Some(bytes) = db.users.get(&key).await? else {
        return Err(AppError::NotFound(format!("users/{key}")));
    };
    let mut stored = decode_user(&bytes, &key)?;
    let businesses = validate(user)?;
    if stored.role == "super_admin"
        && user.role != "super_admin"
        && !hay_otros_super_admin(db, &key).await
    {
        return Err(AppError::Invalid(
            "no se puede quitar el rol super_admin al último super_admin".to_string(),
        ));
    }
    stored.role = user.role.clone();
    stored.nombre = normalize_opt(&user.nombre);
    stored.apellido = normalize_opt(&user.apellido);
    stored.email = normalize_opt(&user.email);
    stored.avatar_url = normalize_opt(&user.avatar_url);
    stored.businesses = businesses;
    if let Some(password) = new_password.filter(|p| !p.trim().is_empty()) {
        stored.password_hash = hash_async(db, password.to_string()).await?;
    }
    put_user(db, &stored).await?;
    Ok(stored.into_user())
}

/// Baja física del usuario (borra el doc `users/{username}`). Se niega a
/// borrar al último `super_admin`. Doc inexistente → [`AppError::NotFound`].
pub async fn delete_user(db: &Db, username: &str) -> Result<(), AppError> {
    let key = user_key(username);
    let Some(bytes) = db.users.get(&key).await? else {
        return Err(AppError::NotFound(format!("users/{key}")));
    };
    let stored = decode_user(&bytes, &key)?;
    if stored.role == "super_admin" && !hay_otros_super_admin(db, &key).await {
        return Err(AppError::Invalid(
            "no se puede eliminar al último super_admin".to_string(),
        ));
    }
    db.users.delete(&key).await?;
    Ok(())
}
