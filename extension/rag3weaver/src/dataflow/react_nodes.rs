//! **Réagir à ce qui a changé : remonter une relation, et transitionner.**
//!
//! Le chaînon qui manquait pour qu'un graphe fasse quelque chose d'un
//! changement. Le réacteur ([`super::reactor`]) sait déjà **quand** lancer un
//! graphe — il attend sur les sujets du bus et déclare sa politique
//! (`each` / `batch` / `debounce`) — et `EventSourceNode` sait vider les
//! événements dans un port. Mais rien ne consommait ce port sauf
//! `TraceSinkNode`, qui les écrit tels quels : entre « un événement est
//! arrivé » et « ces lignes-là doivent changer d'état », il n'y avait pas de
//! pièce.
//!
//! **Rien ici ne nomme une entité, une relation ni un état.** Le gabarit les
//! donne, le catalogue dit ce qu'ils relient, et la machine à états déclarée
//! dit si le passage est permis. Pour la mémoire longue : une mémoire ancrée à
//! un sujet dont le contenu bouge passe `à revoir` ; pour un autre backend, une
//! fiche dont la source a changé repasse `à vérifier`. Le nœud ne sait ni l'un
//! ni l'autre.
//!
//! **La transition se nomme, elle ne se décrit pas.** Le gabarit donne le *nom*
//! d'une transition déclarée (`review`), pas un couple d'états : c'est la
//! déclaration qui porte `from` et `to`, et il n'y a donc aucun moyen d'écrire
//! ici un passage que la machine interdit. Une ligne qui n'est pas dans l'état
//! de départ n'est pas refusée — elle n'est **pas concernée**, et elle est
//! comptée à part, parce que « zéro ligne transitionnée » et « zéro ligne
//! concernée » ne veulent pas dire la même chose.
//!
//! **L'écriture passe par le verbe du catalogue**, jamais par du Cypher : c'est
//! `UpdateRecordNode` qui réapplique la garde de cycle de vie
//! (`record_nodes.rs`), et un nœud qui écrirait l'état directement la
//! contournerait — le genre de porte dérobée que l'audit du 4 octobre 2026 a
//! passé la nuit à fermer ailleurs.
//!
//! **La forme de la requête** suit la règle du journal (§6, 3 octobre 2026) :
//! `UNWIND $uuids AS u MATCH (…{_uuid: u})`, une liste de valeurs simples que
//! le moteur joint par hachage — jamais `item.champ`, qui le fait balayer la
//! table par produit cartésien.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};
use crate::catalog::Catalog;
use crate::connection::{CypherValue, QueryParam};

/// Lignes transitionnées par exécution, au plus.
pub const DEFAULT_LIMIT: usize = 500;

/// Ce que le gabarit déclare.
#[derive(Debug, Clone)]
pub struct ReactTransitionConfig {
    /// L'entité dont les changements déclenchent. Vide : n'importe laquelle.
    pub source: String,
    /// La relation à remonter.
    pub relation: String,
    /// Le sens, vu de la **cible**. `true` (le défaut) : la relation part de la
    /// cible vers la source — `(cible)-[r]->(source)`, qui est la forme d'une
    /// ancre. `false` : l'inverse.
    pub from_target: bool,
    /// L'entité dont les lignes atteintes reçoivent la transition.
    pub target: String,
    /// Le **nom** d'une transition déclarée sur la cible.
    pub transition: String,
    /// Plafond de lignes transitionnées par exécution.
    pub limit: usize,
}

impl Default for ReactTransitionConfig {
    fn default() -> Self {
        Self {
            source: String::new(),
            relation: String::new(),
            from_target: true,
            target: String::new(),
            transition: String::new(),
            limit: DEFAULT_LIMIT,
        }
    }
}

/// **Un nom qui entre dans du Cypher doit être un nom.**
///
/// Ces valeurs viennent d'un gabarit, et un gabarit peut avoir été écrit par un
/// modèle. Elles sont interpolées dans la requête — un label ni une relation ne
/// se passent en paramètre — donc elles se vérifient avant, et le refus nomme
/// la valeur fautive. Tout le reste du fichier passe par `$uuids`.
fn identifiant(valeur: &str, quoi: &str) -> Result<String, String> {
    if valeur.is_empty() {
        return Err(format!("ReactTransitionNode: '{quoi}' est vide"));
    }
    if !valeur.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!(
            "ReactTransitionNode: '{quoi}' doit être un nom simple (lettres, chiffres, souligné) \
             et vaut « {valeur} »"
        ));
    }
    Ok(valeur.to_string())
}

pub struct ReactTransitionNode {
    node_name: String,
    cfg: ReactTransitionConfig,
}

impl ReactTransitionNode {
    pub fn new(name: &str, cfg: ReactTransitionConfig) -> Self {
        Self { node_name: name.to_string(), cfg }
    }

    /// Les uuid venus des événements : ceux d'un `EntitiesChanged` de la source
    /// déclarée. Un événement d'un autre genre est ignoré sans bruit — le port
    /// porte tout ce que le curseur a vidé, c'est sa nature.
    fn uuids_des_evenements(&self, events: &[serde_json::Value]) -> Vec<String> {
        let mut out = Vec::new();
        for e in events {
            if e.get("kind").and_then(|v| v.as_str()) != Some("EntitiesChanged") {
                continue;
            }
            if !self.cfg.source.is_empty()
                && e.get("entity").and_then(|v| v.as_str()) != Some(self.cfg.source.as_str())
            {
                continue;
            }
            if let Some(liste) = e.get("uuids").and_then(|v| v.as_array()) {
                out.extend(liste.iter().filter_map(|v| v.as_str()).map(str::to_string));
            }
        }
        out
    }
}

impl Node for ReactTransitionNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "ReactTransitionNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "source": self.cfg.source,
            "relation": self.cfg.relation,
            "from_target": self.cfg.from_target,
            "target": self.cfg.target,
            "transition": self.cfg.transition,
            "limit": self.cfg.limit,
        })))
    }
    /// **Les ports d'entrée se déclarent ici aussi, pas seulement au schéma.**
    /// Le constructeur de graphe valide les arêtes contre `inputs()`, dont le
    /// défaut du trait est vide : sans cette redéclaration, une arête vers
    /// `events` est refusée par « input port 'events' not found », alors que le
    /// schéma la déclare. Trouvé par le témoin de bout en bout — aucun test
    /// unitaire du nœud ne pouvait le voir, puisqu'il ne construit pas de
    /// graphe.
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(
            &crate::dataflow::react_nodes::ReactTransitionNodeFactory,
        )
        .0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(
            &crate::dataflow::react_nodes::ReactTransitionNodeFactory,
        )
        .1
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("ReactTransitionNode: 'catalog' service not found")?;

        let relation = identifiant(&self.cfg.relation, "relation")?;
        let target = identifiant(&self.cfg.target, "target")?;
        let source = if self.cfg.source.is_empty() {
            None
        } else {
            Some(identifiant(&self.cfg.source, "source")?)
        };

        // Les deux entrées s'additionnent : le port `events` pour le chemin du
        // réacteur, le port `uuids` pour qu'un graphe appelé à la main — ou un
        // test — puisse jouer le même nœud sans bus.
        let events = ctx
            .take_input("events")
            .and_then(take_or_clone::<serde_json::Value>)
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        let mut vus: BTreeSet<String> = self.uuids_des_evenements(&events).into_iter().collect();
        if let Some(liste) = ctx
            .take_input("uuids")
            .and_then(take_or_clone::<serde_json::Value>)
            .and_then(|v| v.as_array().cloned())
        {
            vus.extend(liste.iter().filter_map(|v| v.as_str()).map(str::to_string));
        }
        let vus: Vec<String> = vus.into_iter().collect();

        let rendre = |ctx: &mut NodeContext,
                      changees: Vec<String>,
                      vus: usize,
                      concernees: usize,
                      hors_etat: usize,
                      refus: Vec<String>| {
            // **Les quatre comptes sont rendus séparément, toujours.** « Zéro
            // transitionnée » ne dit pas si rien n'avait changé (`seen`), si
            // rien n'était ancré (`concerned`), ou si les lignes atteintes
            // n'étaient pas dans l'état de départ (`out_of_state`) : trois
            // causes, trois remèdes. Un compte produit et non rendu est le
            // défaut que l'audit du 4 octobre 2026 a nommé ; ce nœud sort de
            // cet audit, il ne va pas le réécrire.
            ctx.set_output("changed", PortValue::new(serde_json::json!(changees)));
            ctx.set_output(
                "report",
                PortValue::new(serde_json::json!({
                    "seen": vus,
                    "concerned": concernees,
                    "out_of_state": hors_etat,
                    "transitioned": changees.len(),
                    "refused": refus,
                })),
            );
        };

        if vus.is_empty() {
            rendre(ctx, Vec::new(), 0, 0, 0, Vec::new());
            return Ok(());
        }

        // La transition, lue dans la **déclaration** de la cible. C'est elle qui
        // porte `from` et `to` : le gabarit n'en donne que le nom, et ne peut
        // donc pas écrire un passage que la machine interdit.
        let (champ, depuis, vers) = {
            let cat = catalog.lock().map_err(|_| "ReactTransitionNode: catalog poisoned".to_string())?;
            let config = cat
                .entity_configs()
                .get(&target)
                .ok_or_else(|| format!("ReactTransitionNode: entité « {target} » inconnue du catalogue"))?
                .clone();
            let lc = config.lifecycle.clone().ok_or_else(|| {
                format!("ReactTransitionNode: « {target} » n'a pas de machine à états déclarée, il n'y a donc aucune transition à appliquer")
            })?;
            let t = lc
                .transitions
                .iter()
                .find(|t| t.name == self.cfg.transition)
                .ok_or_else(|| {
                    let noms: Vec<&str> = lc.transitions.iter().map(|t| t.name.as_str()).collect();
                    format!(
                        "ReactTransitionNode: « {target} » ne déclare pas la transition « {} » (déclarées : {})",
                        self.cfg.transition,
                        noms.join(", ")
                    )
                })?
                .clone();
            (identifiant(&lc.field, "lifecycle.field")?, t.from, t.to)
        };

        // Remonter la relation, par un saut que dit le dialecte. La table de
        // la source est posée si elle est déclarée ; sans elle le nœud de
        // départ est retrouvé par son seul uuid.
        let sens = if self.cfg.from_target { rag3weaver_ir::Direction::Incoming } else { rag3weaver_ir::Direction::Outgoing };
        let mut saut = rag3weaver_ir::Hop::new("", &relation, &target, sens);
        saut.start = source.clone();
        saut.returns.push(rag3weaver_ir::Column::Node(champ.clone()));

        let lignes = {
            let cat = catalog.lock().map_err(|_| "ReactTransitionNode: catalog poisoned".to_string())?;
            let cypher = cat.dialect_arc().hop(&saut).map_err(|e| format!("ReactTransitionNode: {e}"))?;
            cat.execute_raw_with_params(
                &cypher,
                &[QueryParam::new(
                    "uuids",
                    CypherValue::List(vus.iter().map(|u| CypherValue::String(u.clone())).collect()),
                )],
            )
            .map_err(|e| format!("ReactTransitionNode: {e}"))?
        };

        // Dédoublonné et borné : une même cible peut être atteinte par
        // plusieurs sources changées, et elle ne se transitionne qu'une fois.
        let mut concernees: BTreeSet<String> = BTreeSet::new();
        let mut hors_etat = 0usize;
        for ligne in &lignes.rows {
            // La première colonne est l'uuid de départ : la cible vient après.
            let Some(uuid) = ligne.get(1).and_then(|v| v.as_str()) else { continue };
            // Seules les lignes **dans l'état de départ** sont concernées. Les
            // autres ne sont pas un refus : la transition ne les regarde pas.
            match ligne.get(2).and_then(|v| v.as_str()) {
                Some(etat) if etat == depuis => {
                    concernees.insert(uuid.to_string());
                }
                _ => hors_etat += 1,
            }
        }
        let concernees: Vec<String> = concernees.into_iter().take(self.cfg.limit).collect();

        let mut changees = Vec::new();
        let mut refus = Vec::new();
        for uuid in &concernees {
            let mut data: BTreeMap<String, CypherValue> = BTreeMap::new();
            data.insert(champ.clone(), CypherValue::String(vers.clone()));
            let mut cat = catalog.lock().map_err(|_| "ReactTransitionNode: catalog poisoned".to_string())?;
            match cat.update(&target, uuid, data) {
                Ok(()) => changees.push(uuid.clone()),
                // **Un refus par ligne ne fait pas tomber le lot.** La garde de
                // cycle de vie a pu changer d'avis entre la lecture et
                // l'écriture — une autre session est passée — et la cause
                // qu'elle rend est plus précise que tout ce qu'on dirait ici.
                Err(e) => refus.push(format!("{uuid} : {e}")),
            }
        }

        let (vus_n, concernees_n) = (vus.len(), concernees.len());
        rendre(ctx, changees, vus_n, concernees_n, hors_etat, refus);
        Ok(())
    }
}

pub struct ReactTransitionNodeFactory;

impl NodeFactory for ReactTransitionNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let texte = |cle: &str| -> String {
            config.get(cle).and_then(|v| v.as_str()).unwrap_or_default().trim().to_string()
        };
        let cfg = ReactTransitionConfig {
            source: texte("source"),
            relation: texte("relation"),
            from_target: config.get("from_target").and_then(|v| v.as_bool()).unwrap_or(true),
            target: texte("target"),
            transition: texte("transition"),
            limit: config
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|l| (l as usize).clamp(1, DEFAULT_LIMIT))
                .unwrap_or(DEFAULT_LIMIT),
        };
        // Vérifié à la **construction** et pas seulement à l'exécution : un
        // gabarit fautif se refuse quand on le pose, pas la première fois qu'un
        // événement arrive — sans quoi l'erreur tomberait dans un réacteur que
        // personne ne regarde.
        identifiant(&cfg.relation, "relation")?;
        identifiant(&cfg.target, "target")?;
        if cfg.transition.is_empty() {
            return Err("ReactTransitionNode: 'transition' est vide ; c'est le nom d'une transition déclarée sur la cible".into());
        }
        if !cfg.source.is_empty() {
            identifiant(&cfg.source, "source")?;
        }
        Ok(Box::new(ReactTransitionNode::new(name, cfg)))
    }
    fn node_type(&self) -> &'static str {
        "ReactTransitionNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "ReactTransitionNode".into(),
            description: "Applies a declared transition to the rows of 'target' reached from the uuids of EntitiesChanged events (port 'events') or of the 'uuids' port, by walking 'relation'; outputs the transitioned uuids and a report {seen, concerned, transitioned, refused}".into(),
            inputs: vec![
                PortDef { name: "events".into(), port_type: PortType::Map, required: false },
                PortDef { name: "uuids".into(), port_type: PortType::Map, required: false },
            ],
            outputs: vec![
                PortDef { name: "changed".into(), port_type: PortType::Map, required: false },
                PortDef { name: "report".into(), port_type: PortType::Map, required: false },
            ],
            config_params: vec![
                ConfigParam {
                    name: "target".into(),
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Entity whose rows receive the transition; it must declare a lifecycle".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "transition".into(),
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Name of a transition declared on 'target'; its from/to come from the declaration, never from here".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "relation".into(),
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Relation walked from the target to the changed rows".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "source".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Entity whose changes trigger; empty means any".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "from_target".into(),
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(true)),
                    description: "true: (target)-[relation]->(changed row), the shape of an anchor; false: the reverse".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "limit".into(),
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(DEFAULT_LIMIT)),
                    description: "Maximum rows transitioned per execution".into(),
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

    fn cfg() -> ReactTransitionConfig {
        ReactTransitionConfig {
            source: "Subject".into(),
            relation: "ANCHORED_TO".into(),
            target: "Memory".into(),
            transition: "review".into(),
            ..Default::default()
        }
    }

    /// **Le tri des événements, sans base.** C'est la seule logique du nœud qui
    /// se juge sans moteur, et elle décide de tout le reste : un port vidé porte
    /// tout ce que le curseur avait, pas seulement ce qui nous concerne.
    #[test]
    fn seuls_les_changements_de_la_source_declaree_comptent() {
        let n = ReactTransitionNode::new("r", cfg());
        let events = vec![
            serde_json::json!({ "kind": "EntitiesChanged", "entity": "Subject", "uuids": ["a", "b"] }),
            // Une autre entité : ignorée.
            serde_json::json!({ "kind": "EntitiesChanged", "entity": "Autre", "uuids": ["z"] }),
            // Un autre genre d'événement : ignoré sans bruit.
            serde_json::json!({ "kind": "SearchCompleted", "kb": "x", "results": 3 }),
            // Un `EntitiesChanged` sans uuid : rien à remonter.
            serde_json::json!({ "kind": "EntitiesChanged", "entity": "Subject" }),
        ];
        assert_eq!(n.uuids_des_evenements(&events), vec!["a".to_string(), "b".to_string()]);
    }

    /// Source vide : n'importe quelle entité déclenche. C'est le cas d'un
    /// gabarit qui ancre vers plusieurs entités — la raison pour laquelle
    /// l'ancre est polymorphe.
    #[test]
    fn une_source_vide_accepte_toutes_les_entites() {
        let n = ReactTransitionNode::new("r", ReactTransitionConfig { source: String::new(), ..cfg() });
        let events = vec![
            serde_json::json!({ "kind": "EntitiesChanged", "entity": "Subject", "uuids": ["a"] }),
            serde_json::json!({ "kind": "EntitiesChanged", "entity": "Autre", "uuids": ["z"] }),
        ];
        assert_eq!(n.uuids_des_evenements(&events), vec!["a".to_string(), "z".to_string()]);
    }

    /// **Un nom qui entre dans du Cypher se vérifie, et le refus nomme la
    /// valeur.** Un gabarit peut avoir été écrit par un modèle.
    #[test]
    fn un_nom_qui_nest_pas_un_nom_est_refuse_a_la_construction() {
        let mauvais = serde_json::json!({
            "target": "Memory", "transition": "review",
            "relation": "ANCHORED_TO) MATCH (x) DETACH DELETE x //",
        });
        // `unwrap_err` demanderait `Debug` sur `Box<dyn Node>`, que le trait
        // n'a pas : on nomme donc le cas réussi comme un échec du test.
        let e = match ReactTransitionNodeFactory.create("r", &mauvais) {
            Err(e) => e,
            Ok(_) => panic!("une relation qui porte du Cypher doit être refusée à la construction"),
        };
        assert!(e.contains("relation") && e.contains("nom simple"), "{e}");

        let sans_transition = serde_json::json!({ "target": "Memory", "relation": "ANCHORED_TO" });
        let e = match ReactTransitionNodeFactory.create("r", &sans_transition) {
            Err(e) => e,
            Ok(_) => panic!("un nœud sans transition nommée n'a rien à appliquer"),
        };
        assert!(e.contains("transition"), "{e}");
    }
}
