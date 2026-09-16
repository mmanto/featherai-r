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
//! # Descubrimiento en la red interna
//! La instancia mDNS que arma guardian-db adentro usa el servicio `irohv1` y
//! no expone su stream, así que acá se crea una propia con servicio
//! [`MDNS_SERVICE`]: sólo los nodos feathrai la publican y la escuchan. Al
//! registrarla en el endpoint (`address_lookup().add`) Iroh publica las
//! direcciones locales actuales, y `subscribe()` enumera los nodos feathrai
//! de la red. No hace falta internet ni relay: las direcciones directas de la
//! LAN alcanzan (`conn_type` lo reporta como "direct").
//!
//! # Configuración
//! - `FEATHRAI_PEERS=id,id,…` — pares fijos (no editables desde Ajustes).
//! - `<data dir>/peers.txt` — pares guardados desde Ajustes, uno por línea
//!   (`#` comenta). Se conectan en cada arranque.
//! - `FEATHRAI_LAN_WAIT_MS=ms` — espera del descubrimiento LAN antes de abrir
//!   la base (por defecto [`DEFAULT_LAN_WAIT`]; `0` no espera y deja sólo el
//!   watcher).

use futures_lite::StreamExt;
use guardian_db::p2p::network::client::IrohClient;
use iroh::EndpointId;
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Servicio mDNS de los nodos feathrai (el de guardian-db adentro es `irohv1`).
pub const MDNS_SERVICE: &str = "featherai";

/// Pares fijos a conectar en cada arranque: `id,id,…` (hex de 64 o base32).
pub const PEERS_ENV: &str = "FEATHRAI_PEERS";

/// Milisegundos de espera del descubrimiento LAN antes de abrir los stores.
pub const LAN_WAIT_ENV: &str = "FEATHRAI_LAN_WAIT_MS";

/// Pares guardados desde Ajustes (una línea por id; `#` comenta).
pub const PEERS_FILE: &str = "peers.txt";

/// Espera por defecto: cubre un anuncio mDNS del otro nodo (los anuncios se
/// repiten cada pocos segundos).
pub const DEFAULT_LAN_WAIT: Duration = Duration::from_millis(2500);

/// Tope del dial al arrancar: la conexión no hace falta para el intercambio
/// de tickets (basta con que el par quede registrado como conocido, lo que
/// `connect_gossip` hace *antes* de dialear), así que un par apagado no puede
/// demorar la apertura de la base.
const START_DIAL: Duration = Duration::from_secs(2);

/// Tope del dial pedido desde Ajustes ("Conectar").
const UI_DIAL: Duration = Duration::from_secs(10);

/// Configuración de la fase de arranque de la red de pares.
pub struct Join {
    /// Conectar los pares explícitos (`FEATHRAI_PEERS` + `peers.txt`).
    pub configured: bool,
    /// Descubrir pares de la red interna por mDNS (y seguir escuchando).
    pub lan: bool,
    /// Espera del descubrimiento LAN antes de abrir los stores.
    pub lan_wait: Duration,
}

impl Join {
    /// Configuración de la app: pares configurados + mDNS, con la espera de
    /// `FEATHRAI_LAN_WAIT_MS` (o [`DEFAULT_LAN_WAIT`]).
    pub fn app() -> Self {
        Self {
            configured: true,
            lan: true,
            lan_wait: env_lan_wait().unwrap_or(DEFAULT_LAN_WAIT),
        }
    }

    /// Sin red de pares: no lee entorno, no abre mDNS, no conecta nada.
    #[cfg(test)]
    pub const OFF: Self = Self {
        configured: false,
        lan: false,
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

/// Origen de un par (para Ajustes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// `FEATHRAI_PEERS`.
    Env,
    /// Guardado en `peers.txt` desde Ajustes.
    Saved,
    /// Visto por mDNS en la red interna en esta corrida.
    Lan,
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
    /// Conectados por nosotros en esta corrida.
    dialed: BTreeSet<EndpointId>,
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
        } else {
            Source::Lan
        }
    }

    fn ids(&self) -> BTreeSet<EndpointId> {
        self.env_peers
            .iter()
            .chain(&self.saved)
            .chain(&self.lan)
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

        // Pares explícitos: el dial puede tardar (par apagado), pero lo que
        // importa —`note_known_peer`— ya pasó cuando vuelve.
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
        // escuchando después del arranque).
        let mdns = if join.lan {
            start_mdns(&inner, client).await
        } else {
            None
        };

        if join.lan && !join.lan_wait.is_zero() {
            tokio::time::sleep(join.lan_wait).await;
            // Pase final: los descubiertos en el último tramo de la espera
            // pueden no haber sido dialeados por el watcher todavía.
            let pending: Vec<EndpointId> = {
                let st = lock(&inner);
                st.lan.difference(&st.dialed).copied().collect()
            };
            for id in pending {
                let _ = dial(&inner, &client.clone(), id, START_DIAL).await;
            }
        }

        let peers = Self {
            inner: Arc::clone(&inner),
            client: client.clone(),
            mdns,
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
        write_saved(&st).map_err(|e| format!("guardando {}: {e}", PEERS_FILE))
    }

    /// Resumen para el log de arranque.
    pub fn summary(&self) -> String {
        let st = lock(&self.inner);
        let configured = st.env_peers.len() + st.saved.len();
        let connected = st.dialed.len();
        let lan = st.lan.len();
        let mdns = if self.mdns.is_some() { "sí" } else { "no" };
        format!(
            "pares: {configured} configurados ({connected} con conexión), \
             {lan} por red interna; mDNS: {mdns}"
        )
    }
}

/// `true` si el par quedó registrado como conocido (aunque el dial no llegue
/// a completarse: `connect_gossip` registra antes de dialear).
async fn dial(
    inner: &Arc<Mutex<State>>,
    client: &IrohClient,
    id: EndpointId,
    budget: Duration,
) -> bool {
    let ok = tokio::time::timeout(budget, client.connect_gossip(id))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false);
    let mut st = lock(inner);
    st.lan.insert(id);
    if ok {
        st.dialed.insert(id);
    }
    if st.started {
        st.late.insert(id);
    }
    if !ok {
        warn(&format!("sin conexión con el par {id}"));
    }
    ok
}

/// Levanta el descubrimiento mDNS propio y deja un watcher que conecta cada
/// nodo feathrai que aparezca en la red interna (ahora y después).
async fn start_mdns(inner: &Arc<Mutex<State>>, client: &IrohClient) -> Option<MdnsAddressLookup> {
    let endpoint = match client.backend().get_endpoint().await {
        Ok(e) => {
            let guard = e.read().await;
            match guard.as_ref() {
                Some(ep) => ep.clone(),
                None => return None,
            }
        }
        Err(e) => {
            warn(&format!("sin endpoint para mDNS: {e}"));
            return None;
        }
    };

    let mdns = match MdnsAddressLookup::builder()
        .service_name(MDNS_SERVICE)
        .build(endpoint.id())
    {
        Ok(m) => m,
        Err(e) => {
            warn(&format!("mDNS no disponible: {e}"));
            return None;
        }
    };

    // Registrar publica las direcciones locales actuales en el servicio.
    match endpoint.address_lookup() {
        Ok(services) => services.add(mdns.clone()),
        Err(e) => warn(&format!("mDNS sin publicación: {e}")),
    }

    let mut events = mdns.subscribe().await;
    let this = client.node_id();
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

    Some(mdns)
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
