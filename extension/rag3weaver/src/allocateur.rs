//! **Rendre au système la mémoire que l'allocateur garde** (19 octobre 2026).
//!
//! Après un gros travail, la mémoire libérée ne revient pas au système :
//! l'allocateur de glibc la garde dans ses arènes et ne rend une page qu'en
//! haut du tas. Le banc l'a mesuré : après un gros travail et la base fermée,
//! `VmRSS` restait à 1,7 Gio et tombait à 205 Mio par `malloc_trim(0)` (ticket
//! « l'hôte ne rend pas la mémoire d'un gros index »). L'hôte appelle donc
//! [`rendre_au_systeme`] à la fin d'un index et quand il se met au repos.
//!
//! `malloc_trim` n'existe que dans glibc : ailleurs (musl, macOS, Windows),
//! la fonction ne fait rien et le dit.

/// La mémoire résidente du processus en Mio (`VmRSS` de `/proc/self/status`),
/// `None` hors Linux.
pub fn residente_mio() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let ligne = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    let kio: u64 = ligne.split_whitespace().nth(1)?.parse().ok()?;
    Some(kio / 1024)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn trim() -> bool {
    extern "C" {
        fn malloc_trim(pad: usize) -> std::os::raw::c_int;
    }
    // SAFETY : malloc_trim ne prend qu'un entier et ne touche que les arènes de
    // l'allocateur, sous ses propres verrous.
    unsafe { malloc_trim(0) != 0 }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn trim() -> bool {
    false
}

/// **Rend au système la mémoire libérée que l'allocateur garde**, et dit sur
/// la sortie d'erreur ce qui est tombé : `(avant, après)` en Mio de `VmRSS`,
/// `None` là où il n'y a ni glibc ni `/proc`.
pub fn rendre_au_systeme(raison: &str) -> Option<(u64, u64)> {
    let avant = residente_mio()?;
    if !cfg!(all(target_os = "linux", target_env = "gnu")) {
        return None;
    }
    let debut = std::time::Instant::now();
    let rendu = trim();
    let apres = residente_mio()?;
    eprintln!(
        "[mémoire] {raison} : VmRSS {avant} → {apres} Mio ({}, {} ms)",
        if rendu { "rendue par malloc_trim" } else { "rien à rendre" },
        debut.elapsed().as_millis()
    );
    Some((avant, apres))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le travail du témoin, joué seul dans un processus fils (la mémoire
    /// résidente est celle du processus : les autres tests l'auraient fait
    /// bouger). Sans `RAG3WEAVER_SONDE_ALLOCATEUR`, ne fait rien.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn sonde_allocateur_dans_un_fils() {
        if std::env::var_os("RAG3WEAVER_SONDE_ALLOCATEUR").is_none() {
            return;
        }
        // 512 Mio en blocs de 64 Kio, sous le seuil de mmap : dans le tas.
        let blocs: Vec<Vec<u8>> = (0..8192).map(|i| vec![(i % 251) as u8; 64 * 1024]).collect();
        let pleine = residente_mio().unwrap();
        // Tout libéré sauf le dernier bloc, qui tient le haut du tas : glibc
        // ne rend rien seule.
        let dernier = blocs.into_iter().last();
        let (avant, apres) = rendre_au_systeme("témoin").unwrap();
        drop(dernier);
        println!("SONDE pleine={pleine} avant={avant} apres={apres}");
    }

    /// **Une mémoire libérée revient au système** : dans un fils seul, 512 Mio
    /// écrits puis libérés restent résidents tant que glibc les garde ;
    /// `rendre_au_systeme` en rend plus de la moitié. Sur un autre allocateur,
    /// rien n'est promis, rien n'est joué.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn une_memoire_liberee_revient_au_systeme() {
        let sortie = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "allocateur::tests::sonde_allocateur_dans_un_fils", "--nocapture", "--test-threads=1"])
            .env("RAG3WEAVER_SONDE_ALLOCATEUR", "1")
            .output()
            .unwrap();
        let texte = String::from_utf8_lossy(&sortie.stdout);
        // La ligne du fils peut suivre le nom du test, sur la même ligne.
        let ligne = texte.lines().find_map(|l| l.find("SONDE ").map(|i| &l[i..])).unwrap_or_else(|| {
            panic!("le fils n'a rien mesuré : {texte}{}", String::from_utf8_lossy(&sortie.stderr))
        });
        let val = |k: &str| -> u64 {
            ligne.split_whitespace().find_map(|m| m.strip_prefix(&format!("{k}="))).unwrap().parse().unwrap()
        };
        let (pleine, avant, apres) = (val("pleine"), val("avant"), val("apres"));
        assert!(pleine >= 400, "512 Mio écrits, VmRSS à {pleine} Mio");
        assert!(avant >= 400, "libérée, la mémoire reste tenue par glibc : {ligne}");
        assert!(avant.saturating_sub(apres) >= 256, "plus de la moitié des 512 Mio doit revenir : {ligne}");
    }
}
