//! **Un graphe de formes**, confié entier au dialecte. Un nœud est une forme
//! ([`Form`]) ; une arête dit que des colonnes rendues par un nœud
//! deviennent l'entrée d'un autre. Le dialecte décide ensuite ce qu'il
//! fond en une instruction et ce qu'il exécute en plusieurs ; voir
//! `docs/11-octobre-2026-embarquements/01-chainage-des-formes.md`.
//!
//! Ici : la structure et la **vérification commune** à tous les dialectes
//! (pas de cycle, des colonnes qui existent, des types qui conviennent), faite
//! une fois, avant tout dialecte.

use std::collections::{BTreeMap, BTreeSet};

use crate::{Column, Count, Hop, QueryParam, Select, TranslateError, Write};

/// Une forme, comme nœud d'un graphe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Form {
    Hop(Hop),
    Count(Count),
    Select(Select),
    Write(Write),
}

/// La nature d'une colonne rendue, pour savoir où elle peut entrer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    /// L'uuid d'une ligne : peut nourrir une entrée `$uuids`.
    Uuid,
    /// Un champ quelconque.
    Value,
    /// Un compte.
    Count,
    /// Une ligne entière.
    Whole,
}

/// L'entrée d'une forme qu'une arête peut nourrir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Le paramètre `$uuids` : une liste d'uuids.
    Uuids,
    /// Le paramètre `$items` : une carte par ligne.
    Items,
}

impl Form {
    /// Le nom de la forme, tel que le refus d'un dialecte le cite.
    pub fn name(&self) -> &'static str {
        match self {
            Form::Hop(_) => "hop",
            Form::Count(_) => "count",
            Form::Select(_) => "select",
            Form::Write(w) => match w {
                Write::Upsert { .. } => "write.upsert",
                Write::Link { .. } => "write.link",
                Write::Update { .. } => "write.update",
                Write::Mark { .. } => "write.mark",
                Write::Delete { .. } => "write.delete",
                Write::Unlink { .. } => "write.unlink",
                Write::Load { .. } => "write.load",
                Write::Detach { .. } => "write.detach",
            },
        }
    }

    pub fn validate(&self) -> Result<(), TranslateError> {
        match self {
            Form::Hop(h) => h.validate(),
            Form::Count(c) => c.validate(),
            Form::Select(s) => s.validate(),
            Form::Write(w) => w.validate(),
        }
    }

    /// L'entrée que la forme lit, s'il y en a une.
    pub fn slot(&self) -> Option<Slot> {
        match self {
            Form::Hop(_) => Some(Slot::Uuids),
            Form::Count(Count::Edges { .. }) => Some(Slot::Uuids),
            Form::Count(_) => None,
            Form::Select(s) => s.by_uuids.then_some(Slot::Uuids),
            Form::Write(Write::Mark { .. } | Write::Delete { .. }) => Some(Slot::Uuids),
            Form::Write(Write::Load { .. } | Write::Detach { .. }) => None,
            Form::Write(_) => Some(Slot::Items),
        }
    }

    /// Les colonnes rendues, dans l'ordre, par leur nature.
    pub fn output(&self) -> Vec<ColumnKind> {
        fn of(c: &Column) -> ColumnKind {
            match c {
                Column::Node(f) if f == "_uuid" => ColumnKind::Uuid,
                Column::Whole => ColumnKind::Whole,
                Column::Node(_) | Column::Edge(_) | Column::Null | Column::Label | Column::Collect(_) => ColumnKind::Value,
            }
        }
        match self {
            Form::Hop(h) => std::iter::once(ColumnKind::Uuid).chain(h.returns.iter().map(of)).collect(),
            Form::Count(Count::Edges { .. }) => vec![ColumnKind::Uuid, ColumnKind::Count],
            Form::Count(_) => vec![ColumnKind::Count],
            Form::Select(s) => {
                let tete = s.by_uuids.then_some(ColumnKind::Uuid);
                tete.into_iter().chain(s.returns.iter().map(of)).collect()
            }
            Form::Write(Write::Upsert { .. }) => vec![ColumnKind::Value, ColumnKind::Uuid],
            Form::Write(Write::Delete { by, count: true, .. }) => {
                vec![if by == "_uuid" { ColumnKind::Uuid } else { ColumnKind::Value }, ColumnKind::Count]
            }
            Form::Write(_) => Vec::new(),
        }
    }

    pub fn is_write(&self) -> bool {
        matches!(self, Form::Write(_))
    }
}

/// Un nœud : une forme, et les paramètres que l'appelant lui donne (ceux
/// qu'une arête ne fournit pas : les valeurs d'un filtre, `$uuids` d'un
/// départ, `$items` d'une écriture).
#[derive(Debug, Clone)]
pub struct PlanNode {
    pub id: String,
    pub form: Form,
    pub params: Vec<QueryParam>,
}

/// Une arête : les colonnes `columns` (des positions) rendues par `from`
/// nourrissent l'entrée de `to`. **Zéro colonne** serait une arête d'ordre
/// seul : le type l'admet, la vérification la refuse pour l'instant
/// (décision de Lucie, 11 octobre 2026).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub columns: Vec<usize>,
}

/// **Un graphe de formes.** `outputs` : les nœuds dont l'appelant lit les
/// lignes.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub nodes: Vec<PlanNode>,
    pub edges: Vec<Edge>,
    pub outputs: Vec<String>,
}

impl Plan {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajouter un nœud ; rend le plan pour enchaîner.
    pub fn node(mut self, id: impl Into<String>, form: Form, params: Vec<QueryParam>) -> Self {
        self.nodes.push(PlanNode { id: id.into(), form, params });
        self
    }

    /// Ajouter une arête d'une colonne.
    pub fn edge(mut self, from: impl Into<String>, column: usize, to: impl Into<String>) -> Self {
        self.edges.push(Edge { from: from.into(), to: to.into(), columns: vec![column] });
        self
    }

    pub fn output(mut self, id: impl Into<String>) -> Self {
        self.outputs.push(id.into());
        self
    }

    pub fn get(&self, id: &str) -> Option<&PlanNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// L'arête qui nourrit l'entrée de `id`, s'il y en a une.
    pub fn input_of(&self, id: &str) -> Option<&Edge> {
        self.edges.iter().find(|e| e.to == id)
    }

    /// **La vérification commune**, sans dialecte : chaque forme est
    /// valide, les identifiants des nœuds sont uniques, chaque arête relie
    /// deux nœuds connus et désigne des colonnes qui existent et conviennent
    /// à l'entrée, une entrée n'a qu'une arête, le graphe n'a pas de cycle,
    /// et chaque sortie désigne un nœud. Rend les nœuds dans un ordre
    /// topologique (stable : l'ordre de déclaration quand rien ne l'impose).
    pub fn check(&self) -> Result<Vec<&PlanNode>, TranslateError> {
        let invalide = |m: String| Err(TranslateError::Invalid(format!("graphe : {m}")));
        let mut vus = BTreeSet::new();
        for n in &self.nodes {
            if !vus.insert(n.id.as_str()) {
                return invalide(format!("deux nœuds s'appellent « {} »", n.id));
            }
            if let Err(e) = n.form.validate() {
                return invalide(format!("nœud « {} » : {e}", n.id));
            }
        }
        let mut nourris = BTreeSet::new();
        for e in &self.edges {
            let (Some(de), Some(vers)) = (self.get(&e.from), self.get(&e.to)) else {
                let absent = if self.get(&e.from).is_none() { &e.from } else { &e.to };
                return invalide(format!("une arête désigne le nœud « {absent} », qui n'existe pas"));
            };
            if e.columns.is_empty() {
                return invalide(format!(
                    "l'arête « {} » → « {} » ne porte aucune colonne : une arête d'ordre seul n'est pas encore admise",
                    e.from, e.to
                ));
            }
            if !nourris.insert(e.to.as_str()) {
                return invalide(format!("l'entrée de « {} » est nourrie par deux arêtes", e.to));
            }
            let sortie = de.form.output();
            for &c in &e.columns {
                if c >= sortie.len() {
                    return invalide(format!(
                        "l'arête « {} » → « {} » désigne la colonne {c}, et « {} » n'en rend que {}",
                        e.from, e.to, e.from, sortie.len()
                    ));
                }
            }
            match vers.form.slot() {
                None => return invalide(format!("« {} » ({}) n'a pas d'entrée à nourrir", e.to, vers.form.name())),
                Some(Slot::Items) => {
                    return invalide(format!(
                        "l'entrée `$items` de « {} » ({}) ne se nourrit pas encore d'une arête",
                        e.to,
                        vers.form.name()
                    ))
                }
                Some(Slot::Uuids) => {
                    if e.columns.len() != 1 || sortie[e.columns[0]] != ColumnKind::Uuid {
                        return invalide(format!(
                            "l'entrée `$uuids` de « {} » attend une colonne d'uuids, et l'arête depuis « {} » donne {:?}",
                            e.to,
                            e.from,
                            e.columns.iter().map(|&c| sortie[c]).collect::<Vec<_>>()
                        ));
                    }
                }
            }
        }
        for o in &self.outputs {
            if self.get(o).is_none() {
                return invalide(format!("la sortie « {o} » ne désigne aucun nœud"));
            }
        }
        // Kahn, en reprenant toujours le premier nœud prêt dans l'ordre de
        // déclaration.
        let mut entrants: BTreeMap<&str, usize> = self.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
        for e in &self.edges {
            *entrants.get_mut(e.to.as_str()).expect("vérifié plus haut") += 1;
        }
        let mut ordre = Vec::with_capacity(self.nodes.len());
        let mut faits = BTreeSet::new();
        while ordre.len() < self.nodes.len() {
            let Some(n) = self.nodes.iter().find(|n| !faits.contains(n.id.as_str()) && entrants[n.id.as_str()] == 0) else {
                let reste: Vec<&str> = self.nodes.iter().map(|n| n.id.as_str()).filter(|id| !faits.contains(id)).collect();
                return invalide(format!("un cycle passe par {}", reste.join(", ")));
            };
            faits.insert(n.id.as_str());
            for e in self.edges.iter().filter(|e| e.from == n.id) {
                *entrants.get_mut(e.to.as_str()).expect("vérifié plus haut") -= 1;
            }
            ordre.push(n);
        }
        Ok(ordre)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Direction;

    fn sel() -> Form {
        Form::Select(Select::by_uuids("Order", vec![Column::Node("_uuid".into())]))
    }
    fn hop() -> Form {
        Form::Hop(Hop::new("Order", "defines", "Symbol", Direction::Outgoing))
    }
    fn deg() -> Form {
        Form::Count(Count::Edges { start: "Symbol".into(), relation: "uses".into(), direction: Direction::Incoming })
    }
    fn refus(p: &Plan) -> String {
        match p.check() {
            Err(TranslateError::Invalid(m)) => m,
            autre => panic!("refus attendu, rendu {autre:?}"),
        }
    }

    #[test]
    fn une_chaine_et_un_noeud_qui_en_nourrit_deux_passent_dans_l_ordre() {
        let p = Plan::new()
            .node("deg", deg(), vec![])
            .node("hop", hop(), vec![])
            .node("sel", sel(), vec![])
            .node("deg2", deg(), vec![])
            .edge("sel", 0, "hop")
            .edge("hop", 1, "deg")
            .edge("sel", 0, "deg2")
            .output("deg")
            .output("deg2");
        let ordre: Vec<&str> = p.check().unwrap().iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ordre, ["sel", "hop", "deg", "deg2"]);
    }

    #[test]
    fn un_cycle_est_refuse_en_nommant_ses_noeuds() {
        let p = Plan::new().node("a", hop(), vec![]).node("b", hop(), vec![]).edge("a", 1, "b").edge("b", 1, "a");
        assert_eq!(refus(&p), "graphe : un cycle passe par a, b");
    }

    #[test]
    fn une_colonne_qui_n_existe_pas_est_refusee() {
        let p = Plan::new().node("sel", sel(), vec![]).node("hop", hop(), vec![]).edge("sel", 5, "hop");
        assert!(refus(&p).contains("désigne la colonne 5, et « sel » n'en rend que 2"), "{}", refus(&p));
    }

    #[test]
    fn un_compte_n_entre_pas_dans_des_uuids() {
        let p = Plan::new().node("deg", deg(), vec![]).node("hop", hop(), vec![]).edge("deg", 1, "hop");
        assert!(refus(&p).contains("attend une colonne d'uuids"), "{}", refus(&p));
    }

    #[test]
    fn une_arete_d_ordre_seul_n_est_pas_encore_admise() {
        let mut p = Plan::new().node("sel", sel(), vec![]).node("hop", hop(), vec![]);
        p.edges.push(Edge { from: "sel".into(), to: "hop".into(), columns: vec![] });
        assert!(refus(&p).contains("arête d'ordre seul n'est pas encore admise"), "{}", refus(&p));
    }

    #[test]
    fn une_entree_n_a_qu_une_arete() {
        let p = Plan::new().node("a", sel(), vec![]).node("b", sel(), vec![]).node("hop", hop(), vec![]).edge("a", 0, "hop").edge("b", 0, "hop");
        assert!(refus(&p).contains("nourrie par deux arêtes"), "{}", refus(&p));
    }

    #[test]
    fn un_noeud_inconnu_une_sortie_inconnue_un_nom_en_double_une_forme_fausse() {
        assert!(refus(&Plan::new().node("a", sel(), vec![]).edge("a", 0, "z")).contains("« z », qui n'existe pas"));
        assert!(refus(&Plan::new().node("a", sel(), vec![]).output("z")).contains("la sortie « z »"));
        assert!(refus(&Plan::new().node("a", sel(), vec![]).node("a", hop(), vec![])).contains("deux nœuds s'appellent « a »"));
        let faux = Form::Select(Select::by_uuids("a`b", vec![]));
        assert!(refus(&Plan::new().node("f", faux, vec![])).starts_with("graphe : nœud « f » : sélection"));
    }

    #[test]
    fn une_ecriture_a_items_et_un_compte_sans_entree_ne_se_nourrissent_pas() {
        let up = Form::Write(Write::Upsert { table: "Order".into(), columns: vec!["_uuid".into()] });
        let p = Plan::new().node("sel", sel(), vec![]).node("up", up, vec![]).edge("sel", 0, "up");
        assert!(refus(&p).contains("`$items` de « up » (write.upsert) ne se nourrit pas encore"), "{}", refus(&p));
        let rows = Form::Count(Count::Rows { table: "Order".into() });
        let p = Plan::new().node("sel", sel(), vec![]).node("n", rows, vec![]).edge("sel", 0, "n");
        assert!(refus(&p).contains("« n » (count) n'a pas d'entrée"), "{}", refus(&p));
    }
}
