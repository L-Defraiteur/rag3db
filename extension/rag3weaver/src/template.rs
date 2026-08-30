//! **Le catalogue de gabarits** : ce qu'on pose au lieu de l'écrire.
//!
//! Un backend n'a pas à être codé depuis rien. On tient d'avance des `user`,
//! des `conversation`, des `product` — et un agent les **pose**, puis les
//! modifie. C'est la cible du
//! [doc 08](../docs/vision_roadmap_08_2026/08-des-catalogues-de-gabarits.md),
//! et elle ferme la boucle du doc 01 au lieu d'y ajouter un morceau : ranger,
//! retrouver, adopter et modifier demandent exactement ce qu'on a construit
//! sans l'avoir cherché.
//!
//! ## Le contenu vit sur le disque, la fiche dans la base
//!
//! Décidé le 29 août 2026. La base indexe **la fiche** — nom, famille,
//! catégorie, description, chemin, empreinte — et le contenu reste un fichier.
//! Trois raisons :
//!
//! - c'est déjà ce qui marche pour `templates/tools/*.mmd`, et ça n'a jamais
//!   demandé de moteur ;
//! - un `git diff` reste lisible, et on édite un composant avec ses outils
//!   habituels plutôt qu'une chaîne dans une colonne ;
//! - le [doc 04](../docs/vision_roadmap_08_2026/04-le-catalogue-comme-graphe.md)
//!   dit que le catalogue est un graphe de **références**, pas un entrepôt.
//!
//! ## Deux axes, parce qu'ils répondent à deux questions
//!
//! La **famille** est structurelle et fermée — elle dit *ce qu'on peut en
//! faire*, et le moteur la connaît. La **catégorie** est thématique et
//! ouverte — elle dit *de quoi ça parle*, elle vient de qui écrit le gabarit.
//! Un `user` est de famille `Entity` et de catégorie `auth` ; un écran de
//! connexion est de famille `Component`, même catégorie. C'est ce qui permet
//! de demander « tout ce qui touche à l'authentification » et d'obtenir le
//! schéma **et** l'écran.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::{EntityConfig, FieldType, SimpleFieldDef};

/// L'entité qui porte les fiches de gabarits.
pub const TEMPLATE_ENTITY: &str = "Template";

/// **Ce qu'on peut faire d'un gabarit.** Fermée : le moteur la connaît, et
/// chaque famille a son verbe « poser ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    /// Un `EntityConfig` : champs, signaux, découpage, cycle de vie. Posé =
    /// `register_entity`.
    Entity,
    /// Un graphe-outil (`.mmd`). Posé = attaché à une palette.
    Graph,
    /// Un composant d'interface. Posé = un fichier écrit dans le projet.
    Component,
    /// **Transversal** : ne remplace pas une entité, s'applique à une entité.
    /// Versionné, effacé en douceur, audité, possédé, étiqueté. La famille la
    /// plus rentable, parce qu'un motif sert à chaque fois là où un gabarit
    /// d'entité sert une fois.
    Pattern,
}

impl Family {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Graph => "graph",
            Self::Component => "component",
            Self::Pattern => "pattern",
        }
    }

    /// Le sous-répertoire de `templates/` où cette famille se range.
    pub fn dir(self) -> &'static str {
        match self {
            Self::Entity => "entities",
            Self::Graph => "tools",
            Self::Component => "components",
            Self::Pattern => "patterns",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "entity" => Some(Self::Entity),
            "graph" => Some(Self::Graph),
            "component" => Some(Self::Component),
            "pattern" => Some(Self::Pattern),
            _ => None,
        }
    }

    pub const ALL: [Family; 4] = [Family::Entity, Family::Graph, Family::Component, Family::Pattern];
}

/// **La fiche d'un gabarit** — ce qui va dans la base. Le contenu reste au
/// bout de `path`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRef {
    pub name: String,
    pub family: Family,
    /// Thématique, ouverte : `auth`, `commerce`, `messagerie`… Vide si on n'en
    /// a pas déclaré.
    #[serde(default)]
    pub category: String,
    /// Une phrase. C'est elle qu'un agent lit avant de poser.
    #[serde(default)]
    pub description: String,
    /// Où est le contenu, relatif à la racine des gabarits.
    pub path: String,
    /// L'empreinte du contenu, pour savoir qu'il a bougé sans le relire.
    #[serde(default)]
    pub content_hash: String,
}

/// L'en-tête qu'un fichier de gabarit porte, quand il en porte un.
///
/// Pour un JSON, c'est un objet frère du contenu ; pour un `.mmd`, ce sont les
/// lignes `%%`. On ne l'invente pas : sans en-tête, le nom vient du fichier et
/// le reste est vide — un gabarit sans description est un gabarit qu'on
/// trouvera mal, et c'est un défaut à voir, pas à masquer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Header {
    #[serde(default)]
    pub category: String,
    /// **Ce que la chose modélise, et rien d'autre.**
    ///
    /// C'est le champ de contenu : c'est lui qui est embarqué, donc lui qui
    /// décide de ce qu'une recherche par sens trouve. Le 29 août, les
    /// premières descriptions mélangeaient le domaine et un commentaire sur
    /// le gabarit (« volontairement pauvre, un projet ajoute ce qui lui est
    /// propre ») — et `user` arrivait **dernier** sur « de quoi savoir qui est
    /// connecté ». Un commentaire de conception embarqué est du bruit dans le
    /// vecteur.
    #[serde(default)]
    pub description: String,
    /// Le commentaire de conception, **non embarqué** : pourquoi le gabarit
    /// est ce qu'il est. Utile à qui le lit, invisible à la recherche.
    #[serde(default)]
    pub note: String,
}

fn field(t: FieldType) -> SimpleFieldDef {
    SimpleFieldDef { field_type: t, ..Default::default() }
}

/// Le schéma de la fiche.
///
/// `hashsafe` sur `(family, name)` : deux gabarits de familles différentes
/// peuvent porter le même nom — un `user` entité et un `user` composant sont
/// deux choses, et c'est le cas courant, pas une collision.
///
/// Cherchable comme le reste, et c'est **tout l'argument** : un agent trouve
/// ses gabarits avec les moyens qu'il emploie pour trouver un document, sans
/// qu'on ait écrit un second mécanisme (doc 04).
pub fn template_config() -> EntityConfig {
    let mut fields = HashMap::new();
    // Le nom est le titre, la description le contenu : c'est sur elle que la
    // recherche par sens travaille, et c'est ce qu'un agent lit avant de poser.
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("description".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    fields.insert("family".into(), field(FieldType::String));
    fields.insert("category".into(), field(FieldType::String));
    fields.insert("path".into(), field(FieldType::String));
    fields.insert("content_hash".into(), field(FieldType::String));
    EntityConfig {
        fields,
        hashsafe: Some(vec!["family".into(), "name".into()]),
        // **On garde les chunks**, et c'est délibéré.
        //
        // Une fiche tient en une phrase : `chunked: false` semblait
        // l'économie évidente. Elle est exactement contraire au but — l'index
        // vectoriel vit sur la table de chunks, donc une entité sans chunks
        // n'est trouvable qu'au mot près. Or tout l'intérêt d'un catalogue est
        // qu'un agent trouve `user` en demandant « de quoi savoir qui est
        // connecté », sans reprendre les mots de la fiche.
        //
        // Le coût est d'un chunk par gabarit, ce qui est le plancher.
        // (Le catalogue refuse d'ailleurs la combinaison, et c'est ce refus
        // qui a rattrapé l'erreur.)
        return_fields: Some(vec!["family".into(), "category".into(), "path".into()]),
        ..Default::default()
    }
}

/// Déclare l'entité. Idempotent.
pub fn register_template_schema(catalog: &mut crate::Catalog) -> Result<(), crate::catalog::CatalogError> {
    if !catalog.is_registered_entity(TEMPLATE_ENTITY) {
        catalog.register_entity(TEMPLATE_ENTITY, template_config())?;
    }
    Ok(())
}

impl TemplateRef {
    /// Les colonnes, telles que le catalogue les attend.
    pub fn data(&self) -> std::collections::BTreeMap<String, crate::connection::CypherValue> {
        use crate::connection::CypherValue as V;
        std::collections::BTreeMap::from([
            ("name".to_string(), V::String(self.name.clone())),
            ("family".to_string(), V::String(self.family.as_str().to_string())),
            ("category".to_string(), V::String(self.category.clone())),
            ("description".to_string(), V::String(self.description.clone())),
            ("path".to_string(), V::String(self.path.clone())),
            ("content_hash".to_string(), V::String(self.content_hash.clone())),
        ])
    }
}

/// **Lire un catalogue depuis le disque.**
///
/// Une racine, un sous-répertoire par famille. Ce qui n'a pas d'en-tête garde
/// son nom de fichier et rien d'autre : on ne devine pas une description.
pub fn scan(root: &Path) -> Result<Vec<TemplateRef>, String> {
    let mut out = Vec::new();
    for family in Family::ALL {
        let dir = root.join(family.dir());
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(nom) = base_name(&path) else { continue };
            let contenu = std::fs::read_to_string(&path)
                .map_err(|e| format!("gabarit '{}' : {e}", path.display()))?;
            let header = header_of(&path, &contenu);
            out.push(TemplateRef {
                name: nom,
                family,
                category: header.category,
                description: header.description,
                path: format!("{}/{}", family.dir(), path.file_name().unwrap_or_default().to_string_lossy()),
                content_hash: crate::hash::content_hash(&contenu),
            });
        }
    }
    out.sort_by(|a, b| (a.family.as_str(), &a.name).cmp(&(b.family.as_str(), &b.name)));
    Ok(out)
}

/// Le nom d'un gabarit : le fichier sans **toutes** ses extensions.
/// `results.md.jinja` est le gabarit `results`, pas `results.md`.
fn base_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    Some(name.split('.').next()?.to_string())
}

/// L'en-tête, selon la forme du fichier.
///
/// `.mmd` porte des lignes `%% clé: valeur` — c'est déjà la convention des
/// graphes-outils, et on la relit plutôt que d'en inventer une seconde. Un
/// JSON porte un objet `template` à côté du contenu. Le reste n'en a pas.
fn header_of(path: &Path, contenu: &str) -> Header {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mmd") => {
            let mut h = Header::default();
            for ligne in contenu.lines().take_while(|l| l.starts_with("%%")) {
                let l = ligne.trim_start_matches('%').trim();
                if let Some(v) = l.strip_prefix("description:") {
                    h.description = v.trim().to_string();
                } else if let Some(v) = l.strip_prefix("category:") {
                    h.category = v.trim().to_string();
                }
            }
            h
        }
        Some("json") => serde_json::from_str::<serde_json::Value>(contenu)
            .ok()
            .and_then(|v| v.get("template").cloned())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        _ => Header::default(),
    }
}

/// **Poser un gabarit d'entité** : le JSON devient un `register_entity`.
///
/// Le contenu porte `template` (la fiche) et `entity` (la configuration) : on
/// lit la seconde. Poser sous un autre nom est le cas courant — c'est la
/// même règle que pour les outils, le nom appartient à qui adopte.
pub fn place_entity(
    catalog: &mut crate::Catalog,
    contenu: &str,
    sous_le_nom: &str,
) -> Result<(), String> {
    let v: serde_json::Value = serde_json::from_str(contenu).map_err(|e| format!("gabarit illisible : {e}"))?;
    let config = v.get("entity").ok_or("gabarit d'entité sans clé 'entity'")?;
    let config: EntityConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("configuration d'entité illisible : {e}"))?;
    catalog.register_entity(sous_le_nom, config).map_err(|e| e.to_string())
}

/// **Un motif** : ce qu'il ajoute à une entité, et comment il en change
/// l'identité.
///
/// Il ne remplace pas une entité, il s'y **applique** — c'est ce qui le rend
/// rentable : un motif sert à chaque fois là où un gabarit d'entité sert une
/// fois.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pattern {
    /// La famille à laquelle il s'applique. `entity` aujourd'hui.
    #[serde(default)]
    pub applies_to: String,
    /// Les champs qu'il ajoute.
    #[serde(default)]
    pub adds_fields: HashMap<String, SimpleFieldDef>,
    /// Ce qu'il ajoute à la **clé d'identité**. C'est tout le motif
    /// « versionné » : `hashsafe` *est* la politique d'identité (doc 04), et
    /// y ajouter la révision fait une entité par révision au lieu d'une par
    /// chose. Aucun mécanisme nouveau — un choix nommé.
    #[serde(default)]
    pub hashsafe_append: Vec<String>,
    #[serde(default)]
    pub note: String,
}

impl Pattern {
    /// Lit un motif depuis le contenu de son fichier.
    pub fn parse(contenu: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(contenu).map_err(|e| format!("motif illisible : {e}"))?;
        let p = v.get("pattern").ok_or("motif sans clé 'pattern'")?;
        serde_json::from_value(p.clone()).map_err(|e| format!("motif illisible : {e}"))
    }

    /// **Applique le motif à une configuration, avant de l'enregistrer.**
    ///
    /// Avant, et pas après : appliquer un motif à une entité déjà déclarée
    /// changerait sa clé d'identité, donc l'uuid de chaque ligne déjà écrite —
    /// c'est une migration, pas une pose. On garde ici le cas courant, où
    /// l'agent demande « un `product` versionné » et reçoit une configuration
    /// déjà fusionnée. La migration viendra si quelqu'un en a besoin, et elle
    /// s'appellera migration.
    ///
    /// Un champ déjà présent n'est **pas** écrasé : l'entité a le dernier mot
    /// sur ce qu'elle déclare, le motif ne fait qu'ajouter ce qui manque.
    pub fn apply(&self, mut config: EntityConfig) -> Result<EntityConfig, String> {
        for (nom, def) in &self.adds_fields {
            config.fields.entry(nom.clone()).or_insert_with(|| def.clone());
        }
        if !self.hashsafe_append.is_empty() {
            let mut clef = config.hashsafe.clone().ok_or_else(|| {
                format!(
                    "le motif ajoute {:?} à la clé d'identité, mais l'entité n'en déclare pas — \
                     un motif d'identité ne s'applique qu'à une entité qui en a une",
                    self.hashsafe_append
                )
            })?;
            for champ in &self.hashsafe_append {
                if !clef.contains(champ) {
                    clef.push(champ.clone());
                }
            }
            config.hashsafe = Some(clef);
        }
        config.validate().map_err(|e| format!("l'entité n'est plus valide après le motif : {e}"))?;
        Ok(config)
    }
}

/// **Poser une entité, éventuellement avec des motifs.**
///
/// L'ordre est celui des motifs donnés ; chacun ajoute ce qui manque et rien
/// de plus. La validation a lieu à chaque étape, pour qu'un motif fautif se
/// nomme lui-même au lieu de faire échouer le suivant.
pub fn place_entity_with(
    catalog: &mut crate::Catalog,
    contenu: &str,
    motifs: &[&str],
    sous_le_nom: &str,
) -> Result<(), String> {
    let config = preparer_entity(contenu, motifs)?;
    catalog.register_entity(sous_le_nom, config).map_err(|e| e.to_string())
}

/// **Ce qu'on va poser, avant de le poser.**
///
/// Séparé de l'enregistrement pour une raison précise : celui qui pose doit
/// pouvoir *dire ce qu'il a posé* — combien de champs, lesquels, quels signaux.
/// Un outil qui répond « c'est fait » sans montrer le résultat oblige son
/// appelant à une seconde requête pour savoir ce qu'il vient de créer.
pub fn preparer_entity(contenu: &str, motifs: &[&str]) -> Result<EntityConfig, String> {
    let v: serde_json::Value = serde_json::from_str(contenu).map_err(|e| format!("gabarit illisible : {e}"))?;
    let config = v.get("entity").ok_or("gabarit d'entité sans clé 'entity'")?;
    let mut config: EntityConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("configuration d'entité illisible : {e}"))?;
    for m in motifs {
        let motif = Pattern::parse(m)?;
        config = motif.apply(config)?;
    }
    Ok(config)
}

/// **Retrouver un gabarit par sa famille et son nom**, avec son contenu.
///
/// L'erreur dit ce qui existe. C'est ce qui fait la différence entre un agent
/// qui se corrige en un tour et un agent qui redemande la liste : « gabarit
/// 'users' inconnu » l'envoie deviner, « inconnu — il y a conversation,
/// product, user » lui donne la réponse dans le refus.
pub fn lire(root: &Path, family: Family, nom: &str) -> Result<String, String> {
    let refs = scan(root)?;
    match refs.iter().find(|r| r.family == family && r.name == nom) {
        Some(r) => std::fs::read_to_string(root.join(&r.path))
            .map_err(|e| format!("gabarit '{}' illisible : {e}", r.path)),
        None => {
            let mut voisins: Vec<&str> =
                refs.iter().filter(|r| r.family == family).map(|r| r.name.as_str()).collect();
            voisins.sort_unstable();
            if voisins.is_empty() {
                Err(format!("aucun gabarit de famille '{}' sous {}", family.as_str(), root.display()))
            } else {
                Err(format!(
                    "gabarit '{nom}' inconnu dans la famille '{}' — il y a {}",
                    family.as_str(),
                    voisins.join(", ")
                ))
            }
        }
    }
}

/// La racine des gabarits fournis, à côté du crate.
pub fn builtin_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le catalogue se lit depuis le disque, en-têtes compris.**
    ///
    /// Ce que le test fixe n'est pas le nombre de gabarits — il va grandir —
    /// mais qu'un gabarit arrive avec sa famille, sa catégorie et sa phrase.
    /// Un gabarit sans description est un gabarit qu'on trouvera mal.
    #[test]
    fn le_catalogue_fourni_se_lit() {
        let fiches = scan(&builtin_root()).expect("lire les gabarits fournis");
        assert!(!fiches.is_empty());

        let par_nom = |n: &str| fiches.iter().find(|f| f.name == n).cloned();

        let user = par_nom("user").expect("un gabarit `user`");
        assert_eq!(user.family, Family::Entity);
        assert_eq!(user.category, "auth");
        assert!(user.description.contains("compte"), "{}", user.description);
        assert_eq!(user.path, "entities/user.json");
        assert!(!user.content_hash.is_empty());

        // Un motif est une famille à part : il ne se pose pas comme une entité.
        let v = par_nom("versioned").expect("le motif `versioned`");
        assert_eq!(v.family, Family::Pattern);

        // Les graphes-outils y sont aussi, et leur en-tête `%%` est relu tel
        // quel — on ne s'invente pas une seconde convention.
        let search = par_nom("search").expect("le graphe `search`");
        assert_eq!(search.family, Family::Graph);
        assert!(search.description.contains("Cherche"), "{}", search.description);

        // Le nom perd **toutes** les extensions : `results.md.jinja` est
        // `results`, pas `results.md`.
        assert!(fiches.iter().all(|f| !f.name.contains('.')), "{:?}", fiches.iter().map(|f| &f.name).collect::<Vec<_>>());
    }

    /// **Deux axes, et ils ne se recouvrent pas.** La famille dit ce qu'on
    /// peut faire du gabarit ; la catégorie de quoi il parle. Filtrer par
    /// catégorie doit pouvoir traverser les familles — c'est tout l'intérêt :
    /// « ce qui touche à l'authentification » rendra un jour le schéma **et**
    /// l'écran.
    #[test]
    fn la_famille_et_la_categorie_sont_deux_questions() {
        let fiches = scan(&builtin_root()).unwrap();
        let auth: Vec<&TemplateRef> = fiches.iter().filter(|f| f.category == "auth").collect();
        assert!(!auth.is_empty());

        let familles: std::collections::BTreeSet<&str> =
            fiches.iter().map(|f| f.family.as_str()).collect();
        assert!(familles.contains("entity") && familles.contains("graph") && familles.contains("pattern"));

        // La famille est fermée et se relit ; la catégorie est libre.
        for f in &fiches {
            assert_eq!(Family::parse(f.family.as_str()), Some(f.family));
        }
    }

    /// **Poser, c'est enregistrer** — et sous le nom qu'on veut. Le nom
    /// appartient à qui adopte, comme pour les outils.
    #[test]
    fn poser_un_gabarit_d_entite_le_declare_sous_le_nom_choisi() {
        let contenu = std::fs::read_to_string(builtin_root().join("entities/product.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&contenu).unwrap();
        let config: EntityConfig = serde_json::from_value(v["entity"].clone()).expect("l'entité se relit");

        // Ce que le gabarit promet : un titre, un contenu, une identité.
        assert!(config.fields.contains_key("sku") && config.fields.contains_key("description"));
        assert_eq!(config.hashsafe, Some(vec!["sku".to_string()]));
        // Et il passe la validation du catalogue — un gabarit qu'on ne pourrait
        // pas poser ne serait pas un gabarit.
        config.validate().expect("le gabarit fourni doit être posable");
    }

    /// **Un motif s'applique avant l'enregistrement, et il ne fait qu'ajouter.**
    ///
    /// « Versionné » ne contient aucun mécanisme : il ajoute `revision` à la
    /// clé d'identité, et c'est tout. `hashsafe` *est* la politique
    /// d'identité (doc 04) — le motif nomme un choix qui existe.
    #[test]
    fn le_motif_versionne_change_l_identite_et_rien_d_autre() {
        let entite = std::fs::read_to_string(builtin_root().join("entities/product.json")).unwrap();
        let motif = std::fs::read_to_string(builtin_root().join("patterns/versioned.json")).unwrap();

        let v: serde_json::Value = serde_json::from_str(&entite).unwrap();
        let nu: EntityConfig = serde_json::from_value(v["entity"].clone()).unwrap();
        assert_eq!(nu.hashsafe, Some(vec!["sku".to_string()]));

        let p = Pattern::parse(&motif).expect("le motif se relit");
        assert_eq!(p.applies_to, "entity");
        let versionne = p.apply(nu.clone()).expect("posable");

        // L'identité gagne la révision : une entité par révision, plus une par
        // chose.
        assert_eq!(versionne.hashsafe, Some(vec!["sku".to_string(), "revision".to_string()]));
        assert!(versionne.fields.contains_key("revision"));
        // Et rien de ce que l'entité déclarait n'a bougé.
        for champ in nu.fields.keys() {
            assert!(versionne.fields.contains_key(champ), "{champ} a disparu");
        }
        assert_eq!(versionne.fields["name"].is_title, nu.fields["name"].is_title);
    }

    /// **L'entité a le dernier mot.** Un motif ajoute ce qui manque ; il
    /// n'écrase pas ce qu'une entité a choisi de déclarer autrement.
    #[test]
    fn un_motif_n_ecrase_pas_ce_que_l_entite_declare() {
        let motif = std::fs::read_to_string(builtin_root().join("patterns/versioned.json")).unwrap();
        let p = Pattern::parse(&motif).unwrap();

        let mut fields = HashMap::new();
        fields.insert("id".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
        fields.insert("body".to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
        // L'entité déclare déjà `revision`, et en Texte plutôt qu'en chaîne.
        fields.insert("revision".to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
        // `chunked: false` impose `BM25` seul : l'index vectoriel vit sur la
        // table de chunks, une entité sans chunks serait introuvable par lui.
        // Le catalogue le refuse, et c'est bien ce qu'on veut.
        let config = EntityConfig {
            fields,
            hashsafe: Some(vec!["id".into()]),
            chunked: Some(false),
            signals: crate::search::SearchSignals::BM25,
            ..Default::default()
        };

        let out = p.apply(config).unwrap();
        assert_eq!(out.fields["revision"].field_type, FieldType::Text, "le motif n'écrase pas");
        // Mais l'identité, elle, gagne bien la révision — sans doublon.
        assert_eq!(out.hashsafe, Some(vec!["id".to_string(), "revision".to_string()]));
        assert_eq!(p.apply(out.clone()).unwrap().hashsafe, out.hashsafe, "l'appliquer deux fois ne double pas la clé");
    }

    /// Un motif d'identité sur une entité qui n'en a pas se **refuse**, et le
    /// dit. Sans clé, « une par révision » ne veut rien dire.
    #[test]
    fn un_motif_d_identite_exige_une_identite() {
        let motif = std::fs::read_to_string(builtin_root().join("patterns/versioned.json")).unwrap();
        let p = Pattern::parse(&motif).unwrap();

        let mut fields = HashMap::new();
        fields.insert("titre".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
        fields.insert("corps".to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
        let sans_clef = EntityConfig {
            fields,
            hashsafe: None,
            chunked: Some(false),
            signals: crate::search::SearchSignals::BM25,
            ..Default::default()
        };

        let e = p.apply(sans_clef).unwrap_err();
        assert!(e.contains("clé d'identité"), "{e}");
    }

    /// Les quatre familles connaissent leur répertoire, et il est distinct.
    #[test]
    fn chaque_famille_a_son_rayon() {
        let dirs: std::collections::BTreeSet<&str> = Family::ALL.iter().map(|f| f.dir()).collect();
        assert_eq!(dirs.len(), Family::ALL.len());
        assert_eq!(Family::Graph.dir(), "tools", "les graphes-outils sont déjà là, on ne les déplace pas");
    }
}
