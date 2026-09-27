//! Estado de usuarios — espejo de `views/projects/state.rs` para el ABM.
//!
//! Nativo: [`UsersProvider`] arranca con el snapshot que `init_backend`
//! (main.rs) tomó de GuardianDB y lo refresca al montar y cada 3 s mientras la
//! vista está montada, para que los cambios de otro nodo (sync Iroh) aparezcan
//! sin reiniciar. Las acciones persisten vía `crate::services::auth` (que
//! valida) y devuelven el usuario canónico con el que se reconcilia el estado.
//!
//! Web/wasm no hay persistencia: el listado inicial es `demo::demo_users` y las
//! acciones mutan el estado en memoria.

pub use crate::models::User;
use dioxus::prelude::*;

/// Intervalo del refresco automático del listado en nativo (mismo valor que el
/// de proyectos y el de pares en Ajustes).
#[cfg(not(target_arch = "wasm32"))]
const USERS_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);

/// Estado global del módulo de usuarios.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct UsersState {
    pub users: Vec<User>,
}

/// Listado inicial: nativo, el snapshot persistido que tomó `init_backend`
/// (main.rs); web/wasm, la siembra demo en memoria.
#[cfg(not(target_arch = "wasm32"))]
fn initial_users() -> Vec<User> {
    crate::persistence::initial_users()
}

#[cfg(target_arch = "wasm32")]
fn initial_users() -> Vec<User> {
    crate::demo::demo_users()
}

/// Proveedor del estado de usuarios.
///
/// En nativo refresca el listado desde GuardianDB al montar y luego cada 3 s
/// mientras la vista está montada.
#[component]
pub fn UsersProvider(children: Element) -> Element {
    let state = use_signal(|| UsersState {
        users: initial_users(),
    });
    use_context_provider(|| state);

    #[cfg(not(target_arch = "wasm32"))]
    use_effect(move || {
        let state = state.clone();
        spawn(async move {
            let mut state = state;
            loop {
                match crate::services::auth::list_users(crate::persistence::db()).await {
                    Ok(users) => {
                        let mut s = state.write();
                        if s.users != users {
                            s.users = users;
                        }
                    }
                    Err(e) => eprintln!("[featherai] refrescando usuarios: {e}"),
                }
                tokio::time::sleep(USERS_REFRESH_INTERVAL).await;
            }
        });
    });

    rsx! { {children} }
}

/// Acceso al estado de usuarios.
///
/// # Panics
/// Fuera de un [`UsersProvider`].
pub fn use_users() -> Signal<UsersState> {
    use_context::<Signal<UsersState>>()
}

/// Alta de usuario: persiste (nativo) y agrega el usuario canónico al estado.
/// Devuelve el usuario guardado para que la vista pueda refrescar la sesión
/// propia si el alta fuera de ella misma.
pub async fn add_user(
    mut state: Signal<UsersState>,
    user: User,
    password: String,
) -> Result<User, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let created = crate::services::auth::create_user(crate::persistence::db(), &user, &password)
            .await
            .map_err(|e| e.to_string())?;
        let mut s = state.write();
        s.users.push(created.clone());
        s.users.sort_by(|a, b| a.username.cmp(&b.username));
        return Ok(created);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = password; // demo web: sin credenciales persistidas
        let mut s = state.write();
        s.users.push(user.clone());
        s.users.sort_by(|a, b| a.username.cmp(&b.username));
        return Ok(user);
    }
}

/// Modificación de usuario: persiste primero y recién con `Ok` reemplaza la
/// entrada del estado (sin actualización optimista: el modal debe seguir
/// mostrando lo que está guardado). `username` es la clave del doc y no
/// cambia. Devuelve el usuario canónico.
pub async fn update_user(
    mut state: Signal<UsersState>,
    username: &str,
    user: User,
    new_password: Option<String>,
) -> Result<User, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let updated = crate::services::auth::update_user(
            crate::persistence::db(),
            username,
            &user,
            new_password.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;
        let mut s = state.write();
        if let Some(slot) = s.users.iter_mut().find(|u| u.username == username) {
            *slot = updated.clone();
        }
        return Ok(updated);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = new_password; // demo web: sin credenciales persistidas
        let mut s = state.write();
        if let Some(slot) = s.users.iter_mut().find(|u| u.username == username) {
            *slot = user.clone();
        }
        return Ok(user);
    }
}

/// Baja física del usuario: borra en la base y después lo quita del estado.
pub async fn delete_user(mut state: Signal<UsersState>, username: &str) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    crate::services::auth::delete_user(crate::persistence::db(), username)
        .await
        .map_err(|e| e.to_string())?;
    state.write().users.retain(|u| u.username != username);
    Ok(())
}
