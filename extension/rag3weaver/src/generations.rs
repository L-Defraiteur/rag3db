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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// **La table des générations** : une ligne par génération publiée d'une couche (page 03, § 2).
/// Interne au catalogue, comme `_catalog_meta` : elle n'est pas une entité des outils.
pub const TABLE: &str = "_generation";

/// La colonne de la couche d'une ligne.
pub const COUCHE: &str = "_couche";
/// La couche des lignes indexées depuis le backend lui-même (ni branche suivie, ni vue) : ses lignes de la
/// génération 0 gardent l'uuid de leur seule clé. Ici et non dans `code`, que l'identité lit sans sa feature.
pub const COUCHE_DU_BACKEND: &str = "backend";
/// La colonne du rang qui a posé la ligne.
pub const DEPUIS: &str = "_depuis";
/// La colonne du rang qui l'a retirée (vide : vivante).
pub const JUSQU_A: &str = "_jusqu_a";

/// **Ce qu'une lecture voit** : les tables à générations, la couche lue, le rang lu.
/// **Les ensembles posés une fois par (couche, rang)** (2a′-2, page
/// `docs/everything-declarative/14-11-2026/05-la-vue-publiee-par-couche-et-rang.md`) : ce qu'une
/// recherche admet, calculé à la publication d'une génération et non à chaque requête. Partagés par
/// le catalogue et ses vues de lecture (la `Visibilite` est copiée, l'`Arc` reste le même).
#[derive(Default)]
pub struct EnsemblesPoses {
    /// Combien de fois le masque des versions a été résolu **en liste** (les références par lots, puis
    /// un `_uuid IN [...]`) : un témoin y lit qu'il ne l'est plus, le masque étant un prédicat.
    pub(crate) resolutions: AtomicUsize,
}

impl EnsemblesPoses {
    /// Le nombre de résolutions du masque des versions en liste depuis l'ouverture.
    pub fn resolutions(&self) -> usize {
        self.resolutions.load(Ordering::Relaxed)
    }
    pub(crate) fn compter_une_resolution(&self) {
        self.resolutions.fetch_add(1, Ordering::Relaxed);
    }
}

impl std::fmt::Debug for EnsemblesPoses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnsemblesPoses").field("resolutions", &self.resolutions()).finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct Visibilite {
    pub tables: BTreeSet<String>,
    pub couche: String,
    pub rang: i64,
    /// **Toutes les lignes sont visibles** : aucune ligne d'une autre couche, ni retirée, ni posée
    /// après `rang`. C'est l'état de toute base de 2a′-1, dont rien n'écrit encore une seconde
    /// génération ; l'écrivain qui en pose une le défait (2a′-2). Il ne sert qu'à un lecteur sans
    /// filtre, qui rendrait sinon l'ensemble des lignes visibles au lieu de « toutes » (page 03, § 3).
    pub tout_visible: bool,
    /// **Le dernier rang publié** de la couche (2a′-2, page 06, § 1). `rang`, le rang lu sans vue, est le
    /// rang en attente : `rang_publie + 1`.
    pub rang_publie: i64,
    /// Les ensembles posés de cette base ; partagés par toutes les copies de la visibilité.
    pub ensembles: Arc<EnsemblesPoses>,
    /// **Les étages parents de la pile** (2b′), du plus proche à la racine, chacun avec ce que les
    /// étages plus proches lui retirent. Vide pour une pile d'un étage : tout se lit alors comme avant.
    pub parents: Vec<ParentLayer>,
    /// La colonne du fichier de chaque table à générations, pour les tombstones de fichier.
    pub file_columns: BTreeMap<String, String>,
}

/// **Un étage parent lu** (2b′) : sa couche, son rang épinglé, et l'union des tombstones des
/// étages plus proches (par uuid, par fichier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentLayer {
    pub layer: String,
    pub rank: i64,
    pub masked_keys: BTreeSet<String>,
    pub masked_files: BTreeSet<String>,
}

impl PartialEq for Visibilite {
    fn eq(&self, autre: &Self) -> bool {
        self.tables == autre.tables
            && self.couche == autre.couche
            && self.rang == autre.rang
            && self.tout_visible == autre.tout_visible
            && self.rang_publie == autre.rang_publie
            && self.parents == autre.parents
    }
}
impl Eq for Visibilite {}

impl Visibilite {
    /// **La visibilité d'une vue** (2b′) : la pile chargée par [`crate::views::load_stack`] sur
    /// les tables de celle-ci. Le sommet de la pile devient la couche et le rang lus ; chaque parente
    /// garde l'union des tombstones des étages plus proches. Une pile d'un étage rend la même
    /// visibilité, couche et rang mis à part. `rang_publie`, l'état de l'écrivain, ne change pas : une vue
    /// se pose sur la visibilité d'une lecture (l'instantané d'un appel), jamais sur celle qui écrit.
    pub fn with_stack(&self, pile: &crate::views::ReadStack) -> Visibilite {
        let mut v = self.clone();
        let Some((sommet, reste)) = pile.layers.split_first() else { return v };
        v.couche = sommet.layer.clone();
        v.rang = sommet.rank;
        let mut cles = sommet.removed_keys.clone();
        let mut fichiers = sommet.removed_files.clone();
        v.parents = reste
            .iter()
            .map(|e| {
                let lu = ParentLayer { layer: e.layer.clone(), rank: e.rank, masked_keys: cles.clone(), masked_files: fichiers.clone() };
                cles.extend(e.removed_keys.iter().cloned());
                fichiers.extend(e.removed_files.iter().cloned());
                lu
            })
            .collect();
        // « Tout visible » ne tient que pour la couche et le rang posés à l'ouverture, sans parente.
        if v.couche != self.couche || v.rang != self.rang || !v.parents.is_empty() {
            v.tout_visible = false;
        }
        v
    }

    /// Le masque d'un étage parent sur `table`.
    fn mask_for(&self, etage: &ParentLayer, table: &str) -> rag3weaver_ir::Mask {
        rag3weaver_ir::Mask {
            keys: etage.masked_keys.iter().cloned().collect(),
            files: self
                .file_columns
                .get(table)
                .filter(|_| !etage.masked_files.is_empty())
                .map(|c| (c.clone(), etage.masked_files.iter().cloned().collect())),
        }
    }

    /// Le prédicat de visibilité d'une table, si elle a des générations. Une pile d'un étage donne
    /// la condition d'avant 2b′, au bit près ; une pile de plusieurs, l'un de ses étages visible et
    /// non masqué.
    pub fn predicat(&self, table: &str) -> Option<Predicate> {
        if !self.tables.contains(table) {
            return None;
        }
        let sommet = Predicate::Visible { couche: self.couche.clone(), rang: self.rang, mask: Default::default() };
        if self.parents.is_empty() {
            return Some(sommet);
        }
        let mut etages = vec![sommet];
        etages.extend(
            self.parents
                .iter()
                .map(|e| Predicate::Visible { couche: e.layer.clone(), rang: e.rank, mask: self.mask_for(e, table) }),
        );
        Some(Predicate::AnyOf(etages))
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
        if self.parents.is_empty() {
            return dialect.condition_visible_sur(alias, &self.couche, self.rang);
        }
        dialect.condition_sur_predicat(alias, &self.predicat(table)?)
    }

    /// **Les conditions de filtre de la visibilité** sur la table `table`, aux champs préfixés par
    /// `prefixe` (un lecteur qui compose un `FilterCondition`, comme le masque de la recherche) ;
    /// sans préfixe, sur l'entité lue elle-même.
    /// Une pile d'un étage rend les trois conditions d'avant 2b′, dans le même ordre.
    pub fn filter_conditions(&self, table: &str, prefixe: &str) -> Vec<rag3weaver_ir::FilterCondition> {
        use rag3weaver_ir::{FilterCondition, FilterOp, FilterValue};
        let champ = |nom: &str, valeur: FilterValue| FilterCondition::Field {
            key: if prefixe.is_empty() { nom.to_string() } else { format!("{prefixe}.{nom}") },
            value: valeur,
        };
        let etage = |couche: &str, rang: i64| {
            vec![
                champ(COUCHE, FilterValue::Direct(CypherValue::String(couche.to_string()))),
                champ(DEPUIS, FilterValue::Ops(vec![FilterOp::Lte(CypherValue::Int(rang))])),
                FilterCondition::Should(vec![
                    champ(JUSQU_A, FilterValue::Ops(vec![FilterOp::IsNull])),
                    champ(JUSQU_A, FilterValue::Ops(vec![FilterOp::Gt(CypherValue::Int(rang))])),
                ]),
            ]
        };
        if self.parents.is_empty() {
            return etage(&self.couche, self.rang);
        }
        let texte = |v: &String| CypherValue::String(v.clone());
        let mut etages = vec![FilterCondition::Must(etage(&self.couche, self.rang))];
        for e in &self.parents {
            let mut c = etage(&e.layer, e.rank);
            if !e.masked_keys.is_empty() {
                c.push(champ("_uuid", FilterValue::Ops(vec![FilterOp::NotIn(e.masked_keys.iter().map(texte).collect())])));
            }
            if let Some(colonne) = self.file_columns.get(table).filter(|_| !e.masked_files.is_empty()) {
                c.push(champ(colonne, FilterValue::Ops(vec![FilterOp::NotIn(e.masked_files.iter().map(texte).collect())])));
            }
            etages.push(FilterCondition::Must(c));
        }
        vec![FilterCondition::Should(etages)]
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
        let visible = |c: &str, rang: i64| {
            couche.as_str() == Some(c)
                && entier(champ(DEPUIS)).is_some_and(|d| d <= rang)
                && entier(champ(JUSQU_A)).is_none_or(|j| j > rang)
        };
        if visible(&self.couche, self.rang) {
            return true;
        }
        let uuid = champ("_uuid").and_then(|v| v.as_str());
        self.parents.iter().any(|e| {
            let fichier = self.file_columns.iter().find_map(|(_, col)| champ(col)).and_then(|v| v.as_str());
            visible(&e.layer, e.rank)
                && uuid.is_none_or(|u| !e.masked_keys.contains(u))
                && fichier.is_none_or(|f| !e.masked_files.contains(f))
        })
    }
}

/// Les champs d'un `find_by_field` avec ceux de la visibilité quand la table en a, et ceux qui ont
/// été ajoutés (pour les ôter du résultat).
pub fn champs_et_visibilite<'a>(v: Option<&'a Visibilite>, table: &str, fields: &[&'a str]) -> (Vec<&'a str>, Vec<&'a str>) {
    let mut f = fields.to_vec();
    let mut ajoutes = Vec::new();
    if v.is_some_and(|v| v.tables.contains(table)) {
        for c in [COUCHE, DEPUIS, JUSQU_A] {
            if !f.contains(&c) {
                f.push(c);
                ajoutes.push(c);
            }
        }
        // Une pile de plusieurs étages masque par uuid et par fichier : `admet` les lit.
        if let Some(v) = v.filter(|v| !v.parents.is_empty()) {
            if !f.contains(&"_uuid") {
                f.push("_uuid");
                ajoutes.push("_uuid");
            }
            if let Some(col) = v.file_columns.get(table).map(String::as_str) {
                if !f.contains(&col) {
                    f.push(col);
                    ajoutes.push(col);
                }
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

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::*;
    use crate::connection::DbConnection;
    use crate::views::{StackLayer, ReadStack};

    fn base() -> Visibilite {
        Visibilite {
            tables: ["T".to_string()].into(),
            couche: "backend".into(),
            rang: 0,
            tout_visible: true,
            rang_publie: -1,
            ensembles: Arc::new(EnsemblesPoses::default()),
            parents: Vec::new(),
            file_columns: [("T".to_string(), "path".to_string())].into(),
        }
    }

    fn etage(couche: &str, rang: i64, cles: &[&str], fichiers: &[&str]) -> StackLayer {
        StackLayer {
            layer: couche.into(),
            rank: rang,
            removed_keys: cles.iter().map(|s| s.to_string()).collect(),
            removed_files: fichiers.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// La pile `enfant:1` → `milieu:3` → `backend:0` : l'enfant retire `u2` par sa clé et `vieux.rs`
    /// par son fichier (dans les deux étages en dessous), le milieu retire `u6`.
    fn pile() -> ReadStack {
        ReadStack {
            layers: vec![etage("enfant", 1, &["u2"], &["vieux.rs"]), etage("milieu", 3, &["u6"], &[]), etage("backend", 0, &[], &[])],
            root_without_parent: true,
        }
    }

    /// Les lignes : (uuid, path, couche, depuis, jusqu_a) et si la vue de [`pile`] doit la voir.
    const LIGNES: &[(&str, &str, &str, i64, Option<i64>, bool)] = &[
        ("u1", "a.rs", "backend", 0, None, true),
        ("u2", "b.rs", "backend", 0, None, false),          // retirée par clé dans l'enfant
        ("u3", "vieux.rs", "backend", 0, None, false),      // retirée par fichier dans l'enfant
        ("u4", "c.rs", "backend", 1, None, false),          // posée après le rang épinglé de la racine
        ("u5", "d.rs", "backend", 0, Some(1), true),        // retirée au rang 1, vivante au rang 0
        ("u6", "e.rs", "backend", 0, None, false),          // retirée par clé dans l'étage du milieu
        ("m1", "vieux.rs", "milieu", 3, None, false),       // l'enfant retire aussi ce fichier au milieu
        ("m2", "f.rs", "milieu", 2, None, true),
        ("m3", "g.rs", "milieu", 4, None, false),           // posée après le rang du milieu
        ("c1", "h.rs", "enfant", 1, None, true),
        ("c2", "i.rs", "enfant", 2, None, false),           // posée après le rang de la vue
        ("c3", "vieux.rs", "enfant", 1, None, true),        // l'enfant voit son propre vieux.rs
        ("x1", "a.rs", "autre", 0, None, false),            // couche hors de la pile
    ];

    fn attendus() -> BTreeSet<String> {
        LIGNES.iter().filter(|l| l.5).map(|l| l.0.to_string()).collect()
    }

    fn table() -> crate::Rag3dbConnection {
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        conn.execute("CREATE NODE TABLE T(_uuid STRING, path STRING, _couche STRING, _depuis INT64, _jusqu_a INT64, PRIMARY KEY(_uuid))").unwrap();
        for (u, p, c, d, j, _) in LIGNES {
            let j = j.map_or("NULL".to_string(), |j| j.to_string());
            conn.execute(&format!("CREATE (:T {{_uuid: '{u}', path: '{p}', _couche: '{c}', _depuis: {d}, _jusqu_a: {j}}})")).unwrap();
        }
        conn
    }

    fn uuids(r: crate::connection::QueryResult) -> BTreeSet<String> {
        r.rows.iter().filter_map(|l| l.first().and_then(|v| v.as_str()).map(str::to_string)).collect()
    }

    /// **Une pile de trois étages se compose par la clause** (celle de `resoudre_le_filtre` et du
    /// filtre vectoriel) et par le prédicat des plans, qui en est la même traduction.
    #[test]
    fn a_stack_composes_through_the_clause() {
        let conn = table();
        let v = base().with_stack(&pile());
        assert!(!v.tout_visible, "une pile de plusieurs étages n'est pas « tout visible »");
        assert_eq!((v.couche.as_str(), v.rang, v.parents.len()), ("enfant", 1, 2));
        let clause = v.clause_sur(&crate::dialect::Rag3dbDialect, "T", "p").expect("T a des générations");
        let lu = uuids(conn.execute(&format!("MATCH (p:T) WHERE {clause} RETURN p._uuid")).unwrap());
        assert_eq!(lu, attendus(), "clause : {clause}");
    }

    /// **La même pile par `admet`**, le lecteur à la main (`get`, `find_by_field`), qui lit la ligne
    /// avec les colonnes que `champs_et_visibilite` ajoute.
    #[test]
    fn a_stack_composes_through_admet() {
        let v = base().with_stack(&pile());
        let (champs, ajoutes) = champs_et_visibilite(Some(&v), "T", &["path"]);
        assert!(champs.contains(&"_uuid") && ajoutes.contains(&"_uuid"), "l'uuid se lit pour les tombstones : {champs:?}");
        let admises: BTreeSet<String> = LIGNES
            .iter()
            .filter(|(u, p, c, d, j, _)| {
                let mut l = BTreeMap::new();
                l.insert("_uuid".to_string(), CypherValue::String(u.to_string()));
                l.insert("path".to_string(), CypherValue::String(p.to_string()));
                l.insert(COUCHE.to_string(), CypherValue::String(c.to_string()));
                l.insert(DEPUIS.to_string(), CypherValue::Int(*d));
                l.insert(JUSQU_A.to_string(), j.map_or(CypherValue::Null, CypherValue::Int));
                v.admet(&l)
            })
            .map(|l| l.0.to_string())
            .collect();
        assert_eq!(admises, attendus());
    }

    /// **La même pile par le filtre** du masque de la recherche (`filter_conditions`), compilé
    /// par le dialecte.
    #[test]
    fn a_stack_composes_through_the_filter() {
        let conn = table();
        let v = base().with_stack(&pile());
        let filtre = rag3weaver_ir::FilterCondition::Must(v.filter_conditions("T", ""));
        let relations = std::collections::HashMap::new();
        let lu = crate::filter::FilterParser::new(&relations, &crate::dialect::Rag3dbDialect).parse_condition(&filtre, "T", "p").expect("le filtre se compile");
        let clause = lu.combine_where();
        let lu = uuids(conn.execute_with_params(&format!("MATCH (p:T) WHERE {clause} RETURN p._uuid"), &lu.params).unwrap());
        assert_eq!(lu, attendus(), "filtre : {clause}");
    }

    /// **Une pile d'un étage ne change rien** (le témoin 0 au niveau de la visibilité) : la même
    /// visibilité, le même prédicat, la même clause, les mêmes conditions de filtre.
    #[test]
    fn a_one_layer_stack_changes_nothing() {
        let avant = base();
        let apres = avant.with_stack(&ReadStack { layers: vec![etage("backend", 0, &[], &[])], root_without_parent: true });
        assert_eq!(apres, avant);
        assert_eq!(apres.tout_visible, avant.tout_visible);
        assert_eq!(apres.predicat("T"), Some(Predicate::Visible { couche: "backend".into(), rang: 0, mask: Default::default() }));
        use crate::dialect::SchemaDialect;
        let d = crate::dialect::Rag3dbDialect;
        assert_eq!(apres.clause_sur(&d, "T", "p"), d.condition_visible_sur("p", "backend", 0));
        assert_eq!(format!("{:?}", apres.filter_conditions("T", "p")), format!("{:?}", avant.filter_conditions("T", "p")));
        let (c, a) = champs_et_visibilite(Some(&apres), "T", &["path"]);
        assert_eq!((c, a), (vec!["path", COUCHE, DEPUIS, JUSQU_A], vec![COUCHE, DEPUIS, JUSQU_A]));
    }
}
