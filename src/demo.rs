//! Datos de demostración — port de los seeds de feathrai-frontend.
//!
//! En el camino nativo los consume `services::project::seed_demo` (primera
//! corrida sobre GuardianDB vacío); en web/wasm puebla el estado inicial de
//! `ProjectProvider` (sin persistencia). Target-neutral: solo depende del
//! modelo.

use crate::models::{Priority, Project, ProjectStatus, Task, TaskStatus};

/// Proyectos demo (ids `p1..p3`, tareas `t1..t10`) — equivalente de los
/// seeds originales del `ProjectContext` de React.
pub fn demo_projects() -> Vec<Project> {
    let p1 = Project {
        id: "p1".into(),
        name: "Sistema de Gestión de Inventarios".into(),
        description: Some("Plataforma web para control de stock y reposiciones.".into()),
        business: Some("AgileTeam Corp".into()),
        start_date: Some("2026-08-03".into()),
        end_date: Some("2026-10-30".into()),
        status: ProjectStatus::Active,
        priority: Priority::High,
        team: vec!["Juan Pérez".into(), "María García".into()],
        color: "#00A3F0".into(),
        created_at: None,
        tasks: vec![
            Task {
                id: "t1".into(),
                title: "Modelo de datos de productos".into(),
                description: Some("Entidades, relaciones y migraciones iniciales.".into()),
                status: TaskStatus::Done,
                assignee: Some("María García".into()),
                priority: Priority::High,
                start_date: Some("2026-08-03".into()),
                end_date: Some("2026-08-14".into()),
                estimated_hours: Some(24.0),
                tags: vec!["backend".into(), "database".into()],
            },
            Task {
                id: "t2".into(),
                title: "API de movimientos de stock".into(),
                description: Some("Alta, baja y ajuste de existencias.".into()),
                status: TaskStatus::InProgress,
                assignee: Some("Juan Pérez".into()),
                priority: Priority::High,
                start_date: Some("2026-08-15".into()),
                end_date: Some("2026-09-12".into()),
                estimated_hours: Some(40.0),
                tags: vec!["backend".into(), "api".into()],
            },
            Task {
                id: "t3".into(),
                title: "Pantalla de inventario".into(),
                description: Some("Tabla con filtros y exportación a CSV.".into()),
                status: TaskStatus::InProgress,
                assignee: None,
                priority: Priority::Medium,
                start_date: Some("2026-09-01".into()),
                end_date: Some("2026-09-30".into()),
                estimated_hours: Some(32.0),
                tags: vec!["frontend".into(), "ui".into()],
            },
            Task {
                id: "t4".into(),
                title: "Alertas de reposición".into(),
                description: Some("Notificaciones por email bajo umbral mínimo.".into()),
                status: TaskStatus::Todo,
                assignee: Some("Juan Pérez".into()),
                priority: Priority::Low,
                start_date: Some("2026-10-01".into()),
                end_date: Some("2026-10-23".into()),
                estimated_hours: Some(16.0),
                tags: vec!["notifications".into()],
            },
        ],
    };

    let p2 = Project {
        id: "p2".into(),
        name: "Portal de Ventas E-commerce".into(),
        description: Some("Catálogo, carrito y checkout para tienda online.".into()),
        business: Some("NovaSoft".into()),
        start_date: Some("2026-09-01".into()),
        end_date: Some("2026-12-15".into()),
        status: ProjectStatus::Planning,
        priority: Priority::Medium,
        team: vec!["Carlos López".into()],
        color: "#8b5cf6".into(),
        created_at: None,
        tasks: vec![
            Task {
                id: "t5".into(),
                title: "Definición de alcance".into(),
                description: Some("Requerimientos y mapeo de flujos de compra.".into()),
                status: TaskStatus::InProgress,
                assignee: Some("Carlos López".into()),
                priority: Priority::High,
                start_date: Some("2026-09-01".into()),
                end_date: Some("2026-09-15".into()),
                estimated_hours: Some(20.0),
                tags: vec!["planning".into()],
            },
            Task {
                id: "t6".into(),
                title: "Diseño del catálogo".into(),
                description: Some("Wireframes de listado y detalle de producto.".into()),
                status: TaskStatus::Todo,
                assignee: None,
                priority: Priority::Medium,
                start_date: Some("2026-09-16".into()),
                end_date: Some("2026-10-05".into()),
                estimated_hours: Some(30.0),
                tags: vec!["design".into(), "ux".into()],
            },
            Task {
                id: "t7".into(),
                title: "Integración de pagos".into(),
                description: Some("Evaluación de proveedores y sandbox.".into()),
                status: TaskStatus::Todo,
                assignee: None,
                priority: Priority::Medium,
                start_date: Some("2026-10-06".into()),
                end_date: Some("2026-11-20".into()),
                estimated_hours: Some(48.0),
                tags: vec!["payments".into(), "integration".into()],
            },
        ],
    };

    let p3 = Project {
        id: "p3".into(),
        name: "App Móvil de Fidelización".into(),
        description: Some("Programa de puntos y beneficios para clientes.".into()),
        business: Some("TechCorp".into()),
        start_date: Some("2026-03-10".into()),
        end_date: Some("2026-07-20".into()),
        status: ProjectStatus::Completed,
        priority: Priority::Low,
        team: vec!["Ana Ruiz".into(), "Pedro Díaz".into()],
        color: "#10b981".into(),
        created_at: None,
        tasks: vec![
            Task {
                id: "t8".into(),
                title: "Módulo de puntos".into(),
                description: Some("Acumulación y canje de puntos por compras.".into()),
                status: TaskStatus::Done,
                assignee: Some("Ana Ruiz".into()),
                priority: Priority::High,
                start_date: Some("2026-03-10".into()),
                end_date: Some("2026-04-30".into()),
                estimated_hours: Some(50.0),
                tags: vec!["backend".into()],
            },
            Task {
                id: "t9".into(),
                title: "App nativa iOS/Android".into(),
                description: Some("Cliente móvil con wallet de beneficios.".into()),
                status: TaskStatus::Done,
                assignee: Some("Pedro Díaz".into()),
                priority: Priority::Medium,
                start_date: Some("2026-04-01".into()),
                end_date: Some("2026-06-15".into()),
                estimated_hours: Some(80.0),
                tags: vec!["mobile".into(), "ui".into()],
            },
            Task {
                id: "t10".into(),
                title: "Lanzamiento en tiendas".into(),
                description: Some("Publicación y revisión de releases.".into()),
                status: TaskStatus::Done,
                assignee: Some("Ana Ruiz".into()),
                priority: Priority::Low,
                start_date: Some("2026-06-20".into()),
                end_date: Some("2026-07-20".into()),
                estimated_hours: Some(12.0),
                tags: vec!["release".into()],
            },
        ],
    };

    vec![p1, p2, p3]
}
