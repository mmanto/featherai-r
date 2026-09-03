//! Tema claro/oscuro — port a Dioxus de `ThemeContext.tsx` de feathrai-frontend.
//!
//! La paleta vive en CSS (`assets/styling/main.css`): `:root` es el tema claro
//! y `[data-theme="dark"]` el oscuro, igual que `variables.css` de React.
//! El proveedor solo setea `data-theme` en `<html>` y persiste la elección en
//! `localStorage` con la misma key que React (`app-theme-id`).

use dioxus::prelude::*;

/// Key de persistencia — idéntica a `STORAGE_KEY` de `ThemeContext.tsx`.
const STORAGE_KEY: &str = "app-theme-id";

/// Tema activo — equivalente de `activeThemeId` (`'light' | 'dark'`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeId {
    Light,
    Dark,
}

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeId::Light => "light",
            ThemeId::Dark => "dark",
        }
    }

    /// Alterna claro ↔ oscuro — equivalente de `toggleTheme()`.
    pub fn toggled(self) -> Self {
        match self {
            ThemeId::Light => ThemeId::Dark,
            ThemeId::Dark => ThemeId::Light,
        }
    }
}

/// Proveedor del tema — equivalente de `ThemeProvider`. Envuelve al resto de
/// la app y aplica `data-theme` sobre `document.documentElement` en cada
/// cambio (el CSS se encarga de las variables).
#[component]
pub fn ThemeProvider(children: Element) -> Element {
    let mut theme = use_signal(|| ThemeId::Light);

    // Restaurar la elección guardada. En SSR/desktop el eval puede no estar
    // disponible o no devolver valor: se ignora y queda el default claro.
    use_effect(move || {
        let eval = document::eval(&format!(
            "localStorage.getItem('{STORAGE_KEY}') || 'light';"
        ));
        spawn(async move {
            let mut eval = eval;
            if let Ok(id) = eval.recv::<String>().await {
                if id == ThemeId::Dark.as_str() {
                    theme.set(ThemeId::Dark);
                }
            }
        });
    });

    // Aplicar el atributo y persistir en cada cambio.
    use_effect(move || {
        let id = theme().as_str();
        document::eval(&format!(
            "document.documentElement.setAttribute('data-theme', '{id}'); localStorage.setItem('{STORAGE_KEY}', '{id}');"
        ));
    });

    use_context_provider(|| theme);
    rsx! { {children} }
}

/// Acceso al tema activo — equivalente de `useTheme().activeThemeId`.
///
/// # Panics
/// Fuera de un [`ThemeProvider`].
pub fn use_theme() -> Signal<ThemeId> {
    use_context::<Signal<ThemeId>>()
}
