//! Tests de la resolución de data dirs: la migración de los heredados (layout
//! previo a v0.0.1 → layout por SO) y el path interno de Android. Correr con:
//!
//! ```text
//! cargo test --bin featherai persistence
//! ```
//!
//! No tocan variables de entorno (son globales del proceso y el harness corre
//! los tests en paralelo): [`migrate_from_candidates`] recibe los paths
//! explícitos, que es lo único que depende del filesystem.

use super::{android_files_dir, migrate_from_candidates};

/// Arma un data dir "con base": `guardian/` (el store redb), un archivo suelto
/// adentro y el log en la raíz — el layout que deja `open` en el data dir.
fn data_dir_con_base(dir: &std::path::Path) -> std::path::PathBuf {
    let guardian = dir.join("guardian");
    std::fs::create_dir_all(&guardian).unwrap();
    std::fs::write(guardian.join("projects.redb"), b"redb").unwrap();
    std::fs::write(dir.join("featherai.log"), b"log viejo").unwrap();
    dir.to_path_buf()
}

#[test]
fn migra_el_data_dir_heredado() {
    let tmp = tempfile::tempdir().unwrap();
    let viejo = data_dir_con_base(&tmp.path().join("viejo"));
    let nuevo = tmp.path().join("nuevo");

    let notas = migrate_from_candidates(&nuevo, vec![viejo.clone()]);

    assert!(
        nuevo.join("guardian/projects.redb").exists(),
        "la base quedó en el layout nuevo"
    );
    assert!(
        nuevo.join("featherai.log").exists(),
        "el log se movió junto con la base"
    );
    assert!(!viejo.exists(), "el data dir viejo no se deja atrás");
    assert!(
        notas.iter().any(|n| n.contains("entradas movidas")),
        "deja nota: {notas:?}"
    );
}

#[test]
fn no_pisa_una_base_del_layout_nuevo() {
    let tmp = tempfile::tempdir().unwrap();
    let viejo = data_dir_con_base(&tmp.path().join("viejo"));
    let nuevo = data_dir_con_base(&tmp.path().join("nuevo"));
    std::fs::write(nuevo.join("guardian/projects.redb"), b"base actual").unwrap();

    let notas = migrate_from_candidates(&nuevo, vec![viejo.clone()]);

    assert!(notas.is_empty(), "no hay nada que reportar: {notas:?}");
    assert_eq!(
        std::fs::read(nuevo.join("guardian/projects.redb")).unwrap(),
        b"base actual",
        "la base del layout nuevo queda intacta"
    );
    assert!(
        viejo.join("guardian/projects.redb").exists(),
        "el data dir viejo no se toca"
    );
}

#[test]
fn sin_candidatos_con_base_no_crea_nada() {
    let tmp = tempfile::tempdir().unwrap();
    let nuevo = tmp.path().join("nuevo");
    // Candidato que existe pero sin base (p. ej. un data dir con solo el log).
    let vacio = tmp.path().join("vacio");
    std::fs::create_dir_all(&vacio).unwrap();

    let notas = migrate_from_candidates(&nuevo, vec![vacio.clone(), tmp.path().join("no-existe")]);

    assert!(notas.is_empty(), "sin base no hay migración: {notas:?}");
    assert!(
        !nuevo.exists(),
        "no se crea el data dir nuevo si no hay nada que migrar"
    );
}

#[test]
fn migra_aunque_el_data_dir_nuevo_ya_tenga_el_log() {
    let tmp = tempfile::tempdir().unwrap();
    let viejo = data_dir_con_base(&tmp.path().join("viejo"));
    let nuevo = tmp.path().join("nuevo");
    std::fs::create_dir_all(&nuevo).unwrap();
    std::fs::write(nuevo.join("featherai.log"), b"log de esta corrida").unwrap();

    let notas = migrate_from_candidates(&nuevo, vec![viejo.clone()]);

    assert!(
        nuevo.join("guardian/projects.redb").exists(),
        "migra igual con el log ya creado"
    );
    assert_eq!(
        std::fs::read(nuevo.join("featherai.log")).unwrap(),
        b"log de esta corrida",
        "el log de esta corrida no se pisa"
    );
    assert!(
        notas.iter().any(|n| n.contains("no se pisaron")),
        "avisa qué no se movió: {notas:?}"
    );
    assert!(
        !viejo.join("guardian").exists(),
        "la base salió del data dir viejo"
    );
    assert!(
        viejo.join("featherai.log").exists(),
        "lo que no se pudo mover queda en el viejo"
    );
}

/// El data dir de Android ubica el directorio del usuario **por uid**
/// (`uid = userId * 100000 + appId`), no en un `/data/data` fijo: un perfil de
/// trabajo o un usuario secundario no comparten el data dir del 0.
#[test]
fn el_data_dir_de_android_sale_del_uid() {
    assert_eq!(
        android_files_dir("com.featherai.app", 10045),
        std::path::PathBuf::from("/data/user/0/com.featherai.app/files"),
        "uid del usuario 0 (emulador, teléfono personal)"
    );
    assert_eq!(
        android_files_dir("com.featherai.app", 1010045),
        std::path::PathBuf::from("/data/user/10/com.featherai.app/files"),
        "uid de un perfil de trabajo (userId 10, mismo appId)"
    );
}
