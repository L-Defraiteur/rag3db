//! **rag3daemon** : le processus qui tient la base, et la sert.
//!
//! ```text
//! rag3daemon --adresse 127.0.0.1:7979 --base /chemin/vers/la/base
//! rag3daemon --adresse 127.0.0.1:7979 --base :memoire:
//! rag3daemon --adresse 0.0.0.0:7979 --base /chemin --exposer   # voir l'issue 05
//! ```
//!
//! Personne n'a normalement à le lancer à la main : `DaemonConnection::assurer`
//! le fait s'il ne répond pas déjà, avec exactement ces arguments.
//!
//! Une base rag3db ne s'ouvre que par un seul processus — `F_WRLCK` posé en
//! `F_SETLK`, refus immédiat pour le second. Ce binaire est ce processus-là,
//! mis derrière une adresse : un seul écrivain, plusieurs programmes qui lui
//! parlent. Voir `src/daemon/db.rs`.

use std::sync::Arc;

use rag3weaver::connection::DbConnection;
use rag3weaver::daemon::DbDaemon;
use rag3weaver::Rag3dbConnection;

/// La base en mémoire, pour un démon qui ne survit à rien — les tests.
const MEMOIRE: &str = ":memoire:";

fn main() -> std::process::ExitCode {
    match servir() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("✗ {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn servir() -> Result<(), String> {
    let (adresse, base, expose) = arguments()?;

    let conn: Arc<dyn DbConnection> = if base == MEMOIRE {
        Arc::new(Rag3dbConnection::in_memory().map_err(|e| format!("base en mémoire : {e}"))?)
    } else {
        Arc::new(Rag3dbConnection::new(&base).map_err(|e| {
            format!(
                "ouverture de {base} : {e}\n  \
                 (une base rag3db ne s'ouvre que par un processus à la fois — \
                 un autre démon la tient peut-être déjà)"
            )
        })?)
    };

    let demon = DbDaemon::new(conn).base(&base).expose(expose);
    // Sur la sortie d'erreur, donc dans le journal du serveur : c'est là qu'on
    // regarde quand `assurer` rend `Muet`.
    // **Refuser avant d'annoncer.** Sinon le journal dit « à l'écoute sur
    // 0.0.0.0 » juste avant d'échouer, et c'est la ligne qu'on croira.
    if !expose && !rag3weaver::daemon::est_local(&adresse) {
        return Err(rag3weaver::daemon::DaemonError::Exposition { adresse }.to_string());
    }
    eprintln!("▸ rag3daemon sur {adresse} — base {base}");
    demon.servir(&adresse).map_err(|e| e.to_string())
}

/// `--adresse <hôte:port>` et `--base <chemin|:memoire:>`.
fn arguments() -> Result<(String, String, bool), String> {
    let mut args = std::env::args().skip(1);
    let mut adresse = "127.0.0.1:7979".to_string();
    let mut base = MEMOIRE.to_string();
    let mut expose = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--adresse" => adresse = args.next().ok_or("--adresse attend une valeur")?,
            "--base" => base = args.next().ok_or("--base attend une valeur")?,
            "--exposer" => expose = true,
            autre => return Err(format!("argument inconnu : {autre}")),
        }
    }
    Ok((adresse, base, expose))
}
