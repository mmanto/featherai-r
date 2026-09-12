use dioxus::prelude::*;

/// Diálogo de confirmación para acciones destructivas (borrados).
///
/// Reemplaza al `confirm()` nativo de JS: `document::eval("confirm(…)")` no
/// muestra diálogo en el webview de dioxus-desktop (en el port de React
/// funcionaba porque corría en navegador), así que el borrado quedaba en
/// silencio. Mismo shell visual que los modales de la app; el botón de
/// confirmar usa el estilo de peligro.
#[component]
pub fn ConfirmDialog(
    title: String,
    message: String,
    confirm_label: String,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "fixed inset-0 z-[1050] flex items-center justify-center overflow-y-auto bg-black/50 p-4",
            div { class: "w-full max-w-md",
                div { class: "w-full rounded-lg border border-[var(--card-border)] bg-[var(--card-bg)] shadow-[var(--shadow-lg)]",
                    div { class: "flex items-center justify-between border-b border-[var(--border-color)] px-4 py-3",
                        h5 { class: "text-[1.0625rem] font-medium text-[var(--text-primary)] flex items-center gap-2",
                            i { class: "bi bi-exclamation-triangle text-[var(--danger-color)]" }
                            "{title}"
                        }
                        button {
                            class: "flex h-8 w-8 items-center justify-center rounded text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]",
                            onclick: move |_| on_cancel.call(()),
                            i { class: "bi bi-x-lg" }
                        }
                    }
                    div { class: "p-4",
                        p { class: "mb-4 text-[var(--text-secondary)]", "{message}" }
                        div { class: "flex justify-end gap-2",
                            button {
                                class: "inline-flex items-center justify-center rounded border border-[var(--border-color)] text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] px-3 py-1.5 text-sm",
                                onclick: move |_| on_cancel.call(()),
                                "Cancelar"
                            }
                            button {
                                class: "inline-flex items-center justify-center rounded bg-[var(--danger-color)] text-white hover:opacity-90 px-3 py-1.5 text-sm",
                                onclick: move |_| on_confirm.call(()),
                                i { class: "bi bi-trash me-1" }
                                "{confirm_label}"
                            }
                        }
                    }
                }
            }
        }
    }
}
