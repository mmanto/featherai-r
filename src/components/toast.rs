//! Sistema de notificaciones toast — equivalente Dioxus de:
//!
//! - `context/ToastContext.tsx` + `hooks/useToast.ts`
//! - `components/common/ToastContainer.tsx`
//! - `types/toast.types.ts`
//!
//! El auto-cierrado (6s) se programa con `setTimeout` del lado JS vía
//! `document::eval` (funciona en web y desktop), sin dependencias async.

use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Identificador de dismisión por defecto — equivalente de `AUTO_DISMISS_MS`.
const AUTO_DISMISS_MS: u32 = 6000;

static NEXT_TOAST_ID: AtomicUsize = AtomicUsize::new(0);

/// Variante del toast — equivalente de `ToastVariant`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastVariant {
    Error,
    Success,
    Info,
}

impl ToastVariant {
    /// Clases de estilo — equivalente de `variantStyles` en ToastContainer.
    pub fn classes(self) -> &'static str {
        match self {
            Self::Error => "bg-red-600 text-white",
            Self::Success => "bg-green-600 text-white",
            Self::Info => "bg-gray-800 text-white",
        }
    }
}

/// Ítem de toast — equivalente de `ToastItem`.
#[derive(Clone, PartialEq, Debug)]
pub struct ToastItem {
    pub id: String,
    pub message: String,
    pub variant: ToastVariant,
}

/// Proveedor del contexto de toasts — equivalente de `ToastProvider`.
#[component]
pub fn ToastProvider(children: Element) -> Element {
    let toasts = use_signal(Vec::<ToastItem>::new);
    use_context_provider(|| toasts);
    rsx! { {children} }
}

/// Acceso al contexto de toasts — equivalente de `useToast().toasts`.
pub fn use_toast() -> Signal<Vec<ToastItem>> {
    use_context()
}

/// Cierra un toast — equivalente de `dismissToast`.
pub fn dismiss_toast(mut toasts: Signal<Vec<ToastItem>>, id: String) {
    toasts.write().retain(|t| t.id != id);
}

/// Muestra un toast con auto-cierrado — equivalente de `showToast`.
pub fn show_toast(mut toasts: Signal<Vec<ToastItem>>, message: String, variant: ToastVariant) {
    // Id único por sesión — sin `SystemTime` (panica en wasm32 desde Rust 1.89).
    let id = format!("toast-{}", NEXT_TOAST_ID.fetch_add(1, Ordering::Relaxed));
    toasts.write().push(ToastItem {
        id: id.clone(),
        message,
        variant,
    });

    // Auto-cierro: `setTimeout` del lado JS reenvía un mensaje al canal.
    let eval = document::eval(&format!(
        "setTimeout(() => dioxus.send('toast-dismiss:{id}'), {AUTO_DISMISS_MS})"
    ));
    spawn(async move {
        let mut eval = eval;
        while let Ok(msg) = eval.recv::<String>().await {
            if msg == format!("toast-dismiss:{id}") {
                dismiss_toast(toasts, id.clone());
            }
        }
    });
}

/// Contenedor de toasts — equivalente de `ToastContainer.tsx`.
#[component]
pub fn ToastContainer() -> Element {
    let toasts = use_toast();

    if toasts.read().is_empty() {
        return VNode::empty();
    }

    let toasts_snapshot = toasts.read().clone();
    rsx! {
        div { class: "fixed top-4 right-4 z-[100] flex flex-col gap-2 w-full max-w-sm pointer-events-none",
            for toast in toasts_snapshot {
                div {
                    key: "{toast.id}",
                    role: "alert",
                    class: format!(
                        "pointer-events-auto flex items-start justify-between gap-3 rounded-lg shadow-lg px-4 py-3 text-sm {}",
                        toast.variant.classes(),
                    ),
                    span { class: "flex-1", "{toast.message}" }
                    button {
                        onclick: move |_| dismiss_toast(toasts, toast.id.clone()),
                        class: "shrink-0 opacity-80 hover:opacity-100",
                        aria_label: "Cerrar notificación",
                        "✕"
                    }
                }
            }
        }
    }
}
