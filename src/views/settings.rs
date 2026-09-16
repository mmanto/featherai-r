use crate::components::layout::{use_auth, AppLayout};
use crate::components::protected_route::ProtectedRoute;
use dioxus::prelude::*;

/// Id abreviado para la lista de pares (el completo va en `title`).
#[cfg(not(target_arch = "wasm32"))]
fn peer_short(id: &str) -> String {
    if id.len() <= 18 {
        id.to_string()
    } else {
        format!("{}…{}", &id[..8], &id[id.len() - 6..])
    }
}

/// Origen del par, para la lista.
#[cfg(not(target_arch = "wasm32"))]
fn peer_source(source: crate::net::Source) -> &'static str {
    match source {
        crate::net::Source::Env => "FEATHRAI_PEERS",
        crate::net::Source::Saved => "guardado",
        crate::net::Source::Lan => "red interna",
    }
}

/// Estado del par, para la lista.
#[cfg(not(target_arch = "wasm32"))]
fn peer_state(peer: &crate::net::Peer) -> &'static str {
    match (peer.connected, peer.late) {
        (true, false) => "conectado",
        (true, true) => "conectado · se une a este espacio en el próximo arranque",
        (false, _) => "sin conexión",
    }
}

/// Card de nodos pares (solo nativo): alta, conexión y estado.
///
/// El descubrimiento mDNS sigue en segundo plano, así que la lista se
/// refresca cada 3 s en vez de sólo al montar. El estado real (ids, conexión,
/// origen) vive en `persistence::Db::peers`.
#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn PeersCard() -> Element {
    use crate::components::toast::{show_toast, use_toast, ToastVariant};
    use crate::net::Source;

    let mut input = use_signal(String::new);
    let mut lista = use_signal(|| crate::persistence::db().peers());
    let toasts = use_toast();

    use_effect(move || {
        spawn(async move {
            loop {
                let actuales = crate::persistence::db().peers();
                if lista.peek().as_slice() != actuales.as_slice() {
                    lista.set(actuales);
                }
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        });
    });

    // Alta + conexión del id pegado (guardado en `peers.txt` del data dir).
    let conectar_nuevo = move |_| {
        let id = input().trim().to_string();
        if id.is_empty() {
            return;
        }
        spawn(async move {
            match crate::persistence::db().add_peer(&id).await {
                Ok((canonico, true)) => {
                    show_toast(
                        toasts,
                        format!("Conectado con {}", peer_short(&canonico)),
                        ToastVariant::Success,
                    );
                    input.set(String::new());
                }
                Ok((canonico, false)) => {
                    show_toast(
                        toasts,
                        format!(
                            "{} guardado, sin respuesta todavía (reintenta al arrancar)",
                            peer_short(&canonico)
                        ),
                        ToastVariant::Info,
                    );
                    input.set(String::new());
                }
                Err(e) => show_toast(
                    toasts,
                    format!("No se pudo agregar el par: {e}"),
                    ToastVariant::Error,
                ),
            }
            lista.set(crate::persistence::db().peers());
        });
    };

    rsx! {
        div { class: "rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)] mt-4",
            div { class: "p-4",
                h5 { class: "font-medium text-[var(--text-primary)] mb-1", "Nodos pares" }
                p { class: "mb-3 text-xs text-[var(--text-secondary)]",
                    "Los nodos de la misma red interna se descubren solos (mDNS). Para un nodo en otra red, pegá el id que muestra Ajustes en ese equipo: la sincronización va en los dos sentidos."
                }
                div { class: "grid grid-cols-12 gap-2 mb-3",
                    div { class: "col-span-12 sm:col-span-9",
                        input {
                            class: "w-full rounded border border-[var(--card-border)] bg-[var(--card-bg)] px-3 py-2 text-sm text-[var(--text-primary)]",
                            style: "font-size:0.8rem",
                            placeholder: "id del otro nodo (64 hex)",
                            value: "{input}",
                            oninput: move |e| input.set(e.value()),
                        }
                    }
                    div { class: "col-span-12 sm:col-span-3",
                        button {
                            class: "w-full rounded font-medium text-white bg-[var(--primary-color)] hover:opacity-90 px-2.5 py-2 text-sm",
                            onclick: conectar_nuevo,
                            "Conectar"
                        }
                    }
                }
                if lista().is_empty() {
                    p { class: "mb-0 text-xs text-[var(--text-secondary)]",
                        "Sin pares: esta instalación trabaja sola hasta que aparezca otro nodo."
                    }
                } else {
                    for p in lista() {
                        div {
                            key: "{p.id}",
                            class: "flex flex-wrap items-center justify-between gap-2 border-t border-[var(--card-border)] py-2",
                            div { class: "min-w-0",
                                p {
                                    class: "mb-0 break-all text-[var(--text-primary)]",
                                    style: "font-size:0.8rem",
                                    title: "{p.id}",
                                    "{peer_short(&p.id)}"
                                }
                                p { class: "mb-0 text-xs text-[var(--text-secondary)]",
                                    "{peer_source(p.source)} · {peer_state(&p)}"
                                }
                            }
                            div { class: "flex gap-2",
                                if !p.connected {
                                    button {
                                        class: "inline-flex items-center justify-center rounded border border-[var(--primary-color)] text-[var(--primary-color)] hover:bg-[var(--primary-color)] hover:text-[var(--text-inverse)] px-2.5 py-1 text-sm",
                                        onclick: {
                                            let id = p.id.clone();
                                            move |_| {
                                                let id = id.clone();
                                                spawn(async move {
                                                    match crate::persistence::db().connect_peer(&id).await {
                                                        Ok(canonico) => show_toast(
                                                            toasts,
                                                            format!("Conectado con {}", peer_short(&canonico)),
                                                            ToastVariant::Success,
                                                        ),
                                                        Err(e) => show_toast(
                                                            toasts,
                                                            format!("Sin conexión: {e}"),
                                                            ToastVariant::Error,
                                                        ),
                                                    }
                                                    lista.set(crate::persistence::db().peers());
                                                });
                                            }
                                        },
                                        "Conectar"
                                    }
                                }
                                if p.source == Source::Saved {
                                    button {
                                        class: "inline-flex items-center justify-center rounded border border-[var(--card-border)] text-[var(--text-secondary)] hover:bg-[var(--table-hover)] hover:text-[var(--text-primary)] px-2.5 py-1 text-sm",
                                        onclick: {
                                            let id = p.id.clone();
                                            move |_| {
                                                match crate::persistence::db().forget_peer(&id) {
                                                    Ok(canonico) => show_toast(
                                                        toasts,
                                                        format!("{} olvidado", peer_short(&canonico)),
                                                        ToastVariant::Info,
                                                    ),
                                                    Err(e) => show_toast(
                                                        toasts,
                                                        format!("No se pudo olvidar el par: {e}"),
                                                        ToastVariant::Error,
                                                    ),
                                                }
                                                lista.set(crate::persistence::db().peers());
                                            }
                                        },
                                        "Olvidar"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// En web/wasm no hay persistencia ni red de pares: la card no se muestra.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn PeersCard() -> Element {
    rsx! {}
}


/// Ajustes de cuenta — equivalente de `pages/Settings.tsx` (`/settings`),
/// protegida para cualquier usuario autenticado.
#[component]
pub fn Settings() -> Element {
    let auth = use_auth();
    let user = auth.read().user.clone();

    let role_label = match user.as_ref().map(|u| u.role.as_str()) {
        Some("super_admin") => "Administración general",
        Some("admin") => "Administrador",
        Some("operativo") => "Operativo",
        _ => "",
    };
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();
    let email = user
        .as_ref()
        .and_then(|u| u.email.clone())
        .unwrap_or_else(|| "(No configurado)".into());

    // Datos de sincronización GuardianDB (solo nativo): id del nodo iroh y
    // directorio de datos local-first de esta instalación.
    #[cfg(not(target_arch = "wasm32"))]
    let sync_info = Some((
        crate::persistence::db().node_id().to_string(),
        crate::persistence::db().data_dir().display().to_string(),
    ));
    #[cfg(target_arch = "wasm32")]
    let sync_info: Option<(String, String)> = None;

    rsx! {
        ProtectedRoute {
            AppLayout {
                div { class: "grid grid-cols-12 gap-4 mb-4",
                    div { class: "col-span-12",
                        h1 { class: "text-[1.75rem] font-medium leading-[1.2] mb-1", style: "font-weight:500", "Ajustes" }
                        p { class: "text-[var(--text-secondary)] mb-0", "Información de tu cuenta" }
                    }
                }

                div { class: "rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)]",
                    div { class: "p-4",
                        h5 { class: "font-medium text-[var(--text-primary)] mb-4", "Mi cuenta" }
                        div { class: "grid grid-cols-12 gap-4",
                            div { class: "col-span-12 sm:col-span-4",
                                p { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Usuario" }
                                p { class: "mb-0", "{username}" }
                            }
                            div { class: "col-span-12 sm:col-span-4",
                                p { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Email" }
                                p { class: "mb-0", "{email}" }
                            }
                            div { class: "col-span-12 sm:col-span-4",
                                p { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Rol" }
                                p { class: "mb-0", "{role_label}" }
                            }
                        }
                    }
                }

                if let Some((node_id, data_dir)) = sync_info {
                    div { class: "rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-sm)] mt-4",
                        div { class: "p-4",
                            h5 { class: "font-medium text-[var(--text-primary)] mb-4", "Sincronización (GuardianDB)" }
                            p { class: "mb-1 text-xs text-[var(--text-secondary)]", "Esta instalación guarda una réplica local completa: cada escritura es local y se sincroniza peer-to-peer con otros nodos (Iroh). Los nodos de la misma red interna se encuentran solos; para otra red, agregá abajo el id del otro nodo." }
                            div { class: "grid grid-cols-12 gap-4",
                                div { class: "col-span-12 sm:col-span-8",
                                    p { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Id del nodo" }
                                    p { class: "mb-0 break-all text-[var(--text-primary)]", style: "font-size:0.8rem", "{node_id}" }
                                }
                                div { class: "col-span-12 sm:col-span-4",
                                    p { class: "mb-1 block text-xs font-medium tracking-wide text-[var(--text-secondary)]", "Datos locales" }
                                    p { class: "mb-0 break-all text-[var(--text-primary)]", style: "font-size:0.8rem", "{data_dir}" }
                                }
                            }
                        }
                    }
                }

                PeersCard {}
            }
        }
    }
}
