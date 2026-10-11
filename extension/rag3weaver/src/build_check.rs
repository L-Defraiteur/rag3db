//! **Le binaire et la lib du moteur viennent-ils du même bâti ?** (page
//! `docs/11-octobre-2026-recherche/07-le-magasin-de-libs.md`, §3.2).
//!
//! Le moteur porte son identifiant de bâti dans la section ELF
//! `.rag3db_build_id` de `librag3db.so`. Le binaire porte celui du bâti contre
//! lequel il a été compilé dans `.rag3db_compiled_against`, lu par `build.rs`
//! dans le `build_id.h` généré du bâti (`RAG3DB_INCLUDE_DIR`), jamais dans git.
//!
//! Deux contrôles :
//! - **avant le chargeur** (`rag3db-check-lib <binaire> <lib>`, un petit
//!   binaire qui ne lie pas la lib) : nos binaires sont liés en BIND_NOW, le
//!   chargeur résout tous les symboles avant `main`, et un contrôle dans le
//!   processus viendrait après le « undefined symbol » quand une signature
//!   change. L'`ExecStartPre` d'une unité l'appelle ;
//! - **dans le processus** ([`check_loaded`]), le filet des écarts à ABI
//!   compatible : avant la première base ouverte.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// L'identifiant du bâti contre lequel ce binaire a été compilé ; `None` sans
/// en-tête généré (le moteur rebâti dans la crate, le bâti « bundled »).
pub const COMPILED_AGAINST: Option<&str> = option_env!("RAG3DB_COMPILED_AGAINST");

/// La section du moteur.
pub const LIB_SECTION: &str = ".rag3db_build_id";
/// La section du binaire.
pub const BINARY_SECTION: &str = ".rag3db_compiled_against";

const LEN: usize = match COMPILED_AGAINST {
    Some(s) => s.len() + 1,
    None => 1,
};

const fn nul_terminated<const N: usize>(s: &str) -> [u8; N] {
    let bytes = s.as_bytes();
    let mut out = [0u8; N];
    let mut i = 0;
    while i < bytes.len() && i + 1 < N {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

/// La section du binaire. Gardée par `#[used]`, et tirée dans tout binaire
/// qui ouvre une base, puisque [`check_loaded`] la lit.
#[cfg(target_os = "linux")]
#[used]
#[link_section = ".rag3db_compiled_against"]
static COMPILED_AGAINST_SECTION: [u8; LEN] = nul_terminated::<LEN>(match COMPILED_AGAINST {
    Some(s) => s,
    None => "",
});

/// Le contenu d'une section ELF nommée (jusqu'au premier NUL), lu dans le
/// fichier sans le charger. `Ok(None)` si le fichier n'a pas cette section.
/// ELF 64 bits petit-boutiste seulement (les postes de Lucie).
pub fn read_section(path: &Path, name: &str) -> Result<Option<String>, String> {
    let at = |e: std::io::Error| format!("{} : {e}", path.display());
    let mut f = File::open(path).map_err(at)?;
    let mut h = [0u8; 64];
    f.read_exact(&mut h).map_err(at)?;
    if &h[0..4] != b"\x7fELF" {
        return Err(format!("{} : pas un fichier ELF", path.display()));
    }
    if h[4] != 2 || h[5] != 1 {
        return Err(format!("{} : ELF 64 bits petit-boutiste attendu", path.display()));
    }
    let u16_at = |b: &[u8], o: usize| u16::from_le_bytes([b[o], b[o + 1]]) as usize;
    let u32_at = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
    let u64_at = |b: &[u8], o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
    let (shoff, shentsize, shnum, shstrndx) = (u64_at(&h, 0x28), u16_at(&h, 0x3A), u16_at(&h, 0x3C), u16_at(&h, 0x3E));
    if shnum == 0 || shstrndx >= shnum || shentsize < 0x28 {
        return Err(format!("{} : table des sections absente ou étendue", path.display()));
    }
    let mut table = vec![0u8; shentsize * shnum];
    f.seek(SeekFrom::Start(shoff)).map_err(at)?;
    f.read_exact(&mut table).map_err(at)?;
    let mut read = |offset: u64, size: u64| -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; size as usize];
        f.seek(SeekFrom::Start(offset)).map_err(at)?;
        f.read_exact(&mut buf).map_err(at)?;
        Ok(buf)
    };
    let sh = |i: usize| &table[i * shentsize..(i + 1) * shentsize];
    let names = read(u64_at(sh(shstrndx), 0x18), u64_at(sh(shstrndx), 0x20))?;
    let until_nul = |b: &[u8]| String::from_utf8_lossy(b.split(|c| *c == 0).next().unwrap_or(&[])).into_owned();
    for i in 0..shnum {
        let s = sh(i);
        let n = u32_at(s, 0);
        if n < names.len() && until_nul(&names[n..]) == name {
            let content = read(u64_at(s, 0x18), u64_at(s, 0x20))?;
            return Ok(Some(until_nul(&content)));
        }
    }
    Ok(None)
}

/// **Le contrôle avant le chargeur** : le binaire et la lib portent-ils le
/// même identifiant ? Rend l'identifiant commun, ou un refus qui nomme les
/// deux.
pub fn check_pair(binary: &Path, lib: &Path) -> Result<String, String> {
    let compiled = read_section(binary, BINARY_SECTION)?.filter(|s| !s.is_empty()).ok_or_else(|| {
        format!(
            "{} ne dit pas contre quel bâti du moteur il a été compilé (pas de section {BINARY_SECTION} : \
             un binaire d'avant le contrôle, ou bâti sans en-tête généré)",
            binary.display()
        )
    })?;
    let loaded = read_section(lib, LIB_SECTION)?.ok_or_else(|| {
        format!(
            "{} ne porte pas son identifiant de bâti (pas de section {LIB_SECTION} : une lib d'avant le contrôle)",
            lib.display()
        )
    })?;
    if compiled != loaded {
        return Err(mismatch(binary, &compiled, lib, &loaded));
    }
    Ok(compiled)
}

fn mismatch(binary: &Path, compiled: &str, lib: &Path, loaded: &str) -> String {
    format!(
        "{} a été bâti contre {compiled} ; la lib {} est {loaded} : rebâtir le binaire contre cette lib, \
         ou le lancer avec la lib de son bâti",
        binary.display(),
        lib.display()
    )
}

/// **Le filet dans le processus** : la lib du moteur effectivement chargée
/// (lue dans `/proc/self/maps`) porte-t-elle l'identifiant contre lequel ce
/// binaire a été compilé ? Une seule fois par processus. Sans identifiant d'un
/// côté ou de l'autre, rien n'est comparé.
pub fn check_loaded() -> Result<(), String> {
    static RESULT: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    RESULT
        .get_or_init(|| {
            #[cfg(target_os = "linux")]
            {
                std::hint::black_box(&COMPILED_AGAINST_SECTION);
                let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
                let exe = std::env::current_exe().unwrap_or_default();
                check_against(COMPILED_AGAINST, &maps, &exe)
            }
            #[cfg(not(target_os = "linux"))]
            {
                Ok(())
            }
        })
        .clone()
}

/// [`check_loaded`], sur ce qu'on lui donne : l'identifiant compilé, le texte
/// de `/proc/<pid>/maps` et l'exécutable (le moteur lié en statique y porte sa
/// section).
pub fn check_against(compiled: Option<&str>, maps: &str, exe: &Path) -> Result<(), String> {
    let Some(compiled) = compiled.filter(|s| !s.is_empty()) else { return Ok(()) };
    let lib: PathBuf = maps
        .lines()
        .filter_map(|l| l.split_whitespace().nth(5))
        .find(|p| Path::new(p).file_name().is_some_and(|n| n.to_string_lossy().starts_with("librag3db.so")))
        .map(PathBuf::from)
        .unwrap_or_else(|| exe.to_path_buf());
    match read_section(&lib, LIB_SECTION) {
        Ok(Some(loaded)) if loaded != compiled => {
            let binary = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ce binaire"));
            Err(mismatch(&binary, compiled, &lib, &loaded))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
