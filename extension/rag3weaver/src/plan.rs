//! **Exécuter un graphe de formes** ([`rag3weaver_ir::Plan`]) confié entier
//! au dialecte ; voir
//! `docs/11-octobre-2026-embarquements/01-chainage-des-formes.md`.
//!
//! Ici, le **repli découpé**, commun à tous les dialectes : un nœud, une
//! instruction, dans l'ordre topologique ; une arête passe en `$uuids` les
//! valeurs distinctes de sa colonne, dans l'ordre où elles paraissent. La
//! fusion de plusieurs nœuds en une instruction viendra dialecte par
//! dialecte, prouvée équivalente à ce repli sur le même scénario.
//!
//! L'ordre des étapes ne change pas : la vérification commune
//! ([`Plan::check`](rag3weaver_ir::Plan::check)), puis la traduction de
//! **tous** les nœuds (un refus part avant toute exécution, rien n'est lu ni
//! écrit), puis la décision d'atomicité, puis l'exécution.

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

/// **Exécuter un graphe** par le repli découpé : un nœud, une instruction.
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

    // L'atomicité se décide avant d'exécuter : un nœud est une instruction.
    let capacites = dialect.capabilities();
    let charge = ordre.iter().any(|n| matches!(n.form, Form::Write(rag3weaver_ir::Write::Load { .. })));
    let (atomic, atomic_reason, envelopper) = if ordre.len() <= 1 {
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

    let mut lignes: BTreeMap<&str, QueryResult> = BTreeMap::new();
    for (n, texte) in ordre.iter().zip(&textes) {
        let mut params = n.params.clone();
        if let Some(arete) = plan.input_of(&n.id) {
            let amont = &lignes[arete.from.as_str()];
            let uuids = distinct_column(amont, arete.columns[0]);
            if uuids.is_empty() {
                // Rien à lire ni à écrire : pas d'instruction, aucune ligne.
                lignes.insert(&n.id, QueryResult::default());
                continue;
            }
            params.retain(|p| p.name != "uuids");
            params.push(QueryParam::new("uuids", CypherValue::List(uuids)));
        }
        match conn.execute_with_params(texte, &params) {
            Ok(r) => {
                report.instructions += 1;
                report.fused.push(vec![n.id.clone()]);
                lignes.insert(&n.id, r);
            }
            Err(e) => {
                let rolled_back = envelopper && conn.rollback().is_ok();
                return Err(PlanError::Failed { node: n.id.clone(), error: e.to_string(), rolled_back });
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
        let e = run_plan(&Rag3dbDialect, &conn, &plan, PlanOptions { require_atomic: true }).unwrap_err();
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
        let r = run_plan(&PostgresDialect, &conn, &plan, PlanOptions { require_atomic: true }).unwrap();
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
}
