//! **Le surlignage des morceaux que lucivy n'a pas surlignés** (26 octobre
//! 2026, décision de Lucie relayée par c7 : « calculé à l'affichage sur les
//! chunks trouvés uniquement par dense »).
//!
//! Un morceau trouvé par le seul vecteur n'a pas de spans : rien ne dit à la
//! page quoi montrer. `HighlightNode`, après la pagination (seule la page
//! visible paie), demande à bge-m3 les poids lexicaux creux de la requête et de
//! ces morceaux, en un appel borné, et surligne dans chaque morceau les tokens
//! qui portent le plus du produit requête × morceau. Le service ne rend que des
//! ids de vocabulaire : leurs places se retrouvent en retokenisant le texte
//! chez nous, avec le même `tokenizer.json`. Sans service, sans tokenizer ou
//! sans réponse dans le délai, ce sont les mots de la requête qui sont
//! surlignés, et la méta le dit.
//!
//! Actif seulement sur demande (`options.highlights = "sparse"`) : la page de
//! l'IDE le demande, un agent qui ne voit pas le surlignage ne le paie pas.

use std::sync::Arc;
use std::time::Duration;

use crate::embedder::SparseEmbedder;
use crate::search::{chunk_highlights, Highlight};
use crate::search_strategy::UnifiedResult;
use crate::sparse_index::SparseVector;

use super::node::{Node, NodeContext};
use super::port::{take_or_clone, PortDef, PortValue, QueryPayload};

/// La source des passages surlignés par le creux.
pub const HIGHLIGHT_SPARSE: &str = "sparse";
/// La source du repli : les mots de la requête retrouvés dans le texte.
pub const HIGHLIGHT_QUERY_WORDS: &str = "query_words";
/// Au plus autant de morceaux par appel (mesuré : 318 ms pour 30 extraits de
/// 800 caractères au bge-m3 de luciepc, 25 octobre 2026).
pub const PLAFOND_DES_MORCEAUX: usize = 30;
/// Au-delà, le repli sur les mots de la requête.
pub const DELAI_DU_CREUX: Duration = Duration::from_secs(1);
/// Un token est surligné si sa part du produit requête × morceau atteint
/// cette fraction de la plus forte du morceau.
const PART_MINIMALE: f32 = 0.2;

/// **Les mots de la requête dans un texte** : chaque mot du texte qui contient
/// un terme de la requête (deux caractères au moins), sans tenir compte de la
/// casse. Le repli quand le creux ne répond pas.
pub fn query_word_highlights(query: &str, text: &str) -> Vec<Highlight> {
    let termes: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 2)
        .map(str::to_lowercase)
        .collect();
    if termes.is_empty() {
        return Vec::new();
    }
    // Les mots du texte, par leurs bornes en octets.
    let mut mots: Vec<(usize, usize)> = Vec::new();
    let mut debut: Option<usize> = None;
    for (i, c) in text.char_indices() {
        match (c.is_alphanumeric(), debut) {
            (true, None) => debut = Some(i),
            (false, Some(d)) => {
                mots.push((d, i));
                debut = None;
            }
            _ => {}
        }
    }
    if let Some(d) = debut {
        mots.push((d, text.len()));
    }
    let spans: Vec<(usize, usize, &str)> = mots
        .into_iter()
        .map(|(d, f)| (d, f, &text[d..f]))
        .filter(|(_, _, mot)| {
            let bas = mot.to_lowercase();
            termes.iter().any(|t| bas.contains(t.as_str()))
        })
        .collect();
    chunk_highlights(text, 0, text.len(), &spans, HIGHLIGHT_QUERY_WORDS)
}

/// **Les tokens qui portent le creux, placés dans le texte.** `tokens` : l'id
/// de vocabulaire et les bornes en octets de chaque token du texte (le
/// tokenizer de bge-m3) ; `requete` et `morceau` : les vecteurs creux rendus
/// par le service. La part d'un id est le produit de ses deux poids ; les
/// tokens dont la part atteint [`PART_MINIMALE`] de la plus forte sont
/// surlignés, rognés de leurs blancs, et les morceaux de mot contigus fondus.
pub fn sparse_highlights(
    text: &str,
    tokens: &[(u32, (usize, usize))],
    requete: &SparseVector,
    morceau: &SparseVector,
) -> Vec<Highlight> {
    let poids = |v: &SparseVector, id: u32| -> f32 {
        v.indices.iter().position(|&i| i == id).map(|p| v.values[p]).unwrap_or(0.0)
    };
    let parts: Vec<(u32, f32)> = requete
        .indices
        .iter()
        .zip(&requete.values)
        .map(|(&id, &q)| (id, q * poids(morceau, id)))
        .filter(|&(_, p)| p > 0.0)
        .collect();
    let Some(plus_forte) = parts.iter().map(|&(_, p)| p).reduce(f32::max) else {
        return Vec::new();
    };
    let retenus: Vec<u32> = parts
        .iter()
        .filter(|&&(_, p)| p >= PART_MINIMALE * plus_forte)
        .map(|&(id, _)| id)
        .collect();
    let mut spans: Vec<(usize, usize, &str)> = Vec::new();
    for &(id, (d, f)) in tokens {
        if !retenus.contains(&id) {
            continue;
        }
        let Some(brut) = text.get(d..f) else { continue };
        let tete = brut.len() - brut.trim_start().len();
        let net = brut.trim();
        if !net.is_empty() {
            spans.push((d + tete, d + tete + net.len(), net));
        }
    }
    chunk_highlights(text, 0, text.len(), &spans, HIGHLIGHT_SPARSE)
}

/// Le tokenizer de bge-m3, chargé une fois (`RAG3WEAVER_BGE_M3_TOKENIZER`,
/// sinon `~/.cache/rag3weaver/bge-m3/tokenizer.json`).
#[cfg(feature = "burn-embedder")]
fn tokenizer() -> Result<&'static tokenizers::Tokenizer, String> {
    static T: std::sync::OnceLock<Result<tokenizers::Tokenizer, String>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let chemin = crate::model_source::local_artifact("bge-m3", "RAG3WEAVER_BGE_M3", "TOKENIZER", "tokenizer.json")?;
        tokenizers::Tokenizer::from_file(&chemin).map_err(|e| format!("tokenizer.json de bge-m3 illisible ({}) : {e}", chemin.display()))
    })
    .as_ref()
    .map_err(Clone::clone)
}

/// Sans le tokenizer compilé, le creux ne se place pas : le repli, et sa raison.
#[cfg(not(feature = "burn-embedder"))]
fn surligner_par_le_creux(
    _embarqueur: Option<Arc<dyn SparseEmbedder>>,
    _requete: &str,
    _creux_de_la_requete: Option<&SparseVector>,
    _textes: &[String],
) -> Result<Vec<Vec<Highlight>>, String> {
    Err("le surlignage creux n'est pas compilé ici (feature burn-embedder)".to_string())
}

/// **Le creux de la requête et des morceaux, en un appel borné.** Rend, pour
/// chaque texte, ses passages ; ou la raison du repli.
#[cfg(feature = "burn-embedder")]
fn surligner_par_le_creux(
    embarqueur: Option<Arc<dyn SparseEmbedder>>,
    requete: &str,
    creux_de_la_requete: Option<&SparseVector>,
    textes: &[String],
) -> Result<Vec<Vec<Highlight>>, String> {
    {
        let embarqueur = embarqueur.ok_or("aucun embarqueur creux configuré")?;
        let tok = tokenizer()?;
        // La requête est déjà embarquée en creux quand le signal creux sert ;
        // sinon elle part avec les morceaux, dans le même appel.
        let mut envoi: Vec<String> = Vec::with_capacity(textes.len() + 1);
        if creux_de_la_requete.is_none() {
            envoi.push(requete.to_string());
        }
        envoi.extend(textes.iter().cloned());
        let attendu = envoi.len();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(embarqueur.embed_sparse(&envoi));
        });
        let mut rendus = match rx.recv_timeout(DELAI_DU_CREUX) {
            Ok(Ok(v)) if v.len() == attendu => v,
            Ok(Ok(v)) => return Err(format!("le service a rendu {} vecteurs creux pour {attendu} textes", v.len())),
            Ok(Err(e)) => return Err(format!("le service creux a échoué : {e}")),
            Err(_) => return Err(format!("le service creux n'a pas répondu en {} ms", DELAI_DU_CREUX.as_millis())),
        };
        let requete_creuse = match creux_de_la_requete {
            Some(q) => q.clone(),
            None => rendus.remove(0),
        };
        Ok(textes
            .iter()
            .zip(&rendus)
            .map(|(texte, morceau)| match tok.encode(texte.as_str(), false) {
                Ok(enc) => {
                    let tokens: Vec<(u32, (usize, usize))> =
                        enc.get_ids().iter().copied().zip(enc.get_offsets().iter().copied()).collect();
                    sparse_highlights(texte, &tokens, &requete_creuse, morceau)
                }
                Err(_) => Vec::new(),
            })
            .collect())
    }
}

/// **Le nœud du surlignage à l'affichage** : voir le module.
pub struct HighlightNode {
    node_name: String,
}

impl HighlightNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string() }
    }
}

impl Node for HighlightNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "HighlightNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::HighlightNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::HighlightNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut results = ctx
            .take_input("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .ok_or("HighlightNode: missing 'results' input")?;
        let qp = ctx
            .take_input("query")
            .and_then(|pv| take_or_clone::<QueryPayload>(pv))
            .ok_or("HighlightNode: missing 'query' input")?;

        // Sans demande, rien : ni service consulté, ni méta.
        let mut avertissements: Vec<String> = Vec::new();
        match qp.options.highlights.as_deref() {
            None => {}
            Some(HIGHLIGHT_SPARSE) => {
                let a_surligner: Vec<usize> = results
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.chunk.as_ref().is_some_and(|c| c.highlights.is_empty() && !c.text.is_empty()))
                    .map(|(i, _)| i)
                    .take(PLAFOND_DES_MORCEAUX)
                    .collect();
                if !a_surligner.is_empty() {
                    let textes: Vec<String> = a_surligner
                        .iter()
                        .map(|&i| results[i].chunk.as_ref().map(|c| c.text.clone()).unwrap_or_default())
                        .collect();
                    let embarqueur = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder").cloned();
                    let passages = surligner_par_le_creux(embarqueur, &qp.query, qp.sparse.as_ref(), &textes)
                        .unwrap_or_else(|raison| {
                            avertissements.push(format!(
                                "surlignage creux indisponible ({raison}) : les mots de la requête sont surlignés à sa place"
                            ));
                            vec![Vec::new(); textes.len()]
                        });
                    for ((&i, texte), creux) in a_surligner.iter().zip(&textes).zip(passages) {
                        // Le creux n'a rien retenu dans ce morceau : les mots de
                        // la requête, s'il y en a.
                        let h = if creux.is_empty() { query_word_highlights(&qp.query, texte) } else { creux };
                        if let Some(c) = results[i].chunk.as_mut() {
                            c.highlights = h;
                        }
                    }
                }
            }
            Some(autre) => avertissements.push(format!(
                "highlights « {autre} » inconnu (seul « {HIGHLIGHT_SPARSE} » existe) : rien n'est surligné en plus"
            )),
        }

        ctx.set_output("results", PortValue::new(results));
        if !avertissements.is_empty() {
            for w in &avertissements {
                ctx.warn(w);
            }
            ctx.set_output(
                "meta",
                PortValue::new(crate::search::SearchMeta {
                    query: qp.query.clone(),
                    target: qp.target_name.clone(),
                    signals: crate::search::SearchSignals::NONE,
                    consistency: qp.options.consistency,
                    partial: false,
                    pending_count: 0,
                    vector_count: 0,
                    bm25_count: 0,
                    sparse_count: 0,
                    fused_count: 0,
                    reranked_count: 0,
                    warnings: avertissements,
                    search_time_ms: 0,
                    diagnostics: None,
                }),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en_utf16(texte: &str, h: &Highlight) -> String {
        let u: Vec<u16> = texte.encode_utf16().collect();
        String::from_utf16(&u[h.start..h.end]).unwrap()
    }

    #[test]
    fn les_mots_de_la_requete_se_retrouvent_sans_la_casse() {
        let texte = "Les Vues d'ensemble du moteur : une vue.";
        let h = query_word_highlights("vue d'ensemble", texte);
        let vus: Vec<String> = h.iter().map(|x| en_utf16(texte, x)).collect();
        assert_eq!(vus, vec!["Vues", "ensemble", "vue"], "« d » est trop court pour compter : {vus:?}");
        assert!(h.iter().all(|x| x.source == HIGHLIGHT_QUERY_WORDS));
    }

    #[test]
    fn le_creux_surligne_les_tokens_qui_portent_le_produit() {
        // « fusion des rangs » : trois tokens, le premier en deux morceaux
        // contigus (« fu », « sion ») ; « des » pèse peu dans la requête.
        let texte = "la fusion des rangs";
        let tokens = vec![(1, (0, 2)), (10, (2, 5)), (11, (5, 9)), (12, (9, 13)), (13, (13, 19))];
        let requete = SparseVector { indices: vec![10, 11, 12, 13], values: vec![0.3, 0.3, 0.01, 0.25] };
        let morceau = SparseVector { indices: vec![1, 10, 11, 12, 13], values: vec![0.2, 0.2, 0.2, 0.2, 0.2] };
        let h = sparse_highlights(texte, &tokens, &requete, &morceau);
        let vus: Vec<String> = h.iter().map(|x| en_utf16(texte, x)).collect();
        assert_eq!(vus, vec!["fusion", "rangs"], "les morceaux contigus se fondent, le blanc de tête part, « des » reste : {vus:?}");
        assert!(h.iter().all(|x| x.source == HIGHLIGHT_SPARSE));
    }

    #[test]
    fn sans_id_partage_le_creux_ne_surligne_rien() {
        let requete = SparseVector { indices: vec![5], values: vec![0.5] };
        let morceau = SparseVector { indices: vec![6], values: vec![0.5] };
        assert!(sparse_highlights("abc", &[(6, (0, 3))], &requete, &morceau).is_empty());
    }
}
