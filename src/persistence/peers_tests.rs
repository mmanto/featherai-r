//! Tests de convergencia entre dos nodos (red real: endpoint Iroh + iroh-docs
//! en loopback). Correr con:
//!
//! ```text
//! cargo test --bin featherai peers_tests -- --test-threads=1
//! ```
//!
//! El test de descubrimiento mDNS está `#[ignore]` (necesita multicast en la
//! red local; en CI/contenedores puede no estar disponible):
//!
//! ```text
//! cargo test --bin featherai peers_tests -- --ignored --test-threads=1
//! ```
//!
//! # Qué fijan
//! El espacio de datos de un store KeyValue (namespace de iroh-docs) se decide
//! al abrirlo, contra los pares ya conectados (ver `crate::net`):
//! - dos nodos conectados antes de abrir comparten el store y sincronizan en
//!   los dos sentidos;
//! - conectar en runtime no une espacios ya resueltos: el nodo que se une lo
//!   hace en su próximo arranque (y desde ahí sincroniza en vivo).
//!
//! Los pares se conectan con direcciones explícitas de loopback
//! (`IrohClient::add_node_addr`) en lugar de mDNS: deterministas y sin depender
//! del entorno de red; el camino mDNS es el mismo `connect_gossip`.

use crate::models::{Priority, Project, ProjectStatus};
use crate::persistence::{open_client, open_stores, Db};
use crate::services;
use guardian_db::p2p::network::client::IrohClient;
use guardian_db::p2p::network::config::ClientConfig;
use iroh::{EndpointAddr, TransportAddr};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

/// Direcciones locales del nodo (loopback) para inyectarlas en el otro lado.
async fn loopback_addr(client: &IrohClient) -> EndpointAddr {
    let ep = client.backend().get_endpoint().await.unwrap();
    let ep = {
        let guard = ep.read().await;
        guard.as_ref().unwrap().clone()
    };
    let addrs: Vec<TransportAddr> = ep
        .bound_sockets()
        .into_iter()
        .map(|s| {
            let s = if s.ip().is_unspecified() {
                SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), s.port())
            } else {
                s
            };
            TransportAddr::Ip(s)
        })
        .collect();
    EndpointAddr::new(client.node_id()).with_addrs(addrs)
}

/// Cliente Iroh sobre un data dir (misma configuración que la app).
async fn nuevo_nodo(dir: &Path) -> IrohClient {
    match open_client(dir, ClientConfig::development()).await {
        Ok(c) => c,
        Err(e) => panic!("cliente iroh en {dir:?}: {e}"),
    }
}

/// Proyecto mínimo para las escrituras de prueba.
fn proyecto(id: &str) -> Project {
    Project {
        id: id.to_string(),
        name: format!("proyecto {id}"),
        description: None,
        business: None,
        start_date: None,
        end_date: None,
        status: ProjectStatus::Active,
        priority: Priority::Medium,
        team: Vec::new(),
        color: "#6c757d".to_string(),
        created_at: None,
        tasks: Vec::new(),
    }
}

/// Espera a que el proyecto `id` aparezca en `db` (sync iroh-docs en vivo).
async fn esperar_proyecto(db: &Db, id: &str) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        if let Ok(list) = services::project::list_projects(db).await {
            if list.iter().any(|p| p.id == id) {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    false
}

/// Dos nodos conectados antes de abrir los stores comparten el espacio y
/// sincronizan en los dos sentidos.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dos_nodos_conectados_comparten_los_stores() {
    let tmp_a = tempfile::tempdir().unwrap();
    let tmp_b = tempfile::tempdir().unwrap();

    let a = nuevo_nodo(tmp_a.path()).await;
    let b = nuevo_nodo(tmp_b.path()).await;
    let id_a = a.node_id();
    let id_b = b.node_id();

    // Direcciones explícitas (equivale al mDNS resuelto de la LAN).
    a.add_node_addr(loopback_addr(&b).await).await.unwrap();
    b.add_node_addr(loopback_addr(&a).await).await.unwrap();

    // Conexión mutua ANTES de abrir los stores (lo que hace `net::Peers`).
    let peers_a = crate::net::Peers::start(&a, tmp_a.path(), crate::net::Join::OFF).await;
    let peers_b = crate::net::Peers::start(&b, tmp_b.path(), crate::net::Join::OFF).await;
    peers_a.connect(id_b).await.expect("A → B");
    peers_b.connect(id_a).await.expect("B → A");

    // Abre primero el nodo de id más chico (crea el namespace); el otro
    // importa su ticket al abrir.
    let (small, big, dir_small, dir_big, peers_small, peers_big) = if id_a < id_b {
        (a, b, tmp_a.path(), tmp_b.path(), peers_a, peers_b)
    } else {
        (b, a, tmp_b.path(), tmp_a.path(), peers_b, peers_a)
    };
    let db_small = open_stores(dir_small.to_path_buf(), small, peers_small)
        .await
        .expect("stores del nodo creador");
    let db_big = open_stores(dir_big.to_path_buf(), big, peers_big)
        .await
        .expect("stores del nodo que importa");

    // Escritura en el nodo que importó → visible en el creador.
    let desde_big = services::project::create_project(&db_big, proyecto("desde-big"))
        .await
        .unwrap();
    assert!(
        esperar_proyecto(&db_small, &desde_big.id).await,
        "el nodo creador no ve la escritura del que importó ({})",
        desde_big.id
    );

    // Escritura en el creador → visible en el que importó.
    let desde_small = services::project::create_project(&db_small, proyecto("desde-small"))
        .await
        .unwrap();
    assert!(
        esperar_proyecto(&db_big, &desde_small.id).await,
        "el nodo que importó no ve la escritura del creador ({})",
        desde_small.id
    );

    db_small.close().await.unwrap();
    db_big.close().await.unwrap();
}

/// Dos instalaciones que ya tienen datos propios (espacios distintos): al
/// reiniciar una con el otro nodo conectado, importa su espacio y desde ahí
/// sincroniza en vivo. Conectar en runtime no une espacios ya resueltos.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reiniciar_conectado_une_el_espacio() {
    let tmp_a = tempfile::tempdir().unwrap();
    let tmp_b = tempfile::tempdir().unwrap();

    // 1) Cada nodo abre su base sin ver al otro.
    let a = nuevo_nodo(tmp_a.path()).await;
    let b = nuevo_nodo(tmp_b.path()).await;
    let id_a = a.node_id();
    let id_b = b.node_id();
    let addr_b = loopback_addr(&b).await;
    let addr_a = loopback_addr(&a).await;
    let peers_a_pre = crate::net::Peers::start(&a, tmp_a.path(), crate::net::Join::OFF).await;
    let peers_b_pre = crate::net::Peers::start(&b, tmp_b.path(), crate::net::Join::OFF).await;
    let db_a = open_stores(tmp_a.path().to_path_buf(), a, peers_a_pre)
        .await
        .unwrap();
    let db_b = open_stores(tmp_b.path().to_path_buf(), b, peers_b_pre)
        .await
        .unwrap();
    let en_a = services::project::create_project(&db_a, proyecto("solo-a"))
        .await
        .unwrap();
    let en_b = services::project::create_project(&db_b, proyecto("solo-b"))
        .await
        .unwrap();

    // 2) Se conectan en runtime: no alcanza (el espacio ya quedó resuelto).
    db_a.client.add_node_addr(addr_b.clone()).await.unwrap();
    db_b.client.add_node_addr(addr_a.clone()).await.unwrap();
    db_a.peers.connect(id_b).await.unwrap();
    db_b.peers.connect(id_a).await.unwrap();
    let vistos_a = db_a.peers();
    assert!(
        vistos_a
            .iter()
            .any(|p| p.id == id_b.to_string() && p.connected && p.late),
        "el par conectado después de abrir debe marcarse para el próximo arranque: {vistos_a:?}"
    );
    tokio::time::sleep(Duration::from_secs(2)).await;
    let ve_a = services::project::list_projects(&db_a).await.unwrap();
    let ve_b = services::project::list_projects(&db_b).await.unwrap();
    assert!(
        !ve_a.iter().any(|p| p.id == en_b.id) && !ve_b.iter().any(|p| p.id == en_a.id),
        "conectar en runtime no debería unir espacios ya resueltos"
    );

    // 3) Reinicio de B con A conectado (arranque normal de la app).
    db_b.close().await.unwrap();
    drop(db_b);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let b2 = nuevo_nodo(tmp_b.path()).await;
    b2.add_node_addr(addr_a).await.unwrap();
    let peers_b2 = crate::net::Peers::start(&b2, tmp_b.path(), crate::net::Join::OFF).await;
    peers_b2.connect(id_a).await.unwrap();
    let db_b2 = open_stores(tmp_b.path().to_path_buf(), b2, peers_b2)
        .await
        .unwrap();
    assert!(
        esperar_proyecto(&db_b2, &en_a.id).await,
        "B reiniciado no importó el espacio de A"
    );

    // 4) Y desde ahí sincroniza en los dos sentidos.
    let en_b2 = services::project::create_project(&db_b2, proyecto("post-reinicio"))
        .await
        .unwrap();
    assert!(
        esperar_proyecto(&db_a, &en_b2.id).await,
        "A no ve lo que escribe B después de unirse al espacio"
    );

    db_a.close().await.unwrap();
    db_b2.close().await.unwrap();
}

/// Sin configuración ni conexión previa, dos nodos de la misma red interna se
/// descubren por mDNS (servicio `featherai`) y comparten los stores.
///
/// `#[ignore]`: depende de multicast en la red local. Correr con `--ignored`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requiere multicast en la red local (mDNS)"]
async fn dos_nodos_se_descubren_en_la_red_interna() {
    let tmp_a = tempfile::tempdir().unwrap();
    let tmp_b = tempfile::tempdir().unwrap();
    let a = nuevo_nodo(tmp_a.path()).await;
    let b = nuevo_nodo(tmp_b.path()).await;
    let id_a = a.node_id();
    let id_b = b.node_id();

    let join = || crate::net::Join {
        configured: false,
        lan: true,
        group: false,
        coordinator: false,
        lan_wait: Duration::from_secs(8),
    };
    let (peers_a, peers_b) = tokio::join!(
        crate::net::Peers::start(&a, tmp_a.path(), join()),
        crate::net::Peers::start(&b, tmp_b.path(), join()),
    );
    let vistos_a = peers_a.peers();
    let vistos_b = peers_b.peers();
    assert!(
        vistos_a
            .iter()
            .any(|p| p.id == id_b.to_string() && p.connected),
        "A no descubrió/conectó a B por mDNS: {vistos_a:?}"
    );
    assert!(
        vistos_b
            .iter()
            .any(|p| p.id == id_a.to_string() && p.connected),
        "B no descubrió/conectó a A por mDNS: {vistos_b:?}"
    );

    let (small, big, dir_small, dir_big, peers_small, peers_big) = if id_a < id_b {
        (a, b, tmp_a.path(), tmp_b.path(), peers_a, peers_b)
    } else {
        (b, a, tmp_b.path(), tmp_a.path(), peers_b, peers_a)
    };
    let db_small = open_stores(dir_small.to_path_buf(), small, peers_small)
        .await
        .unwrap();
    let db_big = open_stores(dir_big.to_path_buf(), big, peers_big)
        .await
        .unwrap();

    let desde_big = services::project::create_project(&db_big, proyecto("lan-big"))
        .await
        .unwrap();
    assert!(
        esperar_proyecto(&db_small, &desde_big.id).await,
        "descubiertos por mDNS, los stores no se comparten"
    );

    db_small.close().await.unwrap();
    db_big.close().await.unwrap();
}

/// Sin configurarse entre sí, dos miembros que conocen al mismo coordinador se
/// descubren por el grupo (gossip) — opción A. A es el coordinador; B y C lo
/// agregan como par guardado (`peers.txt`) y, por la malla, se descubren
/// mutuamente aunque ninguno configuró al otro.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dos_miembros_se_descubren_por_el_grupo() {
    let tmp_a = tempfile::tempdir().unwrap();
    let tmp_b = tempfile::tempdir().unwrap();
    let tmp_c = tempfile::tempdir().unwrap();

    let a = nuevo_nodo(tmp_a.path()).await;
    let b = nuevo_nodo(tmp_b.path()).await;
    let c = nuevo_nodo(tmp_c.path()).await;
    let (id_a, id_b, id_c) = (a.node_id(), b.node_id(), c.node_id());

    // Direcciones de loopback entre todos (el gossip dialea a los miembros).
    a.add_node_addr(loopback_addr(&b).await).await.unwrap();
    a.add_node_addr(loopback_addr(&c).await).await.unwrap();
    b.add_node_addr(loopback_addr(&a).await).await.unwrap();
    b.add_node_addr(loopback_addr(&c).await).await.unwrap();
    c.add_node_addr(loopback_addr(&a).await).await.unwrap();
    c.add_node_addr(loopback_addr(&b).await).await.unwrap();

    // B y C guardan a A como par (el "coordinador"); A no configura a nadie.
    std::fs::write(tmp_b.path().join(crate::net::PEERS_FILE), format!("{id_a}\n")).unwrap();
    std::fs::write(tmp_c.path().join(crate::net::PEERS_FILE), format!("{id_a}\n")).unwrap();

    let join_coord = || crate::net::Join {
        configured: false,
        lan: false,
        group: true,
        coordinator: false,
        lan_wait: Duration::ZERO,
    };
    let join_miembro = || crate::net::Join {
        configured: true,
        lan: false,
        group: true,
        coordinator: false,
        lan_wait: Duration::ZERO,
    };

    // `_peers_a` queda vivo hasta el final: mantiene al coordinador (A) en la
    // malla durante toda la prueba.
    let _peers_a = crate::net::Peers::start(&a, tmp_a.path(), join_coord()).await;
    let peers_b = crate::net::Peers::start(&b, tmp_b.path(), join_miembro()).await;
    let peers_c = crate::net::Peers::start(&c, tmp_c.path(), join_miembro()).await;

    // B y C no se configuraron mutuamente: se descubren por el grupo.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let vb = peers_b.peers();
        let vc = peers_c.peers();
        let b_ve_c = vb.iter().any(|p| p.id == id_c.to_string() && p.connected);
        let c_ve_b = vc.iter().any(|p| p.id == id_b.to_string() && p.connected);
        if b_ve_c && c_ve_b {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!("el grupo no los descubrió en 30 s: B={vb:?} C={vc:?}");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// Un nodo que oye a un coordinador (vía el grupo) **no** lo auto-conecta: lo
/// deja pendiente y el usuario lo acepta explícitamente. A es el coordinador; M
/// lo acepta (lo guarda); B entra al grupo vía M y oye a A sin haberlo
/// configurado, así que le aparece como pendiente.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn el_coordinador_requiere_aceptacion() {
    let tmp_a = tempfile::tempdir().unwrap();
    let tmp_m = tempfile::tempdir().unwrap();
    let tmp_b = tempfile::tempdir().unwrap();

    let a = nuevo_nodo(tmp_a.path()).await;
    let m = nuevo_nodo(tmp_m.path()).await;
    let b = nuevo_nodo(tmp_b.path()).await;
    let (id_a, id_m) = (a.node_id(), m.node_id());

    // Direcciones de loopback entre todos (el gossip dialea a los miembros).
    a.add_node_addr(loopback_addr(&b).await).await.unwrap();
    a.add_node_addr(loopback_addr(&m).await).await.unwrap();
    b.add_node_addr(loopback_addr(&a).await).await.unwrap();
    b.add_node_addr(loopback_addr(&m).await).await.unwrap();
    m.add_node_addr(loopback_addr(&a).await).await.unwrap();
    m.add_node_addr(loopback_addr(&b).await).await.unwrap();

    // M acepta a A (lo guarda); B sólo conoce a M.
    std::fs::write(tmp_m.path().join(crate::net::PEERS_FILE), format!("{id_a}\n")).unwrap();
    std::fs::write(tmp_b.path().join(crate::net::PEERS_FILE), format!("{id_m}\n")).unwrap();

    let join_coord = || crate::net::Join {
        configured: false,
        lan: false,
        group: true,
        coordinator: true,
        lan_wait: Duration::ZERO,
    };
    let join_miembro = || crate::net::Join {
        configured: true,
        lan: false,
        group: true,
        coordinator: false,
        lan_wait: Duration::ZERO,
    };

    let _peers_a = crate::net::Peers::start(&a, tmp_a.path(), join_coord()).await;
    let _peers_m = crate::net::Peers::start(&m, tmp_m.path(), join_miembro()).await;
    let peers_b = crate::net::Peers::start(&b, tmp_b.path(), join_miembro()).await;

    // B oye a A (coordinador) y lo deja pendiente, sin auto-conectarlo.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let pend = peers_b.pending_coordinators();
        if pend.iter().any(|p| *p == id_a.to_string()) {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!("B no registró al coordinador A como pendiente: {pend:?}");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    assert!(
        !peers_b.peers().iter().any(|p| p.id == id_a.to_string() && p.connected),
        "B no debería haber conectado al coordinador sin aceptarlo"
    );

    // Aceptar lo guarda, lo conecta y lo saca de pendientes.
    let conectado = peers_b.accept_coordinator(id_a).await.unwrap();
    assert!(conectado, "el coordinador aceptado debería quedar conectado");
    assert!(
        peers_b.pending_coordinators().is_empty(),
        "A debería salir de la lista de pendientes"
    );
    assert!(
        peers_b
            .peers()
            .iter()
            .any(|p| p.id == id_a.to_string() && p.connected && p.source == crate::net::Source::Saved),
        "A debería quedar guardado y conectado"
    );
}
