//! Pantalla de bienvenida mientras arranca la app.
//!
//! La base GuardianDB y el nodo se abren en un hilo aparte (ver `App` en
//! `main.rs`): mientras eso pasa, la ventana ya está creada y muestra este
//! splash con el logo y una barra de progreso. Cuando el backend queda listo,
//! `App` cambia al router normal.
//!
//! Los estilos críticos van inline: si dependieran de `main.css`, el primer
//! paint mostraría el logo a tamaño natural y desacomodado hasta que el
//! stylesheet cargue. La tarjeta además aparece recién cuando el logo terminó
//! de decodificarse (`onload`), así no se ve saltar de tamaño/posición.

use dioxus::prelude::*;

/// Logo con fondo transparente (el ícono de la app conserva su fondo).
const LOGO: Asset = asset!("/assets/icons/logo.png");

/// Pantalla de bienvenida: logo, nombre y barra de progreso.
///
/// `progress` va de 0 a 100. El componente no conoce el backend; sólo pinta lo
/// que recibe, así que también sirve en la build web (donde el arranque es
/// instantáneo y el splash casi no se ve).
#[component]
pub fn SplashScreen(progress: u8) -> Element {
    let p = progress.min(100);
    // `onload` del <img> confirma que el logo ya está listo. Hasta entonces la
    // tarjeta queda transparente (el fondo del splash sí se ve).
    let mut logo_loaded = use_signal(|| false);
    let card_opacity = if logo_loaded() { "1" } else { "0" };

    rsx! {
        div {
            style: "position:fixed; inset:0; display:flex; align-items:center; justify-content:center; background-color:var(--bg-primary,#fff); color:var(--text-primary,#212529); z-index:9999;",
            div {
                style: "display:flex; flex-direction:column; align-items:center; gap:0.75rem; width:min(28rem, calc(100% - 2rem)); padding:2rem; text-align:center; opacity:{card_opacity}; transition:opacity 180ms ease;",
                img {
                    src: LOGO,
                    alt: "featherai",
                    // Atributos + estilo reservan el espacio antes de que el
                    // PNG se decodifique: sin esto el layout salta.
                    width: "112",
                    height: "112",
                    style: "width:7rem; height:7rem; object-fit:contain; filter:drop-shadow(0 8px 24px rgba(0,0,0,0.18));",
                    onload: move |_| logo_loaded.set(true),
                    // Si el logo faltara, no dejamos la pantalla en blanco.
                    onerror: move |_| logo_loaded.set(true),
                }
                h1 {
                    style: "margin:0; font-size:2rem; font-weight:600; letter-spacing:-0.02em;",
                    "featherai"
                }
                p {
                    style: "margin:0; color:var(--text-secondary,#6c757d); font-size:0.95rem;",
                    "Cargando la base y la vista…"
                }
                div {
                    style: "width:100%; height:0.5rem; margin-top:0.5rem; border-radius:999px; background-color:var(--bg-tertiary,#e9ecef); overflow:hidden;",
                    role: "progressbar",
                    "aria-valuemin": "0",
                    "aria-valuemax": "100",
                    "aria-valuenow": "{p}",
                    div {
                        style: "width:{p}%; height:100%; border-radius:999px; background-color:var(--primary-color,#00a3f0); transition:width 0.25s ease;",
                    }
                }
                span {
                    style: "font-size:0.8rem; color:var(--text-secondary,#6c757d); font-variant-numeric:tabular-nums;",
                    "{p}%"
                }
            }
        }
    }
}

/// Pantalla de cierre: se muestra desde que el usuario elige “Salir” hasta que
/// el proceso termina (flush de la base y devolución del data dir al nodo).
///
/// La barra es indeterminada: el cierre no tiene un progreso confiable, pero
/// sí puede tardar varios segundos (shutdown de Iroh).
#[component]
pub fn ShutdownScreen() -> Element {
    rsx! {
        div {
            style: "position:fixed; inset:0; display:flex; align-items:center; justify-content:center; background-color:var(--bg-primary,#fff); color:var(--text-primary,#212529); z-index:9999;",
            div {
                style: "display:flex; flex-direction:column; align-items:center; gap:0.75rem; width:min(28rem, calc(100% - 2rem)); padding:2rem; text-align:center;",
                img {
                    src: LOGO,
                    alt: "featherai",
                    width: "112",
                    height: "112",
                    style: "width:7rem; height:7rem; object-fit:contain; filter:drop-shadow(0 8px 24px rgba(0,0,0,0.18));",
                }
                h1 {
                    style: "margin:0; font-size:2rem; font-weight:600; letter-spacing:-0.02em;",
                    "featherai"
                }
                p {
                    style: "margin:0; color:var(--text-secondary,#6c757d); font-size:0.95rem;",
                    "Cerrando la aplicación…"
                }
                div {
                    style: "width:100%; height:0.5rem; margin-top:0.5rem; border-radius:999px; background-color:var(--bg-tertiary,#e9ecef); overflow:hidden;",
                    div {
                        class: "featherai-shutdown-bar",
                        style: "width:38%; height:100%; border-radius:999px; background-color:var(--primary-color,#00a3f0);",
                    }
                }
                span {
                    style: "font-size:0.8rem; color:var(--text-secondary,#6c757d);",
                    "Guardando datos y liberando la base"
                }
            }
        }
    }
}
