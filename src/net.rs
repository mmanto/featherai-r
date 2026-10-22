//! Red de pares: descubrimiento en la red interna (mDNS) y conexión
//! explícita, **antes** de abrir los stores de GuardianDB.
//!
//! # Por qué la conexión va antes de abrir la base
//! GuardianDB replica contra los pares **conocidos** (`note_known_peer`), y
//! hoy el único camino que los registra es [`IrohClient::connect_gossip`]
//! (el descubrimiento de Iroh 1.x es pull-based: resuelve un id puntual, no
//! enumera la red). Además, el espacio de datos de un store KeyValue — el
//! namespace de iroh-docs — se decide **al abrirlo**: `resolve_shared_ticket`
//! pide el ticket de ese store a los pares conocidos en ese momento y, si
//! nadie responde, crea un namespace propio (aislado). De ahí el orden:
//!
//! 1. `Peers::start` conecta los pares configurados y los de la red interna.
//! 2. Recién después `persistence::open_with_config` abre los tres stores.
//!
//! Con eso, un nodo que arranca después se une al espacio del que ya estaba
//! (recibe su ticket); dos que arrancan a la vez convergen al espacio del id
//! de endpoint más chico (el "creador" que elige `resolve_shared_ticket`). Un
//! par que aparece **después** de abrir los stores queda conectado para el
//! próximo arranque (el proceso ya resolvió su namespace): por eso Ajustes
//! marca esos pares como [`Peer::late`].
//!
//! # Un par se registra sólo si la conexión se estableció
//! `IrohClient::connect_gossip` llama `note_known_peer` **antes** de dialear,
//! así que un par apagado queda "conocido" igual. Eso le cuesta minutos a la
//! apertura: `resolve_shared_ticket` le pide el ticket a cada par conocido y
//! cada pedido espera el timeout de conexión de iroh (30 s, con hasta 11
//! intentos por store cuando hay un par de id menor: ~5,5 min por store) sin
//! ganar nada — un par que no responde no puede entregar el ticket. Por eso
//! [`dial`] primero prueba la conexión (sin registrar) y sólo entonces llama a
//! `connect_gossip`. Un par que no responde al arrancar no demora la ventana:
//! los stores reabren su namespace local (el que ya tenían) y el watcher mDNS
//! sigue intentando en segundo plano.
//!
//! # Descubrimiento en la red interna
//! La instancia mDNS que arma guardian-db adentro usa el servicio `irohv1` y
//! no expone su stream, así que acá se crea una propia con servicio
//! [`MDNS_SERVICE`]: sólo los nodos feathrai la publican y la escuchan. Al
//! registrarla en el endpoint (`address_lookup().add`) Iroh publica las
//! direcciones locales actuales, y `subscribe()` enumera los nodos feathrai
//! de la red. No hace falta internet ni relay: las direcciones directas de la
//! LAN alcanzan (`conn_type` lo reporta como "direct").
//!
//! # Grupo por gossip (coordinador)
//! En redes distintas (sin multicast) el mDNS no alcanza. El grupo resuelve el
//! *intercambio de ids*: un nodo con id estable —el **coordinador**, típicamente
//! un `--node` siempre encendido marcado con [`COORDINATOR_ENV`]— **anuncia su
//! rol** (`coordinator:<id>`) al grupo y publica [`MDNS_COORDINATOR_SERVICE`] en
//! la LAN. Un nodo que lo oye **no** lo auto-conecta: lo deja en
//! `pending_coordinators` y Ajustes le pregunta al usuario (*Aceptar*/*Ignorar*).
//! Al aceptarlo, [`Peers::accept_coordinator`] lo guarda y conecta; desde ahí los
//! miembros se descubren entre sí por el overlay (anuncios `member:<id>`), sin
//! haberse configurado mutuamente. El coordinador sólo reparte ids de membresía;
//! los datos siguen yendo p2p.
//!
//! # Configuración
//! - `FEATHRAI_PEERS=id,id,…` — pares fijos (no editables desde Ajustes).
//! - `FEATHRAI_COORDINATOR=1` — este nodo se identifica como coordinador.
//! - `<data dir>/peers.txt` — pares guardados desde Ajustes, uno por línea
//!   (`#` comenta). Se conectan en cada arranque.
//! - `FEATHRAI_LAN_WAIT_MS=ms` — espera del descubrimiento LAN antes de abrir
//!   la base (por defecto [`DEFAULT_LAN_WAIT`]; `0` no espera y deja sólo el
//!   watcher).

use futures_lite::StreamExt;
use guardian_db::p2p::network::client::IrohClient;
use guardian_db::p2p::network::core::gossip::IrohTopic;
use guardian_db::traits::PubSubTopic;
use iroh::EndpointId;
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Servicio mDNS de los nodos feathrai (el de guardian-db adentro es `irohv1`).
pub const MDNS_SERVICE: &str = "featherai";

/// Topic gossip del grupo de pares (opción A: coordinador/rendezvous). Un nodo
/// que conoce a un miembro del grupo (el coordinador) se une a esta malla y
/// descubre al resto, aunque estén en redes distintas; ver [`start_group`].
pub const GROUP_TOPIC: &str = "featherai";

/// Env que marca a este nodo como **coordinador** (`1`/cualquier valor no
/// vacío). El coordinador anuncia su rol y publica el servicio mDNS
/// [`MDNS_COORDINATOR_SERVICE`]; los demás nodos piden aceptarlo antes de
/// sincronizar.
pub const COORDINATOR_ENV: &str = "FEATHRAI_COORDINATOR";

/// Servicio mDNS del coordinador (distinto del de miembros [`MDNS_SERVICE`]):
/// los nodos que lo oyen **no** lo auto-conectan, sino que piden aceptarlo.
pub const MDNS_COORDINATOR_SERVICE: &str = "featherai-coordinator";

/// Pares fijos a conectar en cada arranque: `id,id,…` (hex de 64 o base32).
pub const PEERS_ENV: &str = "FEATHRAI_PEERS";

/// Milisegundos de espera del descubrimiento LAN antes de abrir los stores.
pub const LAN_WAIT_ENV: &str = "FEATHRAI_LAN_WAIT_MS";

/// Pares guardados desde Ajustes (una línea por id; `#` comenta).
pub const PEERS_FILE: &str = "peers.txt";

/// Espera por defecto: cubre un anuncio mDNS del otro nodo (los anuncios se
/// repiten cada pocos segundos).
pub const DEFAULT_LAN_WAIT: Duration = Duration::from_millis(2500);

/// Tope del dial al arrancar. Un par apagado cuesta este tope (una vez por
/// par configurado): la conexión hace falta para el registro (`note_known_peer`
/// es lo que habilita el intercambio de tickets, ver [`dial`]), así que el
/// arranque no puede esperar más que esto por un par que no responde.
const START_DIAL: Duration = Duration::from_secs(2);

/// Tope del dial pedido desde Ajustes ("Conectar").
const UI_DIAL: Duration = Duration::from_secs(10);

/// Intervalo del anuncio de grupo: cada miembro publica su id al swarm (flood
/// a todos) para que los que se unieron después lo descubran. El overlay no
/// repite mensajes viejos, por eso se re-anuncia.
const GROUP_ANNOUNCE: Duration = Duration::from_secs(10);

/// Configuración de la fase de arranque de la red de pares.
pub struct Join {
    /// Conectar los pares explícitos (`FEATHRAI_PEERS` + `peers.txt`).
    pub configured: bool,
    /// Descubrir pares de la red interna por mDNS (y seguir escuchando).
    pub lan: bool,
    /// Unirse al grupo por gossip y descubrir a los demás miembros (opción A:
    /// coordinador/rendezvous). Usa como bootstrap los pares ya conocidos.
    pub group: bool,
    /// Este nodo se identifica como **coordinador**: anuncia su rol al grupo y
    /// publica el servicio mDNS [`MDNS_COORDINATOR_SERVICE`]. Los demás nodos
    /// piden aceptarlo antes de sincronizar.
    pub coordinator: bool,
    /// Espera del descubrimiento LAN antes de abrir los stores.
    pub lan_wait: Duration,
}

impl Join {
    /// Configuración de la app: pares configurados + mDNS + grupo, con la
    /// espera de `FEATHRAI_LAN_WAIT_MS` (o [`DEFAULT_LAN_WAIT`]) y el rol de
    /// coordinador según [`COORDINATOR_ENV`].
    pub fn app() -> Self {
        Self {
            configured: true,
            lan: true,
            group: true,
            coordinator: is_coordinator(),
            lan_wait: env_lan_wait().unwrap_or(DEFAULT_LAN_WAIT),
        }
    }

    /// Sin red de pares: no lee entorno, no abre mDNS, no conecta nada.
    #[cfg(test)]
    pub const OFF: Self = Self {
        configured: false,
        lan: false,
        group: false,
        coordinator: false,
        lan_wait: Duration::ZERO,
    };
}

/// Espera de LAN pedida por entorno (milisegundos).
fn env_lan_wait() -> Option<Duration> {
    std::env::var(LAN_WAIT_ENV)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_millis)
}

/// `true` si este nodo se identifica como coordinador ([`COORDINATOR_ENV`]).
fn is_coordinator() -> bool {
    env_value(COORDINATOR_ENV).is_some()
}

/// Origen de un par (para Ajustes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// `FEATHRAI_PEERS`.
    Env,
    /// Guardado en `peers.txt` desde Ajustes.
    Saved,
    /// Visto por mDNS en la red interna en esta corrida.
    Lan,
    /// Descubierto por gossip en el grupo (opción A: vía el coordinador).
    Group,
}

/// Par de sincronización tal como lo muestra Ajustes.
#[derive(Clone, PartialEq, Debug)]
pub struct Peer {
    /// Id del endpoint (hex de 64, como lo muestra Ajustes en el otro nodo).
    pub id: String,
    pub source: Source,
    /// El dial de esta corrida llegó a establecer la conexión.
    pub connected: bool,
    /// Apareció después de abrir los stores: su espacio de datos se comparte
    /// en el próximo arranque (ver doc del módulo).
    pub late: bool,
}

/// Estado mutable de la red de pares (lo comparten la app y el watcher mDNS).
#[derive(Default)]
struct State {
    data_dir: PathBuf,
    /// De `FEATHRAI_PEERS` (no editables desde Ajustes).
    env_peers: BTreeSet<EndpointId>,
    /// De `peers.txt` (editables desde Ajustes).
    saved: BTreeSet<EndpointId>,
    /// Vistos por mDNS en esta corrida.
    lan: BTreeSet<EndpointId>,
    /// Vistos por gossip en el grupo en esta corrida (opción A).
    group: BTreeSet<EndpointId>,
    /// Coordinadores oídos (mDNS o gossip) que el usuario todavía no aceptó.
    pending_coordinators: BTreeSet<EndpointId>,
    /// Conectados por nosotros en esta corrida.
    dialed: BTreeSet<EndpointId>,
    /// Pares a los que ya se les intentó conectar (el pase final del arranque
    /// no re-dialea lo que el watcher mDNS ya intentó).
    attempted: BTreeSet<EndpointId>,
    /// Vistos/agregados después de abrir los stores.
    late: BTreeSet<EndpointId>,
    /// Ya se abrieron los stores (todo lo que llegue después es "late").
    started: bool,
}

impl State {
    fn source(&self, id: &EndpointId) -> Source {
        if self.env_peers.contains(id) {
            Source::Env
        } else if self.saved.contains(id) {
            Source::Saved
        } else if self.lan.contains(id) {
            Source::Lan
        } else {
            Source::Group
        }
    }

    /// Ids que muestra Ajustes: los configurados, los vistos por mDNS y los
    /// que se intentaron conectar en esta corrida (un par conectado desde
    /// Ajustes sin estar guardado igual aparece en la lista).
    fn ids(&self) -> BTreeSet<EndpointId> {
        self.env_peers
            .iter()
            .chain(&self.saved)
            .chain(&self.lan)
            .chain(&self.group)
            .chain(&self.attempted)
            .copied()
            .collect()
    }
}

/// Red de pares de esta corrida: estado observable por Ajustes + handles.
pub struct Peers {
    inner: Arc<Mutex<State>>,
    client: IrohClient,
    /// Mantiene vivo al discoverer mDNS (dropear la instancia aborta su tarea).
    /// `None` si mDNS no se pudo levantar (red sin multicast); el resto sigue.
    mdns: Option<MdnsAddressLookup>,
    /// Descubridor mDNS del servicio de coordinadores; `None` si no se pudo.
    mdns_coord: Option<MdnsAddressLookup>,
    /// Este nodo se identifica como coordinador (anuncia su rol).
    coordinator: bool,
    /// Suscripción al grupo por gossip (opción A). Guardarla mantiene viva la
    /// malla; `None` si el gossip no se pudo levantar.
    topic: Option<Arc<IrohTopic>>,
}

impl Peers {
    /// Conecta los pares configurados, arranca el descubrimiento mDNS y
    /// espera `join.lan_wait` para que los pares de la red interna queden
    /// conocidos. Llamar **antes** de abrir los stores.
    pub async fn start(client: &IrohClient, data_dir: &Path, join: Join) -> Self {
        let mut state = State {
            data_dir: data_dir.to_path_buf(),
            ..State::default()
        };
        if join.configured {
            let (ids, invalid) = parse_peer_list(&env_value(PEERS_ENV).unwrap_or_default());
            if !invalid.is_empty() {
                warn(&format!("{PEERS_ENV}: ids ignorados: {}", invalid.join(", ")));
            }
            state.env_peers = ids;
            state.saved = load_saved(data_dir);
        }
        let inner = Arc::new(Mutex::new(state));

        // Pares explícitos: un par que no responde cuesta `START_DIAL` y queda
        // sin registrar (ver [`dial`]), así que no demora la apertura.
        if join.configured {
            let ids: Vec<EndpointId> = {
                let st = lock(&inner);
                st.env_peers.iter().chain(&st.saved).copied().collect()
            };
            for id in ids {
                let _ = dial(&inner, &client.clone(), id, START_DIAL).await;
            }
        }

        // Descubrimiento LAN: el watcher conecta lo que aparezca (y sigue
        // escuchando después del arranque). El coordinador además publica su
        // propio servicio para que los demás lo detecten y pidan aceptarlo.
        let (mdns, mdns_coord) = if join.lan {
            start_mdns(&inner, client, join.coordinator).await
        } else {
            (None, None)
        };

        // Grupo por gossip (opción A): se une con los pares ya conocidos; un
        // listener dialea a los miembros que se anuncian (ahora y después). Se
        // une antes de la espera LAN para que la malla tenga ese tiempo.
        let topic = if join.group {
            start_group(&inner, client, join.coordinator).await
        } else {
            None
        };

        if join.lan && !join.lan_wait.is_zero() {
            tokio::time::sleep(join.lan_wait).await;
            // Pase final: los descubiertos en el último tramo de la espera
            // pueden no haber sido dialeados por el watcher todavía.
            let pending: Vec<EndpointId> = {
                let st = lock(&inner);
                st.lan.difference(&st.attempted).copied().collect()
            };
            for id in pending {
                let _ = dial(&inner, &client.clone(), id, START_DIAL).await;
            }
        }

        let peers = Self {
            inner: Arc::clone(&inner),
            client: client.clone(),
            mdns,
            mdns_coord,
            coordinator: join.coordinator,
            topic,
        };
        lock(&peers.inner).started = true;
        peers
    }

    /// Pares de esta corrida (para Ajustes).
    pub fn peers(&self) -> Vec<Peer> {
        let st = lock(&self.inner);
        let dialed = &st.dialed;
        st.ids()
            .into_iter()
            .map(|id| Peer {
                id: id.to_string(),
                source: st.source(&id),
                connected: dialed.contains(&id),
                late: st.late.contains(&id),
            })
            .collect()
    }

    /// Conecta un par ya guardado (botón "Conectar" de Ajustes).
    pub async fn connect(&self, id: EndpointId) -> Result<(), String> {
        let _ = dial(&self.inner, &self.client, id, UI_DIAL).await;
        let st = lock(&self.inner);
        if st.dialed.contains(&id) {
            Ok(())
        } else {
            Err(format!(
                "no se pudo conectar con {id}: ¿está la app abierta y en la misma red?"
            ))
        }
    }

    /// Guarda un par en `peers.txt` (idempotente) y lo conecta. Devuelve si la
    /// conexión se estableció en esta corrida (el par queda guardado igual).
    pub async fn save(&self, id: EndpointId) -> Result<bool, String> {
        {
            let mut st = lock(&self.inner);
            st.saved.insert(id);
            write_saved(&st).map_err(|e| format!("guardando {}: {e}", PEERS_FILE))?;
        }
        Ok(self.connect(id).await.is_ok())
    }

    /// Olvida un par guardado (no toca `FEATHRAI_PEERS`).
    pub fn forget(&self, id: EndpointId) -> Result<(), String> {
        let mut st = lock(&self.inner);
        st.saved.remove(&id);
        st.lan.remove(&id);
        st.attempted.remove(&id);
        write_saved(&st).map_err(|e| format!("guardando {}: {e}", PEERS_FILE))
    }

    /// Coordinadores oídos que el usuario todavía no aceptó (ids, para Ajustes).
    pub fn pending_coordinators(&self) -> Vec<String> {
        let st = lock(&self.inner);
        st.pending_coordinators.iter().map(|id| id.to_string()).collect()
    }

    /// Acepta un coordinador: lo guarda en `peers.txt`, lo conecta y deja de
    /// mostrarlo como pendiente. Devuelve si la conexión se estableció.
    pub async fn accept_coordinator(&self, id: EndpointId) -> Result<bool, String> {
        let conectado = self.save(id).await?;
        lock(&self.inner).pending_coordinators.remove(&id);
        Ok(conectado)
    }

    /// Ignora un coordinador pendiente (deja de preguntar por él en esta corrida).
    pub fn ignore_coordinator(&self, id: EndpointId) {
        lock(&self.inner).pending_coordinators.remove(&id);
    }

    /// Resumen para el log de arranque.
    pub fn summary(&self) -> String {
        let st = lock(&self.inner);
        let configured = st.env_peers.len() + st.saved.len();
        let connected = st.dialed.len();
        let lan = st.lan.len();
        let group = st.group.len();
        let pendientes = st.pending_coordinators.len();
        let mdns = if self.mdns.is_some() && self.mdns_coord.is_some() { "sí" } else { "no" };
        let grupo = if self.topic.is_some() { "sí" } else { "no" };
        let coord = if self.coordinator { "sí" } else { "no" };
        format!(
            "pares: {configured} configurados ({connected} con conexión), \
             {lan} por red interna, {group} por grupo, {pendientes} coordinadores pendientes; \
             mDNS: {mdns}, grupo: {grupo}, coordinador: {coord}"
        )
    }
}

/// `true` si el par quedó conectado (y por eso registrado como conocido).
///
/// Ver el doc del módulo: probar la conexión **antes** de registrarla evita
/// que `resolve_shared_ticket` espere por un par que no responde al abrir los
/// stores.
async fn dial(
    inner: &Arc<Mutex<State>>,
    client: &IrohClient,
    id: EndpointId,
    budget: Duration,
) -> bool {
    {
        let mut st = lock(inner);
        st.attempted.insert(id);
    }
    let ok = tokio::time::timeout(budget, connect_peer(client, id))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false);
    let mut st = lock(inner);
    if ok {
        st.dialed.insert(id);
    }
    if st.started {
        st.late.insert(id);
    }
    if !ok {
        warn(&format!(
            "sin conexión con el par {id}: no responde (no se sincroniza con él)"
        ));
    }
    ok
}

/// Conecta el par y, sólo si la conexión se estableció, lo deja registrado.
///
/// Es [`IrohClient::connect_gossip`] con el registro **después** del connect:
/// la sonda no registra nada si el par no responde, y con la conexión ya viva
/// `connect_gossip` la reutiliza (quinn reusa la conexión del mismo par y ALPN)
/// y se la entrega a gossip para que la mantenga abierta.
async fn connect_peer(client: &IrohClient, id: EndpointId) -> Result<(), String> {
    let endpoint = {
        let endpoint = client
            .backend()
            .get_endpoint()
            .await
            .map_err(|e| format!("sin endpoint: {e}"))?;
        let guard = endpoint.read().await;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "sin endpoint".to_string())?
    };
    let probe = endpoint
        .connect(id, iroh_gossip::ALPN)
        .await
        .map_err(|e| format!("conectando: {e}"))?;
    let registrado = client
        .connect_gossip(id)
        .await
        .map_err(|e| format!("registrando el par: {e}"));
    // Con la conexión en gossip, soltar la sonda no la cierra.
    drop(probe);
    registrado
}

/// Levanta el descubrimiento mDNS y deja dos watchers:
/// - [`MDNS_SERVICE`] (miembros): conecta cada nodo que aparezca (como antes);
/// - [`MDNS_COORDINATOR_SERVICE`] (coordinadores): los marca pendientes para que
///   el usuario decida en Ajustes, **sin** auto-conectarlos.
///
/// El coordinador publica sólo su propio servicio (`advertise` por rol), así
/// que un miembro no lo auto-conecta como si fuera un par común.
async fn start_mdns(
    inner: &Arc<Mutex<State>>,
    client: &IrohClient,
    coordinator: bool,
) -> (Option<MdnsAddressLookup>, Option<MdnsAddressLookup>) {
    let endpoint = match client.backend().get_endpoint().await {
        Ok(e) => {
            let guard = e.read().await;
            match guard.as_ref() {
                Some(ep) => ep.clone(),
                None => return (None, None),
            }
        }
        Err(e) => {
            warn(&format!("sin endpoint para mDNS: {e}"));
            return (None, None);
        }
    };

    // Miembros: los publica quien NO es coordinador; todos los escuchan.
    let mdns = mdns_lookup(&endpoint, MDNS_SERVICE, !coordinator);
    // Coordinadores: los publica el coordinador; todos los escuchan.
    let mdns_coord = mdns_lookup(&endpoint, MDNS_COORDINATOR_SERVICE, coordinator);

    let this = client.node_id();

    if let Some(mdns) = &mdns {
        let mut events = mdns.subscribe().await;
        let inner = Arc::clone(inner);
        let client = client.clone();
        tokio::spawn(async move {
            while let Some(event) = events.next().await {
                let DiscoveryEvent::Discovered { endpoint_info, .. } = event else {
                    continue;
                };
                let id = endpoint_info.endpoint_id;
                if id == this {
                    continue;
                }
                let first = {
                    let mut st = lock(&inner);
                    st.lan.insert(id)
                };
                if first {
                    let _ = dial(&inner, &client, id, UI_DIAL).await;
                }
            }
        });
    }

    if let Some(mdns_coord) = &mdns_coord {
        let mut events = mdns_coord.subscribe().await;
        let inner = Arc::clone(inner);
        tokio::spawn(async move {
            while let Some(event) = events.next().await {
                let DiscoveryEvent::Discovered { endpoint_info, .. } = event else {
                    continue;
                };
                let id = endpoint_info.endpoint_id;
                if id == this {
                    continue;
                }
                note_coordinator(&inner, id);
            }
        });
    }

    (mdns, mdns_coord)
}

/// Crea, registra y devuelve un lookup mDNS para `service`; publica sólo si
/// `advertise`. `None` si no se pudo levantar.
fn mdns_lookup(
    endpoint: &iroh::Endpoint,
    service: &str,
    advertise: bool,
) -> Option<MdnsAddressLookup> {
    let mdns = match MdnsAddressLookup::builder()
        .service_name(service)
        .advertise(advertise)
        .build(endpoint.id())
    {
        Ok(m) => m,
        Err(e) => {
            warn(&format!("mDNS ({service}) no disponible: {e}"));
            return None;
        }
    };
    // Registrar publica las direcciones locales actuales en el servicio.
    match endpoint.address_lookup() {
        Ok(services) => services.add(mdns.clone()),
        Err(e) => warn(&format!("mDNS ({service}) sin publicación: {e}")),
    }
    Some(mdns)
}

/// Únete al grupo de pares por gossip (opción A: coordinador/rendezvous).
///
/// Suscribirse a [`GROUP_TOPIC`] con los pares ya conocidos como bootstrap une
/// al nodo a la malla. El overlay (`list_peers`) sólo expone una vista parcial
/// (hyparview), así que la membresía completa se resuelve por **anuncio**: cada
/// nodo publica `member:<id>` o `coordinator:<id>` al swarm (broadcast a todos)
/// y un listener dialea a cada **miembro** (`connect_gossip`, que lo registra
/// como conocido) pero deja a los **coordinadores** como pendientes de aceptar.
///
/// Devuelve el handle del topic (`None` si el gossip no se pudo levantar);
/// guardarlo mantiene viva la suscripción.
async fn start_group(
    inner: &Arc<Mutex<State>>,
    client: &IrohClient,
    coordinator: bool,
) -> Option<Arc<IrohTopic>> {
    let pubsub = match client.backend().clone().create_pubsub_interface().await {
        Ok(ps) => Arc::new(ps),
        Err(e) => {
            warn(&format!("sin gossip para el grupo: {e}"));
            return None;
        }
    };

    // Bootstrap: los pares que ya conocemos (configurados + red interna). Un
    // coordinador queda como par configurado (`FEATHRAI_PEERS`/`peers.txt`).
    let bootstrap: Vec<EndpointId> = {
        let st = lock(inner);
        st.env_peers
            .iter()
            .chain(&st.saved)
            .chain(&st.lan)
            .copied()
            .collect()
    };

    let topic = match pubsub
        .get_or_create_topic_with_peers(GROUP_TOPIC, bootstrap)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            warn(&format!("no se pudo unir al grupo: {e}"));
            return None;
        }
    };

    let my_id = client.node_id().to_string();
    // El anuncio lleva el rol: `coordinator:<id>` o `member:<id>`.
    let role = if coordinator { "coordinator" } else { "member" };
    let anuncio = format!("{role}:{my_id}");

    // Anuncio propio: `publish_to_topic` hace broadcast al swarm (flood a todos
    // los miembros). Se re-anuncia cada [`GROUP_ANNOUNCE`] para que los miembros
    // que se unan después nos descubran (el overlay no repite mensajes viejos).
    // Se usa `publish_to_topic` (no `IrohTopic::publish`) para esquivar el
    // choque con el método inherente `publish(&[u8], &Gossip)`.
    let pubsub_a = Arc::clone(&pubsub);
    tokio::spawn(async move {
        loop {
            if let Err(e) = pubsub_a.publish_to_topic(GROUP_TOPIC, anuncio.as_bytes()).await {
                warn(&format!("grupo: no se pudo anunciar: {e}"));
            }
            tokio::time::sleep(GROUP_ANNOUNCE).await;
        }
    });

    // Escucha de anuncios: cada id recibido se dialea, lo que lo registra como
    // conocido (`connect_gossip`) y habilita la replicación GuardianDB.
    let inner_b = Arc::clone(inner);
    let client_b = client.clone();
    let topic_b = topic.clone();
    tokio::spawn(async move {
        let Ok(mut mensajes) = topic_b.watch_messages().await else {
            warn("grupo: sin stream de anuncios");
            return;
        };
        while let Some(msg) = mensajes.next().await {
            let Ok(texto) = std::str::from_utf8(&msg.content) else {
                continue;
            };
            let Some((es_coord, id)) = parse_announce(texto) else {
                continue;
            };
            if es_coord {
                // Coordinador: si ya está aceptado se conecta; si no, queda
                // pendiente para que el usuario decida.
                if is_accepted(&inner_b, &id) {
                    dial_group_member(&inner_b, &client_b, id).await;
                } else {
                    note_coordinator(&inner_b, id);
                }
            } else {
                dial_group_member(&inner_b, &client_b, id).await;
            }
        }
    });

    Some(topic)
}

/// Dialea un miembro del grupo visto por gossip, si es nuevo y no es este nodo.
async fn dial_group_member(inner: &Arc<Mutex<State>>, client: &IrohClient, id: EndpointId) {
    if id == client.node_id() {
        return;
    }
    let nuevo = {
        let mut st = lock(inner);
        st.group.insert(id);
        !st.attempted.contains(&id)
    };
    if nuevo {
        let _ = dial(inner, client, id, START_DIAL).await;
    }
}

/// Parsea un anuncio del grupo: `"coordinator:<id>"` o `"member:<id>"`.
/// Devuelve `(es_coordinador, id)`.
fn parse_announce(texto: &str) -> Option<(bool, EndpointId)> {
    let (role, id) = texto.split_once(':')?;
    let id = parse_peer(id)?;
    Some((role == "coordinator", id))
}

/// `true` si `id` ya es un par aceptado (configurado por entorno o guardado).
fn is_accepted(inner: &Arc<Mutex<State>>, id: &EndpointId) -> bool {
    let st = lock(inner);
    st.env_peers.contains(id) || st.saved.contains(id)
}

/// Marca a `id` como coordinador pendiente, salvo que ya esté aceptado.
fn note_coordinator(inner: &Arc<Mutex<State>>, id: EndpointId) {
    if !is_accepted(inner, &id) {
        lock(inner).pending_coordinators.insert(id);
    }
}

/// Parsea un id de endpoint (`hex` de 64 o base32, como lo muestra Ajustes).
pub fn parse_peer(raw: &str) -> Option<EndpointId> {
    raw.trim().parse().ok()
}

/// Parsea una lista de ids separados por coma, punto y coma o espacios.
/// Devuelve los válidos y, aparte, los textos que no parsearon.
pub fn parse_peer_list(raw: &str) -> (BTreeSet<EndpointId>, Vec<String>) {
    let mut ids = BTreeSet::new();
    let mut invalid = Vec::new();
    for part in raw.split([',', ';', '\n', ' ', '\t']).filter(|p| !p.trim().is_empty()) {
        match parse_peer(part) {
            Some(id) => {
                ids.insert(id);
            }
            None => invalid.push(part.trim().to_string()),
        }
    }
    (ids, invalid)
}

/// Pares guardados en `peers.txt` (una línea por id, `#` comenta).
fn load_saved(data_dir: &Path) -> BTreeSet<EndpointId> {
    let Ok(text) = std::fs::read_to_string(data_dir.join(PEERS_FILE)) else {
        return BTreeSet::new();
    };
    let mut ids = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_peer(line) {
            Some(id) => {
                ids.insert(id);
            }
            None => warn(&format!("{PEERS_FILE}: id ignorado: {line}")),
        }
    }
    ids
}

/// Reescribe `peers.txt` (ids ordenados; cabecera con el formato).
fn write_saved(state: &State) -> std::io::Result<()> {
    let mut text = String::from(
        "# Pares de sincronización feathrai (uno por línea, como los muestra Ajustes).\n\
         # Se conectan en cada arranque; la app los edita sola desde Ajustes.\n",
    );
    for id in &state.saved {
        text.push_str(&id.to_string());
        text.push('\n');
    }
    std::fs::write(state.data_dir.join(PEERS_FILE), text)
}

/// Valor de entorno no vacío.
fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
}

/// Log de la app (stderr: `dx serve`/`cargo run` lo muestran; el archivo del
/// data dir lo escribe `main` con el resumen de arranque).
fn warn(msg: &str) {
    eprintln!("[featherai] red: {msg}");
}

/// Lock del estado, sin propagar el veneno: el estado es un caché de ids.
fn lock(inner: &Arc<Mutex<State>>) -> std::sync::MutexGuard<'_, State> {
    inner.lock().unwrap_or_else(|e| e.into_inner())
}
