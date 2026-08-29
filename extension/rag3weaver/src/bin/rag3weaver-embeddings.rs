//! **Le démon d'embedding** : charge BGE-M3 une fois, le sert à qui le demande.
//!
//! ```text
//! rag3weaver-embeddings --adresse 127.0.0.1:7878
//! ```
//!
//! Personne n'a normalement à le lancer à la main : `DaemonEmbedder::assurer`
//! le fait s'il ne répond pas déjà, avec exactement ces arguments. Le lancer
//! soi-même sert à une chose — voir le chargement, et son journal, en direct.
//!
//! Les poids se trouvent comme partout ailleurs dans la crate :
//! `RAG3WEAVER_BGE_M3_BPK` et `RAG3WEAVER_BGE_M3_TOKENIZER`, ou à défaut
//! `~/.cache/rag3weaver/bge-m3/` — la même convention que les tests E2E, pour
//! que le démon serve exactement les poids qu'ils servaient eux-mêmes. La carte
//! se choisit par `RAG3WEAVER_BURN_DEVICE_EMBEDDER` (ou `RAG3WEAVER_BURN_DEVICE`
//! pour tous les rôles) — et il faut s'en servir : un démon qui vit des heures
//! sur la carte d'affichage rend le poste inutilisable.

use std::path::PathBuf;
use std::sync::Arc;

use rag3weaver::burn_bge_m3_embedder::BurnBgeM3Embedder;
use rag3weaver::burn_device::{BurnDevice, BurnRole};
use rag3weaver::daemon::EmbedDaemon;

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
    let adresse = adresse()?;
    let bpk = artefact("RAG3WEAVER_BGE_M3_BPK", "model.bpk")?;
    let tokenizer = artefact("RAG3WEAVER_BGE_M3_TOKENIZER", "tokenizer.json")?;

    // Tracé sur la sortie d'erreur, donc dans le journal du serveur : c'est
    // là qu'on regarde quand `assurer` rend `Muet`.
    let debut = std::time::Instant::now();
    eprintln!("▸ chargement de BGE-M3 depuis {}", bpk.display());
    let octets = std::fs::read(&bpk).map_err(|e| format!("lecture de {} : {e}", bpk.display()))?;
    // **Le rôle, pas le défaut.** Un démon qui vit des heures et tient 2,2 Go
    // sur la carte qui porte l'affichage rend le poste inutilisable — mesuré
    // le 29 août : carte d'affichage à 100 % et 18,9 Go de VRAM pendant une
    // passe. `RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:N` le déplace ; sans elle on
    // reste sur le défaut, comme avant.
    let carte = BurnDevice::for_role(BurnRole::Embedder);
    let modele = BurnBgeM3Embedder::from_bytes(&octets, &tokenizer, carte)
        .map_err(|e| format!("construction du modèle : {e}"))?;
    let modele = Arc::new(modele);
    eprintln!("  chargé en {:?}", debut.elapsed());

    // Le même objet des trois côtés : BGE-M3 rend dense et creux en une passe,
    // c'est tout son intérêt — le démon n'a aucune raison de le couper en deux.
    // Le creux seul est offert aussi, pour qui n'a pas besoin du dense : c'est
    // du trafic en moins sur le fil, pas du calcul en moins.
    let demon = EmbedDaemon::new(modele.clone())
        .avec_dual(modele.clone())
        .avec_sparse(modele);
    eprintln!("▸ à l'écoute sur {adresse} — {:?}", demon.identite());
    demon.servir(&adresse).map_err(|e| e.to_string())
}

/// `--adresse <hôte:port>`, ou `127.0.0.1:7878`.
fn adresse() -> Result<String, String> {
    let mut args = std::env::args().skip(1);
    let mut adresse = "127.0.0.1:7878".to_string();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--adresse" => {
                adresse = args.next().ok_or("--adresse attend une valeur")?;
            }
            autre => return Err(format!("argument inconnu : {autre}")),
        }
    }
    Ok(adresse)
}

/// Un artefact du modèle : la variable d'environnement si elle est là, sinon
/// `~/.cache/rag3weaver/bge-m3/<nom>` — la convention des tests E2E.
fn artefact(variable: &str, nom: &str) -> Result<PathBuf, String> {
    let chemin = match std::env::var(variable) {
        Ok(v) => PathBuf::from(v),
        Err(_) => cache().join(nom),
    };
    if !chemin.exists() {
        return Err(format!(
            "{} est introuvable — donnez {variable}, ou placez le fichier là",
            chemin.display()
        ));
    }
    Ok(chemin)
}

fn cache() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
        .join(".cache/rag3weaver/bge-m3")
}
