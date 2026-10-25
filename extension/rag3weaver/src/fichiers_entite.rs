//! **Le miroir en fichiers d'une entité** (lot M2 de la mémoire, 18 octobre
//! 2026 ; `docs/18-octobre-2026-memoire-longue/04-les-fiches-dans-des-fichiers.md`).
//!
//! Une entité qui déclare `files` dans le manifeste a, pour chaque ligne, un
//! fichier Markdown sous la racine `files_root` : un en-tête YAML canonique
//! ([`crate::en_tete`]) qui porte l'uuid et les champs déclarés, puis un corps
//! (le texte d'un champ). **Le fichier d'abord, la ligne ensuite** :
//! `EntityRecordNode` écrit le fichier avant d'ingérer la ligne, si bien qu'une
//! ingestion qui échoue laisse un fichier en avance sur l'index — rattrapable —
//! et jamais l'inverse. Rien ici n'est propre à la mémoire.
//!
//! ```json
//! "files_root": "../../fichiers",
//! "entities": {"Source": {…, "files": {"dir": "sources", "name": "{chemin}", "header": ["titre", "origine"], "body": "contenu"}}}
//! ```

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// La clé du service : `HashMap<String, Arc<Miroir>>`, par entité.
pub const SERVICE: &str = "backend_files";

/// La déclaration d'une entité, telle que le manifeste la porte.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FilesDecl {
    /// Le sous-dossier de la racine (vide : la racine).
    #[serde(default)]
    pub dir: String,
    /// Le nom du fichier, relatif à `dir` : `{uuid}` et `{<champ>}` s'y
    /// remplacent ; doit finir par `.md`.
    pub name: String,
    /// Les champs écrits dans l'en-tête, dans cet ordre (l'uuid vient toujours
    /// en premier).
    #[serde(default)]
    pub header: Vec<String>,
    /// Le champ dont le texte est le corps du fichier, tel quel.
    #[serde(default)]
    pub body: Option<String>,
    /// **Un fichier composé** (lot M2c) : un gabarit jinja, relatif au
    /// manifeste, qui rend le corps depuis la ligne et ses lignes liées
    /// (`related`). Le fichier devient une **vue de la base**, réécrite par la
    /// réaction de miroir après chaque changement, de n'importe quel écrivain ;
    /// exclusif de `body`.
    #[serde(default)]
    pub template: Option<String>,
    /// Les lignes liées données au gabarit, par nom :
    /// `{"apports": {"relation": "A_POUR_APPORT", "order_by": "created_at"}}`.
    #[serde(default)]
    pub related: BTreeMap<String, RelatedDecl>,
}

/// Une famille de lignes liées d'un fichier composé.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RelatedDecl {
    pub relation: String,
    /// `outgoing` (la ligne est le départ de la relation, le défaut) ou `incoming`.
    #[serde(default = "sortant")]
    pub direction: String,
    /// Le champ qui ordonne les lignes liées, croissant.
    #[serde(default)]
    pub order_by: Option<String>,
}

fn sortant() -> String {
    "outgoing".into()
}

/// L'ouverture du bloc machine d'un fichier composé (lot M3b) : un commentaire
/// HTML, invisible au rendu (Obsidian comme tout Markdown), en YAML comme
/// l'en-tête.
pub const DEBUT_DU_BLOC: &str = "<!-- rag3weaver: les lignes de ce fichier, pour reconstruire la base si elle est perdue — écrit par le harnais, ne pas éditer\n";
/// Sa fermeture.
pub const FIN_DU_BLOC: &str = "-->\n";

/// **Le bloc machine** : les lignes qu'un fichier composé rend, chacune entière,
/// avec ses liens sortants — de quoi reposer la ligne, ses lignes liées et leurs
/// relations, si la base est perdue. Produit depuis le schéma et le catalogue,
/// pour toute entité à fichier composé, jamais par un gabarit.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BlocMachine {
    pub rows: Vec<LigneDuBloc>,
}

/// Une ligne du bloc : son entité, son uuid, ses champs (sans les colonnes
/// internes ni la cellule), et ses liens sortants par relation.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LigneDuBloc {
    pub entity: String,
    pub uuid: String,
    pub fields: Map<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub links: BTreeMap<String, Vec<LienDuBloc>>,
}

/// Un lien sortant d'une ligne du bloc : sa cible, et ses propriétés s'il en a.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LienDuBloc {
    pub to: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub props: Map<String, Value>,
}

/// Une ligne du bloc, depuis sa ligne de base : ses champs déclarés, et ses
/// liens sortants par toutes les relations qui partent de son entité.
fn ligne_du_bloc(cat: &crate::catalog::Catalog, entite: &str, uuid: &str, record: &Value) -> Result<LigneDuBloc, String> {
    let mut fields = Map::new();
    for (k, v) in record.as_object().into_iter().flatten() {
        // Une colonne que rag3weaver pose lui-même (les vecteurs `embedding…`,
        // qui ne commencent pas par `_`) n'est pas une donnée de la ligne : elle
        // se recalcule, et mille flottants n'ont rien à faire dans un fichier.
        if k.starts_with('_') || k == "uuid" || crate::scope::is_scope_column(k) || crate::schema::is_builtin_column(k) || v.is_null() {
            continue;
        }
        fields.insert(k.clone(), v.clone());
    }
    let mut links = BTreeMap::new();
    let mut relations = cat.relation_names();
    relations.sort();
    for relation in relations {
        let Some(def) = cat.get_relation_def(&relation).cloned() else { continue };
        if def.from != entite {
            continue;
        }
        crate::schema::validate_identifier(&relation, "relation").map_err(|e| e.to_string())?;
        let mut props: Vec<String> = def.properties.as_ref().map(|p| p.keys().cloned().collect()).unwrap_or_default();
        props.sort();
        let colonnes: String = props.iter().map(|p| format!(", r.`{p}`")).collect();
        let requete = format!(
            "MATCH (a:`{entite}` {{_uuid: $u}})-[r:`{relation}`]->(b:`{}`) RETURN b._uuid{colonnes} ORDER BY b._uuid",
            def.to
        );
        let lu = cat
            .execute_raw_with_params(&requete, &[crate::connection::QueryParam::new("u", crate::connection::CypherValue::String(uuid.to_string()))])
            .map_err(|e| e.to_string())?;
        let mut liens = Vec::new();
        for row in &lu.rows {
            let Some(to) = row.first().and_then(|v| v.as_str()) else { continue };
            let mut p = Map::new();
            for (k, v) in props.iter().zip(row.iter().skip(1)) {
                if !matches!(v, crate::connection::CypherValue::Null) {
                    p.insert(k.clone(), serde_json::to_value(v).map_err(|e| e.to_string())?);
                }
            }
            liens.push(LienDuBloc { to: to.to_string(), props: p });
        }
        if !liens.is_empty() {
            links.insert(relation, liens);
        }
    }
    Ok(LigneDuBloc { entity: entite.to_string(), uuid: uuid.to_string(), fields, links })
}

/// Le texte du bloc. Une valeur ne peut pas fermer le commentaire : `-->` et
/// `&` y sont échappés (`--&gt;`, `&amp;`), et [`Miroir::relire_le_bloc`] les
/// rend.
fn ecrire_le_bloc(lignes: &[LigneDuBloc]) -> Result<String, String> {
    let yaml = serde_norway::to_string(&BlocMachine { rows: lignes.to_vec() }).map_err(|e| e.to_string())?;
    let yaml = yaml.replace('&', "&amp;").replace("-->", "--&gt;");
    Ok(format!("\n{DEBUT_DU_BLOC}{yaml}{FIN_DU_BLOC}"))
}

/// Le miroir d'une entité, prêt à écrire.
#[derive(Debug, Clone)]
pub struct Miroir {
    pub entite: String,
    /// La racine, absolue.
    pub racine: PathBuf,
    pub decl: FilesDecl,
    /// Le texte du gabarit d'un fichier composé, lu au chargement.
    pub gabarit: Option<String>,
    /// Le schéma de l'entité, avec `uuid` (que l'en-tête porte toujours) : ce
    /// qui relit un fichier ([`Self::relire`], lot M3).
    pub schema: Value,
    /// Les entités du manifeste **sans fichier à elles**, posées au chargement
    /// une fois tous les miroirs connus : une ligne de l'une d'elles, atteinte
    /// par un lien sortant d'une ligne du bloc, voyage avec le bloc (une Ref
    /// `doc:` citée par un apport). Sans elle, la base perdue la perdrait.
    pub sans_fichier: std::sync::OnceLock<std::collections::BTreeSet<String>>,
    /// Les dossiers des **autres** miroirs sous le sien, posés au chargement :
    /// une entité rangée à la racine (`dir` vide, une Source à son chemin) ne
    /// lit pas comme siens les fichiers d'une fiche ou d'un genre. Relatifs à
    /// la racine : un miroir déplacé sur une autre racine les garde justes.
    pub dossiers_voisins: std::sync::OnceLock<Vec<PathBuf>>,
}

/// Un nom de champ entre accolades dans `name`.
fn trous(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut reste = name;
    while let Some(i) = reste.find('{') {
        let Some(j) = reste[i..].find('}') else { break };
        out.push(reste[i + 1..i + j].to_string());
        reste = &reste[i + j + 1..];
    }
    out
}

impl Miroir {
    /// **Contrôlé au chargement** : les champs nommés existent dans le schéma,
    /// le corps est une chaîne, le nom finit par `.md` et n'a ni `..` ni chemin
    /// absolu, la racine existe ou peut être créée.
    pub fn nouveau(entite: &str, racine: PathBuf, decl: FilesDecl, schema: &Value, dossier_du_manifeste: &Path) -> Result<Self, String> {
        Self::nouveau_avec(entite, racine, decl, schema, dossier_du_manifeste, &crate::lecteur::Disque)
    }

    /// [`Self::nouveau`], par un lecteur (le disque, ou un recouvrement) : le
    /// gabarit d'un miroir se lit comme toute déclaration du chargement.
    pub fn nouveau_avec(
        entite: &str,
        racine: PathBuf,
        decl: FilesDecl,
        schema: &Value,
        dossier_du_manifeste: &Path,
        lecteur: &dyn crate::lecteur::Lecteur,
    ) -> Result<Self, String> {
        let props = schema["properties"].as_object().cloned().unwrap_or_default();
        let ici = |quoi: &str, champ: &str| -> Result<(), String> {
            if props.contains_key(champ) {
                Ok(())
            } else {
                Err(format!("{entite}.files.{quoi} : « {champ} » n'est pas un champ du schéma"))
            }
        };
        for c in &decl.header {
            ici("header", c)?;
        }
        if let Some(b) = &decl.body {
            ici("body", b)?;
            if props[b]["type"] != "string" {
                return Err(format!("{entite}.files.body : « {b} » doit être une chaîne"));
            }
        }
        // Un nom qui finit par un champ (`{chemin}`) est jugé à l'écriture, sur
        // sa valeur ; un nom littéral doit déjà finir par `.md`.
        if !decl.name.ends_with(".md") && !decl.name.ends_with('}') {
            return Err(format!("{entite}.files.name : « {} » doit finir par .md", decl.name));
        }
        for t in trous(&decl.name) {
            if t != "uuid" {
                ici("name", &t)?;
            }
        }
        relatif_sur(&decl.dir, &format!("{entite}.files.dir"))?;
        relatif_sur(&decl.name.replace(['{', '}'], ""), &format!("{entite}.files.name"))?;
        if decl.template.is_some() && decl.body.is_some() {
            return Err(format!("{entite}.files : `template` (un fichier composé) et `body` s'excluent"));
        }
        if decl.template.is_none() && !decl.related.is_empty() {
            return Err(format!("{entite}.files.related : sans `template`, les lignes liées ne vont nulle part"));
        }
        for (nom, r) in &decl.related {
            if !matches!(r.direction.as_str(), "outgoing" | "incoming") {
                return Err(format!("{entite}.files.related.{nom}.direction : outgoing ou incoming, pas « {} »", r.direction));
            }
        }
        let gabarit = match &decl.template {
            Some(g) => {
                let chemin = dossier_du_manifeste.join(g);
                Some(lecteur.lire_texte(&chemin).map_err(|e| format!("{entite}.files.template : {} : {e}", chemin.display()))?)
            }
            None => None,
        };
        let mut schema = schema.clone();
        if let Some(props) = schema["properties"].as_object_mut() {
            props.insert("uuid".into(), serde_json::json!({"type": "string"}));
        }
        Ok(Self { entite: entite.to_string(), racine, decl, gabarit, schema, sans_fichier: std::sync::OnceLock::new(), dossiers_voisins: std::sync::OnceLock::new() })
    }

    /// **Relit le fichier d'une ligne** (lot M3, la reconstruction) : l'en-tête,
    /// typé par le schéma, et le corps dans son champ. Rend l'uuid et les champs.
    /// L'inverse exact de [`Self::rendre`] : un fichier écrit par le harnais se
    /// relit en la ligne qui l'a écrit. Un fichier composé se relit par son bloc
    /// machine, pas d'ici : son corps est une vue.
    pub fn relire(&self, texte: &str) -> Result<(String, Map<String, Value>), String> {
        if self.est_compose() {
            return Err(format!("{} : un fichier composé ne se relit pas par son corps", self.entite));
        }
        let (yaml, corps) = crate::en_tete::separer(texte).ok_or_else(|| "pas d'en-tête fermé par ---".to_string())?;
        let mut champs = crate::en_tete::lire_en_tete(yaml, &self.schema)?;
        let uuid = match champs.remove("uuid") {
            Some(Value::String(u)) if !u.is_empty() => u,
            _ => return Err("l'en-tête ne porte pas d'uuid".into()),
        };
        // Le corps vide d'un champ absent ne devient pas une chaîne vide.
        if let Some(b) = &self.decl.body {
            if !corps.is_empty() {
                champs.insert(b.clone(), Value::String(corps.to_string()));
            }
        }
        Ok((uuid, champs))
    }

    /// **Les fichiers de cette entité**, relatifs à la racine, triés : tous les
    /// `.md` sous `dir`, sous-dossiers compris (un nom par champ peut contenir des
    /// `/`) ; un temporaire d'écriture (`.md.ecriture`) n'en est pas un.
    pub fn lister(&self) -> Result<Vec<String>, String> {
        let base = if self.decl.dir.is_empty() { self.racine.clone() } else { self.racine.join(&self.decl.dir) };
        let mut out = Vec::new();
        if !base.exists() {
            return Ok(out);
        }
        let mut pile = vec![base];
        while let Some(d) = pile.pop() {
            for e in std::fs::read_dir(&d).map_err(|e| format!("{} : {e}", d.display()))? {
                let e = e.map_err(|e| e.to_string())?;
                let chemin = e.path();
                let genre = e.file_type().map_err(|e| e.to_string())?;
                if genre.is_dir() {
                    let rel = chemin.strip_prefix(&self.racine).map(Path::to_path_buf).unwrap_or_default();
                    if self.dossiers_voisins.get().is_some_and(|v| v.contains(&rel)) {
                        continue;
                    }
                    pile.push(chemin);
                } else if chemin.extension().is_some_and(|x| x == "md") {
                    let rel = chemin.strip_prefix(&self.racine).map_err(|e| e.to_string())?;
                    out.push(rel.to_string_lossy().into_owned());
                }
            }
        }
        out.sort();
        Ok(out)
    }

    /// Le texte d'un fichier, relatif à la racine, lu réellement sous elle.
    pub fn lire(&self, rel: &str) -> Result<String, String> {
        let chemin = sous_la_racine(&self.racine, rel)?;
        std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))
    }

    /// Les champs du schéma qu'un fichier de cette entité ne porte pas (ni en
    /// en-tête, ni en corps) : ce qu'une reconstruction depuis les fichiers
    /// perdrait, dit plutôt que perdu en silence.
    pub fn champs_non_portes(&self) -> Vec<String> {
        let portes: std::collections::BTreeSet<&str> =
            self.decl.header.iter().map(String::as_str).chain(self.decl.body.as_deref()).chain(["uuid"]).collect();
        let mut out: Vec<String> = self.schema["properties"]
            .as_object()
            .map(|p| p.keys().filter(|k| !portes.contains(k.as_str())).cloned().collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    /// Un fichier composé : une vue de la base, réécrite par la réaction de miroir.
    pub fn est_compose(&self) -> bool {
        self.gabarit.is_some()
    }

    /// Le chemin du fichier d'une ligne, relatif à la racine.
    pub fn chemin_relatif(&self, uuid: &str, record: &Value) -> Result<String, String> {
        let mut nom = self.decl.name.clone();
        for t in trous(&self.decl.name) {
            let v = if t == "uuid" {
                uuid.to_string()
            } else {
                match &record[&t] {
                    Value::String(s) if !s.is_empty() => s.clone(),
                    Value::Number(n) => n.to_string(),
                    autre => return Err(format!("{} : le nom du fichier demande « {t} », qui vaut {autre}", self.entite)),
                }
            };
            nom = nom.replace(&format!("{{{t}}}"), &v);
        }
        let rel = if self.decl.dir.is_empty() { nom } else { format!("{}/{nom}", self.decl.dir.trim_end_matches('/')) };
        relatif_sur(&rel, &format!("{} : le fichier", self.entite))?;
        if !rel.ends_with(".md") {
            return Err(format!("{} : le fichier « {rel} » doit finir par .md", self.entite));
        }
        Ok(rel)
    }

    /// Le texte du fichier d'une ligne : l'en-tête (`uuid`, puis les champs
    /// déclarés présents), puis le corps.
    pub fn rendre(&self, uuid: &str, record: &Value) -> String {
        let corps = self.decl.body.as_ref().and_then(|b| record[b].as_str()).unwrap_or("");
        self.rendre_avec(uuid, record, corps)
    }

    /// L'en-tête d'une ligne, suivi d'un corps donné.
    fn rendre_avec(&self, uuid: &str, record: &Value, corps: &str) -> String {
        let mut champs = Map::new();
        champs.insert("uuid".into(), Value::String(uuid.to_string()));
        for c in &self.decl.header {
            if let Some(v) = record.get(c) {
                champs.insert(c.clone(), v.clone());
            }
        }
        let mut ordre: Vec<&str> = vec!["uuid"];
        ordre.extend(self.decl.header.iter().map(String::as_str));
        crate::en_tete::assembler(&crate::en_tete::ecrire_en_tete(&champs, &ordre), corps)
    }

    /// **Écrit le fichier d'une ligne**, atomiquement (un temporaire renommé),
    /// réellement sous la racine. Rend son chemin relatif.
    pub fn ecrire(&self, uuid: &str, record: &Value) -> Result<String, String> {
        let rel = self.chemin_relatif(uuid, record)?;
        self.ecrire_texte(&rel, &self.rendre(uuid, record))?;
        Ok(rel)
    }

    /// Écrit `texte` à `rel` atomiquement, réellement sous la racine ; un
    /// fichier déjà identique n'est pas réécrit (sa date ne bouge pas, la montre
    /// ne le revoit pas). Rend vrai s'il a écrit.
    fn ecrire_texte(&self, rel: &str, texte: &str) -> Result<bool, String> {
        let cible = sous_la_racine(&self.racine, rel)?;
        if std::fs::read_to_string(&cible).is_ok_and(|actuel| actuel == texte) {
            return Ok(false);
        }
        if let Some(parent) = cible.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{} : {e}", parent.display()))?;
        }
        let temporaire = cible.with_extension("md.ecriture");
        std::fs::write(&temporaire, texte).map_err(|e| format!("{} : {e}", temporaire.display()))?;
        std::fs::rename(&temporaire, &cible).map_err(|e| format!("{} : {e}", cible.display()))?;
        Ok(true)
    }

    /// **Régénère le fichier composé d'une ligne depuis la base** : la ligne,
    /// ses lignes liées (`related`, ordonnées), le gabarit. Rend le chemin
    /// relatif et s'il a écrit ; `None` si la ligne n'existe plus (son fichier
    /// est alors retiré).
    pub fn regenerer(&self, cat: &crate::catalog::Catalog, uuid: &str) -> Result<Option<(String, bool)>, String> {
        let gabarit = self.gabarit.as_deref().ok_or_else(|| format!("{} : pas un fichier composé", self.entite))?;
        let ligne = cat.get(&self.entite, uuid).map_err(|e| e.to_string())?;
        let Some(ligne) = ligne else {
            return Ok(None);
        };
        let record = serde_json::to_value(&ligne).map_err(|e| e.to_string())?;
        let mut liees = Map::new();
        for (nom, r) in &self.decl.related {
            let sens = if r.direction == "incoming" {
                crate::search_strategy::ExpansionDirection::Incoming
            } else {
                crate::search_strategy::ExpansionDirection::Outgoing
            };
            let voisins = crate::dataflow::search_nodes::fetch_related_in(
                cat.dialect_arc().as_ref(),
                cat.conn(),
                &[uuid.to_string()],
                &r.relation,
                sens,
                0,
            )?;
            let mut lignes: Vec<Value> = voisins
                .get(uuid)
                .map(|v| {
                    v.iter()
                        .map(|c| {
                            let mut d = serde_json::to_value(&c.data).unwrap_or(Value::Null);
                            if let Some(o) = d.as_object_mut() {
                                o.insert("uuid".into(), Value::String(c.uuid.clone()));
                            }
                            d
                        })
                        .collect()
                })
                .unwrap_or_default();
            if let Some(champ) = &r.order_by {
                lignes.sort_by(|a, b| ordre_json(&a[champ], &b[champ]).then_with(|| ordre_json(&a["uuid"], &b["uuid"])));
            } else {
                lignes.sort_by(|a, b| ordre_json(&a["uuid"], &b["uuid"]));
            }
            liees.insert(nom.clone(), Value::Array(lignes));
        }
        let contexte = serde_json::json!({"uuid": uuid, "record": record, "related": liees});
        let mut corps = crate::dataflow::render_nodes::rendre(&contexte, gabarit)?;
        // Le bloc machine (lot M3b) : ce que la reconstruction relit si la base
        // est perdue. Écrit par le harnais, jamais par le gabarit.
        let mut lignes = vec![ligne_du_bloc(cat, &self.entite, uuid, &record)?];
        for (nom, r) in &self.decl.related {
            let Some(def) = cat.get_relation_def(&r.relation) else { continue };
            let entite_liee = if r.direction == "incoming" { def.from.clone() } else { def.to.clone() };
            for l in liees[nom].as_array().into_iter().flatten() {
                if let Some(u) = l["uuid"].as_str() {
                    lignes.push(ligne_du_bloc(cat, &entite_liee, u, l)?);
                }
            }
        }
        // **La fermeture du bloc** : une ligne sans fichier à elle, atteinte par
        // un lien sortant d'une ligne du bloc, y entre aussi, et ainsi de suite
        // (une seule fois chacune). Elle n'a pas d'autre endroit où survivre.
        if let Some(sans) = self.sans_fichier.get() {
            let mut vus: std::collections::BTreeSet<String> = lignes.iter().map(|l| l.uuid.clone()).collect();
            let mut i = 0;
            while i < lignes.len() {
                let mut cibles = Vec::new();
                for (relation, liens) in &lignes[i].links {
                    let Some(def) = cat.get_relation_def(relation) else { continue };
                    if !sans.contains(&def.to) {
                        continue;
                    }
                    for l in liens {
                        cibles.push((def.to.clone(), l.to.clone()));
                    }
                }
                for (entite, u) in cibles {
                    if !vus.insert(u.clone()) {
                        continue;
                    }
                    if let Some(ligne) = cat.get(&entite, &u).map_err(|e| e.to_string())? {
                        let ligne = serde_json::to_value(&ligne).map_err(|e| e.to_string())?;
                        lignes.push(ligne_du_bloc(cat, &entite, &u, &ligne)?);
                    }
                }
                i += 1;
            }
        }
        if !corps.is_empty() && !corps.ends_with('\n') {
            corps.push('\n');
        }
        corps.push_str(&ecrire_le_bloc(&lignes)?);
        let rel = self.chemin_relatif(uuid, &record)?;
        let ecrit = self.ecrire_texte(&rel, &self.rendre_avec(uuid, &record, &corps))?;
        Ok(Some((rel, ecrit)))
    }

    /// **Relit le bloc machine d'un fichier composé** (lot M3b) : les lignes
    /// qu'il porte — la sienne, puis ses lignes liées —, chacune avec ses
    /// champs et ses liens sortants. `None` : un fichier sans bloc (écrit avant
    /// M3b, ou à la main).
    pub fn relire_le_bloc(texte: &str) -> Result<Option<Vec<LigneDuBloc>>, String> {
        let Some(debut) = texte.rfind(DEBUT_DU_BLOC) else { return Ok(None) };
        let reste = &texte[debut + DEBUT_DU_BLOC.len()..];
        let fin = reste.find(FIN_DU_BLOC).ok_or("bloc machine sans fin")?;
        let yaml = reste[..fin].replace("--&gt;", "-->").replace("&amp;", "&");
        let bloc: BlocMachine = serde_norway::from_str(&yaml).map_err(|e| format!("bloc machine illisible : {e}"))?;
        Ok(Some(bloc.rows))
    }

    /// **Les lignes de cette entité qu'un événement du catalogue touche** : ses
    /// propres uuids, ou les propriétaires des lignes liées par une relation de
    /// `related`, ou les bouts d'un lien posé ou retiré par une de ces relations.
    /// Un `RelationsChanged` tronqué d'une relation que ce fichier lit : ses
    /// listes ne nomment pas toutes les paires, [`Self::touchees`] n'y suffit pas.
    /// Le bloc machine porte aussi les liens sortants de la ligne et de ses
    /// lignes liées : une annonce tronquée partie de l'une de leurs entités
    /// compte de même (lot M3b).
    pub fn lit_une_annonce_tronquee(&self, cat: &crate::catalog::Catalog, evenement: &Value) -> bool {
        if evenement["kind"] != "RelationsChanged" || evenement["truncated"] != true {
            return false;
        }
        if self.decl.related.values().any(|r| evenement["relation"] == r.relation.as_str()) {
            return true;
        }
        if !self.est_compose() {
            return false;
        }
        let depart = evenement["from_entity"].as_str().unwrap_or("");
        depart == self.entite
            || self.decl.related.values().any(|r| {
                cat.get_relation_def(&r.relation)
                    .is_some_and(|d| if r.direction == "incoming" { d.from == depart } else { d.to == depart })
            })
    }

    pub fn touchees(&self, cat: &crate::catalog::Catalog, evenement: &Value) -> Vec<String> {
        let mut out = Vec::new();
        let chaines = |v: &Value| -> Vec<String> {
            v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
        };
        match evenement["kind"].as_str() {
            Some("EntitiesChanged") | Some("EntityDeleted") | Some("EntityUpdated") => {
                let entite = evenement["entity"].as_str().unwrap_or("");
                let mut uuids = chaines(&evenement["uuids"]);
                if let Some(u) = evenement["uuid"].as_str() {
                    uuids.push(u.to_string());
                }
                if entite == self.entite {
                    out.extend(uuids.iter().cloned());
                }
                // Une ligne liée qui change : ses propriétaires, en remontant la relation.
                for r in self.decl.related.values() {
                    out.extend(self.proprietaires(cat, r, entite, &uuids));
                }
            }
            Some("RelationsChanged") => {
                let relation = evenement["relation"].as_str().unwrap_or("");
                let bout = |cote: usize| -> Vec<String> {
                    let mut v = Vec::new();
                    for liste in ["added", "removed"] {
                        if let Some(paires) = evenement[liste].as_array() {
                            v.extend(paires.iter().filter_map(|p| p[cote].as_str().map(str::to_string)));
                        }
                    }
                    v
                };
                for r in self.decl.related.values().filter(|r| r.relation == relation) {
                    out.extend(bout(if r.direction == "incoming" { 1 } else { 0 }));
                }
                // **Le bloc machine porte les liens sortants** de la ligne et de
                // ses lignes liées (lot M3b) : un lien posé ou retiré depuis
                // l'une d'elles réécrit le fichier, même par une relation que le
                // gabarit ne lit pas (une ancre vers une Ref).
                if self.est_compose() {
                    let depart = evenement["from_entity"].as_str().unwrap_or("");
                    let departs = bout(0);
                    if depart == self.entite {
                        out.extend(departs.iter().cloned());
                    }
                    for r in self.decl.related.values() {
                        out.extend(self.proprietaires(cat, r, depart, &departs));
                    }
                }
            }
            _ => {}
        }
        out.sort();
        out.dedup();
        out
    }

    /// Les lignes de cette entité qui ont parmi leurs lignes liées (la famille
    /// `r`) l'une des lignes `uuids` de l'entité `entite` : en remontant la
    /// relation de la famille.
    fn proprietaires(&self, cat: &crate::catalog::Catalog, r: &RelatedDecl, entite: &str, uuids: &[String]) -> Vec<String> {
        let Some(def) = cat.get_relation_def(&r.relation) else { return Vec::new() };
        let (bout_lie, sens_retour) = if r.direction == "incoming" {
            (&def.from, crate::search_strategy::ExpansionDirection::Outgoing)
        } else {
            (&def.to, crate::search_strategy::ExpansionDirection::Incoming)
        };
        if bout_lie != entite || uuids.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        if let Ok(proprietaires) = crate::dataflow::search_nodes::fetch_related_in(
            cat.dialect_arc().as_ref(),
            cat.conn(),
            uuids,
            &r.relation,
            sens_retour,
            0,
        ) {
            for enfants in proprietaires.values() {
                out.extend(enfants.iter().filter(|c| c.entity == self.entite).map(|c| c.uuid.clone()));
            }
        }
        out
    }

    /// **Tous les fichiers composés régénérés depuis la base** (au démarrage :
    /// un arrêt brutal entre une écriture et la réécriture laisse un fichier en
    /// retard). Rend combien ont été réécrits.
    pub fn regenerer_tout(&self, cat: &crate::catalog::Catalog) -> Result<usize, String> {
        let requete = format!("MATCH (n:{}) RETURN n._uuid", self.entite);
        crate::schema::validate_identifier(&self.entite, "entity").map_err(|e| e.to_string())?;
        let r = cat.execute_raw_with_params(&requete, &[]).map_err(|e| e.to_string())?;
        let mut ecrits = 0;
        for ligne in &r.rows {
            if let Some(u) = ligne.first().and_then(|v| v.as_str()) {
                if let Some((_, true)) = self.regenerer(cat, u)? {
                    ecrits += 1;
                }
            }
        }
        Ok(ecrits)
    }
}

/// Un ordre total sur des valeurs JSON de même sorte (nombres, chaînes), les
/// nulles d'abord.
fn ordre_json(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64().partial_cmp(&y.as_f64()).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
        (Value::Null, _) => std::cmp::Ordering::Less,
        (_, Value::Null) => std::cmp::Ordering::Greater,
        _ => a.to_string().cmp(&b.to_string()),
    }
}

/// Un chemin relatif, sans `..` ni racine, en texte (les deux séparateurs).
fn relatif_sur(chemin: &str, quoi: &str) -> Result<(), String> {
    if Path::new(chemin).is_absolute()
        || chemin.starts_with(['/', '\\'])
        || chemin.split(['/', '\\']).any(|c| c == "..")
        || Path::new(chemin).components().any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
    {
        return Err(format!("{quoi} : « {chemin} » doit être relatif à la racine, sans « .. »"));
    }
    Ok(())
}

/// Le chemin complet, **réellement** sous la racine : le maillon existant le
/// plus profond, canonisé, doit rester sous la racine canonisée (un lien
/// symbolique qui sort, ou un lien mort, est refusé) — la règle de
/// `code_tools::full_path_within`, sans la feature `code`.
fn sous_la_racine(racine: &Path, rel: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(racine).map_err(|e| format!("{} : {e}", racine.display()))?;
    let complet = racine.join(rel);
    let racine_canon = racine.canonicalize().map_err(|e| format!("{} : {e}", racine.display()))?;
    let mut maillon = complet.as_path();
    while maillon != racine {
        if let Ok(meta) = maillon.symlink_metadata() {
            match maillon.canonicalize() {
                Ok(canon) if canon.starts_with(&racine_canon) => break,
                Ok(_) => return Err(format!("« {rel} » traverse un lien qui sort de la racine des fichiers")),
                Err(_) if meta.file_type().is_symlink() => return Err(format!("« {rel} » traverse un lien mort")),
                Err(e) => return Err(format!("« {rel} » : {e}")),
            }
        }
        match maillon.parent() {
            Some(p) => maillon = p,
            None => break,
        }
    }
    Ok(complet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({"type": "object", "properties": {"chemin": {"type": "string"}, "titre": {"type": "string"},
               "contenu": {"type": "string"}, "rang": {"type": "integer"}}})
    }

    fn miroir(racine: &Path, decl: Value) -> Result<Miroir, String> {
        Miroir::nouveau("Source", racine.to_path_buf(), serde_json::from_value(decl).unwrap(), &schema(), racine)
    }

    #[test]
    fn le_fichier_porte_l_uuid_l_en_tete_et_le_corps_et_se_relit() {
        let d = tempfile::tempdir().unwrap();
        let m = miroir(d.path(), json!({"dir": "sources", "name": "{uuid}.md", "header": ["titre", "rang"], "body": "contenu"})).unwrap();
        let rec = json!({"titre": "Le journal : la reprise", "rang": 2, "contenu": "# Le journal\n\n---\n\nUne règle dans le corps.\n"});
        let rel = m.ecrire("u-1", &rec).unwrap();
        assert_eq!(rel, "sources/u-1.md");
        let texte = std::fs::read_to_string(d.path().join(&rel)).unwrap();
        let (yaml, corps) = crate::en_tete::separer(&texte).unwrap();
        assert_eq!(corps, rec["contenu"].as_str().unwrap(), "le corps est le champ tel quel");
        let mut s = schema();
        s["properties"]["uuid"] = json!({"type": "string"});
        let champs = crate::en_tete::lire_en_tete(yaml, &s).unwrap();
        assert_eq!(champs["uuid"], "u-1");
        assert_eq!(champs["titre"], "Le journal : la reprise");
        assert_eq!(champs["rang"], 2);
        // Réécrire la même ligne rend le même fichier, octet pour octet.
        m.ecrire("u-1", &rec).unwrap();
        assert_eq!(std::fs::read_to_string(d.path().join(&rel)).unwrap(), texte);
    }

    #[test]
    fn un_nom_par_champ_et_les_chemins_qui_sortent_sont_refuses() {
        let d = tempfile::tempdir().unwrap();
        let m = miroir(d.path(), json!({"name": "{chemin}", "header": ["titre"], "body": "contenu"})).unwrap();
        assert_eq!(m.ecrire("u", &json!({"chemin": "sources/2026-10-18-x.md", "contenu": "x"})).unwrap(), "sources/2026-10-18-x.md");
        for mauvais in ["../dehors.md", "/etc/x.md", "sources/../../x.md", "pas-markdown.txt"] {
            let e = m.ecrire("u", &json!({"chemin": mauvais, "contenu": "x"})).unwrap_err();
            assert!(e.contains(mauvais) || e.contains(".md"), "{mauvais} : {e}");
        }
    }

    #[test]
    fn la_declaration_est_controlee_au_chargement() {
        let d = tempfile::tempdir().unwrap();
        for (decl, attendu) in [
            (json!({"name": "{uuid}.md", "header": ["inconnu"]}), "inconnu"),
            (json!({"name": "{uuid}.md", "body": "rang"}), "chaîne"),
            (json!({"name": "{uuid}.txt"}), ".md"),
            (json!({"name": "{absent}.md"}), "absent"),
            (json!({"dir": "../x", "name": "{uuid}.md"}), ".."),
        ] {
            let e = miroir(d.path(), decl.clone()).unwrap_err();
            assert!(e.contains(attendu), "{decl} : {e}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn un_lien_qui_sort_de_la_racine_est_refuse() {
        let d = tempfile::tempdir().unwrap();
        let dehors = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(dehors.path(), d.path().join("sources")).unwrap();
        let m = miroir(d.path(), json!({"dir": "sources", "name": "{uuid}.md", "body": "contenu"})).unwrap();
        let e = m.ecrire("u", &json!({"contenu": "x"})).unwrap_err();
        assert!(e.contains("sort de la racine"), "{e}");
        assert!(std::fs::read_dir(dehors.path()).unwrap().next().is_none(), "rien n'est écrit dehors");
    }
}
