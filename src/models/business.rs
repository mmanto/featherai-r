//! Negocios por nombre — sin entidad propia.
//!
//! Los negocios son nombres compartidos por convención entre `Project.business`
//! y `User.businesses` (ver el doc del campo en `models/project.rs`): no hay
//! store ni id de negocio. Cada formulario arma sus opciones con
//! [`business_options`] (los demo más los nombres que ya usan sus registros) y
//! crea uno nuevo escribiéndolo — el centinela [`NEW_BUSINESS`] no es un nombre
//! válido y se filtra.

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

#[cfg(test)]
mod tests {
    use super::business_options;

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
