//! **Les générations : la visibilité des lignes** (2a′, page
//! `docs/everything-declarative/14-11-2026/03-le-pas-2a-prime-stocker-les-generations-sans-fuite.md`).
//!
//! Une table à générations porte `_couche` (la suivie à laquelle la ligne appartient), `_depuis` (le
//! rang de la génération qui l'a posée) et `_jusqu_a` (le rang de celle qui l'a retirée, vide tant
//! qu'elle vit). Une lecture voit une couche, à un rang : une ligne y est visible si elle est de
//! cette couche, posée au plus tard à ce rang, et pas encore retirée.
//!
//! Le prédicat entre aux points d'exécution des lectures, et pas dans chaque requête écrite : un
//! plan ([`Visibilite::borner_plan`], par `run_read_plan`), un saut ([`Visibilite::borner_hop`], par
//! `graph_walk::run_hop`), une sélection ou un compte que le code traduit lui-même.

use rag3weaver_ir::{Count, Form, Hop, Plan, Predicate, Select};
use crate::connection::CypherValue;
use std::collections::{BTreeMap, BTreeSet};

/// **La table des générations** : une ligne par génération publiée d'une couche (page 03, § 2).
/// Interne au catalogue, comme `_catalog_meta` : elle n'est pas une entité des outils.
pub const TABLE: &str = "_generation";

/// La colonne de la couche d'une ligne.
pub const COUCHE: &str = "_couche";
/// La colonne du rang qui a posé la ligne.
pub const DEPUIS: &str = "_depuis";
/// La colonne du rang qui l'a retirée (vide : vivante).
pub const JUSQU_A: &str = "_jusqu_a";

/// **Ce qu'une lecture voit** : les tables à générations, la couche lue, le rang lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visibilite {
    pub tables: BTreeSet<String>,
    pub couche: String,
    pub rang: i64,
    /// **Toutes les lignes sont visibles** : aucune ligne d'une autre couche, ni retirée, ni posée
    /// après `rang`. C'est l'état de toute base de 2a′-1, dont rien n'écrit encore une seconde
    /// génération ; l'écrivain qui en pose une le défait (2a′-2). Il ne sert qu'à un lecteur sans
    /// filtre, qui rendrait sinon l'ensemble des lignes visibles au lieu de « toutes » (page 03, § 3).
    pub tout_visible: bool,
}

impl Visibilite {
    /// Le prédicat de visibilité d'une table, si elle a des générations.
    pub fn predicat(&self, table: &str) -> Option<Predicate> {
        self.tables.contains(table).then(|| Predicate::Visible { couche: self.couche.clone(), rang: self.rang })
    }

    fn et(filtre: Option<Predicate>, visible: Predicate) -> Predicate {
        match filtre {
            None => visible,
            Some(Predicate::AllOf(mut v)) => {
                v.push(visible);
                Predicate::AllOf(v)
            }
            Some(f) => Predicate::AllOf(vec![f, visible]),
        }
    }

    /// La clause SQL de visibilité de `table` sur `alias`, si elle a des générations.
    pub fn clause_sur(&self, dialect: &dyn crate::dialect::SchemaDialect, table: &str, alias: &str) -> Option<String> {
        if !self.tables.contains(table) {
            return None;
        }
        dialect.condition_visible_sur(alias, &self.couche, self.rang)
    }

    /// Une sélection bornée aux lignes visibles de sa table.
    pub fn borner_select(&self, s: &Select) -> Select {
        let mut s = s.clone();
        if let Some(v) = self.predicat(&s.table) {
            s.filter = Some(Self::et(s.filter.take(), v));
        }
        s
    }

    /// **Un saut borné au nœud atteint.** Ses départs sont donnés par uuid, depuis des lignes déjà
    /// lues sous la même visibilité. Un saut vers n'importe quelle table (`end: None`) n'est pas
    /// borné ici : ses lecteurs le sont un par un (page 03, § 3). Les arêtes n'ont pas de prédicat
    /// avant 2a′-2 : tant que les lignes ne sont pas immuables, une arête n'est jamais retirée alors
    /// que ses deux bouts restent visibles.
    pub fn borner_hop(&self, h: &Hop) -> Hop {
        let mut h = h.clone();
        if let Some(v) = h.end.as_deref().and_then(|t| self.predicat(t)) {
            h.filter = Some(Self::et(h.filter.take(), v));
        }
        h
    }

    /// Un compte borné aux lignes visibles. Le degré (`Edges`) l'est par 2a′-2, avec les arêtes.
    pub fn borner_count(&self, c: &Count) -> Count {
        match c {
            Count::Rows { table } => match self.predicat(table) {
                Some(v) => Count::Matching { table: table.clone(), filter: Some(v) },
                None => c.clone(),
            },
            Count::Matching { table, filter } => match self.predicat(table) {
                Some(v) => Count::Matching { table: table.clone(), filter: Some(Self::et(filter.clone(), v)) },
                None => c.clone(),
            },
            autre => autre.clone(),
        }
    }

    /// Un plan dont chaque lecture est bornée ; ses écritures ne le sont pas.
    pub fn borner_plan(&self, p: &Plan) -> Plan {
        let mut p = p.clone();
        for n in &mut p.nodes {
            n.form = match &n.form {
                Form::Select(s) => Form::Select(self.borner_select(s)),
                Form::Hop(h) => Form::Hop(self.borner_hop(h)),
                Form::Count(c) => Form::Count(self.borner_count(c)),
                Form::Write(w) => Form::Write(w.clone()),
            };
        }
        p
    }

    /// **Une ligne lue à la main est-elle visible ?** Pour les lecteurs qui écrivent leur requête
    /// sans IR (`get`, `find_by_field`) : la ligne porte ses trois colonnes, sous leur nom ou sous
    /// `alias.nom`. Une ligne qui ne les porte pas est d'une table sans générations.
    pub fn admet(&self, ligne: &BTreeMap<String, CypherValue>) -> bool {
        let champ = |nom: &str| {
            ligne.iter().find(|(k, _)| k.as_str() == nom || k.rsplit_once('.').is_some_and(|(_, c)| c == nom)).map(|(_, v)| v)
        };
        let Some(couche) = champ(COUCHE) else { return true };
        let entier = |v: Option<&CypherValue>| v.and_then(|v| v.as_i64());
        couche.as_str() == Some(self.couche.as_str())
            && entier(champ(DEPUIS)).is_some_and(|d| d <= self.rang)
            && entier(champ(JUSQU_A)).is_none_or(|j| j > self.rang)
    }
}

/// Les champs d'un `find_by_field` avec ceux de la visibilité quand la table en a, et ceux qui ont
/// été ajoutés (pour les ôter du résultat).
pub fn champs_et_visibilite<'a>(v: Option<&Visibilite>, table: &str, fields: &[&'a str]) -> (Vec<&'a str>, Vec<&'static str>) {
    let mut f = fields.to_vec();
    let mut ajoutes = Vec::new();
    if v.is_some_and(|v| v.tables.contains(table)) {
        for c in [COUCHE, DEPUIS, JUSQU_A] {
            if !f.contains(&c) {
                f.push(c);
                ajoutes.push(c);
            }
        }
    }
    (f, ajoutes)
}

/// Les lignes visibles d'un `find_by_field`, sans les champs que [`champs_et_visibilite`] a ajoutés.
pub fn garder_les_visibles(
    v: Option<&Visibilite>,
    table: &str,
    lignes: Vec<BTreeMap<String, CypherValue>>,
    ajoutes: &[&str],
) -> Vec<BTreeMap<String, CypherValue>> {
    let Some(v) = v.filter(|v| v.tables.contains(table)) else { return lignes };
    let base = |k: &str| k.rsplit_once('.').map_or(k, |(_, c)| c).to_string();
    lignes
        .into_iter()
        .filter(|l| v.admet(l))
        .map(|mut l| {
            l.retain(|k, _| !ajoutes.contains(&base(k).as_str()));
            l
        })
        .collect()
}
