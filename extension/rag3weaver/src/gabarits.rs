//! **Chaque gabarit livré se charge, et la lib rougit sinon.**
//!
//! Ce test n'existait pas, et son absence a laissé passer une journée entière
//! un gabarit `memory` qui **ne chargeait plus du tout** : j'y avais déclaré une
//! fiche réactive parmi les outils, et un outil de backend ne peut contenir que
//! les nœuds d'une liste blanche — le backend refusait le manifeste **entier**.
//! Mes suites étaient vertes, le banc ne tournait pas, et c'est une sonde écrite
//! à la main qui l'a trouvé (4 octobre 2026).
//!
//! **Un gabarit qui ne charge plus doit rougir la lib, pas attendre une sonde.**
//!
//! Pourquoi ce test tient dans la lib, sans base ni service :
//! `PreparedBackend::load` fait déjà tout ce qui compte **sans ouvrir de base**
//! — il lit le manifeste, lit chaque graphe d'outil sur le disque, le construit,
//! le lie au registre de nœuds, vérifie que chaque liaison désigne un paramètre
//! qui existe, et applique la politique des nœuds permis. C'est exactement
//! l'étage où la régression est tombée.
//!
//! Ce que le test **ne** prouve pas, et le dit : il ne joue aucun verbe. Un
//! gabarit peut charger et rendre faux. C'est le travail des bancs ; celui-ci
//! répond à une seule question — « est-ce que ça monte ? » — et c'est déjà celle
//! qui a manqué.

/// Un gabarit livré, rendu chargeable dans un dossier : ses chemins en absolu, sa
/// base et la racine de son espace de travail dans `ou`. Pour les témoins du crate
/// (ici, et l'IDE de `daemon_host`).
#[cfg(test)]
pub(crate) mod jouable {
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    /// Les chemins d'un gabarit livré sont **relatifs à lui** (`graphs/x.mmd`,
    /// `../notebook/graphs/put.mmd`, `schemas/y.json`) et son `database` est un
    /// emplacement que le produit remplit. On réécrit donc le manifeste en
    /// absolu dans un dossier temporaire — la même chose que fait le banc, et
    /// que ferait n'importe quel appelant réel.
    pub(crate) fn manifeste_jouable(gabarit: &Path, ou: &Path) -> Result<PathBuf, String> {
        let brut = std::fs::read(gabarit.join("backend.json")).map_err(|e| e.to_string())?;
        let mut m: Value = serde_json::from_slice(&brut).map_err(|e| e.to_string())?;

        // **Tout chemin se réécrit, et on le trouve par sa clé, pas par sa
        // place.** Première version : j'énumérais les endroits (`graph`,
        // `schema`, `scripts`) et deux gabarits sont tombés — `input_schema` et
        // `before[].graph` d'un crochet, que j'avais oubliés. Un parcours par
        // clé les prend tous, et prendra ceux qu'on ajoutera.
        fn absolutiser(v: &mut Value, gabarit: &Path) {
            const CLES: &[&str] = &[
                "graph", "schema", "input_schema",
                // Le gabarit jinja d'un fichier composé (`files.template`).
                "template",
                // Les scripts d'un manifeste sont une table nom → chemin : la
                // clé est le nom du script, pas « script ». On les prend donc
                // par leur extension, plus bas.
            ];
            match v {
                Value::Object(o) => {
                    for (k, val) in o.iter_mut() {
                        let chemin_par_la_cle = CLES.contains(&k.as_str());
                        if let Some(s) = val.as_str() {
                            let chemin_par_l_extension = s.ends_with(".rhai");
                            if (chemin_par_la_cle || chemin_par_l_extension) && !s.starts_with('/') {
                                *val = Value::String(
                                    gabarit.join(s).to_string_lossy().to_string(),
                                );
                                continue;
                            }
                        }
                        absolutiser(val, gabarit);
                    }
                }
                Value::Array(a) => {
                    for val in a.iter_mut() {
                        absolutiser(val, gabarit);
                    }
                }
                _ => {}
            }
        }

        m["database"] = Value::String(ou.join("base.rag3db").to_string_lossy().to_string());
        // Un nom (`vector`) est l'extension du bâti, trouvée à l'ouverture ; seul
        // un chemin relatif se réécrit.
        if let Some(ext) = m.get("vector_extension").and_then(|v| v.as_str()) {
            if ext.contains('/') && !ext.starts_with('/') {
                m["vector_extension"] =
                    Value::String(gabarit.join(ext).to_string_lossy().to_string());
            }
        }
        // **La racine d'un espace de travail doit exister**, et le gabarit en
        // déclare une qui n'est pas livrée (`code` dit « workspace ») : c'est
        // l'appelant qui la fournit. On donne donc le dossier temporaire, qui
        // existe — ce test monte un gabarit, il n'indexe rien.
        if let Some(w) = m.get_mut("workspace").and_then(|v| v.as_object_mut()) {
            if w.contains_key("root") {
                w["root"] = Value::String(ou.to_string_lossy().to_string());
            }
        }
        absolutiser(&mut m, gabarit);

        let ecrit = ou.join("backend.json");
        std::fs::write(&ecrit, serde_json::to_vec_pretty(&m).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(ecrit)
    }

}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    use super::jouable::manifeste_jouable;

    fn dossier_des_gabarits() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends")
    }

    #[test]
    fn chaque_gabarit_livre_se_charge() {
        let dossier = dossier_des_gabarits();
        let mut gabarits: Vec<PathBuf> = std::fs::read_dir(&dossier)
            .unwrap_or_else(|e| panic!("les gabarits livrés sont illisibles ({e}) : {}", dossier.display()))
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("backend.json").is_file())
            .collect();
        gabarits.sort();

        // **Zéro gabarit n'est pas une réussite.** Un filet qui ne trouve rien
        // à surveiller est vert pour la mauvaise raison — et le jour où ce
        // dossier déménage, c'est ce qui arriverait.
        assert!(
            gabarits.len() >= 3,
            "attendu au moins trois gabarits livrés dans {}, trouvé {} : si le dossier a \
             déménagé, ce test ne surveille plus rien",
            dossier.display(),
            gabarits.len()
        );

        let tmp = std::env::temp_dir().join(format!("rag3weaver-gabarits-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("dossier temporaire");

        let mut echecs: Vec<String> = Vec::new();
        for gabarit in &gabarits {
            let nom = gabarit.file_name().unwrap_or_default().to_string_lossy().to_string();
            // Sans la feature `code`, un gabarit qui déclare un `workspace` refuse de charger,
            // et c'est voulu (« rebâtir avec elle ») : il n'est essayé qu'avec elle. Les autres
            // le sont toujours (la CI les joue sans feature, 18 octobre 2026).
            if !cfg!(feature = "code") {
                let brut = std::fs::read(gabarit.join("backend.json")).unwrap_or_default();
                let manifeste: Value = serde_json::from_slice(&brut).unwrap_or(Value::Null);
                if manifeste.get("workspace").is_some() {
                    continue;
                }
            }
            let ou = tmp.join(&nom);
            if let Err(e) = std::fs::create_dir_all(&ou) {
                echecs.push(format!("{nom} : dossier temporaire impossible ({e})"));
                continue;
            }
            match manifeste_jouable(gabarit, &ou) {
                Ok(chemin) => {
                    // **Tous les gabarits sont essayés avant de rendre.** Un
                    // test qui s'arrête au premier échec cache les autres, et
                    // on les découvre un par un, une passe à la fois.
                    if let Err(e) = crate::backend::PreparedBackend::load(&chemin) {
                        echecs.push(format!("{nom} : {e}"));
                    }
                }
                Err(e) => echecs.push(format!("{nom} : manifeste illisible ou mal formé ({e})")),
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(
            echecs.is_empty(),
            "des gabarits livrés ne chargent plus — un gabarit qui ne charge plus doit rougir \
             la lib, pas attendre une sonde :\n  {}\n({} gabarits essayés : {})",
            echecs.join("\n  "),
            gabarits.len(),
            gabarits
                .iter()
                .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}
