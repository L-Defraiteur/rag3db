//! **Une base PostgreSQL par passe** (14 novembre 2026, tickets de recherche et d'ED) : deux passes qui jouaient
//! e2e_postgres sur le même service effaçaient chacune les schémas `public` et `rag3weaver` de l'autre (« referenced
//! schema was concurrently dropped », « relation rag3weaver._dataflow_node_state does not exist »). Le schéma
//! `rag3weaver` est fixé par le produit (le dialecte) : c'est la base qui se sépare, pas le schéma.
//!
//! `run_e2e.sh` lance le binaire e2e une fois par suite, donc un processus par suite. Au premier appel, le processus
//! crée `rag3weaver_test_<poste>_<pid>` sur le service que nomme `RAG3WEAVER_PG` (ou le conteneur de 5433), et rend
//! l'adresse de cette base. Il l'efface à sa sortie (`atexit`, `DROP DATABASE … WITH (FORCE)`). Une sortie par signal
//! ne passe pas par `atexit` : au départ, chaque processus efface aussi les bases de son poste dont le pid n'existe
//! plus. `RAG3WEAVER_PG_BASE_FIXE=1` garde la base de `RAG3WEAVER_PG` telle quelle (un service à soi, sans voisin).
use std::sync::OnceLock;

use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::postgres_connection::PostgresConnection;

const PREFIXE: &str = "rag3weaver_test_";

/// L'adresse du service, telle que l'appelant la donne.
pub fn adresse_du_service() -> String {
    std::env::var("RAG3WEAVER_PG").unwrap_or_else(|_| {
        "host=localhost port=5433 user=rag3weaver password=rag3weaver dbname=rag3weaver_test".to_string()
    })
}

/// La même adresse, la base remplacée : `postgres://u:p@h:p/<base>?…` ou `… dbname=<base> …`.
pub fn avec_la_base(adresse: &str, base: &str) -> String {
    if let Some(reste) = adresse.strip_prefix("postgres://").map(|r| ("postgres://", r))
        .or_else(|| adresse.strip_prefix("postgresql://").map(|r| ("postgresql://", r)))
    {
        let (schema, corps) = reste;
        let (avant, requete) = match corps.find('?') {
            Some(i) => (&corps[..i], &corps[i..]),
            None => (corps, ""),
        };
        let autorite = avant.split('/').next().unwrap_or(avant);
        return format!("{schema}{autorite}/{base}{requete}");
    }
    let mut mots: Vec<String> = adresse.split_whitespace().filter(|m| !m.starts_with("dbname=")).map(String::from).collect();
    mots.push(format!("dbname={base}"));
    mots.join(" ")
}

/// Le début du nom des bases d'un poste : préfixe, poste (minuscules, alphanumérique), un souligné.
pub fn prefixe_du_poste(poste: &str) -> String {
    let poste: String = poste.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(24).collect();
    format!("{PREFIXE}{poste}_")
}

/// Le nom de la base d'un processus : le début du poste, puis le pid.
pub fn nom_de_base(poste: &str, pid: u32) -> String {
    format!("{}{pid}", prefixe_du_poste(poste))
}

fn poste() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname").map(|s| s.trim().to_string()).unwrap_or_else(|_| "poste".into())
}

fn admin() -> PostgresConnection {
    PostgresConnection::new(&adresse_du_service())
        .unwrap_or_else(|e| panic!("PostgreSQL injoignable ({e}) : RAG3WEAVER_PG, ou le conteneur sur 5433"))
}

/// Les bases orphelines de ce poste : un nom de la forme de `nom_de_base`, un pid qui n'existe plus.
pub fn orphelines(noms: &[String], poste_ici: &str, vivant: impl Fn(u32) -> bool) -> Vec<String> {
    let debut = prefixe_du_poste(poste_ici);
    noms.iter()
        .filter_map(|n| n.strip_prefix(debut.as_str()).and_then(|pid| pid.parse::<u32>().ok()).map(|pid| (n, pid)))
        .filter(|(_, pid)| !vivant(*pid))
        .map(|(n, _)| n.clone())
        .collect()
}

static BASE: OnceLock<Option<String>> = OnceLock::new();

extern "C" fn effacer_a_la_sortie() {
    if let Some(Some(base)) = BASE.get() {
        if let Ok(conn) = PostgresConnection::new(&adresse_du_service()) {
            let _ = conn.execute(&format!("DROP DATABASE IF EXISTS \"{base}\" WITH (FORCE)"));
        }
    }
}

/// L'adresse de la base de ce processus, créée au premier appel.
pub fn conn_str() -> String {
    if std::env::var("RAG3WEAVER_PG_BASE_FIXE").as_deref() == Ok("1") {
        return adresse_du_service();
    }
    let base = BASE.get_or_init(|| {
        let conn = admin();
        let ici = poste();
        let noms: Vec<String> = conn
            .execute(&format!("SELECT datname FROM pg_database WHERE datname LIKE '{PREFIXE}%'"))
            .map(|r| r.rows.iter().filter_map(|l| match l.first() { Some(CypherValue::String(s)) => Some(s.clone()), _ => None }).collect())
            .unwrap_or_default();
        for orpheline in orphelines(&noms, &ici, |pid| std::path::Path::new(&format!("/proc/{pid}")).exists()) {
            eprintln!("[base_pg] base orpheline effacée : {orpheline}");
            let _ = conn.execute(&format!("DROP DATABASE IF EXISTS \"{orpheline}\" WITH (FORCE)"));
        }
        let base = nom_de_base(&ici, std::process::id());
        let _ = conn.execute(&format!("DROP DATABASE IF EXISTS \"{base}\" WITH (FORCE)"));
        conn.execute(&format!("CREATE DATABASE \"{base}\""))
            .unwrap_or_else(|e| panic!("CREATE DATABASE {base} : {e} (l'utilisateur du service a-t-il CREATEDB ?)"));
        unsafe { libc::atexit(effacer_a_la_sortie) };
        eprintln!("[base_pg] base de la passe : {base}");
        Some(base)
    });
    avec_la_base(&adresse_du_service(), base.as_deref().expect("base de la passe"))
}
