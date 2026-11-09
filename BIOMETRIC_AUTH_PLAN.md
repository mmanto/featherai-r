# Plan: autenticación biométrica nativa del SO en featherai

## Objetivo
Añadir desbloqueo por biometría del sistema operativo (Linux `fprintd`, Windows Hello) al login nativo de featherai. Sin `devbout-oauth`, sin servidor, sin red. Local-first.

## Estado actual (evidencia)
- `src/services/auth.rs`: `login` contra GuardianDB; contraseña con Argon2; sesión en memoria.
- `src/views/login.rs`: el login nativo llama `services::auth::login`; la rama wasm es demo.
- `src/components/layout/state.rs`: `AuthState` con `is_authenticated` + `user`.
- `src/service/`: modo nodo headless; no expone HTTP.
- `src/main.rs`: `LaunchBuilder` + ventana `desktop!`; `App` abre la base en un hilo.

## Alcance
- Solo desktop nativo: Linux y Windows. No web/wasm, no Android/macOS.
- La biometría **verifica al usuario del SO**, no reemplaza el modelo de usuarios de la app.
- Se usa como **desbloqueo local** del usuario actual; no es WebAuthn ni OAuth.

## Pasos de implementación

1. Nuevo módulo `src/biometric/`:
   - `mod.rs`: `verify() -> Result<(), BiometricError>`; `is_available() -> bool`; `BiometricError::{Unsupported, NotEnrolled, Cancelled, Failed(String)}`.
   - `linux.rs`: `fprintd` por D-Bus con `zbus`. Llamar `net.reactivated.Fprint.Manager.GetDevices` y `Device.VerifyStart`; el usuario debe estar enrolado (`fprintd-enroll`). Timeout y cancelación.
   - `windows.rs`: `Windows.Security.Credentials.UI.UserConsentVerifier::RequestVerificationAsync` (Windows Hello) con el crate `windows`. Requiere sesión interactiva.
   - `noop.rs`: `Unsupported`.
2. Dependencias target-specific en `Cargo.toml`:
   - `[target.'cfg(target_os = "linux")'.dependencies] zbus = ...`
   - `[target.'cfg(target_os = "windows")'.dependencies] windows = { features = ["Security_Credentials_UI", ...] }`
3. `src/views/login.rs`: botón "Huella" solo en nativo. Al click:
   - `biometric::verify()`.
   - En éxito: resolver el usuario de la app a partir del usuario del SO (`whoami`) y setear `AuthState`.
   - En error: mostrar mensaje sin filtrar detalles sensibles.
4. Mapeo usuario SO ↔ usuario app:
   - El seed crea `admin`/`admin`; el usuario del SO suele ser otro (`mmanto`).
   - Resolver por `username` en GuardianDB; si no existe, error claro.
   - Opcional: `FEATHRAI_BIOMETRIC_USER` para forzar el mapeo. No meter en `SERVICE_ENV_VARS` (es de la GUI, no del nodo).
5. Tests:
   - Unit tests de `verify()` con un backend mock (sin tocar D-Bus/Windows).
   - `cargo check` nativo Linux; `cargo check --target x86_64-pc-windows-gnu` si el toolchain está.
6. Documentación: README, sección "Login biométrico", con requisitos (`fprintd` + usuario enrolado; Windows Hello configurado) y límites.

## Verificación
- `cargo check` en Linux.
- Smoke Linux: `cargo run --features desktop`; enrolar con `fprintd-enroll`; login con huella; verificar que `AuthState.user` queda con el usuario local.
- Windows: `cargo check --target x86_64-pc-windows-gnu`; prueba manual de `UserConsentVerifier`.
- Fallback: sin `fprintd` o sin Windows Hello, la UI muestra el botón deshabilitado y el login por contraseña sigue funcionando.

## Límites y riesgos
- Linux: `fprintd` debe estar instalado y el usuario enrolado; sin él, `verify()` devuelve `Unsupported`.
- Windows: `UserConsentVerifier` requiere sesión interactiva; puede caer a PIN si no hay biometría.
- No autentica usuarios de la app que no sean el usuario del SO. Para múltiples usuarios de la app, seguir usando contraseña.
- No es WebAuthn: no sirve para passkeys/OAuth ni para Google/Microsoft.
- La webview nativa no es un origen válido para WebAuthn (`dioxus://index.html`); por eso este plan no usa WebAuthn.
