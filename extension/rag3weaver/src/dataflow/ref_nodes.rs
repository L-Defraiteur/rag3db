//! **Citer une chose, et apprendre un genre de chose.**
//!
//! Deux nœuds, un geste chacun, et chacun son refus — c'est le point de la
//! forme à deux outils plutôt qu'un objet structuré à champ optionnel : un
//! `new` qu'on remplit en passant fait entrer le désordre sans qu'on le voie,
//! là où un refus qui porte l'appel exact est suivi même par un modèle faible
//! (mesuré par la session recherche).
//!
//! **Rien ici ne nomme la mémoire longue.** Les entités, les champs et la
//! relation sont des paramètres ; le gabarit les lie. Une fiche de n'importe
//! quel backend peut citer n'importe quoi.
//!
//! **Pourquoi deux nœuds et pas un enchaînement de nœuds existants.** J'ai
//! cherché : la règle d'un genre vit dans une **ligne** (son champ `script`), et
//! `RhaiNode` prend son script en **configuration**, pas par un port — un graphe
//! ne peut donc pas lui passer le script du genre qu'il vient de lire. Et
//! `EntityBatchNode` prend ses enregistrements en configuration aussi. Les deux
//! nœuds sont de la forme de `ReactTransitionNode` : génériques, et ils écrivent
//! par les verbes du catalogue, jamais en Cypher — c'est le catalogue qui
//! réapplique la garde de cycle de vie.
//!
//! **Le harnais d'un genre est facultatif, et son absence se dit.** Une
//! référence d'un genre sans script est écrite avec son état et son empreinte
//! vides : « citée, non vérifiée » est une information, pas une erreur.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::catalog::Catalog;
use crate::connection::CypherValue;

/// Un nom qui sert d'étiquette ou de champ dans une requête doit être un nom.
/// Même garde que `ReactTransitionNode`, pour la même raison : ces valeurs
/// viennent d'une fiche, et une fiche peut avoir été écrite par un modèle.
fn identifiant(valeur: &str, quoi: &str) -> Result<String, String> {
    if valeur.is_empty() {
        return Err(format!("'{quoi}' est vide"));
    }
    if !valeur.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!(
            "'{quoi}' doit être un nom simple (lettres, chiffres, souligné) et vaut « {valeur} »"
        ));
    }
    Ok(valeur.to_string())
}

/// Les genres déclarés, lus dans leur table : `(nom, description, script)`.
///
/// Rendus **triés**, pour qu'un message de refus soit toujours le même — la
/// règle du dépôt pour les listes qui entrent dans une erreur.
fn genres(
    catalog: &Catalog,
    types_entity: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let rows = catalog
        .execute_raw(&format!(
            "MATCH (n:{types_entity}) WHERE n.etat = 'current' \
             RETURN n.name, n.description, n.script"
        ))
        .map_err(|e| format!("les genres de « {types_entity} » sont illisibles : {e}"))?
        .rows;
    let mut out: Vec<(String, String, String)> = rows
        .iter()
        .filter_map(|l| {
            let nom = l.first().and_then(|v| v.as_str())?.to_string();
            if nom.is_empty() {
                return None;
            }
            Some((
                nom,
                l.get(1).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                l.get(2).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            ))
        })
        .collect();
    out.sort();
    Ok(out)
}

/// Ce que rend un harnais rhai, réduit à ce qui nous intéresse.
struct Verdict {
    ok: bool,
    pourquoi: String,
    /// La forme normale, quand le script en rend une.
    normal: Option<String>,
    /// Ce qui fait vieillir, quand le script en rend une.
    empreinte: Option<String>,
}

/// Jouer `validate` d'un genre sur une valeur. Un script absent rend un verdict
/// **non vérifié** et non un échec : c'est le niveau 0 assumé.
fn eprouver(script: &str, valeur: &str, ctx: &NodeContext) -> Verdict {
    if script.trim().is_empty() {
        return Verdict { ok: true, pourquoi: "genre sans harnais : citée, non vérifiée".into(), normal: None, empreinte: None };
    }
    let limites = ctx
        .service::<crate::harness::RhaiLimits>("rhai_limits")
        .cloned()
        .unwrap_or_default();
    let entree = serde_json::json!({ "value": valeur });
    match crate::harness::evaluate(script, &entree, &limites) {
        Ok(v) => {
            let ok = v.get("ok").and_then(|b| b.as_bool()).unwrap_or(false);
            Verdict {
                ok,
                pourquoi: v
                    .get("why")
                    .and_then(|s| s.as_str())
                    .unwrap_or(if ok { "" } else { "refusée par le harnais du genre" })
                    .to_string(),
                normal: v.get("normal").and_then(|s| s.as_str()).map(str::to_string),
                empreinte: v.get("fingerprint").and_then(|s| s.as_str()).map(str::to_string),
            }
        }
        // **Une erreur de script n'est jamais un refus muet** : elle se dit, et
        // la référence reste du texte plutôt que d'être jetée.
        Err(e) => Verdict {
            ok: true,
            pourquoi: format!("le harnais du genre a échoué ({e}) : citée, non vérifiée"),
            normal: None,
            empreinte: None,
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// add_ref
// ═══════════════════════════════════════════════════════════════════════════

pub struct AddRefNode {
    node_name: String,
    cfg: serde_json::Value,
}

impl AddRefNode {
    fn champ(&self, cle: &str) -> String {
        self.cfg.get(cle).and_then(|v| v.as_str()).unwrap_or_default().trim().to_string()
    }
}

impl Node for AddRefNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "AddRefNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.cfg.clone()))
    }
    fn inputs(&self) -> Vec<PortDef> {
        super::node_registry::ports_declares(&AddRefNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        super::node_registry::ports_declares(&AddRefNodeFactory).1
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("AddRefNode: 'catalog' service not found")?;

        let refs_entity = identifiant(&self.champ("refs_entity"), "refs_entity")?;
        let types_entity = identifiant(&self.champ("types_entity"), "types_entity")?;
        let from_entity = identifiant(&self.champ("from_entity"), "from_entity")?;
        let relation = identifiant(&self.champ("relation"), "relation")?;
        let genre = self.champ("genre");
        let valeur = self.champ("valeur");
        let dit_comme = self.champ("dit_comme");

        let rendre = |ctx: &mut NodeContext, v: serde_json::Value| {
            ctx.set_output("report", PortValue::new(v));
        };

        if valeur.is_empty() {
            rendre(ctx, serde_json::json!({
                "executed": false,
                "needs": ["valeur"],
                "why": "une référence sans valeur ne désigne rien",
            }));
            return Ok(());
        }

        let mut cat = catalog.lock().map_err(|_| "AddRefNode: catalog poisoned".to_string())?;
        let liste = genres(&cat, &types_entity)?;

        // **Le genre inconnu n'est pas une erreur : c'est un complément
        // demandé.** La charge porte la liste du jour et l'appel exact — la
        // silhouette que les passes d'agent suivent, même avec un modèle
        // faible. Et rien n'est perdu : la valeur est rendue telle quelle.
        let Some((_, _, script)) = liste.iter().find(|(n, _, _)| *n == genre) else {
            rendre(ctx, serde_json::json!({
                "executed": false,
                "needs": ["genre"],
                "why": format!("le genre « {genre} » n'existe pas encore"),
                "genres": liste.iter().map(|(n, d, _)| serde_json::json!({"name": n, "description": d})).collect::<Vec<_>>(),
                "rappel": format!(
                    "crée-le par create_ref_type(name, description, forme, script), \
                     puis rappelle add_ref(genre: \"<le nom créé>\", valeur: {valeur:?})"
                ),
                "valeur": valeur,
            }));
            return Ok(());
        };

        let verdict = eprouver(script, &valeur, ctx);
        if !verdict.ok {
            rendre(ctx, serde_json::json!({
                "executed": false,
                "needs": ["valeur"],
                "why": verdict.pourquoi,
                "genre": genre,
                "valeur": valeur,
            }));
            return Ok(());
        }
        let normale = verdict.normal.unwrap_or_else(|| valeur.clone());

        let mut record: BTreeMap<String, CypherValue> = BTreeMap::new();
        record.insert("genre".into(), CypherValue::String(genre.clone()));
        record.insert("valeur".into(), CypherValue::String(normale.clone()));
        record.insert("dit_comme".into(), CypherValue::String(dit_comme));
        record.insert(
            "empreinte".into(),
            CypherValue::String(verdict.empreinte.unwrap_or_default()),
        );

        let uuid_ref = cat
            .entity_uuid(&refs_entity, &record)
            .map_err(|e| format!("AddRefNode: l'identité de la référence : {e}"))?;
        cat.ingest_entities(&refs_entity, vec![record])
            .map_err(|e| format!("AddRefNode: {e}"))?;

        // La fiche est désignée par ses champs d'identité, pas par un uuid que
        // l'appelant n'a pas : c'est le catalogue qui sait les réduire.
        let fiche: BTreeMap<String, CypherValue> = self
            .cfg
            .get("fiche")
            .and_then(|v| v.as_object())
            .map(|o| {
                o.iter()
                    .map(|(k, v)| {
                        (k.clone(), CypherValue::String(v.as_str().unwrap_or_default().to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if fiche.is_empty() {
            rendre(ctx, serde_json::json!({
                "executed": true,
                "ref": {"genre": genre, "valeur": normale, "uuid": uuid_ref},
                "lien": "aucun : « fiche » n'était pas donnée, la référence existe seule",
                "verifiee": !script.trim().is_empty(),
            }));
            return Ok(());
        }
        let uuid_fiche = cat
            .entity_uuid(&from_entity, &fiche)
            .map_err(|e| format!("AddRefNode: l'identité de la fiche : {e}"))?;
        cat.link(&relation, uuid_fiche.clone(), uuid_ref.clone(), BTreeMap::new())
            .map_err(|e| format!("AddRefNode: le lien {relation} : {e}"))?;

        rendre(ctx, serde_json::json!({
            "executed": true,
            "ref": {"genre": genre, "valeur": normale, "uuid": uuid_ref},
            "lien": {"relation": relation, "de": uuid_fiche, "vers": uuid_ref},
            // **Dit, jamais supposé** : une référence d'un genre sans harnais
            // est « citée, non vérifiée », et le rapport le porte.
            "verifiee": !script.trim().is_empty(),
            "note": verdict.pourquoi,
        }));
        Ok(())
    }
}

pub struct AddRefNodeFactory;

impl NodeFactory for AddRefNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let n = AddRefNode { node_name: name.into(), cfg: config.clone() };
        for cle in ["refs_entity", "types_entity", "from_entity", "relation"] {
            identifiant(&n.champ(cle), cle)?;
        }
        Ok(Box::new(n))
    }
    fn node_type(&self) -> &'static str {
        "AddRefNode"
    }
    fn schema(&self) -> NodeSchema {
        let p = |name, required, default: Option<serde_json::Value>, description, choices| ConfigParam {
            name,
            param_type: ConfigParamType::String,
            required,
            default,
            description,
            choices,
            json_schema: None,
        };
        NodeSchema {
            node_type: "AddRefNode",
            description: "Cites a thing from a record: validates the value against its genre's optional rhai harness, writes the reference (identity = genre + normal form, so citing twice does not duplicate), and links it. An unknown genre is not an error: it returns the current list and the exact create_ref_type call to make, and loses nothing",
            inputs: vec![],
            outputs: vec![PortDef { name: "report", port_type: PortType::Map, required: false }],
            config_params: vec![
                p("genre", true, None, "Genre of the thing cited; the list is read from the catalogue at each call, so a genre created a moment ago is already there",
                  Some(Choices::Values { entity: "RefType".into(), field: "name".into() })),
                p("valeur", true, None, "The thing cited, in whatever form the caller has it; the genre's harness normalises it", None),
                p("dit_comme", false, Some(serde_json::json!("")), "The words that cited it, when they are not the value itself (« le design d'hier »); empty for a value extracted without a model", None),
                p("refs_entity", false, Some(serde_json::json!("Ref")), "Entity that holds references", None),
                p("types_entity", false, Some(serde_json::json!("RefType")), "Entity that holds the genres", None),
                p("from_entity", false, Some(serde_json::json!("Memory")), "Entity of the record that cites", None),
                p("relation", false, Some(serde_json::json!("CITE")), "Relation from the record to the reference", None),
                ConfigParam {
                    name: "fiche",
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: None,
                    description: "Identity fields of the record that cites; omitted, the reference is written alone",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// create_ref_type
// ═══════════════════════════════════════════════════════════════════════════

pub struct CreateRefTypeNode {
    node_name: String,
    cfg: serde_json::Value,
}

impl CreateRefTypeNode {
    fn champ(&self, cle: &str) -> String {
        self.cfg.get(cle).and_then(|v| v.as_str()).unwrap_or_default().trim().to_string()
    }
}

impl Node for CreateRefTypeNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "CreateRefTypeNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.cfg.clone()))
    }
    fn inputs(&self) -> Vec<PortDef> {
        super::node_registry::ports_declares(&CreateRefTypeNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        super::node_registry::ports_declares(&CreateRefTypeNodeFactory).1
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("CreateRefTypeNode: 'catalog' service not found")?;
        let types_entity = identifiant(&self.champ("types_entity"), "types_entity")?;
        let nom = self.champ("name");
        let description = self.champ("description");
        let forme = self.champ("forme");
        let script = self.champ("script");
        let exemple = self.champ("exemple");
        let confirme = self.cfg.get("confirm").and_then(|v| v.as_bool()).unwrap_or(false);

        let rendre = |ctx: &mut NodeContext, v: serde_json::Value| {
            ctx.set_output("report", PortValue::new(v));
        };

        // Le nom d'un genre entre dans un enum lu par un modèle : on le tient
        // aux mêmes règles que les identifiants du dépôt.
        if let Err(e) = identifiant(&nom, "name") {
            rendre(ctx, serde_json::json!({
                "executed": false, "needs": ["name"], "why": e,
            }));
            return Ok(());
        }
        // **La description est le seul harnais obligatoire.** Un genre de
        // niveau 0 existe ; un genre sans description ne dit rien à personne.
        if description.is_empty() {
            rendre(ctx, serde_json::json!({
                "executed": false,
                "needs": ["description"],
                "why": "la description est obligatoire : c'est le seul harnais qu'un genre doit avoir, \
                        et c'est elle qui empêchera le prochain d'en créer un doublon",
            }));
            return Ok(());
        }

        let mut cat = catalog.lock().map_err(|_| "CreateRefTypeNode: catalog poisoned".to_string())?;
        let liste = genres(&cat, &types_entity)?;

        if liste.iter().any(|(n, _, _)| *n == nom) {
            rendre(ctx, serde_json::json!({
                "executed": false,
                "why": format!("le genre « {nom} » existe déjà"),
                "rappel": format!("appelle add_ref(genre: {nom:?}, valeur: …)"),
            }));
            return Ok(());
        }

        // **Montrer tous les genres, pas les plus proches.** La vision demande
        // « montrer les proches » ; avec une liste close d'une dizaine
        // d'entrées, tout montrer est strictement meilleur — pas de seuil à
        // transporter, pas de proche écarté par un rangement, et le coût est
        // nul. Le jour où cette liste dépassera ce qu'un message porte bien, un
        // rangement reprendra son emploi : c'est **ordonner** que la mesure
        // soutient, pas juger.
        if !confirme {
            rendre(ctx, serde_json::json!({
                "executed": false,
                "needs": ["confirm"],
                "why": format!(
                    "rien n'est écrit. Avant de créer « {nom} », regarde si l'un de ces genres \
                     est celui que tu veux : un genre de trop se paie longtemps"
                ),
                "genres": liste.iter().map(|(n, d, _)| serde_json::json!({"name": n, "description": d})).collect::<Vec<_>>(),
                "rappel": format!(
                    "soit add_ref(genre: \"<un genre de la liste>\", valeur: …), \
                     soit create_ref_type(name: {nom:?}, description: …, confirm: true) pour créer le tien"
                ),
            }));
            return Ok(());
        }

        // **Un harnais s'éprouve avant d'entrer**, sur l'exemple qui l'a fait
        // naître — et il ne doit pas attraper ce qu'un genre existant attrape
        // déjà, sinon « référence d'article » et « DOI » deviennent deux genres.
        let mut epreuve = serde_json::json!({"jouee": false});
        if !script.trim().is_empty() {
            if exemple.is_empty() {
                rendre(ctx, serde_json::json!({
                    "executed": false,
                    "needs": ["exemple"],
                    "why": "un harnais ne s'admet pas sans la valeur qui l'a fait naître : \
                            donne « exemple », on l'éprouve dessus",
                }));
                return Ok(());
            }
            let v = eprouver(&script, &exemple, ctx);
            if !v.ok {
                rendre(ctx, serde_json::json!({
                    "executed": false,
                    "needs": ["script"],
                    "why": format!("le harnais refuse l'exemple qui l'a fait naître : {}", v.pourquoi),
                    "exemple": exemple,
                }));
                return Ok(());
            }
            let deja: Vec<&String> = liste
                .iter()
                .filter(|(_, _, s)| !s.trim().is_empty() && eprouver(s, &exemple, ctx).ok)
                .map(|(n, _, _)| n)
                .collect();
            if !deja.is_empty() {
                rendre(ctx, serde_json::json!({
                    "executed": false,
                    "why": format!(
                        "« {exemple} » est déjà reconnu par : {}. Un genre de plus ne se justifie pas",
                        deja.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                    "rappel": format!("appelle add_ref(genre: {:?}, valeur: {exemple:?})", deja[0]),
                }));
                return Ok(());
            }
            epreuve = serde_json::json!({"jouee": true, "sur": exemple, "normal": v.normal});
        }

        let mut record: BTreeMap<String, CypherValue> = BTreeMap::new();
        record.insert("name".into(), CypherValue::String(nom.clone()));
        record.insert("description".into(), CypherValue::String(description));
        record.insert("forme".into(), CypherValue::String(forme));
        record.insert("script".into(), CypherValue::String(script.clone()));
        cat.ingest_entities(&types_entity, vec![record])
            .map_err(|e| format!("CreateRefTypeNode: {e}"))?;

        rendre(ctx, serde_json::json!({
            "executed": true,
            "genre": nom,
            // Le niveau se voit : un genre sans harnais est dit tel quel, et ses
            // références seront « non vérifiées ».
            "harnais": if script.trim().is_empty() { "description seule" } else { "script" },
            "epreuve": epreuve,
            "rappel": format!("il est dans la liste dès maintenant : add_ref(genre: {nom:?}, valeur: …)"),
        }));
        Ok(())
    }
}

pub struct CreateRefTypeNodeFactory;

impl NodeFactory for CreateRefTypeNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let n = CreateRefTypeNode { node_name: name.into(), cfg: config.clone() };
        identifiant(&n.champ("types_entity"), "types_entity")?;
        Ok(Box::new(n))
    }
    fn node_type(&self) -> &'static str {
        "CreateRefTypeNode"
    }
    fn schema(&self) -> NodeSchema {
        let p = |name, required, default: Option<serde_json::Value>, description| ConfigParam {
            name,
            param_type: ConfigParamType::String,
            required,
            default,
            description,
            choices: None,
            json_schema: None,
        };
        NodeSchema {
            node_type: "CreateRefTypeNode",
            description: "Declares a new genre of reference, for every use to come. Shows the current genres and asks which one is meant before creating (nothing is written on the first call); the description is mandatory, a rhai harness is optional and is tested on the example that gave birth to it — and refused if an existing genre already recognises it",
            inputs: vec![],
            outputs: vec![PortDef { name: "report", port_type: PortType::Map, required: false }],
            config_params: vec![
                p("name", true, None, "Name of the genre, as it will appear in the list the caller sees"),
                p("description", true, None, "What this genre designates, with two or three examples. Mandatory: it is the only harness a genre must have"),
                p("forme", false, Some(serde_json::json!("")), "The normal form in plain words (« path:<relative path> »); read by a human and by a model, it is not what verifies"),
                p("script", false, Some(serde_json::json!("")), "Optional rhai harness: validate (recognise and normalise), resolve, fingerprint. Empty: references of this genre are said « not verified »"),
                p("exemple", false, Some(serde_json::json!("")), "The value that gave birth to the genre; required as soon as a script is given, since the script is tested on it"),
                ConfigParam {
                    name: "confirm",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(false)),
                    description: "false (the default): nothing is written, the current genres are shown and the exact calls given. true: create",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le seul morceau qui se juge sans base**, et il décide de la sûreté des
    /// requêtes : un nom d'entité vient d'une fiche, et une fiche peut avoir été
    /// écrite par un modèle.
    #[test]
    fn un_nom_qui_nest_pas_un_nom_est_refuse() {
        assert!(identifiant("RefType", "e").is_ok());
        assert!(identifiant("ref_type_2", "e").is_ok());
        let e = identifiant("Ref) MATCH (x) DETACH DELETE x //", "refs_entity").unwrap_err();
        assert!(e.contains("refs_entity") && e.contains("nom simple"), "{e}");
        assert!(identifiant("", "genre").unwrap_err().contains("vide"));
    }

    /// Un genre sans harnais **passe** et le dit : « citée, non vérifiée » est
    /// une information, pas une erreur. Et un harnais qui explose ne jette pas
    /// la référence.
    #[test]
    fn un_harnais_absent_ou_cassse_laisse_passer_en_le_disant() {
        let ctx = NodeContext::new();

        let v = eprouver("", "docs/users.md", &ctx);
        assert!(v.ok && v.normal.is_none(), "{}", v.pourquoi);
        assert!(v.pourquoi.contains("non vérifiée"), "{}", v.pourquoi);

        let v = eprouver("ceci n'est pas du rhai (((", "x", &ctx);
        assert!(v.ok, "un script cassé ne jette pas la référence : {}", v.pourquoi);
        assert!(v.pourquoi.contains("échoué"), "{}", v.pourquoi);
    }
}
