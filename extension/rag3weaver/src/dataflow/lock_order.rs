//! **Les verrous du chemin, dans l'ordre** (page des manques de codeparsers,
//! 11 octobre 2026, 15 h 40, points 4 et 5) :
//!
//! - **pris par le départ** : son contenu se lit ligne à ligne ; une ligne qui
//!   porte un verrou (la relation déclarée, `LOCKS` pour le code) est un
//!   verrou pris là ; une ligne qui porte un appel (`CONSUMES`) se déplie à sa
//!   place en ce que l'appelé prend, borné par la profondeur, avec une garde
//!   de cycle. Chaque verrou vient avec la ligne de code du site (la ressource
//!   et le mode tels qu'écrits), son lieu, et la condition qui l'enveloppe ;
//! - **tenus par les appelants avant l'appel** : pour chaque appelant
//!   atteint, ses verrous dont la ligne précède celle de son appel. Une
//!   approximation, dite au rendu : un verrou est supposé tenu jusqu'à la fin
//!   du scope, un `unlock` manuel n'est pas suivi.
//!
//! Rien n'est propre à un langage : des relations, la ligne portée par
//! l'arête, et le texte des scopes.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::graph_walk::{Direction, EdgeMark};
use super::usage_nodes::rel_info;
use crate::catalog::Catalog;
use crate::connection::{CypherValue, QueryParam};
use rag3weaver_ir::Column;

/// Ce que le gabarit déclare.
#[derive(Debug, Clone)]
pub struct LockOrder {
    /// La relation d'un verrou pris (`LOCKS`), vers ce qui est verrouillé.
    pub locks: String,
    /// La relation d'un appel (`CONSUMES`), vers l'appelé.
    pub calls: String,
    /// Le champ de l'arête qui porte la ligne du site (`line`), et celui du
    /// genre (`usage`).
    pub line: String,
    pub usage: String,
    /// Les champs du scope : son texte, sa première ligne, son titre, ses
    /// chemins.
    pub content: String,
    pub start_line: String,
    pub title: String,
    pub path_fields: Vec<String>,
    /// Le nom de ce qui est verrouillé (`name` du symbole).
    pub target_field: String,
    /// Les titres des deux sections.
    pub order_title: String,
    pub held_title: String,
}

/// Les scopes lus au plus par un dépliage.
pub const MAX_SCOPES: usize = 200;

/// Un verrou pris par le départ, à sa place.
#[derive(Debug, Clone, Serialize)]
pub struct LockLine {
    /// 0 : le départ ; 1 : un appelé direct ; …
    pub nesting: usize,
    /// Le scope qui le prend.
    pub scope: String,
    pub path: String,
    pub line: Option<i64>,
    /// La ligne de code du site.
    pub text: String,
    /// Les conditions qui l'enveloppent, de l'appel du départ jusqu'au site,
    /// jointes par « ; » (vide : aucune) : un verrou pris sans condition
    /// dans un appelé appelé sous `if (usesLocks())` hérite de celle-ci.
    pub condition: String,
    /// Ce qui est verrouillé, et le genre.
    pub target: String,
    pub usage: String,
}

/// Un verrou tenu par un appelant avant son appel.
#[derive(Debug, Clone, Serialize)]
pub struct HeldLock {
    pub caller: String,
    pub path: String,
    pub call_line: Option<i64>,
    pub line: Option<i64>,
    pub text: String,
    pub target: String,
    pub usage: String,
}

/// Ce que la lecture rend : les verrous du départ, combien d'appels au-delà
/// de la profondeur prennent un verrou, et ceux des appelants.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LockReport {
    pub order: Vec<LockLine>,
    pub cut_by_depth: usize,
    pub held: Vec<HeldLock>,
    /// Les titres des deux sections, repris de la déclaration.
    #[serde(skip)]
    pub titles: (String, String),
}

struct Fiche {
    title: String,
    path: String,
    start: i64,
    content: String,
}

struct Lecteur<'a> {
    catalog: &'a Catalog,
    lo: &'a LockOrder,
    table: String,
    fiches: HashMap<String, Fiche>,
}

fn texte(v: Option<&CypherValue>) -> String {
    v.and_then(|v| v.as_str()).unwrap_or("").to_string()
}

impl Lecteur<'_> {
    /// Le texte et le lieu de ces scopes, par une lecture par clé.
    fn lire(&mut self, uuids: &[String]) -> Result<(), String> {
        let manquants: Vec<String> = uuids.iter().filter(|u| !self.fiches.contains_key(*u)).cloned().collect();
        if manquants.is_empty() {
            return Ok(());
        }
        let mut colonnes = vec![Column::Node(self.lo.title.clone()), Column::Node(self.lo.start_line.clone()), Column::Node(self.lo.content.clone())];
        colonnes.extend(self.lo.path_fields.iter().map(|f| Column::Node(f.clone())));
        let q = self.catalog.dialect_arc().select(&rag3weaver_ir::Select::by_uuids(&self.table, colonnes)).map_err(|e| format!("verrous : {e}"))?;
        let liste = CypherValue::List(manquants.iter().map(|u| CypherValue::String(u.clone())).collect());
        let rows = self.catalog.execute_raw_with_params(&q, &[QueryParam::new("uuids", liste)]).map_err(|e| format!("verrous : {e}"))?;
        for r in rows.rows {
            let path = (0..self.lo.path_fields.len()).map(|i| texte(r.get(4 + i))).rev().find(|p| !p.is_empty()).unwrap_or_default();
            self.fiches.insert(
                texte(r.first()),
                Fiche { title: texte(r.get(1)), start: r.get(2).and_then(|v| v.as_i64()).unwrap_or(0), content: texte(r.get(3)), path },
            );
        }
        Ok(())
    }

    /// Les arêtes sortantes de `rel` depuis ces scopes : (de, vers, titre ou
    /// nom de la cible, ligne, genre).
    fn aretes(&self, rel: &str, depuis: &[String], champ_cible: &str) -> Result<Vec<(String, String, String, Option<i64>, String)>, String> {
        if depuis.is_empty() {
            return Ok(Vec::new());
        }
        let info = rel_info(self.catalog, rel)?;
        let mut hop = super::graph_walk::hop_of(&info, Direction::Outgoing, &EdgeMark::default());
        hop.returns.push(Column::Node(champ_cible.to_string()));
        hop.returns.push(if info.props.iter().any(|p| p == &self.lo.line) { Column::Edge(self.lo.line.clone()) } else { Column::Null });
        hop.returns.push(if info.props.iter().any(|p| p == &self.lo.usage) { Column::Edge(self.lo.usage.clone()) } else { Column::Null });
        let q = self.catalog.dialect_arc().hop(&hop).map_err(|e| format!("verrous : {e}"))?;
        let liste = CypherValue::List(depuis.iter().map(|u| CypherValue::String(u.clone())).collect());
        let rows = self.catalog.execute_raw_with_params(&q, &[QueryParam::new("uuids", liste)]).map_err(|e| format!("verrous : {e}"))?;
        Ok(rows.rows.iter().map(|r| (texte(r.first()), texte(r.get(1)), texte(r.get(2)), r.get(3).and_then(|v| v.as_i64()), texte(r.get(4)))).collect())
    }

    fn ligne(&self, uuid: &str, line: Option<i64>) -> (String, String) {
        let (Some(f), Some(l)) = (self.fiches.get(uuid), line) else { return (String::new(), String::new()) };
        (line_text(&f.content, f.start, l), enclosing_condition(&f.content, f.start, l))
    }

    /// Le dépliage d'un scope, à sa place, dans `out`.
    #[allow(clippy::too_many_arguments)]
    fn deplier(
        &mut self,
        uuid: &str,
        nesting: usize,
        depth: usize,
        chemin: &mut Vec<String>,
        heritees: &[String],
        out: &mut LockReport,
        budget: &mut usize,
    ) -> Result<(), String> {
        if *budget == 0 {
            return Ok(());
        }
        *budget -= 1;
        self.lire(&[uuid.to_string()])?;
        let verrous = self.aretes(&self.lo.locks.clone(), &[uuid.to_string()], &self.lo.target_field.clone())?;
        let appels = self.aretes(&self.lo.calls.clone(), &[uuid.to_string()], &self.lo.title.clone())?;
        enum Evenement {
            Verrou(String, Option<i64>, String),
            Appel(String, Option<i64>),
        }
        let mut evenements: Vec<(i64, Evenement)> = Vec::new();
        for (_, _, cible, line, usage) in verrous {
            evenements.push((line.unwrap_or(i64::MAX), Evenement::Verrou(cible, line, usage)));
        }
        for (_, vers, _, line, _) in appels {
            if vers != uuid {
                evenements.push((line.unwrap_or(i64::MAX), Evenement::Appel(vers, line)));
            }
        }
        evenements.sort_by_key(|(l, _)| *l);
        let mut au_dela: Vec<String> = Vec::new();
        for (_, e) in evenements {
            match e {
                Evenement::Verrou(cible, line, usage) => {
                    let (text, propre) = self.ligne(uuid, line);
                    let condition = heritees.iter().cloned().chain(Some(propre)).filter(|c| !c.is_empty()).collect::<Vec<_>>().join(" ; ");
                    let f = &self.fiches[uuid];
                    out.order.push(LockLine { nesting, scope: f.title.clone(), path: f.path.clone(), line, text, condition, target: cible, usage });
                }
                Evenement::Appel(appele, line) => {
                    if chemin.contains(&appele) {
                        continue;
                    }
                    if nesting + 1 > depth {
                        au_dela.push(appele);
                        continue;
                    }
                    // La condition de l'appel descend avec lui.
                    let (_, condition) = self.ligne(uuid, line);
                    let mut suite = heritees.to_vec();
                    if !condition.is_empty() {
                        suite.push(condition);
                    }
                    chemin.push(appele.clone());
                    self.deplier(&appele, nesting + 1, depth, chemin, &suite, out, budget)?;
                    chemin.pop();
                }
            }
        }
        // Les appels au-delà de la profondeur : comptés s'ils prennent eux-mêmes
        // un verrou — la liste n'est pas finie, et la réponse le dit.
        if !au_dela.is_empty() {
            au_dela.sort();
            au_dela.dedup();
            let qui: HashSet<String> = self.aretes(&self.lo.locks.clone(), &au_dela, &self.lo.target_field.clone())?.into_iter().map(|a| a.0).collect();
            out.cut_by_depth += qui.len();
        }
        Ok(())
    }
}

/// **Les verrous pris par ces départs, dans l'ordre**, dépliés jusqu'à
/// `depth` appels, et ceux que ces appelants (uuid, ligne de leur appel
/// vers le chemin) tiennent avant d'appeler.
pub fn lock_report(catalog: &Catalog, lo: &LockOrder, starts: &[String], depth: usize, callers: &[(String, Option<i64>)]) -> Result<LockReport, String> {
    let table = rel_info(catalog, &lo.calls)?.from;
    let mut lecteur = Lecteur { catalog, lo, table, fiches: HashMap::new() };
    let mut out = LockReport { titles: (lo.order_title.clone(), lo.held_title.clone()), ..Default::default() };
    let mut budget = MAX_SCOPES;
    for s in starts {
        let mut chemin = vec![s.clone()];
        lecteur.deplier(s, 0, depth, &mut chemin, &[], &mut out, &mut budget)?;
    }
    // Les appelants : leurs verrous avant la ligne de leur appel.
    let avec_ligne: Vec<&(String, Option<i64>)> = callers.iter().filter(|(_, l)| l.is_some()).collect();
    if !avec_ligne.is_empty() {
        let uuids: Vec<String> = avec_ligne.iter().map(|(u, _)| u.clone()).collect();
        let verrous = lecteur.aretes(&lo.locks, &uuids, &lo.target_field)?;
        let appel: HashMap<&str, i64> = avec_ligne.iter().filter_map(|(u, l)| l.map(|l| (u.as_str(), l))).collect();
        let tenus: Vec<_> = verrous.into_iter().filter(|(de, _, _, l, _)| matches!((l, appel.get(de.as_str())), (Some(l), Some(a)) if l < a)).collect();
        let qui: Vec<String> = tenus.iter().map(|t| t.0.clone()).collect();
        lecteur.lire(&qui)?;
        for (de, _, cible, line, usage) in tenus {
            let (text, _) = lecteur.ligne(&de, line);
            let f = &lecteur.fiches[&de];
            out.held.push(HeldLock { caller: f.title.clone(), path: f.path.clone(), call_line: appel.get(de.as_str()).copied(), line, text, target: cible, usage });
        }
        out.held.sort_by(|a, b| (&a.path, a.line, &a.caller).cmp(&(&b.path, b.line, &b.caller)));
    }
    Ok(out)
}

/// La ligne `line` (numérotée dans le fichier) d'un scope qui commence à
/// `start`, sans ses blancs.
pub fn line_text(content: &str, start: i64, line: i64) -> String {
    let i = line - start;
    if i < 0 {
        return String::new();
    }
    content.lines().nth(i as usize).map(|l| l.trim().to_string()).unwrap_or_default()
}

/// **La condition qui enveloppe une ligne** : en remontant dans le scope, la
/// première ligne moins indentée qui ouvre une condition ou une boucle
/// (`if`, `else`, `while`, `for`, `switch`, `case`, `match`), sans son
/// accolade. Vide : aucune. Par l'indentation seule — pas de grammaire.
pub fn enclosing_condition(content: &str, start: i64, line: i64) -> String {
    let lignes: Vec<&str> = content.lines().collect();
    let i = line - start;
    if i <= 0 || i as usize >= lignes.len() {
        return String::new();
    }
    let retrait = |l: &str| l.len() - l.trim_start().len();
    let mut seuil = retrait(lignes[i as usize]);
    for j in (0..i as usize).rev() {
        let l = lignes[j];
        let t = l.trim();
        if t.is_empty() {
            continue;
        }
        let k = retrait(l);
        if k < seuil {
            let tete = t.trim_start_matches('}').trim();
            let mot = tete.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
            if ["if", "else", "while", "for", "switch", "case", "match"].contains(&mot) {
                return tete.trim_end_matches('{').trim().to_string();
            }
            seuil = k;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPDATE: &str = "void NodeTable::update(Transaction* t, offset_t o) {\n    if (t->usesLocks() && committed(o)) {\n        lockIndexForWrite(t);\n        lockRowForWrite(t, o);\n    }\n    doUpdate(o);\n}\n";

    /// Le texte d'un site, et la condition qui l'enveloppe ; une ligne hors
    /// de toute condition n'en a pas.
    #[test]
    fn la_ligne_et_sa_condition() {
        assert_eq!(line_text(UPDATE, 10, 12), "lockIndexForWrite(t);");
        assert_eq!(enclosing_condition(UPDATE, 10, 12), "if (t->usesLocks() && committed(o)) {".trim_end_matches('{').trim());
        assert_eq!(enclosing_condition(UPDATE, 10, 15), "");
        assert_eq!(line_text(UPDATE, 10, 9), "");
    }
}
