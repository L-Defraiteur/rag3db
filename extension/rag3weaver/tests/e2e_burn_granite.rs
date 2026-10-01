//! **Granite embedding 107m / 278m sur burn** : les vecteurs sont unitaires, une
//! paraphrase entre deux langues est plus proche qu'un texte étranger, et une
//! question en français trouve la fonction de code qu'elle décrit. Pas de
//! référence candle ici : la fiche IBM fait CLS puis L2, on vérifie le sens.
//!
//! Poids : `~/.cache/rag3weaver/granite-{107m,278m}/{model.bpk,tokenizer.json}`
//! (voir `generated/README.md`).
#![cfg(feature = "burn-embedder")]

mod common;

use rag3weaver::embedder::Embedder;

fn cos(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>()
}

fn unitaires(e: &dyn Embedder) {
    let v = e.embed(&["fn budget_batches(lens: &[usize]) -> Vec<Range<usize>>".into(), "Le catalogue de gabarits se cherche comme un document.".into()]).unwrap();
    assert_eq!(v.len(), 2);
    for x in &v {
        assert_eq!(x.len(), e.dim());
        let n: f32 = x.iter().map(|a| a * a).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-3, "norme {n}");
    }
}

fn paraphrase_entre_langues(e: &dyn Embedder) {
    let v = e.embed(&[
        "La fonction ouvre le fichier et lit son contenu en mémoire.".into(),
        "The function opens the file and reads its contents into memory.".into(),
        "Le chat dort sur le canapé depuis ce matin.".into(),
    ]).unwrap();
    let (fr_en, fr_chat) = (cos(&v[0], &v[1]), cos(&v[0], &v[2]));
    eprintln!("[granite] fr/en {fr_en:.3} — fr/chat {fr_chat:.3}");
    assert!(fr_en > fr_chat + 0.15, "paraphrase {fr_en} contre étranger {fr_chat}");
}

fn question_trouve_le_code(e: &dyn Embedder) {
    let v = e.embed(&[
        "comment découper une liste de textes en lots selon un budget de caractères ?".into(),
        "pub fn budget_batches(lens: &[usize], max_items: usize, max_chars: usize) -> Vec<Range<usize>> { let mut lots = Vec::new(); /* ferme un lot quand il dépasse max_items ou max_chars */ lots }".into(),
        "pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 { a.iter().zip(b).map(|(x, y)| x * y).sum() }".into(),
        "impl Display for OcrLine { fn fmt(&self, f: &mut Formatter) -> fmt::Result { write!(f, \"{}\", self.text) } }".into(),
    ]).unwrap();
    let scores: Vec<f32> = (1..4).map(|i| cos(&v[0], &v[i])).collect();
    eprintln!("[granite] question → lots {:.3}, cosinus {:.3}, ocr {:.3}", scores[0], scores[1], scores[2]);
    assert!(scores[0] > scores[1] && scores[0] > scores[2], "la question devrait trouver budget_batches : {scores:?}");
}

#[test]
#[ignore]
fn granite_107m_vecteurs_unitaires() { unitaires(common::burn::GRANITE_107M.as_ref()); }
#[test]
#[ignore]
fn granite_107m_paraphrase_entre_langues() { paraphrase_entre_langues(common::burn::GRANITE_107M.as_ref()); }
#[test]
#[ignore]
fn granite_107m_question_trouve_le_code() { question_trouve_le_code(common::burn::GRANITE_107M.as_ref()); }
#[test]
#[ignore]
fn granite_278m_vecteurs_unitaires() { unitaires(common::burn::GRANITE_278M.as_ref()); }
#[test]
#[ignore]
fn granite_278m_paraphrase_entre_langues() { paraphrase_entre_langues(common::burn::GRANITE_278M.as_ref()); }
#[test]
#[ignore]
fn granite_278m_question_trouve_le_code() { question_trouve_le_code(common::burn::GRANITE_278M.as_ref()); }

/// **Un lot de 256 séquences de ~450 mots** : ce que le conseil de lot promet
/// une fois l'attention fusionnée (sans elle, 3,2 Go de scores, refusé).
#[test]
#[ignore]
fn granite_107m_lot_de_256_sequences_longues() {
    let e = common::burn::GRANITE_107M.as_ref();
    let voc = ["fn", "catalogue", "gabarit", "search", "embedding", "ingestion", "relation", "chunk", "scope", "budget"];
    let textes: Vec<String> = (0..256).map(|i| (0..450).map(|j| voc[(i * 7 + j * 13) % voc.len()]).collect::<Vec<_>>().join(" ")).collect();
    let t = std::time::Instant::now();
    let v = e.embed(&textes).unwrap();
    let s = t.elapsed().as_secs_f64();
    eprintln!("[granite] 256 × 450 mots : {} vecteurs en {s:.2} s → {:.0} jetons/s", v.len(), 256.0 * 452.0 / s);
    assert_eq!(v.len(), 256);
}

/// **Le débit selon la taille du lot, sur des séquences longues** (chantier
/// D du 03 optimiseur) : 32, 64, 128 et 256 séquences de ~450 mots, chaque
/// taille jouée deux fois (la première compile les noyaux). Ce qui dit si
/// conseiller 256 plutôt que 128 vaut la peine.
fn lots_longs_selon_la_taille(e: &dyn Embedder, nom: &str) {
    let voc = ["fn", "catalogue", "gabarit", "search", "embedding", "ingestion", "relation", "chunk", "scope", "budget"];
    for n in [32usize, 64, 128, 256] {
        let textes: Vec<String> = (0..n).map(|i| (0..450).map(|j| voc[(i * 7 + j * 13) % voc.len()]).collect::<Vec<_>>().join(" ")).collect();
        let mut debits = Vec::new();
        for _ in 0..2 {
            let t = std::time::Instant::now();
            let v = e.embed(&textes).unwrap();
            assert_eq!(v.len(), n);
            debits.push(n as f64 * 452.0 / t.elapsed().as_secs_f64());
        }
        eprintln!("[{nom}] {n:>3} × 450 mots : {:.0} puis {:.0} jetons/s", debits[0], debits[1]);
    }
}

#[test]
#[ignore]
fn granite_107m_lots_longs_selon_la_taille() { lots_longs_selon_la_taille(common::burn::GRANITE_107M.as_ref(), "granite-107m"); }
#[test]
#[ignore]
fn granite_278m_lots_longs_selon_la_taille() { lots_longs_selon_la_taille(common::burn::GRANITE_278M.as_ref(), "granite-278m"); }

/// **Le modèle chargé contre la référence ONNX**, phrase par phrase, par
/// l'écart absolu maximal — pas par un cosinus arrondi. Les références
/// (`tests/fixtures/granite/reference-{278m,107m}.json`) sont les vecteurs
/// d'onnxruntime (CPU, f32) sur le `model.onnx` d'IBM, pour un jeu fixe de 24
/// phrases : français, anglais, code, autres langues, du mot seul au texte
/// tronqué à 512 jetons. `generated/reference_granite.py` les reproduit.
///
/// C'est ce qui prouve un `model.bpk` régénéré : le fichier n'est pas
/// reproductible à l'octet, ses vecteurs le sont. Chaque phrase passe seule,
/// comme dans la référence, pour qu'aucun remplissage n'entre dans la mesure.
/// Mesuré le 2 octobre 2026 sur un iGPU Radeon 8060S (Vulkan, radv) : 3e-7 en
/// f32 pour les deux modèles, 4,6e-5 (278m) et 7,6e-5 (107m) en Flex32. Le
/// seuil par défaut, 5e-4, est six fois la plus grande de ces mesures ;
/// `RAG3WEAVER_GRANITE_ECART_MAX` le déplace pour une autre carte.
fn contre_la_reference_onnx(e: &dyn Embedder, fichier: &str) {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/granite").join(fichier);
    let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&chemin).unwrap()).unwrap();
    let phrases: Vec<String> = serde_json::from_value(r["phrases"].clone()).unwrap();
    let reference: Vec<Vec<f32>> = serde_json::from_value(r["vecteurs"].clone()).unwrap();
    let jetons: Vec<usize> = serde_json::from_value(r["jetons"].clone()).unwrap();
    let (mut pire, mut ou, mut pire_cos) = (0f32, 0usize, 1f32);
    for (i, (p, attendu)) in phrases.iter().zip(&reference).enumerate() {
        let v = e.embed(std::slice::from_ref(p)).unwrap().remove(0);
        assert_eq!(v.len(), attendu.len(), "dimension de la phrase {i}");
        let ecart = v.iter().zip(attendu).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
        if ecart > pire {
            pire = ecart;
            ou = i;
        }
        pire_cos = pire_cos.min(cos(&v, attendu));
    }
    let seuil: f32 = std::env::var("RAG3WEAVER_GRANITE_ECART_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(5e-4);
    eprintln!(
        "[granite] {fichier} : {} phrases ({}–{} jetons), dim {}, écart absolu max {pire:.7} (phrase {ou}, {} jetons), cosinus minimal {pire_cos:.7}, seuil {seuil}",
        phrases.len(),
        jetons.iter().min().unwrap(),
        jetons.iter().max().unwrap(),
        reference[0].len(),
        jetons[ou]
    );
    assert!(pire < seuil, "{fichier} : écart absolu max {pire} au-dessus de {seuil} (phrase {ou})");
}

#[test]
#[ignore]
fn granite_278m_contre_la_reference_onnx() { contre_la_reference_onnx(common::burn::GRANITE_278M.as_ref(), "reference-278m.json"); }
#[test]
#[ignore]
fn granite_107m_contre_la_reference_onnx() { contre_la_reference_onnx(common::burn::GRANITE_107M.as_ref(), "reference-107m.json"); }
