//! **Exécuter un graphe de formes** ([`rag3weaver_ir::Plan`]) confié entier
//! au dialecte ; voir
//! `docs/11-octobre-2026-embarquements/01-chainage-des-formes.md`.
//!
//! Le **repli découpé** est commun à tous les dialectes : un nœud, une
//! instruction, dans l'ordre topologique ; une arête passe en `$uuids` les
//! valeurs distinctes et non nulles de sa colonne, dans l'ordre où elles
//! paraissent. La **fusion** vient du dialecte
//! ([`SchemaDialect::fuse`]) : un nœud rejoint l'instruction de son amont
//! quand l'amont n'a que lui pour suivant, n'est pas une sortie, et que le
//! dialecte sait les dire ensemble. Elle doit rendre les lignes du repli ;
//! `PlanOptions::split` force le repli, pour le prouver.
//!
//! L'ordre des étapes ne change pas : la vérification commune
//! ([`Plan::check`](rag3weaver_ir::Plan::check)), puis la traduction de
//! **tous** les nœuds (un refus part avant toute exécution, rien n'est lu ni
//! écrit), puis les groupes et la décision d'atomicité, puis l'exécution.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rag3weaver_ir::{Form, Plan, TranslateError};

use crate::connection::{CypherValue, DbConnection, QueryParam, QueryResult};
use crate::dialect::SchemaDialect;

/// Ce que l'appelant demande en plus du graphe.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlanOptions {
    /// Refuser, avant toute exécution, un graphe qui ne pourrait pas être
    /// atomique. Par défaut il s'exécute et le dit (`atomic: false` et sa
    /// raison) ; décision de Lucie, 11 octobre 2026.
    pub require_atomic: bool,
    /// Ne rien fondre : un nœud, une instruction. Pour comparer une fusion au
    /// repli sur le même graphe.
    pub split: bool,
}

/// **Le rapport** : les lignes des sorties, et ce que le dialecte a fait du
/// graphe.
#[derive(Debug, Clone, Default)]
pub struct PlanReport {
    /// Les lignes de chaque nœud désigné comme sortie.
    pub outputs: BTreeMap<String, QueryResult>,
    /// Les instructions envoyées (sans `begin` / `commit` ni l'instantané).
    pub instructions: usize,
    /// Les groupes de nœuds partis ensemble, une instruction par groupe.
    pub fused: Vec<Vec<String>>,
    pub atomic: bool,
    pub atomic_reason: Option<String>,
}

/// Pourquoi un graphe ne s'est pas exécuté, ou pas jusqu'au bout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// La vérification commune l'a refusé : rien n'est parti.
    Invalid(String),
    /// **P3** : le dialecte ne sait pas exécuter la forme de ce nœud, ni
    /// seule ni découpée. Rien n'est parti. `message` est le refus nommé du
    /// dialecte ([`SchemaDialect::untranslated`]).
    Refused { node: String, message: String },
    /// L'appelant exige l'atomicité, et ce graphe ne l'aurait pas : rien
    /// n'est parti.
    NotAtomic { reason: String },
    /// Une instruction a échoué. `rolled_back` : la transaction a été
    /// défaite, rien n'est resté ; sinon les écritures des nœuds d'avant
    /// restent.
    Failed { node: String, error: String, rolled_back: bool },
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(m) => f.write_str(m),
            Self::Refused { node, message } => write!(f, "{message} (nœud « {node} »)"),
            Self::NotAtomic { reason } => write!(f, "atomicity required, and this plan would not be atomic: {reason}"),
            Self::Failed { node, error, rolled_back } => {
                let suite = if *rolled_back { "rolled back, nothing written" } else { "not rolled back: earlier writes stay" };
                write!(f, "node « {node} » failed ({suite}): {error}")
            }
        }
    }
}

impl std::error::Error for PlanError {}

/// Le texte d'un nœud dans la langue du dialecte.
fn translate(dialect: &dyn SchemaDialect, form: &Form) -> Result<String, TranslateError> {
    match form {
        Form::Hop(h) => dialect.hop(h),
        Form::Count(c) => dialect.count(c),
        Form::Select(s) => dialect.select(s),
        Form::Write(w) => dialect.write(w),
    }
}

/// Les valeurs distinctes, non nulles, d'une colonne, dans l'ordre où elles
/// paraissent.
fn distinct_column(rows: &QueryResult, column: usize) -> Vec<CypherValue> {
    let mut vus = BTreeSet::new();
    rows.rows
        .iter()
        .filter_map(|r| r.get(column))
        .filter(|v| !v.is_null())
        .filter(|v| vus.insert(format!("{v:?}")))
        .cloned()
        .collect()
}

/// Des nœuds partis ensemble : une instruction.
struct Group {
    /// Leurs positions dans l'ordre topologique.
    members: Vec<usize>,
    text: String,
    params: Vec<QueryParam>,
    /// Un éventail : les branches (positions), dans l'ordre de leur rang ;
    /// les lignes de l'instruction portent ce rang en tête (`__branch`).
    branches: Option<Vec<usize>>,
}

/// La forme de sortie d'une sœur d'éventail : des sœurs se fondent quand
/// elle est la même (une union exige les mêmes colonnes). `None` : cette
/// forme ne se fond pas en éventail.
fn shape(form: &Form) -> Option<String> {
    use rag3weaver_ir::Count;
    match form {
        Form::Hop(h) => Some(format!("hop {:?}", h.returns)),
        Form::Count(Count::Edges { .. }) => Some("degree".to_string()),
        Form::Select(s) if s.by_uuids => Some(format!("select {:?}", s.returns)),
        _ => None,
    }
}

fn uuids_of(params: &[QueryParam]) -> Option<String> {
    params.iter().find(|p| p.name == "uuids").map(|p| format!("{:?}", p.value))
}

/// **Exécuter un graphe** : les groupes que le dialecte sait fondre, une
/// instruction chacun.
pub fn run_plan(
    dialect: &dyn SchemaDialect,
    conn: &dyn DbConnection,
    plan: &Plan,
    options: PlanOptions,
) -> Result<PlanReport, PlanError> {
    let ordre = plan.check().map_err(|e| PlanError::Invalid(e.to_string()))?;

    // Tout traduire d'abord : un refus ne laisse rien derrière lui.
    let mut textes = Vec::with_capacity(ordre.len());
    for n in &ordre {
        match translate(dialect, &n.form) {
            Ok(t) => textes.push(t),
            Err(TranslateError::Untranslated { form, .. }) => {
                return Err(PlanError::Refused { node: n.id.clone(), message: dialect.untranslated(form) })
            }
            Err(e @ TranslateError::Invalid(_)) => return Err(PlanError::Invalid(format!("nœud « {} » : {e}", n.id))),
        }
    }

    // Les groupes. Un nœud rejoint celui de son amont quand l'amont en est le
    // dernier, n'a que lui pour suivant, n'est pas une sortie, que leurs
    // paramètres ne se recouvrent pas, et que le dialecte sait les fondre.
    // Un groupe s'exécute à la place de son premier nœud : ses autres nœuds
    // ne dépendent que de lui, et personne d'autre ne dépend de ses nœuds
    // intermédiaires.
    //
    // **Les éventails** (page 04) : des sœurs de même forme nourries par la
    // même source partent en une instruction — les suivants d'un nœud qui
    // n'est pas une sortie et n'a qu'eux pour suivants, ou des nœuds sans
    // arête qui reçoivent la même liste `$uuids` de l'appelant.
    let position: BTreeMap<&str, usize> = ordre.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();
    let suivants_de = |id: &str| -> Vec<usize> {
        plan.edges.iter().filter(|e| e.from == id).map(|e| position[e.to.as_str()]).collect()
    };
    let mut groupes: Vec<Group> = Vec::new();
    let mut groupe_de: Vec<Option<usize>> = vec![None; ordre.len()];
    let sans_uuids = |i: usize| -> Vec<QueryParam> {
        let mut p = ordre[i].params.clone();
        p.retain(|q| q.name != "uuids");
        p
    };
    let recouvrent = |a: &[QueryParam], b: &[QueryParam]| a.iter().any(|q| b.iter().any(|r| r.name == q.name));
    for (i, n) in ordre.iter().enumerate() {
        if groupe_de[i].is_some() {
            continue; // déjà branche d'un éventail
        }
        let entree = plan.input_of(&n.id);

        // Un éventail sans amont : les sœurs sans arête, même `$uuids`, même
        // forme de sortie.
        if !options.split && entree.is_none() {
            if let (Some(cle), Some(forme)) = (uuids_of(&n.params), shape(&n.form)) {
                let soeurs: Vec<usize> = (i..ordre.len())
                    .filter(|&j| groupe_de[j].is_none() && plan.input_of(&ordre[j].id).is_none())
                    .filter(|&j| uuids_of(&ordre[j].params).as_ref() == Some(&cle) && shape(&ordre[j].form).as_ref() == Some(&forme))
                    .collect();
                let mut autres: Vec<QueryParam> = Vec::new();
                let libres = soeurs.iter().all(|&j| {
                    let p = sans_uuids(j);
                    let ok = !recouvrent(&p, &autres);
                    autres.extend(p);
                    ok
                });
                if soeurs.len() >= 2 && libres {
                    let branches: Vec<(usize, &str, &Form)> = soeurs.iter().map(|&j| (0, textes[j].as_str(), &ordre[j].form)).collect();
                    if let Some(texte) = dialect.fuse_fanout(None, &branches) {
                        let mut params = autres;
                        params.extend(n.params.iter().filter(|q| q.name == "uuids").cloned());
                        let g = groupes.len();
                        for &j in &soeurs {
                            groupe_de[j] = Some(g);
                        }
                        groupes.push(Group { members: soeurs.clone(), text: texte, params, branches: Some(soeurs) });
                        continue;
                    }
                }
            }
        }

        let mut params: Vec<QueryParam> = n.params.clone();
        if entree.is_some() {
            params.retain(|p| p.name != "uuids");
        }
        let fondu = entree.and_then(|e| {
            let p = position[e.from.as_str()];
            let g = groupe_de[p].expect("l'amont est placé avant");
            let groupe = &groupes[g];
            let libre = !options.split
                && groupe.branches.is_none()
                && groupe.members.last() == Some(&p)
                && suivants_de(&e.from).len() == 1
                && !plan.outputs.contains(&e.from)
                && !recouvrent(&params, &groupe.params);
            if !libre {
                return None;
            }
            dialect.fuse(&groupe.text, &ordre[p].form, e.columns[0], &textes[i]).map(|t| (g, t))
        });
        let g = match fondu {
            Some((g, texte)) => {
                groupes[g].members.push(i);
                groupes[g].text = texte;
                groupes[g].params.extend(params);
                g
            }
            None => {
                groupes.push(Group { members: vec![i], text: textes[i].clone(), params, branches: None });
                groupes.len() - 1
            }
        };
        groupe_de[i] = Some(g);

        // Un éventail sous ce nœud : tous ses suivants, de même forme, et lui
        // ni sortie ni lu par personne d'autre.
        let suivants = suivants_de(&n.id);
        if options.split || suivants.len() < 2 || plan.outputs.contains(&n.id) {
            continue;
        }
        let forme = shape(&ordre[suivants[0]].form);
        if forme.is_none() || suivants.iter().any(|&j| shape(&ordre[j].form) != forme) {
            continue;
        }
        let mut autres = groupes[g].params.clone();
        if !suivants.iter().all(|&j| {
            let p = sans_uuids(j);
            let ok = !recouvrent(&p, &autres);
            autres.extend(p);
            ok
        }) {
            continue;
        }
        let branches: Vec<(usize, &str, &Form)> = suivants
            .iter()
            .map(|&j| (plan.input_of(&ordre[j].id).expect("un suivant a son arête").columns[0], textes[j].as_str(), &ordre[j].form))
            .collect();
        if let Some(texte) = dialect.fuse_fanout(Some((&groupes[g].text, &n.form)), &branches) {
            for &j in &suivants {
                groupe_de[j] = Some(g);
            }
            groupes[g].members.extend(suivants.iter().copied());
            groupes[g].text = texte;
            groupes[g].params = autres;
            groupes[g].branches = Some(suivants);
        }
    }
    let noms = |g: &Group| g.members.iter().map(|&i| ordre[i].id.clone()).collect::<Vec<_>>();

    // L'atomicité se décide avant d'exécuter : un groupe est une instruction.
    let capacites = dialect.capabilities();
    let charge = ordre.iter().any(|n| matches!(n.form, Form::Write(rag3weaver_ir::Write::Load { .. })));
    let (atomic, atomic_reason, envelopper) = if groupes.len() <= 1 {
        (true, None, false)
    } else if !capacites.transactions {
        (false, Some("several statements, and this dialect declares no transactions"), false)
    } else if charge {
        (false, Some("several statements, and a bulk load (Write::Load) is not wrapped in a transaction"), false)
    } else {
        (true, None, true)
    };
    if options.require_atomic && !atomic {
        return Err(PlanError::NotAtomic { reason: atomic_reason.unwrap_or_default().to_string() });
    }

    let mut report = PlanReport { atomic, atomic_reason: atomic_reason.map(str::to_string), ..Default::default() };
    if envelopper {
        conn.begin().map_err(|e| PlanError::Failed { node: ordre[0].id.clone(), error: format!("begin: {e}"), rolled_back: false })?;
        if let Some(instantane) = dialect.snapshot_read().filter(|_| ordre.iter().any(|n| !n.form.is_write())) {
            if let Err(e) = conn.execute(instantane) {
                let _ = conn.rollback();
                return Err(PlanError::Failed { node: ordre[0].id.clone(), error: format!("{instantane}: {e}"), rolled_back: true });
            }
        }
    }

    // Les lignes de chaque groupe sont celles de son dernier nœud.
    let mut lignes: BTreeMap<&str, QueryResult> = BTreeMap::new();
    for g in &groupes {
        let premier = ordre[g.members[0]];
        let dernier = ordre[*g.members.last().expect("un groupe a un nœud")];
        let mut params = g.params.clone();
        if let Some(arete) = plan.input_of(&premier.id) {
            let uuids = distinct_column(&lignes[arete.from.as_str()], arete.columns[0]);
            if uuids.is_empty() {
                // Rien à lire ni à écrire : pas d'instruction, aucune ligne.
                lignes.insert(&dernier.id, QueryResult::default());
                continue;
            }
            params.push(QueryParam::new("uuids", CypherValue::List(uuids)));
        }
        match conn.execute_with_params(&g.text, &params) {
            Ok(r) => {
                report.instructions += 1;
                report.fused.push(noms(g));
                match &g.branches {
                    None => {
                        lignes.insert(&dernier.id, r);
                    }
                    // Les lignes rangées par branche, sans la colonne de rang.
                    Some(branches) => {
                        let colonnes: Vec<String> = r.columns.iter().skip(1).cloned().collect();
                        let mut par_branche: Vec<QueryResult> =
                            branches.iter().map(|_| QueryResult { columns: colonnes.clone(), rows: Vec::new() }).collect();
                        for mut ligne in r.rows {
                            let rang = ligne.first().and_then(|v| v.as_i64()).unwrap_or(-1);
                            if let Some(q) = usize::try_from(rang).ok().and_then(|k| par_branche.get_mut(k)) {
                                ligne.remove(0);
                                q.rows.push(ligne);
                            }
                        }
                        for (&b, q) in branches.iter().zip(par_branche) {
                            let branche = ordre[b];
                            lignes.insert(&branche.id, q);
                        }
                    }
                }
            }
            Err(e) => {
                let rolled_back = envelopper && conn.rollback().is_ok();
                return Err(PlanError::Failed { node: noms(g).join(" + "), error: e.to_string(), rolled_back });
            }
        }
    }
    if envelopper {
        if let Err(e) = conn.commit() {
            let rolled_back = conn.rollback().is_ok();
            return Err(PlanError::Failed { node: ordre[ordre.len() - 1].id.clone(), error: format!("commit: {e}"), rolled_back });
        }
    }
    for o in &plan.outputs {
        report.outputs.insert(o.clone(), lignes.remove(o.as_str()).unwrap_or_default());
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::CallbackConnection;
    use crate::dialect::{PostgresDialect, Rag3dbDialect, SchemaDialect};
    use rag3weaver_ir::{Column, LoadTarget, Select, Write};
    use std::sync::{Arc, Mutex};

    /// Une connexion qui note chaque texte reçu et ne rend rien.
    fn espion() -> (CallbackConnection, Arc<Mutex<Vec<String>>>) {
        let recu = Arc::new(Mutex::new(Vec::new()));
        let r = recu.clone();
        let conn = CallbackConnection::new(move |q: &str, _: &[QueryParam]| {
            r.lock().unwrap().push(q.to_string());
            Ok(QueryResult::default())
        });
        (conn, recu)
    }

    fn charger() -> Form {
        Form::Write(Write::Load { target: LoadTarget::Nodes { table: "Order".into(), columns: vec!["_uuid".into()] }, path: "/x.csv".into() })
    }

    fn lire() -> Form {
        Form::Select(Select::all("Order", vec![Column::Node("_uuid".into())]))
    }

    #[test]
    fn un_dialecte_qui_ne_sait_pas_une_forme_refuse_le_graphe_avant_d_executer_en_nommant_le_noeud() {
        let (conn, recu) = espion();
        let plan = Plan::new().node("lire", lire(), vec![]).node("charger", charger(), vec![]);
        let e = run_plan(&PostgresDialect, &conn, &plan, PlanOptions::default()).unwrap_err();
        assert_eq!(
            e,
            PlanError::Refused { node: "charger".into(), message: "RAG3WEAVER_REFUS__le_dialecte_postgresql_ne_traduit_pas__Write::Load".into() }
        );
        assert_eq!(e.to_string(), "RAG3WEAVER_REFUS__le_dialecte_postgresql_ne_traduit_pas__Write::Load (nœud « charger »)");
        assert!(recu.lock().unwrap().is_empty(), "rien n'est parti : {:?}", recu.lock().unwrap());
    }

    #[test]
    fn exiger_l_atomicite_refuse_avant_d_executer_un_graphe_qui_ne_l_aurait_pas() {
        let (conn, recu) = espion();
        let plan = Plan::new().node("lire", lire(), vec![]).node("charger", charger(), vec![]);
        let e = run_plan(&Rag3dbDialect, &conn, &plan, PlanOptions { require_atomic: true, ..Default::default() }).unwrap_err();
        assert!(matches!(&e, PlanError::NotAtomic { reason } if reason.contains("bulk load")), "{e:?}");
        assert!(recu.lock().unwrap().is_empty());

        // Sans l'exigence, il s'exécute et le dit.
        let r = run_plan(&Rag3dbDialect, &conn, &plan, PlanOptions::default()).unwrap();
        assert!(!r.atomic);
        assert!(r.atomic_reason.as_deref().unwrap_or("").contains("bulk load"), "{r:?}");
        assert_eq!(r.instructions, 2);
        assert_eq!(r.fused, vec![vec!["lire".to_string()], vec!["charger".to_string()]]);
        let recu = recu.lock().unwrap();
        assert!(!recu.iter().any(|q| q.contains("BEGIN")), "pas de transaction autour d'un chargement : {recu:?}");
    }

    #[test]
    fn plusieurs_instructions_partent_dans_une_transaction_et_postgres_y_fixe_l_instantane() {
        let (conn, recu) = espion();
        let plan = Plan::new().node("a", lire(), vec![]).node("b", lire(), vec![]).output("b");
        let r = run_plan(&PostgresDialect, &conn, &plan, PlanOptions { require_atomic: true, ..Default::default() }).unwrap();
        assert!(r.atomic && r.atomic_reason.is_none(), "{r:?}");
        let recu = recu.lock().unwrap();
        let lecture = PostgresDialect.select(&Select::all("Order", vec![Column::Node("_uuid".into())])).unwrap();
        assert_eq!(
            *recu,
            ["BEGIN TRANSACTION", "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ", &lecture, &lecture, "COMMIT"],
            "l'instantané d'abord, puis les deux lectures, dans la transaction"
        );
    }

    #[test]
    fn un_graphe_faux_ne_part_pas() {
        let (conn, recu) = espion();
        let plan = Plan::new().node("a", lire(), vec![]).output("z");
        assert!(matches!(run_plan(&Rag3dbDialect, &conn, &plan, PlanOptions::default()), Err(PlanError::Invalid(_))));
        assert!(recu.lock().unwrap().is_empty());
    }

    fn chaine() -> Plan {
        use rag3weaver_ir::{Count, Direction, Hop};
        Plan::new()
            .node("sel", Form::Select(Select::by_uuids("File", vec![Column::Node("_uuid".into())])), vec![QueryParam::new("uuids", CypherValue::List(vec![]))])
            .node("hop", Form::Hop(Hop::new("File", "defines", "Symbol", Direction::Outgoing)), vec![])
            .node("deg", Form::Count(Count::Edges { start: "Symbol".into(), relation: "uses".into(), direction: Direction::Incoming }), vec![])
            .edge("sel", 0, "hop")
            .edge("hop", 1, "deg")
            .output("deg")
    }

    #[test]
    fn rag3db_fond_une_chaine_en_une_instruction_et_le_dit() {
        let (conn, recu) = espion();
        let r = run_plan(&Rag3dbDialect, &conn, &chaine(), PlanOptions { require_atomic: true, ..Default::default() }).unwrap();
        assert_eq!(r.fused, vec![vec!["sel".to_string(), "hop".to_string(), "deg".to_string()]]);
        assert_eq!(r.instructions, 1);
        assert!(r.atomic && r.atomic_reason.is_none());
        let recu = recu.lock().unwrap();
        assert_eq!(
            *recu,
            ["UNWIND $uuids AS u MATCH (m:`File` {_uuid: u}) \
              WITH DISTINCT u AS __chain WHERE __chain IS NOT NULL WITH __chain AS u \
              MATCH (d:`File` {_uuid: u})-[r:`defines`]->(m:`Symbol`) \
              WITH DISTINCT m.`_uuid` AS __chain WHERE __chain IS NOT NULL WITH __chain AS u \
              MATCH (d:`Symbol` {_uuid: u})<-[r:`uses`]-() RETURN u, count(r)"],
            "une instruction, sans transaction autour"
        );
    }

    #[test]
    fn split_force_le_repli_decoupe() {
        let (conn, recu) = espion();
        let r = run_plan(&Rag3dbDialect, &conn, &chaine(), PlanOptions { split: true, ..Default::default() }).unwrap();
        assert_eq!(r.fused.len(), 1, "l'espion ne rend aucune ligne : la sélection part seule, le reste n'a pas d'entrée");
        assert_eq!(recu.lock().unwrap().iter().filter(|q| q.starts_with("UNWIND")).count(), 1);
    }

    #[test]
    fn un_amont_qui_est_une_sortie_ou_nourrit_deux_noeuds_ne_se_fond_pas() {
        let (conn, _) = espion();
        let r = run_plan(&Rag3dbDialect, &conn, &chaine().output("hop"), PlanOptions::default()).unwrap();
        assert_eq!(r.fused, vec![vec!["sel".to_string(), "hop".to_string()]], "hop est une sortie : deg ne s'y fond pas, et n'a pas d'entrée ici");

        let deux = chaine().node("deg2", Form::Count(rag3weaver_ir::Count::Edges {
            start: "File".into(), relation: "defines".into(), direction: rag3weaver_ir::Direction::Outgoing,
        }), vec![]).edge("sel", 0, "deg2");
        let r = run_plan(&Rag3dbDialect, &conn, &deux, PlanOptions::default()).unwrap();
        assert_eq!(r.fused, vec![vec!["sel".to_string()]], "sel nourrit deux nœuds : il part seul, {r:?}");
    }

    #[test]
    fn postgresql_fond_une_chaine_en_cte() {
        let (conn, recu) = espion();
        let r = run_plan(&PostgresDialect, &conn, &chaine(), PlanOptions { require_atomic: true, ..Default::default() }).unwrap();
        assert_eq!(r.fused, vec![vec!["sel".to_string(), "hop".to_string(), "deg".to_string()]]);
        let recu = recu.lock().unwrap();
        assert_eq!(
            *recu,
            ["WITH __chain(c0, c1) AS (\
              WITH __chain(c0, c1) AS (\
              SELECT v.u, m.\"_uuid\" FROM unnest($uuids::text[]) AS v(u) JOIN \"file\" AS m ON m.\"_uuid\" = v.u) \
              SELECT v.u, r.\"to_uuid\" FROM unnest(ARRAY(SELECT DISTINCT c0 FROM __chain WHERE c0 IS NOT NULL)::text[]) AS v(u) \
              JOIN \"file\" AS d ON d.\"_uuid\" = v.u JOIN \"defines\" AS r ON r.\"from_uuid\" = v.u \
              JOIN \"symbol\" AS m ON m.\"_uuid\" = r.\"to_uuid\") \
              SELECT v.u, count(*) FROM unnest(ARRAY(SELECT DISTINCT c1 FROM __chain WHERE c1 IS NOT NULL)::text[]) AS v(u) \
              JOIN \"uses\" AS r ON r.\"to_uuid\" = v.u GROUP BY v.u"],
            "une instruction, sans transaction autour"
        );
    }

    #[test]
    fn une_ecriture_ne_se_fond_jamais_en_amont() {
        let up = Form::Write(Write::Upsert { table: "Symbol".into(), columns: vec!["_uuid".into(), "note".into()] });
        let sel = Form::Select(Select::by_uuids("Symbol", vec![Column::Node("note".into())]));
        for d in [&Rag3dbDialect as &dyn SchemaDialect, &PostgresDialect] {
            let texte_up = super::translate(d, &up).unwrap();
            let texte_sel = super::translate(d, &sel).unwrap();
            assert_eq!(d.fuse(&texte_up, &up, 1, &texte_sel), None, "{}", d.name());
        }
    }

    #[test]
    fn postgresql_traduit_le_saut_et_refuse_la_ligne_entiere_en_nommant_le_noeud() {
        use rag3weaver_ir::{Direction, Hop};
        let mut saut = Hop::new("File", "defines", "Symbol", Direction::Outgoing);
        saut.returns.push(Column::Node("name".into()));
        assert_eq!(
            PostgresDialect.hop(&saut).unwrap(),
            "SELECT v.u, r.\"to_uuid\", m.\"name\" FROM unnest($uuids::text[]) AS v(u) \
             JOIN \"file\" AS d ON d.\"_uuid\" = v.u JOIN \"defines\" AS r ON r.\"from_uuid\" = v.u \
             JOIN \"symbol\" AS m ON m.\"_uuid\" = r.\"to_uuid\""
        );
        saut.returns.push(Column::Whole);
        let (conn, recu) = espion();
        let plan = Plan::new().node("lire", lire(), vec![]).node("tout", Form::Hop(saut), vec![]);
        let e = run_plan(&PostgresDialect, &conn, &plan, PlanOptions::default()).unwrap_err();
        assert_eq!(e.to_string(), "RAG3WEAVER_REFUS__le_dialecte_postgresql_ne_traduit_pas__Hop (ligne entière) (nœud « tout »)");
        assert!(recu.lock().unwrap().is_empty());
    }

    fn deux_degres() -> Plan {
        use rag3weaver_ir::{Count, Direction};
        let degre = |direction| Form::Count(Count::Edges { start: "Symbol".into(), relation: "uses".into(), direction });
        let u = || vec![QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String("a".into())]))];
        Plan::new().node("in", degre(Direction::Incoming), u()).node("out", degre(Direction::Outgoing), u()).output("in").output("out")
    }

    #[test]
    fn un_eventail_part_en_une_instruction_union_all() {
        let (conn, recu) = espion();
        let r = run_plan(&Rag3dbDialect, &conn, &deux_degres(), PlanOptions::default()).unwrap();
        assert_eq!(r.fused, vec![vec!["in".to_string(), "out".to_string()]]);
        assert_eq!(
            *recu.lock().unwrap(),
            ["UNWIND $uuids AS u MATCH (d:`Symbol` {_uuid: u})<-[r:`uses`]-() RETURN 0 AS __branch, u AS c0, count(r) AS c1 \
              UNION ALL \
              UNWIND $uuids AS u MATCH (d:`Symbol` {_uuid: u})-[r:`uses`]->() RETURN 1 AS __branch, u AS c0, count(r) AS c1"]
        );
        let (conn, recu) = espion();
        run_plan(&PostgresDialect, &conn, &deux_degres(), PlanOptions::default()).unwrap();
        assert_eq!(
            *recu.lock().unwrap(),
            ["SELECT 0 AS __branch, b0.* FROM (SELECT v.u, count(*) FROM unnest($uuids::text[]) AS v(u) \
              JOIN \"uses\" AS r ON r.\"to_uuid\" = v.u GROUP BY v.u) AS b0 \
              UNION ALL \
              SELECT 1 AS __branch, b1.* FROM (SELECT v.u, count(*) FROM unnest($uuids::text[]) AS v(u) \
              JOIN \"uses\" AS r ON r.\"from_uuid\" = v.u GROUP BY v.u) AS b1"]
        );
    }

    #[test]
    fn des_soeurs_de_formes_differentes_ou_d_entrees_differentes_ne_se_fondent_pas() {
        let mut p = deux_degres();
        p.nodes[1].params = vec![QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String("b".into())]))];
        let (conn, _) = espion();
        let r = run_plan(&Rag3dbDialect, &conn, &p, PlanOptions::default()).unwrap();
        assert_eq!(r.fused.len(), 2, "deux listes différentes : {r:?}");
        let r = run_plan(&Rag3dbDialect, &conn, &deux_degres(), PlanOptions { split: true, ..Default::default() }).unwrap();
        assert_eq!(r.fused.len(), 2, "split : {r:?}");
    }
}
