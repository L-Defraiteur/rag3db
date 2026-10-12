//! Les témoins du contrôle de bâti : des ELF faits par `objcopy` (une copie de
//! `/bin/true` à qui l'on pose la section voulue), le binaire de test lui-même.
use super::*;

/// Un ELF qui porte `section` = `id` (ou aucune section si `id` est `None`).
pub(crate) fn elf(dir: &Path, name: &str, section: &str, id: Option<&str>) -> PathBuf {
    let path = dir.join(name);
    std::fs::copy("/bin/true", &path).expect("copier /bin/true");
    if let Some(id) = id {
        let content = dir.join(format!("{name}.section"));
        std::fs::write(&content, format!("{id}\0")).unwrap();
        let ok = std::process::Command::new("objcopy")
            .arg(format!("--add-section={section}={}", content.display()))
            .arg(&path)
            .status()
            .expect("objcopy");
        assert!(ok.success(), "objcopy");
    }
    path
}

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn a_section_is_read_without_loading_the_file() {
    let d = tmp();
    let lib = elf(d.path(), "librag3db.so", LIB_SECTION, Some("rag3db-build-A"));
    assert_eq!(read_section(&lib, LIB_SECTION).unwrap().as_deref(), Some("rag3db-build-A"));
    assert_eq!(read_section(&lib, BINARY_SECTION).unwrap(), None);
    let text = d.path().join("texte");
    std::fs::write(&text, "pas un ELF, mais assez long pour un en-tête de soixante-quatre octets.....").unwrap();
    assert!(read_section(&text, LIB_SECTION).unwrap_err().contains("pas un fichier ELF"));
}

/// **Le contrôle avant le chargeur** : même bâti, il rend l'identifiant ;
/// deux bâtis, il refuse en nommant les deux ; une section absente d'un côté
/// est dite, pas devinée.
#[test]
fn check_pair_names_both_builds() {
    let d = tmp();
    let bin = elf(d.path(), "backend", BINARY_SECTION, Some("rag3db-build-A"));
    let same = elf(d.path(), "lib-a.so", LIB_SECTION, Some("rag3db-build-A"));
    let other = elf(d.path(), "lib-b.so", LIB_SECTION, Some("rag3db-build-B"));
    assert_eq!(check_pair(&bin, &same).unwrap(), "rag3db-build-A");
    let err = check_pair(&bin, &other).unwrap_err();
    assert!(err.contains("rag3db-build-A") && err.contains("rag3db-build-B") && err.contains("rebâtir"), "{err}");
    let old_lib = elf(d.path(), "lib-ancienne.so", LIB_SECTION, None);
    assert!(check_pair(&bin, &old_lib).unwrap_err().contains("ne porte pas son identifiant"));
    let old_bin = elf(d.path(), "backend-ancien", BINARY_SECTION, None);
    assert!(check_pair(&old_bin, &same).unwrap_err().contains("ne dit pas contre quel bâti"));
}

/// **Le filet dans le processus** : la lib lue dans `maps` est comparée à
/// l'identifiant compilé ; sans identifiant compilé, rien n'est comparé ;
/// sans lib dans `maps` (le moteur en statique), c'est l'exécutable qui
/// porte la section.
#[test]
fn check_against_reads_the_loaded_lib() {
    let d = tmp();
    let lib = elf(d.path(), "librag3db.so", LIB_SECTION, Some("rag3db-build-B"));
    let maps = format!("7f00-7f10 r-xp 00000000 00:00 1 {}\n", lib.display());
    let err = check_against(Some("rag3db-build-A"), &maps, Path::new("/nulle/part")).unwrap_err();
    assert!(err.contains("rag3db-build-A") && err.contains("rag3db-build-B"), "{err}");
    check_against(Some("rag3db-build-B"), &maps, Path::new("/nulle/part")).unwrap();
    check_against(None, &maps, Path::new("/nulle/part")).unwrap();
    let static_exe = elf(d.path(), "statique", LIB_SECTION, Some("rag3db-build-C"));
    assert!(check_against(Some("rag3db-build-A"), "", &static_exe).is_err());
    check_against(Some("rag3db-build-C"), "", &static_exe).unwrap();
}

/// **Le binaire porte bien sa section** : ce binaire de test, compilé contre
/// un bâti à en-tête généré, dit lequel ; et la lib qu'il a chargée est la
/// même, ou ne porte pas encore d'identifiant.
#[test]
fn this_binary_carries_its_compiled_build() {
    check_loaded().unwrap();
    if let Some(compiled) = COMPILED_AGAINST {
        let exe = std::env::current_exe().unwrap();
        assert_eq!(read_section(&exe, BINARY_SECTION).unwrap().as_deref(), Some(compiled));
    } else {
        assert!(
            option_env!("RAG3DB_INCLUDE_DIR").is_none(),
            "un en-tête de bâti était posé, l'identifiant compilé aurait dû l'être"
        );
    }
}
