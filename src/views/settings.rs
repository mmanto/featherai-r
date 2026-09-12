use crate::components::layout::{use_auth, AppLayout};
use crate::components::protected_route::ProtectedRoute;
use dioxus::prelude::*;

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
                            p { class: "mb-1 text-xs text-[var(--text-secondary)]", "Esta instalación guarda una réplica local completa: cada escritura es local y se sincroniza peer-to-peer con otros nodos (Iroh). Compartí el id del nodo para conectarlos." }
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
            }
        }
    }
}
