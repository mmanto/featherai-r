use dioxus::prelude::*;

/// Raíz del panel — equivalente del redirect de React `/admin` → `/admin/projects`.
#[component]
pub fn AdminPanel() -> Element {
    let navigator = use_navigator();
    navigator.replace("/admin/projects");
    VNode::empty()
}
