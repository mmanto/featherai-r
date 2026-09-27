//! Tests de integración de la capa de servicios contra GuardianDB real
//! (preset `offline`: sin mDNS ni redes). Correr con:
//!
//! ```text
//! cargo test --bin featherai services -- --test-threads=1
//! ```

use crate::models::{Priority, Project, ProjectStatus, StoredUser, Task, TaskStatus, User};
use crate::net::Join;
use crate::persistence::{open_with_config, AppError, Db};
use guardian_db::p2p::network::config::ClientConfig;

/// Abre una base sobre un directorio temporal con el preset `offline`
/// (persistente en disco; el TempDir vive mientras dure el test).
async fn open_db(dir: &std::path::Path) -> Db {
    open_with_config(dir.to_path_buf(), ClientConfig::offline(), Join::OFF)
        .await
        .expect("abrir GuardianDB offline")
}

fn sample_task(title: &str) -> Task {
    Task {
        id: format!("provisional-{title}"),
        title: title.to_string(),
        description: None,
        status: TaskStatus::Todo,
        assignee: None,
        priority: Priority::Medium,
        start_date: None,
        end_date: None,
        estimated_hours: None,
        tags: Vec::new(),
    }
}

fn sample_project(name: &str) -> Project {
    Project {
        id: "provisional".to_string(),
        name: name.to_string(),
        description: None,
        business: Some("AgileTeam Corp".to_string()),
        start_date: None,
        end_date: None,
        status: ProjectStatus::Planning,
        priority: Priority::Medium,
        team: Vec::new(),
        color: "#6c757d".to_string(),
        created_at: None,
        tasks: Vec::new(),
    }
}

/// Usuario de prueba para el ABM: sin email/avatar y con los negocios dados.
fn sample_user(username: &str, role: &str, businesses: &[&str]) -> User {
    User {
        username: username.to_string(),
        email: None,
        nombre: Some("Test".to_string()),
        apellido: None,
        avatar_url: None,
        role: role.to_string(),
        businesses: businesses.iter().map(|b| (*b).to_string()).collect(),
    }
}

/// Títulos de las tareas de un proyecto en el orden del listado.
async fn project_titles(db: &Db, pid: &str) -> Vec<String> {
    let projects = crate::services::project::list_projects(db).await.expect("list");
    let p = projects.iter().find(|p| p.id == pid).expect("proyecto existe");
    p.tasks.iter().map(|t| t.title.clone()).collect()
}

#[tokio::test]
async fn services_seed_demo_lists_projects_in_order_with_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path()).await;

    let projects = crate::services::project::list_projects(&db).await.expect("list");
    assert_eq!(projects.len(), 3, "la siembra demo crea 3 proyectos");
    let ids: Vec<&str> = projects.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["p1", "p2", "p3"], "orden por created_at = orden demo");

    // Tareas ensambladas y ordenadas por position.
    let p1 = projects.iter().find(|p| p.id == "p1").unwrap();
    let task_ids: Vec<&str> = p1.tasks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(task_ids, vec!["t1", "t2", "t3", "t4"]);
    assert_eq!(p1.tasks[0].title, "Modelo de datos de productos");
    let p3 = projects.iter().find(|p| p.id == "p3").unwrap();
    assert_eq!(p3.tasks.len(), 3);
    assert!(projects.iter().all(|p| p.created_at.is_some()), "seed marca created_at");
}

#[tokio::test]
async fn services_create_project_assigns_uuid_and_appears_last() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path()).await;

    let created = crate::services::project::create_project(&db, sample_project("Nuevo"))
        .await
        .expect("create");
    assert_ne!(created.id, "provisional", "el servicio reemplaza el id provisional");
    assert!(created.created_at.is_some());
    assert!(created.tasks.is_empty());

    let projects = crate::services::project::list_projects(&db).await.unwrap();
    assert_eq!(projects.len(), 4);
    assert_eq!(projects.last().unwrap().name, "Nuevo", "los nuevos van al final");
}

#[tokio::test]
async fn services_task_positions_and_replace_keeps_position() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path()).await;
    let p = crate::services::project::create_project(&db, sample_project("P"))
        .await
        .unwrap();

    let a = crate::services::project::add_task(&db, &p.id, sample_task("A"))
        .await
        .expect("add A");
    let b = crate::services::project::add_task(&db, &p.id, sample_task("B"))
        .await
        .expect("add B");
    assert_ne!(a.id, "provisional-A", "uuid asignado");
    assert_ne!(a.id, b.id, "ids únicos");

    // add_task sobre proyecto inexistente → NotFound.
    let err = crate::services::project::add_task(&db, "no-existe", sample_task("X"))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");

    // replace_task conserva la position y actualiza el contenido: si la
    // moviera al final, el orden posterior sería B, A2, C.
    let mut edited = a.clone();
    edited.title = "A2".into();
    crate::services::project::replace_task(&db, &p.id, &edited)
        .await
        .expect("replace A");
    crate::services::project::add_task(&db, &p.id, sample_task("C"))
        .await
        .expect("add C");
    assert_eq!(project_titles(&db, &p.id).await, vec!["A2", "B", "C"]);

    // replace_task sobre tarea inexistente → NotFound.
    let ghost = sample_task("fantasma");
    let err = crate::services::project::replace_task(&db, &p.id, &ghost)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)), "{err:?}");
}

#[tokio::test]
async fn services_delete_task_and_delete_project() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path()).await;
    let p = crate::services::project::create_project(&db, sample_project("Borrar"))
        .await
        .unwrap();
    let t1 = crate::services::project::add_task(&db, &p.id, sample_task("t1"))
        .await
        .unwrap();
    let t2 = crate::services::project::add_task(&db, &p.id, sample_task("t2"))
        .await
        .unwrap();

    // delete_task: borra, y repetir/inexistente/proyecto ajeno = no-op.
    crate::services::project::delete_task(&db, &p.id, &t1.id).await.expect("delete t1");
    crate::services::project::delete_task(&db, &p.id, &t1.id).await.expect("repetido = no-op");
    crate::services::project::delete_task(&db, &p.id, "nunca-existio").await.expect("no-op");
    crate::services::project::delete_task(&db, "otro-proyecto", &t2.id).await.expect("no-op");

    let projects = crate::services::project::list_projects(&db).await.unwrap();
    let p = projects.iter().find(|p| p.name == "Borrar").unwrap();
    assert_eq!(p.tasks.len(), 1);
    assert_eq!(p.tasks[0].id, t2.id);

    // delete_project: borra proyecto + sus TaskDoc; inexistente/repetido = no-op.
    let pid = p.id.clone();
    crate::services::project::delete_project(&db, &pid).await.expect("delete project");
    crate::services::project::delete_project(&db, &pid).await.expect("repetido = no-op");
    crate::services::project::delete_project(&db, "no-existe").await.expect("no-op");

    let projects = crate::services::project::list_projects(&db).await.unwrap();
    assert!(!projects.iter().any(|x| x.id == pid));

    // Ningún TaskDoc huérfano del proyecto en el store crudo.
    let orphans: Vec<String> = db
        .tasks
        .all()
        .iter()
        .filter(|(_, bytes)| {
            serde_json::from_slice::<serde_json::Value>(bytes)
                .ok()
                .and_then(|v| v.get("project_id").and_then(|v| v.as_str()).map(str::to_string))
                .is_some_and(|id| id == pid)
        })
        .map(|(key, _)| key.clone())
        .collect();
    assert!(orphans.is_empty(), "TaskDoc huérfanos: {orphans:?}");
}

#[tokio::test]
async fn services_persistence_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = open_db(dir.path()).await;
        crate::services::project::create_project(&db, sample_project("Persiste"))
            .await
            .unwrap();
        // Cierre explícito: libera los locks redb para reabrir el mismo dir
        // dentro del proceso (en la app real equivale al cierre al salir).
        db.close().await.expect("cerrar base");
    } // Db se dropea acá, antes del TempDir.

    let db = open_db(dir.path()).await;
    let projects = crate::services::project::list_projects(&db).await.unwrap();
    assert!(
        projects.iter().any(|p| p.name == "Persiste"),
        "lo escrito sobrevive al cierre y re-apertura del mismo dir"
    );
    assert_eq!(projects.len(), 4, "demo + 1: la siembra no se duplica");
}

/// Cierre disparado desde un handler de la UI: dioxus desktop drivea el event
/// loop dentro de un runtime tokio, donde `Handle::block_on` panickea con
/// *"Cannot start a runtime from within a runtime"*.
#[test]
fn services_close_blocking_desde_un_runtime_ajeno() {
    let dir = tempfile::tempdir().unwrap();

    // Runtime donde vive la base (equivale al runtime propio de `init_backend`).
    let db_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let (db, pid) = db_rt.block_on(async {
        let db = open_db(dir.path()).await;
        let project = crate::services::project::create_project(&db, sample_project("Cierre"))
            .await
            .expect("crear proyecto");
        (db, project.id)
    });

    // Runtime que drivea la UI: acá adentro `db.rt.block_on(...)` panickearía.
    let ui_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    ui_rt.block_on(async { db.close_blocking() });
    // Teardown completo antes de reabrir el mismo dir: el `Db` y los dos
    // runtimes (sus tasks de fondo retienen los archivos redb).
    drop(db);
    drop(ui_rt);
    drop(db_rt);

    // El cierre ordenado dejó el dir reabrible y el dato intacto.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let db = rt.block_on(open_db(dir.path()));
    let projects = rt
        .block_on(crate::services::project::list_projects(&db))
        .unwrap();
    assert!(
        projects.iter().any(|p| p.id == pid),
        "el proyecto sobrevive al cierre pedido desde el runtime de la UI"
    );
    rt.block_on(db.close()).expect("cerrar base");
}

#[tokio::test]
async fn services_auth_admin_seed_and_login() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path()).await;

    let user = crate::services::auth::login(&db, "admin", "admin")
        .await
        .expect("login sin error de store")
        .expect("admin/admin válido");
    assert_eq!(user.username, "admin");
    assert_eq!(user.role, "super_admin");
    assert_eq!(user.nombre.as_deref(), Some("Admin"));
    assert_eq!(
        user.businesses,
        vec![
            "AgileTeam Corp".to_string(),
            "NovaSoft".to_string(),
            "TechCorp".to_string()
        ],
        "el admin sembrado arranca con los negocios demo"
    );

    // El doc persistido guarda el hash PHC argon2 (nunca la contraseña).
    let bytes = db.users.get("admin").await.unwrap().expect("doc admin");
    let stored: StoredUser = serde_json::from_slice(&bytes).unwrap();
    assert!(stored.password_hash.starts_with("$argon2"), "hash PHC");
    assert_ne!(stored.password_hash, "admin");

    // Normalización (trim + lowercase) y credenciales inválidas.
    assert!(crate::services::auth::login(&db, "  ADMIN ", "admin").await.unwrap().is_some());
    assert!(crate::services::auth::login(&db, "admin", "mala").await.unwrap().is_none());
    assert!(crate::services::auth::login(&db, "nadie", "x").await.unwrap().is_none());

    // ensure_admin idempotente: segundo llamado no duplica ni rompe.
    crate::services::auth::ensure_admin(&db).await.expect("ensure 2");
    assert_eq!(db.users.all().len(), 1, "un solo admin persistido");
    assert!(crate::services::auth::login(&db, "admin", "admin").await.unwrap().is_some());
}

#[tokio::test]
async fn services_users_abm_crud() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path()).await;

    // Alta: username normalizado (trim + lowercase) y negocios con trim + dedup.
    let created = crate::services::auth::create_user(
        &db,
        &sample_user("Ana.Ruiz", "operativo", &["NovaSoft", " NovaSoft ", "Acme"]),
        "secreto",
    )
    .await
    .expect("alta de usuario");
    assert_eq!(created.username, "ana.ruiz", "username normalizado");
    assert_eq!(
        created.businesses,
        vec!["NovaSoft".to_string(), "Acme".to_string()],
        "negocios normalizados y deduplicados"
    );

    // El listado incluye al admin sembrado y ordena por username.
    let users = crate::services::auth::list_users(&db).await.expect("listado");
    assert_eq!(
        users.iter().map(|u| u.username.as_str()).collect::<Vec<_>>(),
        vec!["admin", "ana.ruiz"]
    );

    assert!(crate::services::auth::login(&db, "ana.ruiz", "secreto").await.unwrap().is_some());

    // Modificación: nombre + negocios y rotación de contraseña (username intacto).
    let mut edit = created.clone();
    edit.nombre = Some("Ana".to_string());
    edit.businesses = vec!["Acme".to_string()];
    let updated = crate::services::auth::update_user(&db, "ana.ruiz", &edit, Some("nueva-clave"))
        .await
        .expect("modificación");
    assert_eq!(updated.username, "ana.ruiz");
    assert_eq!(updated.nombre.as_deref(), Some("Ana"));
    assert_eq!(updated.businesses, vec!["Acme".to_string()]);
    assert!(crate::services::auth::login(&db, "ana.ruiz", "nueva-clave").await.unwrap().is_some());
    assert!(
        crate::services::auth::login(&db, "ana.ruiz", "secreto").await.unwrap().is_none(),
        "la contraseña vieja ya no sirve"
    );

    // Baja física del doc.
    crate::services::auth::delete_user(&db, "ana.ruiz").await.expect("baja");
    assert!(crate::services::auth::login(&db, "ana.ruiz", "nueva-clave").await.unwrap().is_none());
    assert_eq!(
        crate::services::auth::list_users(&db).await.unwrap().len(),
        1,
        "queda sólo el admin"
    );
}

#[tokio::test]
async fn services_users_validaciones_y_guardas() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path()).await;

    // Entradas inválidas: sin negocio, username corto, rol desconocido y
    // contraseña vacía.
    for (user, password) in [
        (sample_user("pepe", "operativo", &[]), "secreto"),
        (sample_user("ab", "operativo", &["Acme"]), "secreto"),
        (sample_user("pepe", "root", &["Acme"]), "secreto"),
        (sample_user("pepe", "operativo", &["Acme"]), ""),
    ] {
        let result = crate::services::auth::create_user(&db, &user, password).await;
        assert!(
            matches!(result, Err(AppError::Invalid(_))),
            "entrada inválida rechazada: {user:?}"
        );
    }

    // Alta válida y duplicado case-insensitive.
    crate::services::auth::create_user(&db, &sample_user("pepe", "operativo", &["Acme"]), "secreto")
        .await
        .expect("alta válida");
    let duplicado =
        crate::services::auth::create_user(&db, &sample_user("PEPE", "operativo", &["Acme"]), "secreto")
            .await;
    assert!(matches!(duplicado, Err(AppError::Invalid(_))), "usuario duplicado rechazado");

    // Guardas del último super_admin: no se borra ni se demueve.
    assert!(matches!(
        crate::services::auth::delete_user(&db, "admin").await,
        Err(AppError::Invalid(_))
    ));
    let demote = crate::services::auth::update_user(
        &db,
        "admin",
        &sample_user("admin", "admin", &["Acme"]),
        None,
    )
    .await;
    assert!(matches!(demote, Err(AppError::Invalid(_))));

    // Con un segundo super_admin, la democión/baja del primero pasa.
    crate::services::auth::create_user(&db, &sample_user("root2", "super_admin", &["Acme"]), "secreto")
        .await
        .expect("segundo super_admin");
    crate::services::auth::delete_user(&db, "admin").await.expect("baja con otro super_admin");

    // Baja de un usuario inexistente → NotFound.
    assert!(matches!(
        crate::services::auth::delete_user(&db, "nadie").await,
        Err(AppError::NotFound(_))
    ));
}

#[tokio::test]
async fn services_users_omite_docs_ajenos_al_formato() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path()).await;

    // Doc escrito por fuera de la app (otro formato, sin `password_hash`): no
    // rompe el listado —ni el arranque, que lo usa— ni las guardas.
    db.users
        .put("gabriel", br#"{"username":"gabriel","role":"admin"}"#.to_vec())
        .await
        .expect("doc ajeno");

    let users = crate::services::auth::list_users(&db).await.expect("listado");
    assert_eq!(
        users.iter().map(|u| u.username.as_str()).collect::<Vec<_>>(),
        vec!["admin"],
        "el doc ajeno se omite del listado"
    );

    // El doc ajeno no cuenta como super_admin (sigue siendo el único el admin).
    assert!(matches!(
        crate::services::auth::delete_user(&db, "admin").await,
        Err(AppError::Invalid(_))
    ));

    // El alta sigue funcionando con el doc ajeno presente.
    crate::services::auth::create_user(&db, &sample_user("pepe", "operativo", &["Acme"]), "secreto")
        .await
        .expect("alta con doc ajeno presente");
    assert_eq!(db.users.all().len(), 3, "admin + gabriel + pepe");
}
