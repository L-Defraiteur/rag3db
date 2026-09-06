//! **Combien de jetons par seconde BGE-M3 rend-il, seul ?** Sans base, sans
//! découpe, sans démon : le modèle sur burn/wgpu et le tokenizer, pour savoir
//! ce que vaut le moteur avant d'accuser le chemin d'indexation.
//!
//! ```sh
//! RAG3WEAVER_SANS_DEMON=1 ./run_e2e.sh --test e2e_banc_bge_m3   # en local
//! ./run_e2e.sh --test e2e_banc_bge_m3                            # par le démon
//! ```
#![cfg(all(feature = "burn-embedder", feature = "daemon"))]
mod common;

use std::time::Instant;

use rag3weaver::embedder::Embedder;

/// Un texte d'environ `mots` mots, varié pour que le tokenizer travaille.
fn texte(mots: usize, graine: usize) -> String {
    let vocabulaire = ["fn", "catalogue", "gabarit", "search", "embedding", "ingestion", "relation", "chunk", "scope", "budget", "régime", "démon", "lot", "vector", "index", "table", "uuid", "drain", "flush", "dialect"];
    (0..mots).map(|i| vocabulaire[(i * 7 + graine * 13) % vocabulaire.len()]).collect::<Vec<_>>().join(" ")
}

fn jetons(e: &dyn Embedder, textes: &[String]) -> usize {
    // ≈ un jeton par mot court ici ; on compte les mots, ce qui sous-estime un peu.
    let _ = e;
    textes.iter().map(|t| t.split_whitespace().count() + 2).sum()
}

#[test]
#[ignore]
fn jetons_par_seconde() {
    let e: &dyn Embedder = common::burn::BGE_M3.as_ref();
    eprintln!("[banc] modèle {} — {}", e.name(), if std::env::var_os("RAG3WEAVER_SANS_DEMON").is_some() { "local" } else { "par le démon" });

    // Chauffe : compilation des noyaux pour une première forme.
    let chauffe: Vec<String> = (0..8).map(|i| texte(100, i)).collect();
    let t = Instant::now();
    e.embed(&chauffe).unwrap();
    eprintln!("[banc] chauffe 8×100 mots : {:?}", t.elapsed());

    for (lot, mots) in [(8, 100), (32, 100), (64, 100), (32, 300), (8, 900), (2, 900)] {
        let textes: Vec<String> = (0..lot).map(|i| texte(mots, i)).collect();
        let n = jetons(e, &textes);
        // Premier appel de cette forme, puis trois répétitions à forme égale.
        let t = Instant::now();
        e.embed(&textes).unwrap();
        let premier = t.elapsed();
        let t = Instant::now();
        for _ in 0..3 {
            e.embed(&textes).unwrap();
        }
        let chaud = t.elapsed() / 3;
        eprintln!(
            "[banc] lot={lot:>2} × {mots:>3} mots ({n:>5} jetons) : premier {:>7.0} ms, chaud {:>7.0} ms → {:>6.0} jetons/s chaud",
            premier.as_secs_f64() * 1000.0,
            chaud.as_secs_f64() * 1000.0,
            n as f64 / chaud.as_secs_f64()
        );
    }

    // Des formes toutes différentes : ce que coûte une longueur jamais vue.
    let t = Instant::now();
    let mut total = 0;
    for i in 0..12 {
        let textes: Vec<String> = (0..16).map(|j| texte(50 + i * 17, j)).collect();
        total += jetons(e, &textes);
        e.embed(&textes).unwrap();
    }
    eprintln!("[banc] 12 formes inédites (16 textes chacune, {total} jetons) : {:?} → {:.0} jetons/s", t.elapsed(), total as f64 / t.elapsed().as_secs_f64());
}

/// **Le f16 rend-il les mêmes vecteurs ?** Deux passages, un par précision
/// (`RAG3WEAVER_BURN_FLOAT`), qui déposent leurs vecteurs dans un fichier ;
/// le second compare au premier par le cosinus. Une précision qui change
/// l'ordre des résultats n'est pas une optimisation.
#[test]
#[ignore]
fn le_f16_rend_les_memes_vecteurs() {
    let e: &dyn Embedder = common::burn::BGE_M3.as_ref();
    let precision = std::env::var("RAG3WEAVER_BURN_FLOAT").unwrap_or_else(|_| "f32".into());
    let textes: Vec<String> = [
        "de quoi savoir qui est connecté sur mon site",
        "vendre des articles avec un prix",
        "fn take_results(ctx: &mut NodeContext, port: &str) -> Vec<UnifiedResult>",
        "Le catalogue de gabarits se cherche comme un document depuis le 29 août.",
        "un fil de discussion entre plusieurs personnes : son sujet, son état",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain((0..11).map(|i| texte(40 + i * 60, i)))
    .collect();
    let vecs = e.embed(&textes).unwrap();
    let dir = std::env::temp_dir().join("rag3weaver-parite");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{precision}.json")), serde_json::to_string(&vecs).unwrap()).unwrap();
    let autre = if precision == "f32" { "f16" } else { "f32" };
    let Ok(texte_autre) = std::fs::read_to_string(dir.join(format!("{autre}.json"))) else {
        eprintln!("[parité] {precision} déposé ; relancer avec RAG3WEAVER_BURN_FLOAT={autre} pour comparer");
        return;
    };
    let ref_: Vec<Vec<f32>> = serde_json::from_str(&texte_autre).unwrap();
    let mut pire = 1.0f32;
    for (a, b) in vecs.iter().zip(&ref_) {
        let cos: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>()
            / (a.iter().map(|x| x * x).sum::<f32>().sqrt() * b.iter().map(|x| x * x).sum::<f32>().sqrt());
        pire = pire.min(cos);
    }
    eprintln!("[parité] {precision} contre {autre} sur {} textes : cosinus minimal {pire:.5}", vecs.len());
    assert!(pire > 0.999, "le f16 change les vecteurs : cosinus {pire}");
}
