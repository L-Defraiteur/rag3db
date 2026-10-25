//! **Un Plan déclaré** (19 octobre 2026) : la forme JSON qu'un gabarit pose
//! dans `plans/`, et que le `PlanNode` lit — fixée avec la mémoire longue
//! (`docs/18-octobre-2026-memoire-longue/03-la-forme-json-d-un-plan.md`).
//!
//! ```json
//! {"let": {"nom": <prédicat>},
//!  "forms": {"id": {"select": {…}} | {"count": {…}}},
//!  "outputs": {"rows": "id", "total": "id", "results": {"form": "id", "uuid": "_uuid", "entity": "E"}}}
//! ```
//!
//! Les formes, les prédicats et les colonnes sont ceux de l'IR, un pour un ;
//! seules la sélection et le compte se déclarent (un Plan déclaré lit). Trois
//! temps :
//!
//! - **la lecture** ([`DeclaredPlan::from_json`]) : un champ inconnu, une
//!   forme ou une condition inconnue (dont `compiled`, la langue d'une base)
//!   sont refusés en les nommant ;
//! - **la vérification au chargement** ([`DeclaredPlan::check`],
//!   [`DeclaredPlan::check_schema`]) : les `let` inconnus ou cycliques, un
//!   `limit` lié sans plafond, une pagination sans ordre total, des sorties
//!   qui ne désignent rien, et, contre le schéma, les tables, champs et
//!   relations (`through`, `collect` : la relation mène-t-elle bien à `end`
//!   dans ce sens ?) ;
//! - **le liage à l'appel** ([`DeclaredPlan::bind`]) : les `let` remplacés, les
//!   `optional` dont un paramètre est absent ou vide retirés, `limit` et
//!   `offset` remplacés par leur nombre (le plafond tenu), et le graphe de
//!   l'IR rendu ([`BoundPlan`]). Un Plan lié n'a plus ni `let` ni `optional`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::form::{Collect, Column, Count, Limit, Offset, OrderBy, Predicate, Select};
use crate::plan::{Form, Plan};
use crate::value::{QueryParam, Value};

/// Une forme d'un Plan déclaré : une sélection ou un compte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DeclaredForm {
    Select(Select),
    Count(Count),
}

/// Ce que le nœud rend, et d'où.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outputs {
    /// La forme dont les lignes vont au port `rows`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<String>,
    /// Le compte qui va au port `total`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<String>,
    /// Les résultats (`results`) : quelle forme, quelle colonne est l'uuid, de
    /// quelle entité.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<ResultsSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultsSpec {
    pub form: String,
    pub uuid: String,
    pub entity: String,
}

/// **Un Plan déclaré**, tel qu'il se lit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredPlan {
    /// Des conditions nommées, citées par `{"let": "nom"}`.
    #[serde(default, rename = "let", skip_serializing_if = "BTreeMap::is_empty")]
    pub lets: BTreeMap<String, Predicate>,
    pub forms: BTreeMap<String, DeclaredForm>,
    #[serde(default)]
    pub outputs: Outputs,
}

/// Ce que décrit un schéma, pour [`DeclaredPlan::check_schema`] : les champs
/// de chaque table (`_uuid` compris) et, pour chaque relation, ses deux bouts.
#[derive(Debug, Clone, Default)]
pub struct PlanSchema {
    pub tables: BTreeMap<String, BTreeSet<String>>,
    pub relations: BTreeMap<String, (String, String)>,
}

/// **Un Plan lié** : le graphe de l'IR, prêt pour un dialecte, et les
/// sorties nommées.
#[derive(Debug, Clone)]
pub struct BoundPlan {
    pub plan: Plan,
    pub rows: Option<String>,
    pub total: Option<String>,
    pub results: Option<ResultsSpec>,
}

impl DeclaredPlan {
    /// Lire un Plan. Un refus nomme ce qui ne se lit pas (`compiled` compris).
    pub fn from_json(texte: &str) -> Result<Self, String> {
        serde_json::from_str(texte).map_err(|e| format!("Plan illisible : {e}"))
    }

    /// **La vérification au chargement**, sans schéma.
    pub fn check(&self) -> Result<(), String> {
        if self.forms.is_empty() {
            return Err("Plan : aucune forme".into());
        }
        // Les `let` : chaque nom cité existe, et aucun ne se cite lui-même par
        // une chaîne.
        for nom in self.lets.keys() {
            self.deplier_let(nom, &mut Vec::new())?;
        }
        for (id, f) in &self.forms {
            let ctx = |m: String| format!("Plan, forme « {id} » : {m}");
            for p in predicats_de(f) {
                self.deplier(p, &mut Vec::new()).map_err(ctx)?;
            }
            if let DeclaredForm::Select(s) = f {
                if let Some(Limit::Param { param, max: None }) = &s.limit {
                    return Err(ctx(format!("le limit lié au paramètre « {param} » n'a pas de plafond (max)")));
                }
                if (s.limit.is_some() || s.offset.is_some()) && !s.total_order() {
                    return Err(ctx(
                        "une pagination (limit ou offset) doit finir son order_by par _uuid, sinon deux pages peuvent sauter ou répéter une ligne".into(),
                    ));
                }
                // Les identifiants, sur la forme dépliée (les `optional`
                // ouverts : ils comptent comme s'ils étaient gardés).
                let mut sonde = s.clone();
                sonde.filter = s.filter.as_ref().map(|p| self.deplier(p, &mut Vec::new())).transpose().map_err(ctx)?.map(ouvrir);
                for c in &mut sonde.returns {
                    if let Column::Collect(c) = c {
                        c.filter = c.filter.as_ref().map(|p| self.deplier(p, &mut Vec::new())).transpose().map_err(ctx)?.map(ouvrir);
                    }
                }
                if let Some(Limit::Param { max: Some(m), .. }) = &sonde.limit {
                    sonde.limit = Some(Limit::Literal(*m));
                }
                if let Some(Offset::Param { .. }) = &sonde.offset {
                    sonde.offset = Some(Offset::Literal(0));
                }
                sonde.validate().map_err(|e| ctx(e.to_string()))?;
            }
            if let DeclaredForm::Count(c) = f {
                if !matches!(c, Count::Matching { .. } | Count::Rows { .. }) {
                    return Err(ctx("seuls `rows` et `matching` se déclarent parmi les comptes".into()));
                }
            }
        }
        let existe = |nom: &str, quoi: &str| -> Result<(), String> {
            if self.forms.contains_key(nom) {
                Ok(())
            } else {
                Err(format!("Plan, sortie « {quoi} » : la forme « {nom} » n'existe pas"))
            }
        };
        if let Some(r) = &self.outputs.rows {
            existe(r, "rows")?;
        }
        if let Some(t) = &self.outputs.total {
            existe(t, "total")?;
            if !matches!(self.forms.get(t), Some(DeclaredForm::Count(_))) {
                return Err(format!("Plan, sortie « total » : la forme « {t} » n'est pas un compte"));
            }
        }
        if let Some(r) = &self.outputs.results {
            existe(&r.form, "results")?;
            let Some(DeclaredForm::Select(s)) = self.forms.get(&r.form) else {
                return Err(format!("Plan, sortie « results » : la forme « {} » n'est pas une sélection", r.form));
            };
            if !s.returns.iter().any(|c| matches!(c, Column::Node(f) if *f == r.uuid)) {
                return Err(format!("Plan, sortie « results » : la forme « {} » ne rend pas la colonne « {} »", r.form, r.uuid));
            }
        }
        Ok(())
    }

    /// **La vérification contre le schéma** : les tables et les champs nommés
    /// existent, et chaque `through` ou `collect` suit une relation qui mène
    /// bien de la table courante à `end` dans sa direction. Un refus nomme la
    /// relation, la table ou le champ.
    pub fn check_schema(&self, schema: &PlanSchema) -> Result<(), String> {
        for (id, f) in &self.forms {
            let ctx = |m: String| format!("Plan, forme « {id} » : {m}");
            let (table, filtre) = match f {
                DeclaredForm::Select(s) => (&s.table, s.filter.as_ref()),
                DeclaredForm::Count(Count::Matching { table, filter }) => (table, filter.as_ref()),
                DeclaredForm::Count(Count::Rows { table }) => (table, None),
                DeclaredForm::Count(_) => continue,
            };
            let champs = schema.tables.get(table).ok_or_else(|| ctx(format!("la table « {table} » n'est pas déclarée")))?;
            if let Some(p) = filtre {
                self.verifier_contre(p, table, schema).map_err(ctx)?;
            }
            if let DeclaredForm::Select(s) = f {
                let champ = |c: &str| -> Result<(), String> {
                    if champs.contains(c) {
                        Ok(())
                    } else {
                        Err(ctx(format!("la table « {table} » n'a pas de champ « {c} »")))
                    }
                };
                for k in &s.order_by {
                    match &k.by {
                        OrderBy::Field(c) => champ(c)?,
                        OrderBy::Coalesce(v) => v.iter().try_for_each(|c| champ(c))?,
                    }
                }
                for c in &s.returns {
                    match c {
                        Column::Node(c) => champ(c)?,
                        Column::Collect(c) => self.verifier_collect(c, table, schema).map_err(ctx)?,
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn verifier_collect(&self, c: &Collect, depuis: &str, schema: &PlanSchema) -> Result<(), String> {
        verifier_bout(&c.relation, c.direction, depuis, &c.end, schema)?;
        let champs = schema.tables.get(&c.end).ok_or_else(|| format!("la table « {} » n'est pas déclarée", c.end))?;
        if let Some(f) = c.fields.iter().find(|f| !champs.contains(*f)) {
            return Err(format!("la liste « {} » : la table « {} » n'a pas de champ « {f} »", c.alias, c.end));
        }
        if let Some(p) = &c.filter {
            self.verifier_contre(p, &c.end, schema)?;
        }
        Ok(())
    }

    fn verifier_contre(&self, p: &Predicate, table: &str, schema: &PlanSchema) -> Result<(), String> {
        let champs = schema.tables.get(table).ok_or_else(|| format!("la table « {table} » n'est pas déclarée"))?;
        match p {
            Predicate::Equals { field, .. }
            | Predicate::Contains { field, .. }
            | Predicate::AtLeast { field, .. }
            | Predicate::In { field, .. }
            | Predicate::StartsWith { field, .. } => {
                if champs.contains(field) {
                    Ok(())
                } else {
                    Err(format!("la table « {table} » n'a pas de champ « {field} »"))
                }
            }
            Predicate::AnyOf(v) | Predicate::AllOf(v) => v.iter().try_for_each(|p| self.verifier_contre(p, table, schema)),
            Predicate::Optional(p) => self.verifier_contre(p, table, schema),
            Predicate::Let(nom) => {
                let p = self.lets.get(nom).ok_or_else(|| format!("la condition « {nom} » n'est pas déclarée dans let"))?;
                self.verifier_contre(p, table, schema)
            }
            Predicate::Through { relation, direction, end, filter } => {
                verifier_bout(relation, *direction, table, end, schema)?;
                match filter {
                    Some(f) => self.verifier_contre(f, end, schema),
                    None => Ok(()),
                }
            }
            Predicate::Compiled(_) => Err("une condition compilée n'a pas sa place dans un Plan déclaré".into()),
        }
    }

    /// Un `let` déplié, ses propres `let` compris ; `chaine` dit le chemin, pour
    /// nommer un cycle.
    fn deplier_let(&self, nom: &str, chaine: &mut Vec<String>) -> Result<Predicate, String> {
        if chaine.iter().any(|n| n == nom) {
            chaine.push(nom.to_string());
            return Err(format!("Plan : les conditions nommées se citent en cercle : {}", chaine.join(" → ")));
        }
        let p = self.lets.get(nom).ok_or_else(|| format!("Plan : la condition « {nom} » n'est pas déclarée dans let"))?;
        chaine.push(nom.to_string());
        let d = self.deplier(p, chaine);
        chaine.pop();
        d
    }

    /// Une condition, ses `let` remplacés (les `optional` gardés).
    fn deplier(&self, p: &Predicate, chaine: &mut Vec<String>) -> Result<Predicate, String> {
        Ok(match p {
            Predicate::Let(nom) => self.deplier_let(nom, chaine)?,
            Predicate::AnyOf(v) => Predicate::AnyOf(v.iter().map(|p| self.deplier(p, chaine)).collect::<Result<_, _>>()?),
            Predicate::AllOf(v) => Predicate::AllOf(v.iter().map(|p| self.deplier(p, chaine)).collect::<Result<_, _>>()?),
            Predicate::Optional(p) => Predicate::Optional(Box::new(self.deplier(p, chaine)?)),
            Predicate::Through { relation, direction, end, filter } => Predicate::Through {
                relation: relation.clone(),
                direction: *direction,
                end: end.clone(),
                filter: filter.as_ref().map(|f| self.deplier(f, chaine).map(Box::new)).transpose()?,
            },
            autre => autre.clone(),
        })
    }

    /// **Le liage** : les paramètres de l'appel, le Plan de l'IR.
    pub fn bind(&self, params: &BTreeMap<String, Value>) -> Result<BoundPlan, String> {
        let mut plan = Plan::new();
        for (id, f) in &self.forms {
            let ctx = |m: String| format!("Plan, forme « {id} » : {m}");
            let mut noms = BTreeSet::new();
            let lier = |p: &Option<Predicate>, noms: &mut BTreeSet<String>| -> Result<Option<Predicate>, String> {
                match p {
                    Some(p) => {
                        let d = self.deplier(p, &mut Vec::new())?;
                        lier_predicat(&d, params, noms)
                    }
                    None => Ok(None),
                }
            };
            let forme = match f {
                DeclaredForm::Select(s) => {
                    let mut s = s.clone();
                    s.filter = lier(&s.filter, &mut noms).map_err(ctx)?;
                    for c in &mut s.returns {
                        if let Column::Collect(c) = c {
                            c.filter = lier(&c.filter, &mut noms).map_err(ctx)?;
                        }
                    }
                    if let Some(Limit::Param { param, max }) = &s.limit {
                        let n = entier(params, param).map_err(ctx)?;
                        if let Some(m) = max {
                            if n > *m {
                                return Err(ctx(format!("le paramètre « {param} » vaut {n}, au-delà du plafond {m}")));
                            }
                        }
                        s.limit = Some(Limit::Literal(n));
                    }
                    if let Some(Offset::Param { param }) = &s.offset {
                        s.offset = Some(Offset::Literal(entier(params, param).map_err(ctx)?));
                    }
                    Form::Select(s)
                }
                DeclaredForm::Count(Count::Matching { table, filter }) => {
                    Form::Count(Count::Matching { table: table.clone(), filter: lier(filter, &mut noms).map_err(ctx)? })
                }
                DeclaredForm::Count(c) => Form::Count(c.clone()),
            };
            let valeurs = noms
                .into_iter()
                .map(|n| QueryParam::new(n.clone(), params.get(&n).cloned().unwrap_or(Value::Null)))
                .collect();
            plan = plan.node(id.clone(), forme, valeurs);
        }
        for id in self.outputs.rows.iter().chain(self.outputs.total.iter()) {
            if !plan.outputs.contains(id) {
                plan = plan.output(id.clone());
            }
        }
        if let Some(r) = &self.outputs.results {
            if !plan.outputs.contains(&r.form) {
                plan = plan.output(r.form.clone());
            }
        }
        plan.check().map_err(|e| e.to_string())?;
        Ok(BoundPlan { plan, rows: self.outputs.rows.clone(), total: self.outputs.total.clone(), results: self.outputs.results.clone() })
    }
}

fn predicats_de(f: &DeclaredForm) -> Vec<&Predicate> {
    match f {
        DeclaredForm::Select(s) => {
            let mut v: Vec<&Predicate> = s.filter.iter().collect();
            for c in &s.returns {
                if let Column::Collect(c) = c {
                    v.extend(c.filter.iter());
                }
            }
            v
        }
        DeclaredForm::Count(Count::Matching { filter, .. }) => filter.iter().collect(),
        DeclaredForm::Count(Count::EdgesMatching { filter, .. }) => vec![filter],
        DeclaredForm::Count(_) => Vec::new(),
    }
}

/// Une condition, ses `optional` ouverts (pour vérifier ses noms).
fn ouvrir(p: Predicate) -> Predicate {
    match p {
        Predicate::Optional(p) => ouvrir(*p),
        Predicate::AnyOf(v) => Predicate::AnyOf(v.into_iter().map(ouvrir).collect()),
        Predicate::AllOf(v) => Predicate::AllOf(v.into_iter().map(ouvrir).collect()),
        Predicate::Through { relation, direction, end, filter } => {
            Predicate::Through { relation, direction, end, filter: filter.map(|f| Box::new(ouvrir(*f))) }
        }
        autre => autre,
    }
}

fn verifier_bout(relation: &str, direction: crate::form::Direction, depuis: &str, end: &str, schema: &PlanSchema) -> Result<(), String> {
    let (de, vers) = schema.relations.get(relation).ok_or_else(|| format!("la relation « {relation} » n'est pas déclarée"))?;
    let (attendu_depuis, attendu_vers, sens) = match direction {
        crate::form::Direction::Outgoing => (de, vers, "outgoing"),
        crate::form::Direction::Incoming => (vers, de, "incoming"),
    };
    if attendu_depuis != depuis || attendu_vers != end {
        return Err(format!(
            "la relation « {relation} » en {sens} mène de « {attendu_depuis} » à « {attendu_vers} », pas de « {depuis} » à « {end} »"
        ));
    }
    Ok(())
}

/// Vide : une liste vide, une chaîne vide. Un entier, un booléen ne le sont
/// jamais.
fn vide(v: &Value) -> bool {
    match v {
        Value::List(l) => l.is_empty(),
        Value::String(s) => s.is_empty(),
        Value::Null => true,
        _ => false,
    }
}

fn entier(params: &BTreeMap<String, Value>, nom: &str) -> Result<usize, String> {
    match params.get(nom) {
        Some(Value::Int(n)) if *n >= 0 => Ok(*n as usize),
        Some(autre) => Err(format!("le paramètre « {nom} » n'est pas un entier positif ({autre:?})")),
        None => Err(format!("le paramètre « {nom} » manque")),
    }
}

/// Les paramètres qu'une condition cite, hors de ses propres `optional`.
fn params_directs(p: &Predicate, out: &mut Vec<String>) {
    match p {
        Predicate::Equals { param, .. }
        | Predicate::Contains { param, .. }
        | Predicate::AtLeast { param, .. }
        | Predicate::In { param, .. }
        | Predicate::StartsWith { param, .. } => out.push(param.clone()),
        Predicate::AnyOf(v) | Predicate::AllOf(v) => v.iter().for_each(|p| params_directs(p, out)),
        Predicate::Through { filter: Some(f), .. } => params_directs(f, out),
        _ => {}
    }
}

/// **Lier une condition** (ses `let` déjà dépliés) : un `optional` dont un
/// paramètre direct est absent ou vide disparaît ; un `all_of` ou un `any_of`
/// vidé par ces retraits disparaît aussi ; hors `optional`, un paramètre
/// absent et une liste vide donnée à `in` sont refusés en les nommant. Les
/// noms des paramètres gardés vont dans `noms`.
fn lier_predicat(p: &Predicate, params: &BTreeMap<String, Value>, noms: &mut BTreeSet<String>) -> Result<Option<Predicate>, String> {
    Ok(match p {
        Predicate::Optional(dedans) => {
            let mut directs = Vec::new();
            params_directs(dedans, &mut directs);
            if directs.iter().any(|n| params.get(n).is_none_or(vide)) {
                None
            } else {
                lier_predicat(dedans, params, noms)?
            }
        }
        Predicate::Equals { param, .. }
        | Predicate::Contains { param, .. }
        | Predicate::AtLeast { param, .. }
        | Predicate::StartsWith { param, .. } => {
            if !params.contains_key(param) {
                return Err(format!("le paramètre « {param} » manque"));
            }
            noms.insert(param.clone());
            Some(p.clone())
        }
        Predicate::In { param, .. } => match params.get(param) {
            None => return Err(format!("le paramètre « {param} » manque")),
            Some(Value::List(l)) if l.is_empty() => {
                return Err(format!("le paramètre « {param} » est une liste vide : `in` ne retiendrait rien (sous optional, il se retire)"))
            }
            Some(Value::List(_)) => {
                noms.insert(param.clone());
                Some(p.clone())
            }
            Some(autre) => return Err(format!("le paramètre « {param} » de `in` n'est pas une liste ({autre:?})")),
        },
        Predicate::AnyOf(v) | Predicate::AllOf(v) if v.is_empty() => Some(p.clone()),
        Predicate::AnyOf(v) | Predicate::AllOf(v) => {
            let mut gardes = Vec::new();
            for q in v {
                if let Some(q) = lier_predicat(q, params, noms)? {
                    gardes.push(q);
                }
            }
            if gardes.is_empty() {
                None
            } else if matches!(p, Predicate::AnyOf(_)) {
                Some(Predicate::AnyOf(gardes))
            } else {
                Some(Predicate::AllOf(gardes))
            }
        }
        Predicate::Through { relation, direction, end, filter } => {
            let filtre = match filter {
                Some(f) => lier_predicat(f, params, noms)?.map(Box::new),
                None => None,
            };
            Some(Predicate::Through { relation: relation.clone(), direction: *direction, end: end.clone(), filter: filtre })
        }
        Predicate::Let(nom) => return Err(format!("la condition « {nom} » n'a pas été dépliée")),
        Predicate::Compiled(_) => return Err("une condition compilée n'a pas sa place dans un Plan déclaré".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn plan(v: serde_json::Value) -> Result<DeclaredPlan, String> {
        DeclaredPlan::from_json(&v.to_string())
    }

    fn params(v: serde_json::Value) -> BTreeMap<String, Value> {
        serde_json::from_value(v).unwrap()
    }

    fn base(filtre: serde_json::Value) -> serde_json::Value {
        json!({"forms": {"f": {"select": {"table": "Memory", "filter": filtre, "returns": ["_uuid"]}}}, "outputs": {"rows": "f"}})
    }

    #[test]
    fn la_forme_de_cited_by_se_lit_et_se_verifie() {
        let texte = include_str!("../tests/cited_by.json");
        let p = DeclaredPlan::from_json(texte).unwrap();
        p.check().unwrap();
        let mut schema = PlanSchema::default();
        let champs = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
        schema.tables.insert(
            "Memory".into(),
            champs(&["_uuid", "claim", "kind", "state", "origin", "reach", "created_at", "updated_at", "reviewed_at"]),
        );
        schema.tables.insert("Ref".into(), champs(&["_uuid", "genre", "valeur", "etat"]));
        schema.tables.insert("Subject".into(), champs(&["_uuid", "name"]));
        schema.relations.insert("CITE".into(), ("Memory".into(), "Ref".into()));
        schema.relations.insert("ANCHORED_TO".into(), ("Memory".into(), "Subject".into()));
        p.check_schema(&schema).unwrap();
        let lie = p
            .bind(&params(json!({"value": "d:src/a.rs", "value_dir": "d:src/a.rs/", "limit": 20, "offset": 0, "genres": [], "subject": ""})))
            .unwrap();
        // Aucun `let` ni `optional` ne reste ; les retirés n'apportent pas
        // leurs paramètres.
        let dbg = format!("{:?}", lie.plan);
        assert!(!dbg.contains("Optional") && !dbg.contains("Let("), "{dbg}");
        assert!(!dbg.contains("genres") && !dbg.contains("\"subject\""), "{dbg}");
        assert_eq!(lie.rows.as_deref(), Some("fiches"));
        assert_eq!(lie.total.as_deref(), Some("total"));
    }

    #[test]
    fn du_cypher_brut_ne_se_lit_pas() {
        let e = plan(base(json!({"compiled": "m.x = 1"}))).unwrap_err();
        assert!(e.contains("compiled"), "{e}");
    }

    #[test]
    fn un_let_inconnu_ou_cyclique_est_refuse_en_nommant_la_chaine() {
        let e = plan(base(json!({"let": "absent"}))).unwrap().check().unwrap_err();
        assert!(e.contains("absent"), "{e}");
        let mut v = base(json!({"let": "a"}));
        v["let"] = json!({"a": {"let": "b"}, "b": {"let": "a"}});
        let e = plan(v).unwrap().check().unwrap_err();
        assert!(e.contains("a → b → a"), "{e}");
    }

    #[test]
    fn optional_absent_vide_et_entier() {
        let p = plan(base(json!({"all_of": [
            {"optional": {"in": {"field": "kind", "param": "kinds"}}},
            {"optional": {"at_least": {"field": "updated_at", "param": "since"}}}
        ]})))
        .unwrap();
        // Absent, puis liste vide : le filtre disparaît entier (un all_of vidé
        // ne vaut pas faux).
        for ps in [json!({}), json!({"kinds": []})] {
            let l = p.bind(&params(ps)).unwrap();
            let Form::Select(s) = &l.plan.nodes[0].form else { panic!() };
            assert_eq!(s.filter, None);
        }
        // Un entier n'est jamais vide : since 0 reste un filtre.
        let l = p.bind(&params(json!({"since": 0}))).unwrap();
        let Form::Select(s) = &l.plan.nodes[0].form else { panic!() };
        assert_eq!(s.filter, Some(Predicate::AllOf(vec![Predicate::AtLeast { field: "updated_at".into(), param: "since".into() }])));
    }

    #[test]
    fn hors_optional_une_liste_vide_et_un_absent_sont_refuses_en_les_nommant() {
        let p = plan(base(json!({"in": {"field": "kind", "param": "kinds"}}))).unwrap();
        assert!(p.bind(&params(json!({"kinds": []}))).unwrap_err().contains("kinds"));
        assert!(p.bind(&params(json!({}))).unwrap_err().contains("kinds"));
    }

    #[test]
    fn la_pagination_exige_un_ordre_total_et_un_plafond() {
        let mut v = base(json!({"equals": {"field": "kind", "param": "k"}}));
        v["forms"]["f"]["select"]["order_by"] = json!([{"field": "updated_at", "desc": true}]);
        v["forms"]["f"]["select"]["limit"] = json!({"param": "limit", "max": 50});
        assert!(plan(v.clone()).unwrap().check().unwrap_err().contains("_uuid"));
        v["forms"]["f"]["select"]["order_by"] = json!([{"field": "updated_at", "desc": true}, {"field": "_uuid"}]);
        v["forms"]["f"]["select"]["limit"] = json!({"param": "limit"});
        assert!(plan(v.clone()).unwrap().check().unwrap_err().contains("max"));
        v["forms"]["f"]["select"]["limit"] = json!({"param": "limit", "max": 50});
        let p = plan(v).unwrap();
        p.check().unwrap();
        assert!(p.bind(&params(json!({"k": "x", "limit": 51}))).unwrap_err().contains("plafond"));
        p.bind(&params(json!({"k": "x", "limit": 50}))).unwrap();
    }

    #[test]
    fn un_through_vers_un_autre_bout_est_refuse_en_nommant_la_relation_et_le_bout() {
        let p = plan(base(json!({"through": {"relation": "CITE", "direction": "outgoing", "end": "Subject"}}))).unwrap();
        let mut schema = PlanSchema::default();
        for t in ["Memory", "Ref", "Subject"] {
            schema.tables.insert(t.into(), ["_uuid".to_string()].into());
        }
        schema.relations.insert("CITE".into(), ("Memory".into(), "Ref".into()));
        let e = p.check_schema(&schema).unwrap_err();
        assert!(e.contains("CITE") && e.contains("Subject"), "{e}");
    }
}
