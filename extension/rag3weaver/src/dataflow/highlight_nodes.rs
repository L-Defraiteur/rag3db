//! **Le surlignage des morceaux que lucivy n'a pas surlignés** (26 octobre
//! 2026, décision de Lucie relayée par c7 : « calculé à l'affichage sur les
//! chunks trouvés uniquement par dense »).
//!
//! Un morceau trouvé par le seul vecteur n'a pas de spans : rien ne dit à la
//! page quoi montrer. `HighlightNode`, après la pagination (seule la page
//! visible paie), demande à bge-m3 le poids de chaque position de la requête et
//! de ces morceaux, en un appel borné (`/embed_sparse` avec `spans`, le service
//! tokenise), et surligne dans chaque morceau les positions qui portent le plus
//! du produit requête × morceau. L'hôte n'a ni tokenizer ni modèle (7 novembre
//! 2026 : la feature burn-embedder ne lui sert plus pour ça). Sans service, avec
//! un service d'avant l'option `spans` ou sans réponse dans le délai, ce sont
//! les mots de la requête qui sont surlignés, et la méta le dit.
//!
//! Actif seulement sur demande (`options.highlights = "sparse"`) : la page de
//! l'IDE le demande, un agent qui ne voit pas le surlignage ne le paie pas.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::embedder::SparseEmbedder;
use crate::search::{chunk_highlights, Highlight};
use crate::search_strategy::UnifiedResult;
use crate::sparse_index::SparseSpan;

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

/// **Le texte d'une position, sans son espace de tête** : le Metaspace de bge-m3 range
/// l'espace dans le jeton qui suit, si bien que le service rend « ␣chat » sur `[2, 7]` pour
/// « Le chat ». Rend les bornes du mot nu et sa clé (en minuscules), ou `None` pour une
/// position vide ou hors des frontières de caractère.
fn position_nue(text: &str, start: usize, end: usize) -> Option<(usize, usize, String)> {
    let brut = text.get(start..end)?;
    let tete = brut.len() - brut.trim_start().len();
    let net = brut.trim();
    (!net.is_empty()).then(|| (start + tete, start + tete + net.len(), net.to_lowercase()))
}

/// **Le poids de chaque jeton de la requête**, par son texte nu : la requête passe par le même
/// service que les morceaux, donc la même découpe ; un jeton répété garde son plus fort poids.
pub fn query_token_weights(query: &str, spans: &[SparseSpan]) -> HashMap<String, f32> {
    let mut poids: HashMap<String, f32> = HashMap::new();
    for s in spans {
        if let Some((_, _, cle)) = position_nue(query, s.start, s.end) {
            let p = poids.entry(cle).or_insert(0.0);
            *p = p.max(s.weight);
        }
    }
    poids
}

/// **Les positions d'un morceau qui portent le creux, placées dans son texte.** `spans` : les
/// positions du morceau rendues par le service (`[début, fin)` en octets du texte envoyé, le
/// poids de la position) ; `requete` : le poids des jetons de la requête, par leur texte nu. La
/// part d'une position est le produit de son poids et de celui du même jeton dans la requête ;
/// celles dont la part atteint [`PART_MINIMALE`] de la plus forte sont surlignées, sans leur
/// espace de tête et étendues à leur mot entier (« Catalog » est trois positions : C, ata, log).
pub fn sparse_highlights(text: &str, spans: &[SparseSpan], requete: &HashMap<String, f32>) -> Vec<Highlight> {
    let parts: Vec<(usize, usize, f32)> = spans
        .iter()
        .filter_map(|s| {
            let (a, b, cle) = position_nue(text, s.start, s.end)?;
            let q = *requete.get(&cle)?;
            let p = q * s.weight;
            (p > 0.0).then_some((a, b, p))
        })
        .collect();
    let Some(plus_forte) = parts.iter().map(|&(_, _, p)| p).reduce(f32::max) else {
        return Vec::new();
    };
    let mut passages: Vec<(usize, usize, &str)> = Vec::new();
    for &(mut a, mut b, p) in &parts {
        if p < PART_MINIMALE * plus_forte {
            continue;
        }
        while let Some(c) = text[..a].chars().next_back().filter(|c| c.is_alphanumeric()) {
            a -= c.len_utf8();
        }
        while let Some(c) = text[b..].chars().next().filter(|c| c.is_alphanumeric()) {
            b += c.len_utf8();
        }
        passages.push((a, b, &text[a..b]));
    }
    chunk_highlights(text, 0, text.len(), &passages, HIGHLIGHT_SPARSE)
}

/// **Le creux de la requête et des morceaux, en un appel borné, par les spans du service**
/// (7 novembre 2026, forme retenue par c7) : le service tokenise et rend le poids de chaque
/// position ; l'hôte n'a ni tokenizer ni modèle. Rend, pour chaque texte, ses passages ; ou la
/// raison du repli (aucun service creux, un service d'avant l'option `spans`, pas de réponse dans
/// le délai).
fn surligner_par_le_creux(
    embarqueur: Option<Arc<dyn SparseEmbedder>>,
    requete: &str,
    textes: &[String],
) -> Result<Vec<Vec<Highlight>>, String> {
    let embarqueur = embarqueur.ok_or("aucun embarqueur creux configuré")?;
    let mut envoi: Vec<String> = Vec::with_capacity(textes.len() + 1);
    envoi.push(requete.to_string());
    envoi.extend(textes.iter().cloned());
    let attendu = envoi.len();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(embarqueur.embed_sparse_spans(&envoi));
    });
    let rendus = match rx.recv_timeout(DELAI_DU_CREUX) {
        Ok(Ok(v)) if v.len() == attendu => v,
        Ok(Ok(v)) => return Err(format!("le service a rendu {} réponses creuses pour {attendu} textes", v.len())),
        Ok(Err(e)) => return Err(format!("le service creux ne rend pas les positions ({e})")),
        Err(_) => return Err(format!("le service creux n'a pas répondu en {} ms", DELAI_DU_CREUX.as_millis())),
    };
    let requete_poids = query_token_weights(requete, &rendus[0].1);
    Ok(textes
        .iter()
        .zip(&rendus[1..])
        .map(|(texte, (_, spans))| sparse_highlights(texte, spans, &requete_poids))
        .collect())
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
                    let passages = surligner_par_le_creux(embarqueur, &qp.query, &textes)
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

    fn spans(texte: &str, plages: &[(usize, usize, f32)]) -> Vec<SparseSpan> {
        plages
            .iter()
            .map(|&(start, end, weight)| {
                assert!(texte.get(start..end).is_some(), "{start}..{end} dans {texte:?}");
                SparseSpan { start, end, weight }
            })
            .collect()
    }

    #[test]
    fn le_creux_surligne_les_positions_qui_portent_le_produit() {
        // La découpe du service : « ␣fu » commence sur l'espace qui le précède, « fusion » est en
        // deux positions ; « des » pèse peu dans la requête.
        let requete = "fusion des rangs";
        let q = query_token_weights(requete, &spans(requete, &[(0, 2, 0.3), (2, 6, 0.3), (6, 10, 0.01), (10, 16, 0.25)]));
        let texte = "la fusion des rangs";
        let d = spans(texte, &[(0, 2, 0.2), (2, 5, 0.2), (5, 9, 0.2), (9, 13, 0.2), (13, 19, 0.2)]);
        let h = sparse_highlights(texte, &d, &q);
        let vus: Vec<String> = h.iter().map(|x| en_utf16(texte, x)).collect();
        assert_eq!(vus, vec!["fusion", "rangs"], "les positions d'un mot se fondent, l'espace de tête part, « des » reste : {vus:?}");
        assert!(h.iter().all(|x| x.source == HIGHLIGHT_SPARSE));
        // Une position seule s'étend à son mot (« ipr » dans « réciproque »).
        let requete = "réciproque";
        let q = query_token_weights(requete, &spans(requete, &[(0, 4, 0.0), (4, 7, 0.4), (7, 11, 0.0)]));
        let texte = "le rang réciproque";
        let d = spans(texte, &[(0, 2, 0.1), (2, 7, 0.1), (7, 12, 0.0), (12, 15, 0.3), (15, 19, 0.0)]);
        let h = sparse_highlights(texte, &d, &q);
        assert_eq!(h.iter().map(|x| en_utf16(texte, x)).collect::<Vec<_>>(), vec!["réciproque"]);
    }

    /// **Par le factice d'embarquement** (`MockSparseEmbedder`, un span par mot, poids 1/n) :
    /// l'appel entier, la requête et les morceaux d'un coup, sans modèle ni tokenizer.
    #[test]
    fn le_creux_passe_par_les_spans_du_factice() {
        let embarqueur: Arc<dyn SparseEmbedder> = Arc::new(crate::embedder::MockSparseEmbedder::new());
        let textes = vec!["la fusion des rangs".to_string(), "rien à voir ici".to_string()];
        let rendus = surligner_par_le_creux(Some(embarqueur), "fusion rangs", &textes).expect("le factice rend ses spans");
        let vus: Vec<String> = rendus[0].iter().map(|x| en_utf16(&textes[0], x)).collect();
        assert_eq!(vus, vec!["fusion", "rangs"], "{vus:?}");
        assert!(rendus[0].iter().all(|x| x.source == HIGHLIGHT_SPARSE));
        assert!(rendus[1].is_empty(), "un morceau sans jeton de la requête ne surligne rien : {:?}", rendus[1]);
    }

    /// Un embarqueur creux qui ne rend pas les positions (un service d'avant l'option) : la raison
    /// du repli, pas un surlignage inventé.
    #[test]
    fn sans_positions_le_creux_dit_pourquoi_il_se_replie() {
        struct SansSpans;
        impl SparseEmbedder for SansSpans {
            fn embed_sparse(&self, textes: &[String]) -> Result<Vec<crate::sparse_index::SparseVector>, crate::embedder::EmbedError> {
                Ok(textes.iter().map(|_| crate::sparse_index::SparseVector { indices: vec![1], values: vec![1.0] }).collect())
            }
        }
        let e = surligner_par_le_creux(Some(Arc::new(SansSpans)), "fusion", &["la fusion".to_string()]).unwrap_err();
        assert!(e.contains("ne rend pas les positions"), "{e}");
    }

    #[test]
    fn sans_jeton_partage_le_creux_ne_surligne_rien() {
        let q = query_token_weights("abc", &spans("abc", &[(0, 3, 0.5)]));
        assert!(sparse_highlights("xyz", &spans("xyz", &[(0, 3, 0.5)]), &q).is_empty());
    }

    /// **La sonde du vrai chemin** : le service creux (`RAG3WEAVER_SONDE_CREUX`, une adresse),
    /// ses spans et le placement, sans tokenizer chez nous. À jouer nommément
    /// (`-- --ignored sonde_le_creux_reel --nocapture`) ; sans l'adresse, elle échoue au lieu de
    /// se sauter.
    #[cfg(feature = "daemon")]
    #[test]
    #[ignore]
    fn sonde_le_creux_reel() {
        let adresse = std::env::var("RAG3WEAVER_SONDE_CREUX").expect("RAG3WEAVER_SONDE_CREUX : l'adresse du service bge-m3");
        let service = crate::daemon::embeddings::DaemonEmbedder::joindre(&adresse).expect("le service répond");
        let embarqueur: Arc<dyn SparseEmbedder> = Arc::new(service);
        let requete = "fusion des rangs réciproques";
        let textes = vec![
            "/// La fusion RRF : chaque signal pèse 1/(k + rang), k = 60.\npub fn fuse_signals(listes: &[Vec<Resultat>]) -> Vec<Resultat> {".to_string(),
            "Le rang réciproque d'un document additionne ses places dans chaque liste.".to_string(),
            "pub fn ouvrir_la_base(chemin: &Path) -> Result<Base, Erreur> {".to_string(),
        ];
        let t = std::time::Instant::now();
        let rendus = surligner_par_le_creux(Some(embarqueur), requete, &textes).expect("le creux répond");
        eprintln!("[sonde creux] {} ms pour {} textes", t.elapsed().as_millis(), textes.len());
        for (texte, h) in textes.iter().zip(&rendus) {
            let vus: Vec<String> = h.iter().map(|x| format!("{}:{}", x.source, en_utf16(texte, x))).collect();
            eprintln!("[sonde creux] {vus:?} dans {texte:?}");
        }
        let premiers: Vec<String> = rendus[0].iter().map(|x| en_utf16(&textes[0], x).to_lowercase()).collect();
        assert!(
            rendus[0].iter().all(|x| x.source == HIGHLIGHT_SPARSE) && premiers.iter().any(|p| p.contains("fusion")),
            "le creux surligne « fusion » dans le premier texte : {premiers:?}"
        );
    }

}
