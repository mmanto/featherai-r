# ABM de usuarios (cada usuario pertenece a uno o más negocios)

## Contexto

La app sólo permite administrar **proyectos/tareas** desde `/admin/projects`; los usuarios viven en el
store `users` de GuardianDB y únicamente se usan para el login (se siembra `admin`/`admin`). No hay
pantalla, servicio ni contrato de alta/baja/modificación de usuarios.

Se pide un **ABM de usuarios** (alta, baja y modificación) donde **un usuario pertenece a uno o más
negocios**. Decisiones ya fijadas con el usuario:

1. Los "negocios" siguen siendo **nombres** (no entidad propia): `Vec<String>` en el usuario, con la
   misma convención que `Project.business` (documentada en `src/models/project.rs:190-193`). Sin
   store nuevo, sin migración de docs.
2. **Baja física** del doc `users/{username}` (sin flag `disabled`); guardas: no se puede borrar al
   último `super_admin` ni (desde la UI) a uno mismo.
3. Roles asignables: **super_admin / admin / operativo** (los tres que ya etiqueta Ajustes).
4. La edición incluye **contraseña opcional** (vacío = conserva el hash actual).

Estado final: nueva ruta `/admin/users` con ítem en el sidebar/móvil, listado con búsqueda, alta y
edición por modal y borrado con confirmación; validación "al menos un negocio" en formulario y
servicio; todo persistido en el store `users` (y disponible en web/wasm con datos demo en memoria,
igual que proyectos).

## Enfoque

### 1. Modelo: `businesses` en el usuario + helper de negocios compartido

**`src/models/user.rs`**

- `User` (tipo de vista): agregar campo después de `role`:
  ```rust
  /// Negocios a los que pertenece (nombres; al menos uno).
  pub businesses: Vec<String>,
  ```
  Actualizar el doc de `role` a "Rol (`super_admin`/`admin`/`operativo`)" y **borrar
  `#[allow(dead_code)]`** de `role` (está obsoleto: se lee en `components/protected_route.rs:41`,
  `views/settings.rs` y ahora en el ABM).
- `StoredUser` (doc persistido): agregar
  ```rust
  /// Negocios a los que pertenece (nombres). `#[serde(default)]`: los docs
  /// viejos sin el campo deserializan a lista vacía.
  #[serde(default)]
  pub businesses: Vec<String>,
  ```
- `into_user`: mapear `businesses: self.businesses`.
- Nuevos ítems target-neutral (sin `cfg`), al final del archivo:
  ```rust
  /// Roles que puede asignar el ABM (mismos valores que persiste el seed).
  pub const ROLES: [&str; 3] = ["super_admin", "admin", "operativo"];

  /// `true` si `role` es uno de [`ROLES`].
  pub fn is_valid_role(role: &str) -> bool { ROLES.contains(&role) }

  /// Etiqueta del rol para la UI — misma tabla que hoy tiene inline `Settings`.
  pub fn role_label(role: &str) -> &'static str {
      match role {
          "super_admin" => "Administración general",
          "admin" => "Administrador",
          "operativo" => "Operativo",
          _ => "",
      }
  }
  ```

**`src/models/business.rs` (nuevo)**

No hay entidad `Business`: los negocios son nombres compartidos por convención entre
`Project.business` y `User.businesses`, y el selector de cada formulario ofrece los nombres demo más
los ya usados, con un centinela para escribir uno nuevo.

```rust
//! Negocios por nombre — sin entidad propia.
//!
//! Los negocios son nombres compartidos por convención entre `Project.business`
//! y `User.businesses` (ver el doc del campo en `models/project.rs`). Cada
//! formulario arma sus opciones con [`business_options`] (demo + los nombres que
//! ya usan sus registros) y crea uno nuevo escribiéndolo: el centinela
//! [`NEW_BUSINESS`] no es un nombre válido y se filtra.

/// Negocios demo que se ofrecen en los selectores (en React venían de
/// `businessService.getAll()`).
pub const DEMO_BUSINESSES: [&str; 3] = ["AgileTeam Corp", "NovaSoft", "TechCorp"];

/// Valor centinela del selector de negocio: muestra el campo para escribir uno
/// nuevo. No puede chocar con un nombre real (se filtra y la creación lo
/// rechaza).
pub const NEW_BUSINESS: &str = "__new__";

/// Opciones del selector: los demo + `used` (nombres ya usados por proyectos o
/// usuarios), sin vacíos ni duplicados y en orden de aparición.
pub fn business_options(used: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut options: Vec<String> = DEMO_BUSINESSES.iter().map(|b| (*b).to_string()).collect();
    for name in used {
        let name = name.trim();
        if !name.is_empty() && name != NEW_BUSINESS && !options.iter().any(|o| o == name) {
            options.push(name.to_string());
        }
    }
    options
}
```

**`src/models/mod.rs`**: agregar `pub mod business;` y re-exportar
`pub use business::{business_options, DEMO_BUSINESSES, NEW_BUSINESS};`, y
`pub use user::{is_valid_role, role_label, User, ROLES};` (junto al `User` ya existente).

**`src/models/project.rs`**: en el doc del campo `business` (líneas ~190-206) agregar que el nombre
es el mismo espacio que `User.businesses` (el helper compartido vive en `models::business`).

### 2. Servicios: ABM en `services::auth`

**`src/services/auth.rs`** — mismo módulo (ya es dueño del store `users` y del hashing argon2).
Actualizar el doc del módulo a "Servicios de autenticación y ABM de usuarios sobre GuardianDB".

- `ensure_admin`: agregar `businesses: DEMO_BUSINESSES.iter().map(|b| (*b).to_string()).collect()` al
  `StoredUser` sembrado (el admin arranca con los tres negocios demo, consistente con la regla
  "al menos un negocio"; los docs de instalaciones existentes no se tocan porque el seed sólo corre
  con el store vacío).
- Helpers privados:
  ```rust
  /// Trim de un opcional vacío → `None`.
  fn normalize_opt(value: &Option<String>) -> Option<String>

  /// Nombres de negocio sin vacíos, sin el centinela y sin duplicados.
  fn normalize_businesses(names: &[String]) -> Vec<String>

  /// `true` si hay otro `super_admin` además de `excepto`.
  async fn hay_otros_super_admin(db: &Db, excepto: &str) -> Result<bool, AppError>
  ```
- Firmas nuevas (todas `pub`):
  ```rust
  /// Lista los usuarios (sin hash) ordenados por `username`.
  pub async fn list_users(db: &Db) -> Result<Vec<User>, AppError>

  /// Alta: normaliza el username (trim + lowercase), valida y hashea la
  /// contraseña. Usuario existente → [`AppError::Invalid`].
  pub async fn create_user(db: &Db, user: &User, password: &str) -> Result<User, AppError>

  /// Modificación: `username` es la clave (inmutable; se ignora
  /// `user.username`). `new_password` `None`/vacío conserva el hash actual.
  /// Doc inexistente → [`AppError::NotFound`].
  pub async fn update_user(
      db: &Db,
      username: &str,
      user: &User,
      new_password: Option<&str>,
  ) -> Result<User, AppError>

  /// Baja física del doc. Doc inexistente → [`AppError::NotFound`].
  pub async fn delete_user(db: &Db, username: &str) -> Result<(), AppError>
  ```
- Validaciones/guardas exactas (mensajes literales; todas `AppError::Invalid` salvo lo indicado):
  | Caso | Mensaje |
  |---|---|
  | `username` normalizado con menos de 3 chars | `el usuario debe tener al menos 3 caracteres` |
  | `password` vacía en alta | `la contraseña es obligatoria` |
  | `is_valid_role(&user.role)` falso | `` rol inválido: {role} `` |
  | `normalize_businesses(&user.businesses)` vacío | `el usuario debe pertenecer a al menos un negocio` |
  | alta con doc existente | `` ya existe el usuario {username} `` |
  | baja/democión del último `super_admin` | `no se puede eliminar al último super_admin` / `no se puede quitar el rol super_admin al último super_admin` |
- Detalles de implementación: la clave siempre es `username.trim().to_lowercase()`; `create_user`
  verifica existencia con `db.users.get(&key)` antes de hashear; el hashing reusa
  `hash_password` + `db.rt.spawn_blocking` (igual que `ensure_admin`); `update_user` conserva
  `password_hash` salvo que `new_password` sea `Some` no vacío, y **no** permite renombrar
  (`updated.username = key`); `list_users` decodifica cada doc (`AppError::Serde` si el JSON es
  inválido, como `list_projects`) y ordena con `sort_by(|a, b| a.username.cmp(&b.username))`;
  `delete_user` pre-chequea existencia (el `delete` del KV falla si la key no existe).
- `login` queda igual (el doc pasa por `into_user`, que ya arrastra `businesses`).

### 3. Persistencia y arranque

**`src/persistence.rs`**

- Import: `use crate::models::{Project, User};`.
- Junto a `INITIAL_PROJECTS`:
  ```rust
  static INITIAL_USERS: OnceLock<Vec<User>> = OnceLock::new();

  /// Listado inicial de usuarios (el que se mostró al arrancar, tras la siembra
  /// del admin si correspondía) — para el estado de la UI.
  pub fn initial_users() -> Vec<User> {
      INITIAL_USERS.get().cloned().unwrap_or_default()
  }
  ```
- `pub fn set_global(db: Db, initial_users: Vec<User>, initial: Vec<Project>)` (agregar el parámetro
  `initial_users` y el `INITIAL_USERS.set(initial_users)`). Único llamador: `src/main.rs:68`.

**`src/main.rs`**

- `init_backend`: después del listado inicial de proyectos,
  ```rust
  let initial_users = rt
      .block_on(services::auth::list_users(&db))
      .expect("listado inicial de usuarios");
  persistence::set_global(db, initial_users, initial);
  ```
- `use views::{AdminPanel, Login, Projects, Settings, Users};`
- En el enum `Route`, después de `/admin/projects`:
  ```rust
  #[route("/admin/users")]
  Users {},
  ```

**`src/views/projects/state.rs`**: sin cambios (sigue leyendo `persistence::initial_projects()`).

### 4. Ruta, navegación y componente compartido

**`src/components/confirm_dialog.rs`**: mover el archivo desde `src/views/projects/confirm_dialog.rs`
(contenido idéntico; el doc agrega "lo comparten proyectos y usuarios"). Cutover sin alias:

- `src/views/projects/mod.rs`: borrar `mod confirm_dialog;` y actualizar el doc del módulo (la línea
  que dice que `ConfirmDialog` vive ahí).
- `src/components/mod.rs`: agregar `pub mod confirm_dialog;` (primera línea de módulos).
- Actualizar los 4 imports `use crate::views::projects::confirm_dialog::ConfirmDialog;` →
  `use crate::components::confirm_dialog::ConfirmDialog;` en:
  `views/projects/task_detail.rs:2`, `views/projects/project_tasks_list.rs:2`,
  `views/projects/kanban_board.rs:1`, `views/projects/projects_grid.rs:2`.

**`src/components/layout/state.rs`**

- `NavIcon`: nueva variante `People` con clase `"bi bi-people"` (agregar al enum y al `match`).
- `NAV_LINKS: [NavLink; 3]` = `[Proyectos → /admin/projects, Usuarios → /admin/users, Ajustes → /settings]`
  (la entrada nueva va **en medio**, con `label: "Usuarios"`, `icon: NavIcon::People`).
- `ADMIN_SECTIONS: [NavLink; 2] = [NAV_LINKS[0], NAV_LINKS[1]];`
- `ACCOUNT_SECTIONS: [NavLink; 1] = [NAV_LINKS[2]];`

(Sidebar y `MobileNav` recorren esas constantes: no hay que tocarlos.)

### 5. Vista de usuarios

**`src/demo.rs`**: nueva función target-neutral (nativo no la usa; sólo web/wasm):

```rust
/// Usuarios demo (web/wasm sin persistencia): el admin sembrado y un operativo.
pub fn demo_users() -> Vec<User>
```

Dos entradas: `admin` (`role: "super_admin"`, `nombre: Some("Admin")`,
`businesses: DEMO_BUSINESSES.to_vec()`) y `ana.ruiz` (`role: "operativo"`,
`nombre: Some("Ana")`, `apellido: Some("Ruiz")`, `email: Some("ana.ruiz@novasoft.example")`,
`businesses: vec!["NovaSoft".to_string()]`). Agregar `User` y `DEMO_BUSINESSES` al `use` del módulo.

**`src/views/users/mod.rs` (nuevo)** — mismo patrón que `views/projects/mod.rs`:

```rust
//! Página de Usuarios — ABM de usuarios del panel (`/admin/users`).

mod state;
mod user_form_modal;
mod users_management;

pub use users_management::Users;
```

**`src/views/users/state.rs` (nuevo)** — espejo de `views/projects/state.rs`:

```rust
pub use crate::models::User;

#[derive(Clone, PartialEq, Debug, Default)]
pub struct UsersState {
    pub users: Vec<User>,
}

#[cfg(not(target_arch = "wasm32"))]
const USERS_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);

#[cfg(not(target_arch = "wasm32"))]
fn initial_users() -> Vec<User> { crate::persistence::initial_users() }
#[cfg(target_arch = "wasm32")]
fn initial_users() -> Vec<User> { crate::demo::demo_users() }

#[component] pub fn UsersProvider(children: Element) -> Element
pub fn use_users() -> Signal<UsersState>

pub async fn add_user(state: Signal<UsersState>, user: User, password: String) -> Result<User, String>
pub async fn update_user(
    state: Signal<UsersState>,
    username: &str,
    user: User,
    new_password: Option<String>,
) -> Result<User, String>
pub async fn delete_user(state: Signal<UsersState>, username: &str) -> Result<(), String>
```

- `UsersProvider`: `use_signal(|| UsersState { users: initial_users() })` +
  `use_context_provider(|| state)`; en nativo, `use_effect` con un `spawn` que en loop llama
  `crate::services::auth::list_users(crate::persistence::db())`, reemplaza `s.users` si cambió y
  duerme `USERS_REFRESH_INTERVAL` (mismo patrón que `ProjectProvider`, incluido el `eprintln!` en
  `Err`).
- Acciones (bloques `#[cfg(not(target_arch = "wasm32"))]` / `#[cfg(target_arch = "wasm32")]` como en
  `login.rs`):
  - `add_user`: nativo llama `services::auth::create_user(db, &user, &password)`, inserta el usuario
    canónico en el estado y re-ordena por `username`; devuelve ese usuario. Web: inserta el `user`
    recibido (ignora `password`) y devuelve el mismo.
  - `update_user`: nativo persiste **primero** (`services::auth::update_user(db, username, &user,
    new_password.as_deref())`) y recién con `Ok` reemplaza la entrada del estado por el usuario
    canónico (sin actualización optimista: el modal no debe quedar con datos que no se guardaron).
    Web: reemplaza la entrada y devuelve `user`.
  - `delete_user`: nativo borra en la base y después hace `state.write().users.retain(|u| u.username != username)`;
    web: sólo estado.

**`src/views/users/user_form_modal.rs` (nuevo)**

```rust
#[component]
pub fn UserFormModal(
    /// `Some` = edición (username inmutable), `None` = alta.
    editing: Option<User>,
    /// `true` si el usuario editado es el de la sesión (deshabilita el rol).
    is_self: bool,
    /// Opciones del selector de negocios (ver `models::business_options`).
    options: Vec<String>,
    on_close: EventHandler<()>,
    on_submit: EventHandler<(User, Option<String>)>, // (usuario, contraseña)
) -> Element
```

Shell visual idéntico a `NewProjectModal` (`views/projects/project_management.rs:268-376`): overlay
`fixed inset-0 z-[1050] … bg-black/50`, tarjeta `max-w-2xl`, header con título
(`"Nuevo Usuario"` / `"Editar Usuario"`) y botón `bi bi-x-lg`, body `p-4`, footer
`flex justify-end gap-2 border-t …` con `Cancelar` (border) y `Guardar` (primario).

Signals internos (se reinician solos: el modal se monta al abrirse): `username`, `password`,
`nombre`, `apellido`, `email`, `avatar_url`, `role` (`String`, default `"operativo"`),
`businesses` (`Vec<String>`, inicializado de `editing`), `new_business` (`String`),
`form_error` (`Option<String>`).

Cuerpo, en este orden:
1. Bloque de error inline (mismo markup que el error de `views/login.rs:39-43`) si `form_error()`.
2. `Usuario *`: `input` con `value: username()`; **`disabled: editing.is_some()`** (la clave no se
   renombra) y `placeholder: "ana.ruiz"`.
3. `Contraseña *` en alta / `Nueva contraseña (opcional)` en edición: `input r#type: "password"`.
4. Grid 12 col: `Nombre`, `Apellido` (`col-span-12 md:col-span-6`).
5. Grid 12 col: `Email` (`col-span-12 md:col-span-6`), `URL de avatar` (`col-span-12 md:col-span-6`).
6. `Rol`: `select` con una `option` por cada valor de `crate::models::ROLES`
   (`value: role_value`, label `crate::models::role_label(role_value)`), `value: role()`,
   `disabled: is_self` (no te quitás el super_admin a vos mismo).
7. `Negocios *`: por cada nombre de `shown` (los `options` en su orden + los seleccionados que no
   estén, al final) una `label` con `input { r#type: "checkbox", checked: businesses().contains(&name),
   onchange: … }` que agrega/quita el nombre de `businesses` según `e.checked()`. Debajo, fila con
   `input` (`placeholder: "Nuevo negocio"`) + botón `Agregar` que hace
   `let name = new_business().trim().to_string();` y, si no es vacío ni `NEW_BUSINESS`, lo agrega a
   `businesses` (sin duplicar) y limpia el input. Nota visible: "Un usuario pertenece a uno o más
   negocios."

`Guardar` (`onclick`) valida en orden y setea `form_error` sin llamar a `on_submit` si algo falla:
username normalizado (`trim().to_lowercase()`) con `len() < 3` → `"El usuario debe tener al menos 3
caracteres"`; **alta** con contraseña vacía → `"La contraseña es obligatoria"`;
`businesses().is_empty()` → `"Seleccioná al menos un negocio"`. Si pasa, llama
`on_submit.call((user, password))` con
`User { username: editing.map(|u| u.username).unwrap_or(username_normalizado), role: role(), nombre,
apellido, email, avatar_url (trim → None si vacío), businesses: businesses() }` y
`password = Some(v) ` si el campo no está vacío, `None` si lo está.

**`src/views/users/users_management.rs` (nuevo)**

```rust
#[component]
pub fn Users() -> Element {
    rsx! {
        ProtectedRoute {
            roles: Some(SUPER_ADMIN_ROLES),
            AppLayout {
                UsersProvider {
                    UsersManagement {}
                }
            }
        }
    }
}
```

`UsersManagement` (privado): `state = use_users()`, `toasts = use_toast()`, `auth = use_auth()`,
signals `search: String`, `show_form: bool`, `editing: Option<User>`, `confirm_delete: Option<String>`.

- Título `Usuarios` + subtítulo `Usuarios del panel y los negocios a los que pertenecen`, mismo
  markup de encabezado que `views/settings.rs:224-232`.
- Filtro: `search` (lowercase `contains`) contra `username`, `nombre`, `apellido`, `email` y cada
  negocio; la tabla muestra la lista filtrada y, cuando hay búsqueda, `"{n} resultado(s)"`.
- Tabla (mismo markup/clases que `views/projects/projects_grid.rs:398-460`, sin ordenamiento por
  columna) con 5 columnas:
  1. `Usuario` (25%): avatar — `img { src: "{url}", class: "h-8 w-8 rounded-full object-cover" }` si
     `avatar_url` no está vacío; si no, `div` redondo 32px con `style: "background-color: var(--primary-color)"`
     y la inicial de `username` en mayúscula. Al lado, `username` en negrita y (si hay) `email` en
     `text-[var(--text-secondary)]` tamaño `0.8rem`.
  2. `Nombre` (20%): `user.full_name()`.
  3. `Rol` (15%): `crate::models::role_label(u.role.as_str())`.
  4. `Negocios` (25%): un badge por nombre
     (`span class="inline-flex items-center rounded-full border border-[var(--border-color)] bg-[var(--bg-secondary)] px-2 py-0.5 text-xs text-[var(--text-secondary)]"`);
     sin negocios → `span "—"` en `text-[var(--text-secondary)]`.
  5. Acciones (15%): botón `bi bi-pencil` (`title: "Editar"`) y, **sólo si el username no es el de la
     sesión**, botón `bi bi-trash` (`title: "Eliminar"`).
- Footer de controles: input de búsqueda (`placeholder: "Buscar usuarios..."`, ancho 260px) + botón
  primario `bi bi-plus-lg "Nuevo Usuario"` (`show_form = true`, `editing = None`).
- Estados vacíos: sin usuarios → `bi bi-people` + `"No hay usuarios aún"` + botón `"Crear el primer
  usuario"`; con búsqueda sin resultados → `i.bi.bi-search` + `No se encontraron usuarios que coincidan con "{search()}"`.
- `if show_form() { UserFormModal { editing: editing(), is_self: editing().as_ref().is_some_and(|u| Some(&u.username) == auth.read().user.as_ref().map(|a| &a.username)), options: crate::models::business_options(state.read().users.iter().flat_map(|u| u.businesses.iter().cloned())), on_close: …, on_submit: … } }`.
- `on_submit((user, password))`: `let es_alta = editing().is_none();` y `spawn(async move { … })`:
  - alta → `add_user(state, user, password.unwrap_or_default())`; edición →
    `update_user(state, &editing().unwrap().username, user, password)`. El username de la edición se
    captura **antes** del `await`.
  - `Ok(saved)`: si `auth.read().user` tiene el mismo username, `auth.write().user = Some(saved)`
    (así Ajustes y el menú de usuario no quedan con datos viejos); cerrar el modal
    (`show_form = false`, `editing = None`).
  - `Err(e)`: `show_toast(toasts, format!("No se pudo {} el usuario: {e}", if es_alta { "crear" } else { "guardar" }), ToastVariant::Error)`
    y **el modal queda abierto** con lo cargado.
- `if let Some(username) = confirm_delete() {` → `ConfirmDialog { title: "Eliminar usuario", message:
  format!("¿Eliminar al usuario \"{username}\"? Esta acción no se puede deshacer."), confirm_label: "Eliminar", on_confirm, on_cancel }`;
  `on_confirm` cierra el diálogo y hace `spawn` de `delete_user(state, &username)`, con toast de error
  si falla.

**`src/views/mod.rs`**: `mod users;` (orden alfabético: admin, login, projects, settings, users) y
`pub use users::Users;`.

### 6. Formulario de proyecto: una sola implementación del selector

**`src/views/projects/project_management.rs`**: borrar `BUSINESSES`, `NEW_BUSINESS` y
`business_options` locales (líneas ~12-33) e importar `crate::models::{business_options, NEW_BUSINESS}`.
En `NewProjectModal` (línea ~199) reemplazar
`let options = business_options(&state.read().projects);` por
```rust
let options = business_options(state.read().projects.iter().filter_map(|p| p.business.clone()));
```
El resto del modal (incluida la `option` centinela) no cambia: el comportamiento es idéntico al
actual.

### 7. Tests

**`src/services/tests.rs`** (nativo, GuardianDB real con preset `offline`; agregar `User` a los
imports de `crate::models`):

- Helper `fn sample_user(username: &str, role: &str, businesses: &[&str]) -> User`.
- `services_users_abm_crud`:
  - `create_user` con `username: "Ana.Ruiz"`, rol `operativo` y negocios
    `["NovaSoft", " NovaSoft ", "Acme"]` → devuelve username `ana.ruiz` y negocios
    `["NovaSoft", "Acme"]` (normalizado + dedup + trim).
  - `list_users` → `["admin", "ana.ruiz"]` (orden por username, con el admin sembrado).
  - `update_user` cambiando `nombre` a `Some("Ana")`, `businesses` a `["Acme"]` y
    `new_password: Some("nueva-clave")` → `login("ana.ruiz", "nueva-clave")` es `Some` y
    `login("ana.ruiz", "secreto")` es `None`.
  - `delete_user` → `login` vuelve a `None` y `list_users().len() == 1`.
- `services_users_validaciones_y_guardas`:
  - `create_user` sin negocios, con username de 2 chars, con rol `"root"` y con password vacía →
    los cuatro `Err(AppError::Invalid(_))`.
  - Alta válida de `pepe` → `Ok`; alta de `PEPE` → `Err(AppError::Invalid(_))` (duplicado).
  - `delete_user("admin")` → `Err(AppError::Invalid(_))` (último super_admin) y `update_user` de
    `admin` con `role: "admin"` → `Err(AppError::Invalid(_))`.
  - Alta de un segundo `super_admin`, y **entonces** `delete_user("admin")` → `Ok`.
- Extender `services_auth_admin_seed_and_login` (línea ~264) con
  `assert_eq!(user.businesses, vec!["AgileTeam Corp".to_string(), "NovaSoft".to_string(), "TechCorp".to_string()])`.

**`src/models/business.rs`**: `#[cfg(test)] mod tests` con un test de
`business_options(vec!["Beta".to_string(), " AgileTeam Corp ".to_string(), String::new(), "Beta".to_string()])`
→ `["AgileTeam Corp", "NovaSoft", "TechCorp", "Beta"]` (demo primero, sin duplicados ni vacíos).

## Archivos críticos

- `src/services/auth.rs` — ABM completo, validaciones y guardas; single source of truth del contrato.
- `src/models/user.rs` + `src/models/business.rs` — campos persistidos (`businesses` con
  `#[serde(default)]`) y opciones del selector compartidas.
- `src/views/users/{state.rs,users_management.rs,user_form_modal.rs}` — estado, listado y formulario.
- `src/persistence.rs` (`INITIAL_USERS`/`set_global`) + `src/main.rs` (seed, ruta y `Routes`) — wiring.
- `src/components/layout/state.rs` — ítem de navegación nuevo (afecta sidebar y móvil).

## Verificación

Todo desde `/home/mmanto/Investigando/dioxus/featherai-r`.

1. Compila: `cargo check`.
2. Tests de servicios (GuardianDB real, preset `offline`, sin red):
   `cargo test --bin featherai services -- --test-threads=1` → pasan los tests nuevos y
   `services_auth_admin_seed_and_login`.
3. Smoke end-to-end de la UI (lo que pide el pedido no lo cubren los tests):
   `dx serve --platform desktop` (o `cargo run`), login `admin`/`admin`.
   - El sidebar (y la barra inferior en ≤768px) muestra `Usuarios`; la ruta `/admin/users` lista al
     admin con sus 3 badges de negocio.
   - **Alta**: `Nuevo Usuario` → `tester`, contraseña `secreto1`, rol `Administración general`,
     tildar `NovaSoft` y agregar `Acme` con `Agregar` → `Guardar` cierra el modal y la fila aparece
     con los 2 negocios.
   - **Validación**: abrir `Nuevo Usuario`, destildar todos los negocios → `Guardar` muestra el error
     inline y el modal no se cierra. Cargar un username ya existente → toast de error y el modal
     sigue abierto con los datos.
   - **Persistencia + login**: `Salir` (menú de usuario) y volver a abrir; `tester` sigue listado.
     Cerrar sesión y entrar con `tester`/`secreto1` → abre el panel (prueba que el hash, el rol y los
     negocios se round-tripean por el store real).
   - **Modificación**: como `admin`, editar `tester` → cambiar nombre, destildar `NovaSoft` y cargar
     `nueva-clave` en `Nueva contraseña` → `Guardar`; cerrar sesión y entrar con `nueva-clave` (la
     vieja pasa a ser inválida con `Usuario o contraseña inválidos`).
   - **Baja**: con `admin`, `Eliminar` en la fila de `tester` → `ConfirmDialog` → confirmar; la fila
     desaparece y el login de `tester` falla. En la fila de `admin` **no** aparece el botón de borrar.
   - **Guardas**: crear un segundo super_admin y verificar que recién entonces se puede borrar al
     `admin` (si se prueba, recrear el estado con otro data dir: `FEATHRAI_DATA_DIR=/tmp/featherai-smoke dx serve --platform desktop`).
4. Paridad wasm (best-effort; ver contingencia): `cargo check --target wasm32-unknown-unknown
   --no-default-features --features web`.

## Supuestos y contingencias

- **Paridad web/wasm**: el crate mantiene ramas `#[cfg(target_arch = "wasm32")]` para proyectos y
  login, así que el módulo de usuarios replica esa estructura (lista demo en memoria). Si el chequeo
  del punto 4 falla por razones ajenas a este cambio (p. ej. `desktop!`/`dioxus::desktop` en
  `main.rs`), se omite: el camino entregado es desktop.
- **Rol y acceso**: `/admin/users` exige `super_admin` (igual que `/admin/projects`); un usuario
  `admin`/`operativo` creado por el ABM sólo puede ver `/settings` (la app hoy no tiene rutas para
  esos roles). No se agregan rutas ni permisos nuevos.
- **Negocios por nombre**: cada formulario ofrece los demo + los nombres ya usados **por sus propios
  registros**; un negocio creado escribiéndolo en el ABM de usuarios no aparece como opción en el
  formulario de proyectos hasta que un proyecto lo use (y viceversa), pero se puede escribir de nuevo
  con `Nuevo negocio…`. No se agrega store de negocios ni lectura cruzada entre stores.
- **Guardas de UI además de las del servicio**: el botón de borrar no se dibuja en la fila propia y
  el `select` de rol queda deshabilitado al editarse a uno mismo (las guardas equivalentes viven
  también en el servicio, que es el contrato).
- **`disabled` / renombrar usuario / avatar subido**: fuera de alcance (baja física, username
  inmutable y avatar por URL, que es lo que el modelo ya soporta).
