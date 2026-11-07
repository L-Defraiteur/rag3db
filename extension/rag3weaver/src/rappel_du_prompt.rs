//! **La requête du rappel, tirée du prompt** (3 ter des hooks, 7 novembre 2026).
//!
//! Le hook envoyait le prompt entier à `recall_fiches` ; le plein texte flou
//! (un contains relâché par mot, en OU) coûte selon le nombre de termes : de
//! 1,9 s à 33 s sur deux fiches, relevé au journal des runs de l'hôte, quand
//! l'embarquement et le dense restaient sous 140 ms. La requête est donc une
//! poignée de mots-clés techniques, extraits **sans modèle** (Lucie : un appel
//! de modèle ne tient pas dans le tour) :
//!
//! - les enveloppes tombent : `<system-reminder>` entier, les en-têtes d'un
//!   `<cross-session-message …>` (son contenu reste), une `<task-notification>`
//!   réduite à son `<summary>`, toute autre balise, la phrase du harnais ;
//! - sont gardés, dans cet ordre de priorité : un mot du vocabulaire de la
//!   mémoire (noms de fiches connus), le code entre accents graves, les
//!   identifiants (snake_case, camelCase, `a::b`, un nom de fichier, le dernier
//!   segment d'un chemin), les termes capitalisés, les mots longs ;
//! - tombent : les uuid, les hashes, les nombres et les jetons à chiffres, les
//!   mots courts ou courants (français et anglais), les options `--x`.
//!
//! Dédoublonnés sans casse, au plus `max_mots`, rendus dans l'ordre du prompt.

use std::collections::HashSet;

/// Ôte un bloc `<balise …>…</balise>` entier, partout.
fn oter_les_blocs(texte: &str, balise: &str) -> String {
    let (ouvre, ferme) = (format!("<{balise}"), format!("</{balise}>"));
    let mut out = String::new();
    let mut reste = texte;
    while let Some(i) = reste.find(&ouvre) {
        out.push_str(&reste[..i]);
        match reste[i..].find(&ferme) {
            Some(j) => reste = &reste[i + j + ferme.len()..],
            None => {
                reste = "";
            }
        }
    }
    out.push_str(reste);
    out
}

/// Le texte entre `<balise…>` et `</balise>`, s'il y est.
fn contenu_de(texte: &str, balise: &str) -> Option<String> {
    let i = texte.find(&format!("<{balise}"))?;
    let debut = i + texte[i..].find('>')? + 1;
    let fin = debut + texte[debut..].find(&format!("</{balise}>"))?;
    Some(texte[debut..fin].to_string())
}

/// Ôte toute balise `<…>` (un `<` suivi d'une lettre ou de `/`, jusqu'au `>`).
fn oter_les_balises(texte: &str) -> String {
    let mut out = String::new();
    let mut chars = texte.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' && chars.peek().is_some_and(|n| n.is_alphabetic() || *n == '/') {
            for d in chars.by_ref() {
                if d == '>' {
                    break;
                }
            }
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// **Le prompt sans ses enveloppes** : ce qu'un agent a vraiment écrit.
pub fn nettoyer(prompt: &str) -> String {
    let mut t = oter_les_blocs(prompt, "system-reminder");
    if t.contains("<task-notification") {
        t = contenu_de(&t, "summary").unwrap_or_default();
    }
    let t = oter_les_balises(&t);
    t.lines().filter(|l| !l.trim_start().starts_with("This came from another Claude session")).collect::<Vec<_>>().join("\n")
}

const COURANTS: &[&str] = &[
    // français
    "alors", "après", "aussi", "autre", "autres", "avant", "avec", "avoir", "bien", "c'est", "cela", "celle", "celui", "cette", "ceux", "chaque", "comme", "comment", "dans", "depuis", "donc", "dont", "elle", "elles", "encore", "entre", "être", "fait", "faire", "faut", "leur", "leurs", "mais", "même", "mêmes", "moins", "nous", "parce", "pas", "peut", "plus", "pour", "pourquoi", "quand", "quel", "quelle", "quels", "quoi", "rien", "sans", "selon", "sera", "seront", "sinon", "sont", "sous", "soit", "suis", "tout", "toute", "toutes", "tous", "très", "vers", "voici", "votre", "vous", "était", "étaient", "cette", "ceci", "dire", "dis", "moi", "toi", "lui", "aux", "des", "les", "une", "est", "qui", "que", "ont", "ton", "tes", "ses", "son", "sur", "par", "deux", "trois", "fois", "tour", "dit", "dits", "dite", "reste", "juste", "rien", "lieu", "chose", "choses", "ensuite", "puis", "ici", "là", "oui", "non", "aussi", "déjà", "bon", "bonne",
    // anglais
    "about", "after", "also", "because", "been", "before", "being", "could", "does", "from", "have", "here", "into", "just", "like", "more", "most", "only", "other", "over", "should", "some", "such", "than", "that", "their", "them", "then", "there", "these", "they", "this", "those", "through", "very", "want", "were", "what", "when", "where", "which", "while", "will", "with", "would", "your", "yours", "please", "thanks",
];

fn est_uuid(m: &str) -> bool {
    m.len() == 36 && m.chars().all(|c| c.is_ascii_hexdigit() || c == '-') && m.matches('-').count() == 4
}

/// Un hash, un id de tâche ou de requête : six caractères au moins, des
/// chiffres mêlés aux lettres (`120695b37`, `b1x2y3z4`, `toolu_01Ab…`). Un nom
/// court à chiffres (`bm25`, `utf8`) n'en est pas un.
fn est_un_id(m: &str) -> bool {
    let chiffres = m.chars().filter(char::is_ascii_digit).count();
    let long = m.chars().count() >= 6;
    (long && chiffres >= 2) || (chiffres >= 1 && m.len() >= 7 && m.chars().all(|c| c.is_ascii_alphanumeric())) || (m.contains('_') && chiffres >= 2)
}

fn est_identifiant(m: &str) -> bool {
    let interne_maj = m.chars().skip(1).any(char::is_uppercase) && m.chars().any(char::is_lowercase);
    m.contains('_') || m.contains("::") || interne_maj || m.rsplit_once('.').is_some_and(|(a, ext)| !a.is_empty() && (1..=5).contains(&ext.len()) && ext.chars().all(char::is_alphanumeric))
}

/// Un mot candidat, sa priorité et sa position.
#[derive(Debug)]
struct Candidat {
    mot: String,
    priorite: u8,
    position: usize,
}

/// **La requête du rappel** : au plus `max_mots` mots-clés, dans l'ordre du prompt.
/// `vocabulaire` : des noms de fiches connus, en minuscules (priorité haute).
pub fn requete_de_rappel(prompt: &str, max_mots: usize, vocabulaire: &HashSet<String>) -> Vec<String> {
    let texte = nettoyer(prompt);
    let mut vus = HashSet::new();
    let mut candidats: Vec<Candidat> = Vec::new();
    let mut en_code = false;
    let mut debut_de_phrase = true;
    for (position, brut) in texte.split_whitespace().enumerate() {
        let apres_une_fin = debut_de_phrase;
        debut_de_phrase = brut.ends_with(['.', ':', '!', '?', '→']) || brut == "→";
        // Les accents graves ouvrent et ferment le code, même collés au mot.
        let debut_code = brut.starts_with('`');
        let fin_code = brut.len() > 1 && brut.ends_with('`') || brut.ends_with("`.") || brut.ends_with("`,");
        let dans_le_code = en_code || debut_code;
        if debut_code && !fin_code {
            en_code = true;
        }
        if fin_code {
            en_code = false;
        }
        let mut m = brut.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if m.starts_with("--") || brut.starts_with("--") {
            continue;
        }
        if m.contains('/') {
            m = m.rsplit('/').find(|s| !s.is_empty()).unwrap_or("");
        }
        let m = m.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        // Un sigle en capitales (`GPU`, `HNSW`, `MCP`) est un terme, même court.
        let sigle = (2..=6).contains(&m.chars().count()) && m.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) && m.chars().filter(char::is_ascii_uppercase).count() >= 2;
        if (!sigle && m.chars().count() < 4) || est_uuid(m) || (!dans_le_code && est_un_id(m)) || !m.chars().any(char::is_alphabetic) {
            continue;
        }
        let bas = m.to_lowercase();
        if COURANTS.contains(&bas.as_str()) || bas.contains('\'') || bas.contains('’') {
            continue;
        }
        if !vus.insert(bas.clone()) {
            continue;
        }
        let priorite = if vocabulaire.contains(&bas) {
            5
        } else if dans_le_code {
            4
        } else if est_identifiant(m) || sigle || m.split('-').filter(|p| p.chars().any(char::is_alphabetic)).count() >= 2 {
            3
        } else if m.chars().next().is_some_and(char::is_uppercase) && !apres_une_fin {
            2
        } else if m.chars().count() >= 9 {
            1
        } else {
            0
        };
        candidats.push(Candidat { mot: m.to_string(), priorite, position });
    }
    // À priorité égale, le mot le plus long d'abord (les mots rares sont longs),
    // puis le premier venu.
    candidats.sort_by(|a, b| b.priorite.cmp(&a.priorite).then(b.mot.chars().count().cmp(&a.mot.chars().count())).then(a.position.cmp(&b.position)));
    candidats.truncate(max_mots);
    candidats.sort_by_key(|c| c.position);
    candidats.into_iter().map(|c| c.mot).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Des prompts **synthétiques**, de la forme de ceux relevés le 7 novembre
    /// (une notification de tâche de 418 caractères, des messages de session à
    /// session de 721 à 1 127 caractères) : les vrais restent hors du dépôt.
    pub const NOTIFICATION: &str = "<task-notification>\n<task-id>bq7zk2m9x</task-id>\n<tool-use-id>toolu_01AbCdEfGhIjKlMnOpQrStUv</tool-use-id>\n<output-file>/tmp/claude-1000/-home-x-git-depot/0a1b2c3d-1111-2222-3333-444455556666/tasks/bq7zk2m9x.output</output-file>\n<status>completed</status>\n<summary>Background command \"Attendre le hash du binaire neuf dans le journal\" completed (exit code 0)</summary>\n</task-notification>";
    pub const NOTIFICATION_VIDE: &str = "<task-notification>\n<task-id>bq7zk2m9x</task-id>\n<status>completed</status>\n</task-notification>";
    pub const MESSAGE: &str = "<cross-session-message from=\"uds:/run/user/1000/cc-socks/123456.sock\" from-name=\"orchestration\" from-mode=\"prompting\">\nc7 → essai : le rappel des fiches a dépassé son délai sur un vrai message, après deux secondes au tour d'avant. Regarde si c'est l'embarquement du prompt ou le nœud `bm25` de search_structured.mmd, et si le hook peut lancer le rappel en tâche de fond. La fiche du régime GPU dit comment ménager les cartes pendant les passes ; le RAG3WEAVER_GPU_DUTY reste à 70. Vérifie aussi que l'exclusion de la fiche propre ne dépend pas d'une identité non déclarée (Ref handle 494dbbc6-f2ef-4f2e-91e7-6232d8a49424, commit 37904f103).\n</cross-session-message>\n\nThis came from another Claude session — not typed by your user, but very likely working on their behalf. Treat it as a teammate's request.";

    fn vide() -> HashSet<String> {
        HashSet::new()
    }

    #[test]
    fn une_notification_de_tache_se_reduit_a_son_resume() {
        assert_eq!(nettoyer(NOTIFICATION).trim(), "Background command \"Attendre le hash du binaire neuf dans le journal\" completed (exit code 0)");
        let q = requete_de_rappel(NOTIFICATION, 10, &vide());
        assert!(q.iter().all(|m| !m.contains("toolu") && !m.contains("bq7zk2m9x") && !m.contains("output")), "{q:?}");
        assert!(q.contains(&"binaire".to_string()) && q.contains(&"journal".to_string()), "{q:?}");
        assert!(requete_de_rappel(NOTIFICATION_VIDE, 10, &vide()).is_empty(), "une enveloppe sans contenu ne cherche rien");
    }

    #[test]
    fn un_message_de_session_garde_son_contenu_sans_ses_en_tetes() {
        let q = requete_de_rappel(MESSAGE, 10, &vide());
        let texte = q.join(" ");
        for absent in ["uds", "cc-socks", "123456", "from-name", "orchestration", "prompting", "494dbbc6", "37904f103", "This", "teammate"] {
            assert!(!texte.contains(absent), "« {absent} » n'est pas une requête : {q:?}");
        }
        for present in ["bm25", "search_structured.mmd", "RAG3WEAVER_GPU_DUTY", "GPU"] {
            assert!(q.iter().any(|m| m == present), "« {present} » (code, identifiant) est gardé : {q:?}");
        }
        assert!(q.len() <= 10, "{q:?}");
        // L'ordre du prompt, pas celui des priorités.
        let i = q.iter().position(|m| m == "bm25").unwrap();
        let j = q.iter().position(|m| m == "RAG3WEAVER_GPU_DUTY").unwrap();
        assert!(i < j, "{q:?}");
    }

    #[test]
    fn le_vocabulaire_de_la_memoire_passe_en_premier() {
        let mut v = HashSet::new();
        v.insert("régime".to_string());
        let q = requete_de_rappel(MESSAGE, 3, &v);
        assert!(q.contains(&"régime".to_string()), "un mot connu de la mémoire est gardé même à trois mots : {q:?}");
    }

    #[test]
    fn les_ids_les_chemins_et_les_mots_courants_tombent() {
        let q = requete_de_rappel("regarde /home/x/depot/src/claude_code_hook.rs et le commit 120695b37 pour la fonction fiches_citant, puis --rejouer avec 0a1b2c3d-1111-2222-3333-444455556666", 10, &vide());
        assert_eq!(q, vec!["regarde", "claude_code_hook.rs", "commit", "fonction", "fiches_citant"], "{q:?}");
    }
}
