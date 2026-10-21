//! Negocios por nombre — sin entidad propia.
//!
//! Los negocios son nombres compartidos por convención entre `Project.business`
//! y `User.businesses` (ver el doc del campo en `models/project.rs`): no hay
//! store ni id de negocio. Cada formulario arma sus opciones con
//! [`business_options`] (los demo más los nombres que ya usan sus registros) y
//! crea uno nuevo escribiéndolo — el centinela [`NEW_BUSINESS`] no es un nombre
//! válido y se filtra. La elección del selector se resuelve con
//! [`resolve_business`], común a los formularios de alta y edición.

/// Negocios demo que se ofrecen en los selectores (en React venían de
/// `businessService.getAll()`).
pub const DEMO_BUSINESSES: [&str; 3] = ["AgileTeam Corp", "NovaSoft", "TechCorp"];

/// Valor centinela del selector de negocio: muestra el campo para escribir uno
/// nuevo. No puede chocar con un nombre real (se filtra y la creación lo
/// rechaza).
pub const NEW_BUSINESS: &str = "__new__";

/// Opciones del selector: los demo + `used` (nombres ya usados por proyectos o
/// usuarios), sin vacíos ni duplicados y en orden de aparición.
pub fn business_options(used: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut options: Vec<String> = DEMO_BUSINESSES.iter().map(|b| (*b).to_string()).collect();
    for name in used {
        let name = name.trim();
        if !name.is_empty() && name != NEW_BUSINESS && !options.iter().any(|o| o == name) {
            options.push(name.to_string());
        }
    }
    options
}

/// Opciones del selector de negocio de un formulario de proyecto, según el
/// alcance del rol de la sesión:
///
/// - `scope` `None` (administración general): [`business_options`] — los demo
///   más `used`.
/// - `scope` `Some(negocios)` (`admin`/`operativo`): sólo esos negocios; si no,
///   el proyecto quedaría fuera del listado de quien lo crea o edita.
///
/// `current` es el negocio actual del proyecto al editar: se agrega si quedó
/// fuera de las opciones (p. ej. la sesión ya no lo incluye), así editar otro
/// campo no lo pierde. Vacío y el centinela [`NEW_BUSINESS`] no son nombres.
pub fn project_business_options(
    scope: Option<&[String]>,
    used: impl IntoIterator<Item = String>,
    current: Option<&str>,
) -> Vec<String> {
    let mut options = match scope {
        None => business_options(used),
        Some(names) => names.to_vec(),
    };
    if let Some(current) = current
        .map(str::trim)
        .filter(|c| !c.is_empty() && *c != NEW_BUSINESS)
        && !options.iter().any(|o| o == current)
    {
        options.push(current.to_string());
    }
    options
}

/// Resuelve la elección del selector de negocio de un formulario (alta o
/// edición de proyecto):
///
/// - `Some(None)` — se eligió "sin negocio".
/// - `Some(Some(nombre))` — negocio elegido: uno existente o el recién
///   escrito cuando la selección es [`NEW_BUSINESS`] (el nombre va sin
///   espacios).
/// - `None` — pidió crear un negocio pero el campo quedó vacío (o con solo el
///   centinela): selección inválida, el formulario no debe guardar.
pub fn resolve_business(selection: &str, new_name: &str) -> Option<Option<String>> {
    if selection == NEW_BUSINESS {
        let created = new_name.trim();
        if created.is_empty() || created == NEW_BUSINESS {
            return None;
        }
        Some(Some(created.to_string()))
    } else if selection.trim().is_empty() {
        Some(None)
    } else {
        Some(Some(selection.trim().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{business_options, project_business_options, resolve_business, NEW_BUSINESS};

    #[test]
    fn opciones_de_negocio_segun_alcance() {
        let usados = vec!["Beta".to_string(), "NovaSoft".to_string()];
        let alcance = vec!["NovaSoft".to_string(), "Acme".to_string()];

        // Administración general (sin alcance): demo + los ya usados.
        assert_eq!(
            project_business_options(None, usados.clone(), None),
            vec!["AgileTeam Corp", "NovaSoft", "TechCorp", "Beta"]
        );

        // admin/operativo: sólo sus negocios, sin demo ni "sin negocio".
        assert_eq!(
            project_business_options(Some(&alcance), usados, None),
            alcance
        );

        // Al editar, el negocio actual se agrega si quedó fuera del alcance;
        // no se duplica si ya está, y vacío o centinela no son nombres.
        assert_eq!(
            project_business_options(Some(&alcance), vec![], Some("Viejo")),
            vec!["NovaSoft", "Acme", "Viejo"]
        );
        assert_eq!(
            project_business_options(Some(&alcance), vec![], Some(" Acme ")),
            alcance
        );
        assert_eq!(
            project_business_options(Some(&alcance), vec![], Some("  ")),
            alcance
        );
        assert_eq!(
            project_business_options(Some(&alcance), vec![], Some(NEW_BUSINESS)),
            alcance
        );
    }

    #[test]
    fn seleccion_de_negocio() {
        // Existente y sin negocio.
        assert_eq!(
            resolve_business("NovaSoft", ""),
            Some(Some("NovaSoft".to_string()))
        );
        assert_eq!(resolve_business("", "ignorado"), Some(None));

        // Recién escrito: se recorta; vacío o centinela → selección inválida.
        assert_eq!(
            resolve_business(NEW_BUSINESS, "  Acme  "),
            Some(Some("Acme".to_string()))
        );
        assert_eq!(resolve_business(NEW_BUSINESS, "   "), None);
        assert_eq!(resolve_business(NEW_BUSINESS, NEW_BUSINESS), None);
    }

    #[test]
    fn opciones_demo_mas_usadas_sin_duplicados() {
        let options = business_options(vec![
            "Beta".to_string(),
            " AgileTeam Corp ".to_string(),
            String::new(),
            "Beta".to_string(),
        ]);
        assert_eq!(
            options,
            vec!["AgileTeam Corp", "NovaSoft", "TechCorp", "Beta"]
        );
    }
}
