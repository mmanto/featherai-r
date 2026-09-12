use crate::components::layout::use_auth;
use dioxus::prelude::*;

/// Página de login — port de `LoginForm.tsx` de feathrai-frontend.
///
/// Tarjeta centrada con estilo inline idéntico al de React (`BRAND_COLOR`,
/// campos con foco azul vía `.feather-input:focus`, botón "INICIAR SESIÓN").
///
/// Nativo: valida credenciales contra los usuarios persistidos en
/// GuardianDB (`services::auth::login`; la siembra crea `admin`/`admin`).
/// Web/wasm: demo sin backend — cualquier credencial no vacía entra como
/// `super_admin`.
#[component]
pub fn Login() -> Element {
    let mut auth = use_auth();
    let navigator = use_navigator();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut form_error = use_signal(|| Option::<String>::None);

    // ── Estilos (idénticos a `styles` de LoginForm.tsx) ──
    let input_style = "width:100%;padding:11px 14px;font-size:0.9rem;color:#1a1a1a;background-color:#FAFAF9;border:1px solid #E0DDD8;border-radius:4px;outline:none;transition:border-color 0.15s ease;box-sizing:border-box;font-family:'DM Sans',sans-serif;";

    rsx! {
        div {
            style: "position:fixed;inset:0;background-color:#F7F6F3;display:flex;align-items:center;justify-content:center;font-family:'DM Sans',sans-serif;overflow-y:auto;padding:24px;",
            div {
                style: "background-color:#ffffff;border-radius:8px;box-shadow:0 1px 3px rgba(0,0,0,0.06), 0 4px 16px rgba(0,0,0,0.06);padding:48px;width:100%;max-width:400px;",
                h1 { style: "font-family:'Cormorant Garamond',serif;font-weight:300;font-size:2.25rem;letter-spacing:0.02em;color:#1a1a1a;text-align:center;margin-bottom:8px;",
                    "featherpro"
                }
                p { style: "font-size:0.75rem;color:#9b9b9b;text-align:center;letter-spacing:0.12em;text-transform:uppercase;margin-bottom:40px;",
                    "Inicia sesión para continuar"
                }
                div { style: "height:1px;background-color:#EDE9E3;margin-bottom:32px;" }

                if let Some(err) = form_error() {
                    div { style: "background-color:#FEF2F2;border:1px solid #FECACA;color:#B91C1C;padding:10px 14px;border-radius:4px;font-size:0.8rem;line-height:1.5;margin-bottom:20px;",
                        "{err}"
                    }
                }

                form {
                    onsubmit: move |event| {
                        event.prevent_default();
                        if username().trim().is_empty() || password().is_empty() {
                            *form_error.write() = Some("Completá usuario y contraseña".to_string());
                            return;
                        }
                        *form_error.write() = None;
                        let user_input = username().trim().to_string();
                        let pass = password().to_string();
                        spawn(async move {
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                match crate::services::auth::login(
                                    crate::persistence::db(),
                                    &user_input,
                                    &pass,
                                )
                                .await
                                {
                                    Ok(Some(user)) => {
                                        auth.write().is_authenticated = true;
                                        auth.write().user = Some(user);
                                        navigator.push("/admin/projects");
                                    }
                                    Ok(None) => {
                                        *form_error.write() =
                                            Some("Usuario o contraseña inválidos".to_string());
                                    }
                                    Err(e) => {
                                        *form_error.write() =
                                            Some(format!("Error de almacenamiento: {e}"));
                                    }
                                }
                            }
                            #[cfg(target_arch = "wasm32")]
                            {
                                // Demo web: cualquier credencial entra como super_admin.
                                let _ = &pass;
                                auth.write().is_authenticated = true;
                                auth.write().user = Some(crate::models::User {
                                    username: user_input,
                                    nombre: Some("Admin".to_string()),
                                    role: "super_admin".to_string(),
                                    ..Default::default()
                                });
                                navigator.push("/admin/projects");
                            }
                        });
                    },
                    style: "display:flex;flex-direction:column;gap:20px;",

                    div { style: "display:flex;flex-direction:column;gap:6px;",
                        label { style: "font-size:0.65rem;font-weight:500;letter-spacing:0.1em;text-transform:uppercase;color:#9b9b9b;", "Usuario" }
                        input {
                            class: "feather-input",
                            style: input_style,
                            r#type: "text",
                            value: username(),
                            oninput: move |e| username.set(e.value()),
                            autocapitalize: "none",
                            spellcheck: "false",
                        }
                    }

                    div { style: "display:flex;flex-direction:column;gap:6px;",
                        label { style: "font-size:0.65rem;font-weight:500;letter-spacing:0.1em;text-transform:uppercase;color:#9b9b9b;", "Contraseña" }
                        input {
                            class: "feather-input",
                            style: input_style,
                            r#type: "password",
                            value: password(),
                            oninput: move |e| password.set(e.value()),
                        }
                    }

                    button {
                        r#type: "submit",
                        style: "width:100%;padding:13px;background-color:#00A3F0;color:#ffffff;border:none;border-radius:4px;font-size:0.8rem;font-weight:500;letter-spacing:0.08em;text-transform:uppercase;cursor:pointer;font-family:'DM Sans',sans-serif;margin-top:8px;transition:opacity 0.15s ease;",
                        "Iniciar Sesión"
                    }
                }
            }
        }
    }
}
