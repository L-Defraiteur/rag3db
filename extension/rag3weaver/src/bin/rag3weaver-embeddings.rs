//! **Le démon d'embedding** : charge BGE-M3 une fois, le sert à qui le demande.
//!
//! ```text
//! rag3weaver-embeddings --adresse 127.0.0.1:7878
//! rag3weaver-embeddings --adresse 0.0.0.0:7878 --exposer   # voir l'issue 05
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
use rag3weaver::embedder::Embedder;

fn main() -> std::process::ExitCode {
    // Ce que burn et cubecl disent de la carte passe par `log` : sans
    // collecteur, ça n'existe pas. `RUST_LOG=cubecl_wgpu=debug,cubecl_runtime=info`
    // pour les tailles CMMA vues et chaque autotune ; `warn` par défaut.
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).try_init();
    match servir() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("✗ {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Le modèle servi quand `RAG3WEAVER_EMBED_MODEL` ne dit rien (décision de
/// Lucie, 7 septembre 2026, sur le banc de qualité).
pub const MODELE_PAR_DEFAUT: &str = "granite-278m";

fn servir() -> Result<(), String> {
    let (adresse, expose) = adresse()?;
    // **Le modèle servi se choisit par `RAG3WEAVER_EMBED_MODEL`** (6 septembre
    // 2026) : `granite-278m` (768 d, **le défaut** depuis le 7 septembre :
    // devant BGE-M3 sur nos requêtes de code, MRR 0,844 contre 0,793, à 2,5
    // fois sa vitesse — `docs/optimiseur/6-septembre-2026-16h30/04`),
    // `granite-107m` (384 d, le régime rapide : à 6–9 points derrière, deux
    // fois plus vite, un index deux fois plus petit), `bge-m3` (1 024 d, dense
    // + creux, 8 192 jetons : le second étage et les documents). Le nom et la
    // dimension partent dans l'Identite ; c'est au catalogue de refuser un
    // index construit avec l'un et interrogé avec l'autre
    // (`check_embedding_model`) — un index en 107m ne se lit pas en 278m, c'est
    // voulu. Les poids : `RAG3WEAVER_<MODELE>_BPK` / `_TOKENIZER`, sinon
    // `~/.cache/rag3weaver/<modele>/`.
    let nom = std::env::var("RAG3WEAVER_EMBED_MODEL").ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| MODELE_PAR_DEFAUT.into());
    let (dossier, prefixe) = match nom.as_str() {
        "bge-m3" => ("bge-m3", "RAG3WEAVER_BGE_M3"),
        "granite-107m" => ("granite-107m", "RAG3WEAVER_GRANITE_107M"),
        "granite-278m" => ("granite-278m", "RAG3WEAVER_GRANITE_278M"),
        autre => return Err(format!("RAG3WEAVER_EMBED_MODEL={autre} : granite-278m (défaut), granite-107m ou bge-m3")),
    };
    let bpk = artefact(&format!("{prefixe}_BPK"), dossier, "model.bpk")?;
    let tokenizer = artefact(&format!("{prefixe}_TOKENIZER"), dossier, "tokenizer.json")?;

    // Tracé sur la sortie d'erreur, donc dans le journal du serveur : c'est
    // là qu'on regarde quand `assurer` rend `Muet`.
    let debut = std::time::Instant::now();
    eprintln!("▸ chargement de {nom} depuis {}", bpk.display());
    let octets = std::fs::read(&bpk).map_err(|e| format!("lecture de {} : {e}", bpk.display()))?;
    // **Le rôle, pas le défaut.** Un démon qui vit des heures et tient 2,2 Go
    // sur la carte qui porte l'affichage rend le poste inutilisable — mesuré
    // le 29 août : carte d'affichage à 100 % et 18,9 Go de VRAM pendant une
    // passe. `RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:N` le déplace ; sans elle on
    // reste sur le défaut, comme avant.
    let carte = BurnDevice::for_role(BurnRole::Embedder);
    let demon = match nom.as_str() {
        "bge-m3" => {
            let modele = BurnBgeM3Embedder::from_bytes(&octets, &tokenizer, carte)
                .map_err(|e| format!("construction du modèle : {e}"))?;
            let modele = Arc::new(modele);
            eprintln!("  chargé en {:?}", debut.elapsed());
            // Le même objet des trois côtés : BGE-M3 rend dense et creux en une
            // passe, c'est tout son intérêt — le démon n'a aucune raison de le
            // couper en deux. Le creux seul est offert aussi, pour qui n'a pas
            // besoin du dense : c'est du trafic en moins sur le fil, pas du
            // calcul en moins.
            EmbedDaemon::new(modele.clone()).avec_dual(modele.clone()).avec_sparse(modele)
        }
        "granite-107m" => {
            let modele: Arc<dyn Embedder> = Arc::new(
                rag3weaver::BurnGranite107m::from_bytes(&octets, &tokenizer, carte)
                    .map_err(|e| format!("construction du modèle : {e}"))?,
            );
            eprintln!("  chargé en {:?}", debut.elapsed());
            EmbedDaemon::new(modele)
        }
        _ => {
            let modele: Arc<dyn Embedder> = Arc::new(
                rag3weaver::BurnGranite278m::from_bytes(&octets, &tokenizer, carte)
                    .map_err(|e| format!("construction du modèle : {e}"))?,
            );
            eprintln!("  chargé en {:?}", debut.elapsed());
            EmbedDaemon::new(modele)
        }
    }
    .expose(expose);
    // **Refuser avant d'annoncer.** Sinon le journal dit « à l'écoute sur
    // 0.0.0.0 » juste avant d'échouer, et c'est la ligne qu'on croira.
    if !expose && !rag3weaver::daemon::est_local(&adresse) {
        return Err(rag3weaver::daemon::DaemonError::Exposition { adresse }.to_string());
    }
    // **Chauffer avant de servir.** Le cache d'autotune est global et chaud,
    // mais chaque processus recompile ses noyaux (SPIR-V, puis le pipeline
    // radv) à la première rencontre de chaque classe de forme : un démon
    // fraîchement remplacé faisait payer ça au premier drain (36 s → 44 s sur
    // src/dataflow, 6 septembre 2026). Ici on passe les classes typiques —
    // longueurs 64, 128, 256, 512 jetons, lots 1, 8, 32 et le conseil du
    // modèle — avant d'annoncer l'adresse. `RAG3WEAVER_DEMON_CHAUFFE=0` pour
    // s'en passer.
    if std::env::var("RAG3WEAVER_DEMON_CHAUFFE").map(|v| v != "0").unwrap_or(true) {
        let t = std::time::Instant::now();
        let (lot_max, _) = demon.identite().lot_conseille.unwrap_or((32, 512));
        let mut formes = 0;
        for mots in [40usize, 90, 190, 380] {
            for lot in [1usize, 8, 32, lot_max] {
                let textes: Vec<String> = (0..lot)
                    .map(|i| (0..mots).map(|j| format!("m{}", (i * 7 + j * 13) % 97)).collect::<Vec<_>>().join(" "))
                    .collect();
                if demon.embedder().embed(&textes).is_ok() {
                    formes += 1;
                }
            }
        }
        eprintln!("  chauffe : {formes} classes de forme en {:?}", t.elapsed());
    }
    eprintln!("▸ à l'écoute sur {adresse} — {:?}", demon.identite());
    demon.servir(&adresse).map_err(|e| e.to_string())
}

/// `--adresse <hôte:port>`, ou `127.0.0.1:7878`.
fn adresse() -> Result<(String, bool), String> {
    let mut args = std::env::args().skip(1);
    let mut adresse = "127.0.0.1:7878".to_string();
    let mut expose = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--adresse" => {
                adresse = args.next().ok_or("--adresse attend une valeur")?;
            }
            "--exposer" => expose = true,
            autre => return Err(format!("argument inconnu : {autre}")),
        }
    }
    Ok((adresse, expose))
}

/// Un artefact du modèle : la variable d'environnement si elle est là, sinon
/// `~/.cache/rag3weaver/bge-m3/<nom>` — la convention des tests E2E.
fn artefact(variable: &str, dossier: &str, nom: &str) -> Result<PathBuf, String> {
    let chemin = match std::env::var(variable) {
        Ok(v) => PathBuf::from(v),
        Err(_) => cache(dossier).join(nom),
    };
    if !chemin.exists() {
        return Err(format!(
            "{} est introuvable — donnez {variable}, ou placez le fichier là",
            chemin.display()
        ));
    }
    Ok(chemin)
}

fn cache(dossier: &str) -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
        .join(".cache/rag3weaver")
        .join(dossier)
}
