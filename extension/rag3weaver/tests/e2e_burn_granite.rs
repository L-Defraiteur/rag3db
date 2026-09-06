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
