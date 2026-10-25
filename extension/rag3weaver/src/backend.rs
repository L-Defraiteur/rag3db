//! A backend is a manifest, payload schemas and explicitly exposed graph tools.
//! No domain vocabulary lives in this module. One host owns the catalogue.
use crate::dataflow::{
    build_definition, builtin_graph_tools, port_value_to_checkpoint,
    reactor::{Reactor, ReactorHandle},
    DataflowRuntime, GraphDefinition, GraphTool, NodeRegistry, NodeTypePolicy, ServiceRegistry,
};
use crate::{
    catalog::Catalog,
    config::{CatalogConfig, EntityConfig, RelationDef},
    connection::DbConnection,
    embedder::Embedder,
    json_schema::JsonSchemaMapping,
    search_backend::MoteurTexte,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Le nom de l'extension vectorielle du bâti : le défaut de
/// `vector_extension`, et ce que les gabarits livrés déclarent.
pub const DEFAULT_VECTOR_EXTENSION: &str = "vector";

fn default_vector_extension() -> PathBuf {
    PathBuf::from(DEFAULT_VECTOR_EXTENSION)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackendManifest {
    pub version: u32,
    /// Script sources and hook resources are chosen by the host, never tool callers.
    #[serde(default)]
    pub scripts: BTreeMap<String, PathBuf>,
    pub name: String,
    pub database: PathBuf,
    /// La déclaration d'avant de l'embarquement dense. Gardée : c'est un
    /// alias de `models.embed` — l'une ou l'autre, pas les deux.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<EmbeddingService>,
    /// **Les modèles, par capacité** — en local, en service à nous, ou chez
    /// un tiers : la même déclaration pour toutes
    /// (`crate::model_source::ModelSource`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub models: BTreeMap<crate::model_source::Capability, crate::model_source::ModelSource>,
    /// **L'extension vectorielle** : un **nom** (`vector`, le défaut) veut
    /// dire « celle du bâti », trouvée par [`PreparedBackend::vector_extension_path`] ;
    /// un **chemin** (un séparateur, ou le nom de fichier entier) est un
    /// forçage, relatif au manifeste.
    #[serde(default = "default_vector_extension")]
    pub vector_extension: PathBuf,
    pub entities: BTreeMap<String, BackendEntity>,
    #[serde(default)]
    pub relations: HashMap<String, BackendRelation>,
    /// Only these attachments become tools. Bindings cannot be overridden by callers.
    pub tools: BTreeMap<String, ToolAttachment>,
    /// **Les graphes réactifs, à côté des outils et jamais parmi eux.** Un
    /// outil est appelé par l'agent, une réaction par un événement du bus ;
    /// le moteur fait déjà respecter la distinction, et c'est lui qui me l'a
    /// apprise — déclarer une fiche réactive dans `tools` faisait refuser le
    /// manifeste **entier**. Sa liste blanche est la sienne
    /// ([`crate::backend_code::reaction_nodes`]) et n'ouvre aucune capacité
    /// de fichier ni de commande. Proposition du 5 octobre 2026
    /// (`docs/5-octobre-2026-00h00/01`), tranchée par l'orchestration.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reactions: BTreeMap<String, ReactionAttachment>,
    /// **Où l'hôte tient les cases d'écran de portée `hôte`** (les graphes de clic des
    /// composants) : `{"entity": "<Entité>"}`, une entité de ce manifeste. Sans lui, une case de
    /// portée `hôte` est refusée à la déclaration (`dataflow::clic`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_state: Option<UiState>,
    /// **Les lignes que le gabarit sème** : `{"RefType": [{…}, …]}`. Posées à
    /// l'ouverture **si elles manquent** — l'identité de la ligne (sa clé
    /// `hashsafe`, dans la cellule du catalogue) décide —, par le même chemin
    /// qu'une écriture d'outil (`EntityRecordNode`) : politique d'écriture,
    /// dates du serveur, validation par le schéma de l'entité. Une ligne déjà
    /// là n'est jamais réécrite : ce qu'un utilisateur en a fait (un genre
    /// retiré, une description reprise) l'emporte sur le gabarit. Une ligne
    /// qui ne valide pas refuse l'ouverture en se nommant. Un gabarit qui ne
    /// pouvait pas semer ses propres genres laissait un pas manuel à l'agent
    /// (18 octobre 2026, les genres `path` et `symbol` de la mémoire).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub seed: BTreeMap<String, Vec<Value>>,
    /// **La racine des fichiers** des entités qui déclarent `files`, relative
    /// au manifeste ([`crate::fichiers_entite`]). Requise dès qu'une entité en
    /// déclare.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_root: Option<PathBuf>,
    /// **Les routes** : `"GET /products/{key}": {"tool": …, "view": …}`. Une
    /// adresse mène à un outil déclaré (le contrôleur), son résultat à une vue
    /// du dossier `views/` ([`crate::routes`]).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub routes: BTreeMap<String, crate::routes::RouteDecl>,
    /// Opt in to discovery and ad-hoc read-only Mermaid search tools.
    #[serde(default)]
    pub search_graphs: bool,
    /// **L'outil `graph`** : le graphe d'un outil, d'une réaction ou d'un
    /// composant, en JSON (le même que la route `/ide/graphe`). Opt-in, comme
    /// `search_graphs` : il change la liste d'outils que voit l'agent.
    #[serde(default)]
    pub graph_tool: bool,
    /// **Qui applique une proposition de déclarations** (page 08) :
    /// `approbation` (le défaut) — l'agent propose, une personne applique dans
    /// l'IDE ; `auto` — l'agent déclare directement et applique.
    #[serde(default)]
    pub declarations: crate::propositions::ModeDeclarations,
    /// Creation-time Lucivy option; existing indexes retain their persisted setting.
    #[serde(default)]
    pub fts_positions: Option<bool>,
    /// La surface de code (3 octobre 2026) : quelle source de fichiers, sous
    /// quelle racine, avec quelle porte de commandes. Absente : aucun outil
    /// de code ne peut lire ni écrire — le défaut sûr.
    #[serde(default)]
    pub workspace: Option<crate::backend_code::WorkspaceConfig>,
    /// **Les montres de fichiers** : chacune balaie la source du workspace et
    /// publie un `FileChanged` par fichier créé, modifié ou supprimé, sur le
    /// sujet `files` — de quoi réagir à une édition faite hors de nos outils.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub watches: BTreeMap<String, crate::watch::WatchConfig>,
    /// **Le tampon du moteur**, en octets. Absent : la règle du produit
    /// (`rag3db_connection::buffer_pool_choice`), que la variable
    /// `RAG3DB_BUFFER_POOL_SIZE` surpasse toujours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buffer_pool: Option<u64>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingService {
    /// Vide ou absente : l'adresse vient de `RAG3WEAVER_EMBED_SERVICE`
    /// (`DaemonEmbedder::from_service`), choisie par le modèle demandé.
    #[serde(default)]
    pub address: String,
    pub model: String,
    pub dimensions: usize,
    #[serde(default)]
    pub provider: EmbeddingProvider,
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Le délai d'un appel, en secondes (voir `ModelSource::timeout_secs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProvider {
    #[default]
    Daemon,
    Compatible,
}
impl EmbeddingService {
    /// La même chose, dans la déclaration commune.
    pub fn source(&self) -> crate::model_source::ModelSource {
        use crate::model_source::{Addresses, Fallback, ModelSource, Provider};
        ModelSource {
            provider: match self.provider {
                EmbeddingProvider::Daemon => Provider::Service,
                EmbeddingProvider::Compatible => Provider::Compatible,
            },
            model: self.model.clone(),
            address: Addresses::parse(&self.address),
            protocol: None,
            api_key_env: self.api_key_env.clone(),
            fallback: Fallback::Refuse,
            dimensions: Some(self.dimensions),
            context_tokens: None,
            params: Default::default(),
            timeout_secs: self.timeout_secs,
        }
    }
}

impl BackendManifest {
    /// **L'embarquement dense de ce backend** : `models.embed`, ou la section
    /// `embeddings` d'avant. Les deux à la fois, ou aucune, c'est une erreur
    /// dite au chargement.
    pub fn embed_source(&self) -> Result<crate::model_source::ModelSource, String> {
        use crate::model_source::Capability;
        match (self.models.get(&Capability::Embed), &self.embeddings) {
            (Some(_), Some(_)) => Err("`models.embed` et `embeddings` déclarent tous deux l'embarquement : gardez-en un".into()),
            (Some(source), None) => match source.dimensions {
                Some(_) => Ok(source.clone()),
                None => Err("`models.embed` doit déclarer `dimensions`".into()),
            },
            (None, Some(legacy)) => Ok(legacy.source()),
            (None, None) => Err("ce backend ne déclare pas d'embarquement : `models.embed` (ou `embeddings`) est requis".into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackendEntity {
    pub schema: PathBuf,
    pub config: EntityConfig,
    #[serde(default)]
    pub writes: WritePolicy,
    /// Ce que cette entité **est**, pour un agent qui choisit ses outils —
    /// reprise dans la description générée de chaque outil qui la vise et en
    /// tête du schéma de ses filtres. La leçon du deck builder : tous les
    /// `search_*` disaient le même texte, l'agent choisissait au hasard du
    /// nom. Dire **quand** prendre l'outil, pas ce qu'il fait techniquement.
    #[serde(default)]
    pub description: Option<String>,
    /// **Le miroir en fichiers** de ses lignes ([`crate::fichiers_entite`]) :
    /// un fichier Markdown par ligne sous `files_root`, écrit avant la ligne.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<crate::fichiers_entite::FilesDecl>,
}
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WritePolicy {
    /// Optional server-generated epoch-millisecond fields, declared as integers in the schema.
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub revision: Option<String>,
    /// Immutable rows can be inserted or replayed identically, never overwritten.
    #[serde(default)]
    pub immutable: bool,
    #[serde(default)]
    pub transition_dates: Vec<TransitionDate>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionDate {
    pub field: String,
    pub to: Value,
    pub timestamp_field: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolAttachment {
    pub graph: PathBuf,
    /// Surcharge la ligne `%% description:` du gabarit — un produit change
    /// une phrase sans dupliquer un `.mmd`. Absente : le gabarit garde son
    /// texte.
    #[serde(default)]
    pub description: Option<String>,
    /// Ce que cet outil a le droit de faire (fichiers, commandes) — tout
    /// `false` par défaut : la base déclarative seule, comme avant. C'est la
    /// politique qui fait deux produits d'un même moteur.
    #[serde(default)]
    pub policy: crate::backend_code::ToolPolicy,
    /// **La réponse dit sa fraîcheur** : sur quel commit et quelle branche l'index a été bâti,
    /// et combien de fichiers changés attendent leur réindexation — une ligne en tête
    /// (markdown) et un champ `freshness` (toujours). Pour les outils de lecture d'un index.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub freshness: bool,
    #[serde(default)]
    pub harness: ToolHarness,
    #[serde(default)]
    pub bindings: serde_json::Map<String, Value>,
    /// Derive caller payloads from the entity schema, excluding server-owned fields.
    #[serde(default)]
    pub input_payloads: BTreeMap<String, InputPayload>,
    /// Terminal metadata ports to include alongside the declared result.
    #[serde(default)]
    pub metadata: Vec<OutputPort>,
    /// **Le crochet après outil** : un petit graphe qui ajoute une section
    /// au rendu de cet outil — et qui a le droit de se taire. Proposition du
    /// 3 octobre 2026 (docs/3-octobre-2026-23h05/01), validée.
    #[serde(default)]
    pub after: Option<AfterToolHook>,
}

/// **Une relation du manifeste** : sa définition au catalogue, à plat, et la
/// politique d'écriture que le backend lui ajoute — comme `writes` pour une
/// entité.
///
/// `from_states` / `to_states` : le bout doit être dans l'un de ces états de son
/// cycle de vie pour qu'un lien **s'y pose** (on n'ancre pas une mémoire
/// `superseded`). Vérifiés au chargement (le bout a un cycle de vie, les états y
/// existent) et à l'écriture par **les nœuds du backend qui posent des liens**.
///
/// **Le prix, dit ici parce que c'est ici qu'on déclare** : un appel nu à
/// `Catalog::link` ne les voit pas. C'est un verbe brut, comme du SQL ; la
/// garde tient pour les outils du backend, pas pour qui écrit au catalogue
/// directement (relecture de rag3db-97, 11 octobre 2026).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendRelation {
    #[serde(flatten)]
    pub def: RelationDef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_states: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_states: Option<Vec<String>>,
}

/// La clé du service qui porte les politiques d'écriture des relations
/// (`HashMap<String, BackendRelation>`), pour les nœuds qui posent des liens.
pub const RELATION_POLICIES_SERVICE: &str = "backend_relation_policies";

impl std::ops::Deref for BackendRelation {
    type Target = RelationDef;
    fn deref(&self) -> &RelationDef {
        &self.def
    }
}

/// **Une réaction** : un graphe monté au chargement du backend et déclenché
/// par un sujet du bus, non par l'agent.
///
/// Même forme d'attachement qu'un outil — un graphe et ses liaisons — parce
/// qu'un gabarit qui sait déclarer un outil doit savoir déclarer une réaction
/// sans apprendre une seconde grammaire. Ce qu'elle porte en plus, elle le
/// porte **déjà dans sa fiche** : les sujets (`%% on:`) et la politique de
/// déclenchement (`%% policy: each | batch <ms> | debounce <ms>`). Rien à
/// ajouter à la grammaire des fiches.
///
/// Et ce qu'elle n'a pas, volontairement : **pas de `policy` de capacités**.
/// Une réaction part sans personne devant l'écran ; elle écrit en base par
/// les verbes du catalogue — donc sous la garde de cycle de vie — et ne
/// touche ni fichier ni commande.
/// Ce que le backend offre à l'écran de ses composants : l'écran de chaque
/// outil et le lieu des cases de portée `hôte`.
fn ecran_du_backend(manifest: &BackendManifest, manifest_path: &Path) -> crate::components::EcranDuBackend {
    crate::components::EcranDuBackend {
        outils: manifest.tools.iter().map(|(nom, t)| (nom.clone(), t.policy.ecran)).collect(),
        etat_hote: manifest.ui_state.as_ref().map(|u| u.entity.clone()),
        // Le chemin canonique, celui que l'hôte connaît (`/ide/etat`) et que `/ide/call` reçoit.
        manifeste: manifest_path.canonicalize().unwrap_or_else(|_| manifest_path.to_path_buf()).display().to_string(),
    }
}

/// Le lieu des cases d'écran de portée `hôte` (`BackendManifest::ui_state`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UiState {
    /// L'entité du manifeste qui garde ces cases.
    pub entity: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReactionAttachment {
    pub graph: PathBuf,
    /// **Les liaisons déterminent la réaction entièrement.** Vérifié au
    /// chargement : la fiche doit s'instancier avec ces seules liaisons, car
    /// personne n'est là pour fournir un argument manquant à l'exécution.
    #[serde(default)]
    pub bindings: serde_json::Map<String, Value>,
    /// Surcharge la ligne `%% description:` de la fiche, comme pour un outil.
    /// Elle ne paraît dans aucun jeu d'outils — une réaction n'est pas
    /// appelable — mais elle se lit au journal et au reçu d'ouverture.
    #[serde(default)]
    pub description: Option<String>,
}

/// Les nœuds qu'une réaction peut traverser, et le refus qui dit quoi faire.
///
/// Trois formes de refus, parce qu'un refus utile nomme le remède et pas
/// seulement la règle (la leçon du constructeur de decks) : un nœud d'outil
/// à capacité (« déclare-le dans `tools`, avec sa politique »), un nœud
/// inconnu, et le cas tout court. La liste, elle, vit auprès de ses sœurs
/// dans `backend_code` : une liste de sécurité rangée loin des autres est une
/// liste que personne ne relira.
fn valider_noeuds_de_reaction(
    reaction: &str,
    node_types: impl IntoIterator<Item = String>,
) -> Result<(), String> {
    let permis = crate::backend_code::reaction_nodes();
    for node_type in node_types {
        if permis.contains(&node_type.as_str()) {
            continue;
        }
        return Err(match crate::backend_code::capacite_pour(&node_type) {
            Some(capacite) => format!(
                "la réaction « {reaction} » contient {node_type}, qui demande la capacité \
                 {capacite} : une réaction n'en a aucune — déclarez cette fiche dans \
                 \"tools\" avec \"policy\": {{\"{capacite}\": true}}, ou retirez le nœud"
            ),
            None => format!(
                "la réaction « {reaction} » contient {node_type}, qui n'est pas un nœud de \
                 graphe réactif"
            ),
        });
    }
    Ok(())
}

/// **Les relations qu'un graphe nomme**, pour un accès donné (lu ou écrit),
/// d'après la marque que chaque nœud pose sur ses paramètres
/// (`Choices::Relations { access }`) — jamais d'après un nom de type de nœud.
///
/// Une valeur de paramètre est un littéral, ou `$p` résolu par les liaisons du
/// manifeste puis par le défaut de la fiche. Rend `(noms, toute)` : `toute` est
/// vrai quand un paramètre reste ouvert (l'appelant choisira la relation à
/// l'appel) — un tel outil peut écrire n'importe quelle relation déclarée. Un
/// littéral vide ne nomme rien (« pas d'expansion » de la recherche de code).
fn relations_nommees(
    graphe: &crate::dataflow::GraphTool,
    bindings: &serde_json::Map<String, Value>,
    nodes: &NodeRegistry,
    garder: impl Fn(crate::dataflow::node_registry::RelationAccess) -> bool,
) -> (BTreeSet<String>, bool) {
    use crate::dataflow::node_registry::Choices;
    let mut noms = BTreeSet::new();
    let mut toute = false;
    for noeud in &graphe.template().nodes {
        let Some(schema) = nodes.schema(&noeud.node_type) else { continue };
        for cp in &schema.config_params {
            let Some(Choices::Relations { access }) = &cp.choices else { continue };
            if !garder(*access) {
                continue;
            }
            let brute = noeud
                .config
                .get(cp.name.as_ref())
                .cloned()
                .or_else(|| cp.default.clone());
            let Some(brute) = brute.as_ref().and_then(Value::as_str).map(str::to_string) else {
                continue;
            };
            let valeur = match brute.strip_prefix('$') {
                None => Some(brute),
                Some(p) => bindings
                    .get(p)
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        graphe
                            .params()
                            .iter()
                            .find(|q| q.name == p)
                            .and_then(|q| q.default.as_ref())
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    }),
            };
            match valeur {
                Some(v) if v.is_empty() => {}
                Some(v) => {
                    noms.insert(v);
                }
                None => toute = true,
            }
        }
    }
    (noms, toute)
}

/// **Tout ce qu'une réaction doit satisfaire pour être montée**, en une
/// fonction pure : la fiche est déjà lue et liée, il ne reste que les règles.
///
/// Elle est séparée du chargement exprès. Les quatre refus sont la vraie
/// valeur de cette section du manifeste, et un refus qu'on ne peut éprouver
/// qu'en fabriquant un manifeste sur le disque est un refus qu'on n'éprouve
/// pas — c'est ainsi qu'on finit par les découvrir à la première panne.
fn valider_une_reaction(
    nom: &str,
    graphe: &crate::dataflow::GraphTool,
    bindings: &serde_json::Map<String, Value>,
    est_aussi_un_outil: bool,
    scripted: &BTreeSet<String>,
) -> Result<(), String> {
    // 1. Un nom partagé entre les deux sections n'est pas une coquette
    //    ambiguïté : le curseur du réacteur **est** ce nom
    //    (`bus.cursor(sujet, nom)`), donc deux choses homonymes
    //    partageraient un curseur en silence.
    if est_aussi_un_outil {
        return Err(format!(
            "« {nom} » est déclaré à la fois dans \"tools\" et dans \"reactions\" : le \
             curseur du réacteur EST ce nom, donc les deux partageraient un curseur sans \
             le dire — renommez l'une des deux"
        ));
    }
    // 0. **Une réaction n'écoute pas les runs des réactions** (`reactions`) :
    //    elle se relancerait sur ses propres runs.
    crate::dataflow::reactor::refuser_le_sujet_des_reactions(nom, graphe.on())?;
    // 1 bis. **La clé du manifeste EST le curseur, donc elle doit être le nom
    //    de la fiche.** `Reactor::watch` ouvre `bus.cursor(sujet,
    //    tool.name())` — le nom de la FICHE, pas la clé — et la fiche nomme
    //    elle-même ce curseur dans son `EventSourceNode(cursor='…')`. Déclarée
    //    sous une autre clé, la réaction se lirait sous un nom et se vérifierait
    //    sous un autre : le refus homonyme ci-dessus porterait sur une clé que
    //    le bus ignore, et deux clés sur la même fiche partageraient un curseur
    //    en silence (le défaut que le point 1 prétend fermer).
    if graphe.name() != nom {
        return Err(format!(
            "la réaction « {nom} » porte la fiche « {} » : pour une réaction, la clé du \
             manifeste EST le curseur du réacteur, donc elle doit être le nom de la fiche — \
             renommez la clé en « {} », ou donnez-lui sa propre fiche",
            graphe.name(),
            graphe.name()
        ));
    }
    // 2. La liste blanche réactive.
    valider_noeuds_de_reaction(
        nom,
        graphe
            .template()
            .nodes
            .iter()
            .map(|n| n.node_type.clone())
            .filter(|t| !scripted.contains(t)),
    )?;
    // 3. Sans `%% on:`, le réacteur refuse déjà, mot pour mot — mais il
    //    refuserait au montage, loin du manifeste fautif. Ici le refus nomme
    //    la réaction et l'autre porte de sortie.
    if graphe.on().is_empty() {
        return Err(format!(
            "la réaction « {nom} » : rien à surveiller — sa fiche n'a pas de ligne \
             « %% on: <sujet> ». Ajoutez-la, ou déclarez ce graphe dans \"tools\" si c'est \
             l'agent qui doit l'appeler"
        ));
    }
    // 3 bis. **Les deux curseurs doivent porter le même nom.** Le réacteur
    //    ouvre le curseur du graphe (`bus.cursor(sujet, nom)`) au `watch`,
    //    AVANT ce qu'on veut observer ; le graphe le relit par le nom écrit
    //    dans son `EventSourceNode(cursor='…')`. Deux noms différents, et le
    //    curseur relu n'existe pas encore : il s'ouvre à la première
    //    exécution, donc il **manque tout ce qui a été émis avant** — une
    //    réaction qui dort silencieusement sur son premier lot. Vu le
    //    11 octobre 2026 sur une fiche de témoin (`cursor='r'` pour une
    //    réaction nommée `watch`), signalé par « everything declarative ».
    for noeud in &graphe.template().nodes {
        if noeud.node_type != "EventSourceNode" {
            continue;
        }
        let Some(curseur) = noeud.config.get("cursor").and_then(Value::as_str) else { continue };
        if curseur != nom {
            return Err(format!(
                "la réaction « {nom} » : son {} nomme le curseur « {curseur} », mais le \
                 réacteur ouvre « {nom} » (le curseur EST le nom de la réaction, et il est \
                 ouvert avant ce qu'on observe). Écrivez cursor='{nom}' — sinon la réaction \
                 manque tout ce qui a été émis avant sa première exécution",
                noeud.name
            ));
        }
    }
    for cle in bindings.keys() {
        if !graphe.params().iter().any(|p| p.name == *cle) {
            return Err(format!(
                "la réaction « {nom} » : liaison inconnue {cle} — sa fiche ne déclare pas ce \
                 paramètre"
            ));
        }
    }
    // 4. **Déterminée entièrement par ses liaisons**, ou refusée. Une
    //    réaction part sans personne pour compléter un argument : un
    //    paramètre obligatoire non lié n'est pas un défaut d'exécution à
    //    venir, c'est un manifeste faux maintenant.
    graphe.instantiate(&Value::Object(bindings.clone())).map_err(|e| {
        format!(
            "la réaction « {nom} » n'est pas déterminée par ses liaisons ({e}) : personne ne \
             fournira l'argument manquant à l'exécution — liez-le dans \"bindings\", ou \
             donnez-lui un défaut dans sa fiche"
        )
    })?;
    Ok(())
}

/// Une section au rendu d'un outil, depuis un petit graphe. Il ne peut ni
/// écrire, ni bloquer l'outil (toute erreur se journalise et se tait), ni
/// lancer quoi que ce soit ([`crate::backend_code::hook_nodes`]). Le silence
/// — section vide — est la conduite par défaut, et il s'observe : le journal
/// du backend (stderr) compte déclenché / tu / en erreur.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AfterToolHook {
    pub graph: PathBuf,
    /// Le titre de la section. Défaut : « À voir aussi ».
    #[serde(default = "titre_a_voir_aussi")]
    pub title: String,
    /// Le budget de lignes de la section ; au-delà, tronquée et avouée.
    #[serde(default = "douze_lignes")]
    pub max_lines: usize,
    /// Passé au gabarit s'il déclare `$threshold` — un seuil de similarité,
    /// calibré au banc comme les poids.
    #[serde(default)]
    pub threshold: Option<f64>,
    /// La base sans l'écriture, seule, sauf déclaration (`read_files` pour
    /// un crochet qui balaye). `run_commands` est ignoré : jamais dans un
    /// crochet.
    #[serde(default)]
    pub policy: crate::backend_code::ToolPolicy,
    /// **Se taire tant que les vecteurs de cette entité ne sont pas prêts.**
    /// Le client « motif ailleurs » cherche par similarité : pendant une
    /// indexation, il n'a rien d'honnête à dire — try_lock puis
    /// `index_state_for(entité)`, et tout ce qui n'est pas `Ready` (verrou
    /// occupé compris) est un silence, compté « tu ».
    #[serde(default)]
    pub silent_unless_vectors_ready: Option<String>,
    /// **Ce que l'outil a résolu, exposé au crochet** : le port de l'outil
    /// (ex. `{"node": "render", "port": "results"}`) dont les `uuid` des
    /// résultats sont passés au gabarit sous `$result_uuids` — s'il les
    /// déclare. Jamais de re-calcul : un crochet qui relancerait la
    /// recherche pour retrouver les identités en divergerait (règle de la
    /// session mémoire). Les pseudo-uuids du balayage (`scan:…`) sont
    /// écartés : ils ne désignent rien en base.
    #[serde(default)]
    pub results_port: Option<OutputPort>,
}

fn titre_a_voir_aussi() -> String {
    "À voir aussi".into()
}

fn douze_lignes() -> usize {
    12
}
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolHarness {
    pub input_schema: Option<PathBuf>,
    #[serde(default)]
    pub before: Vec<HookGraph>,
    #[serde(default)]
    pub after: Vec<HookGraph>,
    /// Post-acceptance transformations. Failure is delivery failure, not rollback.
    #[serde(default)]
    pub on_accept: Vec<HookGraph>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HookGraph {
    pub graph: PathBuf,
    #[serde(default)]
    pub data: BTreeMap<String, PathBuf>,
}
/// **La vérification du repli par branche** (5 octobre 2026) : un nœud de
/// signal dont le port `status` ne va à aucune fusion est une déclaration
/// acceptée qui ne fait pas ce qu'on croit — le repli reste DIT (les
/// warnings), mais un échec de TOUTES les branches rendra un résultat vide
/// au lieu d'une erreur. Ça se dit au chargement du gabarit, pas à la
/// première panne. Avertissement, jamais une erreur : un graphe à signal
/// seul, sans fusion, n'a rien à câbler.
fn statuts_non_cables(outil: &str, def: &crate::dataflow::GraphDefinition) -> Vec<String> {
    const SIGNAUX: [&str; 3] = ["VectorSearchNode", "BM25SearchNode", "SparseSearchNode"];
    let une_fusion = def.nodes.iter().any(|m| m.node_type == "FuseResultsNode");
    if !une_fusion {
        return vec![];
    }
    def.nodes
        .iter()
        .filter(|n| SIGNAUX.contains(&n.node_type.as_str()))
        .filter(|n| {
            !def.edges.iter().any(|e| {
                e.from_node == n.name
                    && e.from_port == "status"
                    && def
                        .nodes
                        .iter()
                        .any(|m| m.name == e.to_node && m.node_type == "FuseResultsNode")
            })
        })
        .map(|n| {
            format!(
                "{outil} : le port status de {} ne va à aucune fusion — un échec \
                 de toutes les branches rendra un résultat vide (status non câblé)",
                n.name
            )
        })
        .collect()
}

/// **Un refus dit la règle et la taille, jamais la donnée.** Le message de
/// `jsonschema` recopie la valeur fautive entière : 2 Mo de JSON pour 20 000
/// `records`, avant « has more than 512 items ». Au-delà de 200 caractères, la
/// valeur cède la place à sa forme : « un tableau de 20000 éléments ».
fn schema_refusal(e: &jsonschema::ValidationError<'_>) -> String {
    let message = e.to_string();
    let instance = e.instance().to_string();
    if instance.len() <= 200 {
        return message;
    }
    let forme = match e.instance().as_ref() {
        Value::Array(a) => format!("un tableau de {} éléments", a.len()),
        Value::Object(o) => format!("un objet de {} champs", o.len()),
        Value::String(t) => format!("une chaîne de {} caractères", t.chars().count()),
        autre => format!("une valeur de {} octets", autre.to_string().len()),
    };
    message.replacen(&instance, &forme, 1)
}

struct PreparedHook {
    tool: GraphTool,
    data: Value,
}
struct PreparedHarness {
    before: Vec<PreparedHook>,
    after: Vec<PreparedHook>,
    on_accept: Vec<PreparedHook>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InputPayload {
    pub entity: String,
    pub view: PayloadView,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadView {
    Identity,
    Editable,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputPort {
    pub node: String,
    pub port: String,
}

/// **Ce que l'hôte impose à un backend par-dessus son manifeste** (une tâche,
/// page 09) : sa base (la copie de la tâche) et son mode de déclarations
/// (`auto` : la tâche est l'espace de l'agent), et son workspace (la racine
/// dans le worktree de la tâche, écrivable ou non). Le manifeste n'est pas réécrit
/// — c'est une déclaration suivie, et une fusion l'emporterait ; chaque
/// rechargement réapplique les surcharges ([`PreparedBackend::recharger`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Surcharges {
    pub database: Option<PathBuf>,
    /// La racine des fichiers des entités (`files_root`), à la place de celle
    /// du manifeste : la vérification à blanc (lot M3c) relit une copie des
    /// dossiers, pour qu'aucune réaction ne réécrive les vrais.
    pub files_root: Option<PathBuf>,
    /// Ouvrir sans monter les réactions, le miroir ni les montres : la base
    /// temporaire de la vérification à blanc (lot M3c) ne doit rien déclencher
    /// (une réaction qui appelle un modèle, un miroir qui réécrit).
    pub sans_reactions: bool,
    pub declarations: Option<crate::propositions::ModeDeclarations>,
    /// La racine du workspace (absolue) et si la tâche y écrit : jamais la
    /// racine de l'arbre principal quand celle-ci est dans un dépôt git.
    pub workspace: Option<(PathBuf, bool)>,
}

/// Une base copiée sous la porte des écrivains
/// ([`Backend::copier_la_base_sous_la_porte`]).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CopieDeBase {
    /// Le fichier principal de la copie (ce que la tâche ouvre).
    pub base: PathBuf,
    /// Les fichiers copiés.
    pub fichiers: Vec<PathBuf>,
    /// Le reflink a pris pour tous ; sinon une copie pleine, de `octets`.
    pub reflink: bool,
    pub octets: u64,
    /// Combien de temps les écrivains ont attendu à la porte.
    pub tenue_ms: u64,
}

pub struct PreparedBackend {
    /// Le manifeste lu, pour le relire : un rechargement repart de lui.
    manifest_path: PathBuf,
    /// Ce que l'hôte impose par-dessus le manifeste ([`Surcharges`]).
    surcharges: Surcharges,
    /// Les copies locales de gabarits livrés qui en diffèrent
    /// ([`crate::builtin_templates::drift`]).
    template_drift: Vec<crate::builtin_templates::Drift>,
    /// Les types des nœuds scriptés du dossier `nodes/` (lot 2) : purs — ni
    /// fichiers, ni réseau, ni commande —, ils entrent dans toutes les
    /// politiques où entre un calcul.
    scripted_types: BTreeSet<String>,
    /// Les composants d'interface (`components/`) et leurs bibliothèques
    /// (`libs.json`), vérifiés au chargement comme les nœuds scriptés.
    components: Arc<crate::components::Components>,
    /// Les routes vérifiées et les vues du dossier `views/`.
    routes: crate::routes::Routes,
    pub manifest: BackendManifest,
    /// L'embarquement dense, résolu au chargement (`BackendManifest::embed_source`).
    embed: crate::model_source::ModelSource,
    pub directory: PathBuf,
    entities: HashMap<String, EntityConfig>,
    /// Les définitions nues des relations, sans leur politique d'écriture :
    /// ce que voient la recherche, le catalogue et la comparaison
    /// d'ouverture — une garde d'états se recharge, une relation non.
    relation_defs: HashMap<String, RelationDef>,
    mappings: HashMap<String, JsonSchemaMapping>,
    validators: HashMap<String, Arc<jsonschema::Validator>>,
    /// Les miroirs en fichiers, par entité qui déclare `files`.
    miroirs: HashMap<String, Arc<crate::fichiers_entite::Miroir>>,
    tools: BTreeMap<String, GraphTool>,
    /// Le registre des nœuds, en `Arc` : le chemin des outils le prête le
    /// temps d'un appel, le réacteur des réactions le garde le temps de son
    /// service.
    nodes: Arc<NodeRegistry>,
    tool_schemas: BTreeMap<String, Value>,
    tool_validators: HashMap<String, jsonschema::Validator>,
    harnesses: HashMap<String, PreparedHarness>,
    /// Les graphes des crochets après outil, chargés et validés au même
    /// moment que les outils qu'ils suivent.
    after_hooks: BTreeMap<String, GraphTool>,
    /// Les graphes réactifs, lus et liés au chargement. **Personne ne les
    /// monte encore** : `Reactor::watch` attend l'exécution asynchrone. Ils
    /// sont ici pour que le montage les prenne tels quels — et exposés par
    /// [`PreparedBackend::reaction_graphs`], parce qu'une déclaration que
    /// rien ne peut consulter est le défaut qu'on écrit en boucle.
    reactions: BTreeMap<String, GraphTool>,
    scripts: BTreeMap<String, String>,
    /// Les entités **décrites** que chaque outil vise (payloads, bindings,
    /// nœuds de sélection/recherche) : ce que `describe()` reprend.
    tool_entities: BTreeMap<String, BTreeSet<String>>,
}
/// Lire un fichier du manifeste en nommant le chemin dans l'erreur : un
/// « The system cannot find the path specified. (os error 3) » sans chemin
/// ne dit pas lequel des graphes ou des schémas manque (vu au premier
/// `--describe` sous Windows, 10 octobre 2026).
/// Sous Windows, `canonicalize` rend un chemin verbatim (`\\?\D:\…`), que le
/// noyau prend à la lettre : les `..` des graphes relatifs au gabarit
/// (`../../tools/edit.mmd`) n'y sont plus résolus, et le fichier « manque »
/// (treizième essai du paquet npm, 10 octobre 2026). On rend au dossier du
/// manifeste sa forme ordinaire (`D:\…`) ; ailleurs, rien ne change.
fn sans_prefixe_verbatim(chemin: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let mut composants = chemin.components();
        if let Some(Component::Prefix(prefixe)) = composants.next() {
            if let Prefix::VerbatimDisk(lettre) = prefixe.kind() {
                let mut ordinaire = PathBuf::from(format!("{}:\\", lettre as char));
                for c in composants {
                    match c {
                        Component::RootDir => {}
                        autre => ordinaire.push(autre.as_os_str()),
                    }
                }
                return ordinaire;
            }
        }
    }
    chemin
}
fn lire_octets(lecteur: &dyn crate::lecteur::Lecteur, chemin: &Path) -> Result<Vec<u8>, String> {
    lecteur.lire(chemin).map_err(|e| format!("{} : {e}", chemin.display()))
}
fn lire_texte(lecteur: &dyn crate::lecteur::Lecteur, chemin: &Path) -> Result<String, String> {
    lecteur.lire_texte(chemin).map_err(|e| format!("{} : {e}", chemin.display()))
}

impl PreparedBackend {
    /// Compile transport-independent search verbs without opening or querying a DB.
    pub fn compile_search_program(
        &self,
        program: &crate::dataflow::search_chain::SearchProgram,
    ) -> Result<crate::dataflow::search_chain::SearchPlan, String> {
        program.compile(
            &crate::dataflow::search_chain::SearchSchema {
                entities: &self.entities,
                relations: &self.relation_defs,
            },
            &self.nodes,
        )
    }

    /// **Les graphes réactifs déclarés, lus et validés**, par leur nom.
    ///
    /// Le montage les prendra ici — un par `Reactor::watch`, avec les
    /// liaisons de leur attachement (`manifest.reactions[nom].bindings`), que
    /// `run_tool` ne sait pas encore porter. Vide tant qu'aucun gabarit ne
    /// déclare de section `"reactions"`, ce qui est le cas aujourd'hui.
    pub fn reaction_graphs(&self) -> &BTreeMap<String, GraphTool> {
        &self.reactions
    }

    /// Les copies locales de gabarits livrés que ce manifeste désigne et qui en
    /// diffèrent.
    pub fn template_drift(&self) -> &[crate::builtin_templates::Drift] {
        &self.template_drift
    }

    /// Pure preparation: read and validate descriptors before opening a database.
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::load_avec(path, &crate::lecteur::Disque)
    }

    /// **Le chargement par un lecteur** : le disque ([`crate::lecteur::Disque`]), ou
    /// le disque sous une proposition en mémoire ([`crate::lecteur::Recouvrement`]).
    /// Toute déclaration se lit par lui : c'est ce qui rend vrai « valider sans
    /// écrire » (`Backend::valider_sur`).
    pub fn load_avec(path: &Path, lecteur: &dyn crate::lecteur::Lecteur) -> Result<Self, String> {
        let brut = lire_octets(lecteur, path)?;
        // **Les gabarits livrés désignés par leur nom** (`builtin:tools/list.mmd`,
        // `crate::builtin_templates`) : résolus vers le gabarit extrait du binaire
        // avant toute lecture. Sans `builtin:`, la lecture directe garde la ligne
        // et la colonne de ses erreurs.
        let manifest: BackendManifest = if brut.windows(crate::builtin_templates::PREFIX.len()).any(|w| w == crate::builtin_templates::PREFIX.as_bytes()) {
            let mut v: serde_json::Value = serde_json::from_slice(&brut).map_err(|e| e.to_string())?;
            crate::builtin_templates::resolve(&mut v)?;
            serde_json::from_value(v).map_err(|e| e.to_string())?
        } else {
            serde_json::from_slice(&brut).map_err(|e| e.to_string())?
        };
        if manifest.version != 1 {
            return Err("unsupported backend manifest version".into());
        }
        let embed = manifest.embed_source()?;
        if manifest.database.as_os_str().is_empty()
            || manifest.database.to_string_lossy() == ":memoire:"
        {
            return Err("backend requires a persistent database path".into());
        }
        let directory = sans_prefixe_verbatim(
            path.canonicalize()
                .map_err(|e| format!("{} : {e}", path.display()))?
                .parent()
                .unwrap()
                .to_path_buf(),
        );
        if let Some(w) = &manifest.workspace {
            if let Some(nom) = &w.index {
                if !crate::backend_code::KNOWN_INDEX_SCHEMAS.contains(&nom.as_str()) {
                    return Err(format!(
                        "workspace.index : schéma « {nom} » inconnu — les schémas connus : {}",
                        crate::backend_code::KNOWN_INDEX_SCHEMAS.join(", ")
                    ));
                }
            }
            // **Les signaux montés sur le schéma nommé se valident ici, dans
            // les deux sens** : un signal qui demande un modèle refuse sans
            // lui, et un modèle que rien ne lit refuse aussi.
            if !w.locks_via.is_empty() && w.index.as_deref() != Some("code") {
                return Err("workspace.locks_via sans workspace.index = \"code\" : les fonctions de \
                     verrou se lisent à l'ingestion du code — une déclaration que rien ne lit"
                    .to_string());
            }
            if let Some(q) = &w.queries_via {
                if w.index.as_deref() != Some("code") {
                    return Err("workspace.queries_via sans workspace.index = \"code\" : les tests par \
                         requêtes se relient à l'ingestion du code — une déclaration que rien ne lit"
                        .to_string());
                }
                // Une expression fautive se refuse au chargement, en la
                // nommant, pas à la première ingestion.
                #[cfg(feature = "code")]
                crate::queries_via::CompiledQueriesVia::compile(q)?;
                #[cfg(not(feature = "code"))]
                let _ = q;
            }
            if !w.index_signals.is_empty() && w.index.is_none() {
                return Err(
                    "workspace.index_signals sans workspace.index : les signaux se montent \
                     sur les entités d'un schéma nommé — déclarez l'index, ou retirez la clé"
                        .into(),
                );
            }
            let mut index_veut_creux = false;
            for (entite, signaux) in &w.index_signals {
                for signal in signaux {
                    match signal.as_str() {
                        "bm25" | "vector" => {}
                        "sparse" => index_veut_creux = true,
                        autre => {
                            return Err(format!(
                                "workspace.index_signals.{entite} : signal « {autre} » \
                                 inconnu (bm25, vector, sparse)"
                            ));
                        }
                    }
                }
            }
            // **`auto` sans bac à sable refuse** : la sentinelle peut accorder
            // librement, il faut que le noyau tienne — c'est la condition qui
            // rend ce mode honnête. Et le mode landlock se prouve au
            // chargement : un noyau qui ne sait pas le dit ici, pas au
            // milieu d'un tour d'agent.
            #[cfg(feature = "code")]
            {
                use crate::backend_code::{CommandGate, SandboxMode};
                if w.commands == CommandGate::Auto && w.sandbox.mode == SandboxMode::Off {
                    return Err(
                        "workspace.commands: \"auto\" exige un bac à sable — retirez \
                         \"sandbox\": {\"mode\": \"off\"}, ou passez en \"approval\""
                            .into(),
                    );
                }
                #[cfg(target_os = "linux")]
                if w.sandbox.mode == SandboxMode::Landlock {
                    if let Err(e) = crate::commande::BacASable::autour("/").ruleset() {
                        return Err(format!(
                            "le bac à sable Landlock ne se monte pas sur ce noyau ({e}) — \
                             déclarez \"sandbox\": {{\"mode\": \"off\"}} en le sachant"
                        ));
                    }
                }
                #[cfg(not(target_os = "linux"))]
                if w.sandbox.mode == SandboxMode::Landlock && w.commands != CommandGate::Off {
                    return Err(
                        "le bac à sable Landlock demande Linux — \"sandbox\": \
                         {\"mode\": \"off\"} en le sachant"
                            .into(),
                    );
                }
            }
            if index_veut_creux
                && !manifest.models.contains_key(&crate::model_source::Capability::Sparse)
            {
                return Err(
                    "workspace.index_signals déclare le signal sparse : `models.sparse` est \
                     requis (par exemple {\"provider\": \"service\", \"model\": \"bge-m3\"})"
                        .into(),
                );
            }
            if cfg!(not(feature = "code")) {
                return Err(
                    "la clé `workspace` demande un binaire bâti avec la feature `code` — \
                     rebâtir avec elle, ou retirer la clé du manifeste"
                        .into(),
                );
            }
            let root = if w.root.is_absolute() {
                w.root.clone()
            } else {
                directory.join(&w.root)
            };
            if !root.is_dir() {
                return Err(format!(
                    "workspace : la racine {} n'existe pas — donnez un dossier existant, relatif au manifeste ou absolu",
                    root.display()
                ));
            }
        }
        let (mut nodes, _) = builtin_graph_tools().map_err(|e| e.to_string())?;
        nodes.register(Box::new(crate::backend_nodes::EntityRecordFactory));
        nodes.register(Box::new(crate::backend_nodes::EntityBatchFactory));
        nodes.register(Box::new(crate::backend_nodes::SnapshotFinishFactory));
        nodes.register(Box::new(crate::backend_nodes::SnapshotUndoFactory));
        nodes.register(Box::new(crate::backend_nodes::SnapshotSessionFactory));
        nodes.register(Box::new(crate::backend_nodes::RelationBatchFactory));
        nodes.register(Box::new(crate::dataflow::plan_node::PlanNodeFactory));
        // ── Les nœuds scriptés du dossier `nodes/` ──────────────────────────
        //
        // Découverts, pas listés : un dossier de déclarations se copie tel
        // quel. Revérifiés à chaque chargement, donc à chaque rechargement.
        let mut scripted_types = BTreeSet::new();
        let declared = crate::dataflow::scripted_node::discover_avec(
            &directory.join("nodes"),
            &crate::script::ScriptLimits::default(),
            lecteur,
        )
        .map_err(|errors| format!("nodes/ : {}", errors.join(" ; ")))?;
        for factory in declared {
            let name = crate::dataflow::NodeFactory::node_type(&factory).to_string();
            if nodes.has(&name) {
                return Err(format!(
                    "nodes/ : le nœud déclaré « {name} » porte le nom d'un nœud fourni — \
                     renommez-le"
                ));
            }
            scripted_types.insert(name);
            nodes.register(Box::new(factory));
        }
        // ── Les composants d'interface et leurs bibliothèques ──────────────
        let components = Arc::new(
            crate::components::Components::load_in_avec(&directory, &ecran_du_backend(&manifest, path), lecteur)
                .map_err(|errors| format!("components/ : {}", errors.join(" ; ")))?,
        );
        let mut schemas = HashMap::new();
        let mut mappings = HashMap::new();
        let mut entities = HashMap::new();
        let mut validators = HashMap::new();
        let mut miroirs = HashMap::new();
        for (name, entity) in &manifest.entities {
            crate::schema::validate_identifier(name, "entity").map_err(|e| e.to_string())?;
            let schema: Value = serde_json::from_slice(
                &lire_octets(lecteur, &directory.join(&entity.schema))?,
            )
            .map_err(|e| e.to_string())?;
            validators.insert(
                name.clone(),
                Arc::new(jsonschema::validator_for(&schema).map_err(|e| e.to_string())?),
            );
            if let Some(decl) = &entity.files {
                let racine = manifest.files_root.as_ref().ok_or_else(|| {
                    format!("{name}.files : déclarez `files_root` (la racine des fichiers, relative au manifeste)")
                })?;
                miroirs.insert(
                    name.clone(),
                    Arc::new(crate::fichiers_entite::Miroir::nouveau_avec(name, directory.join(racine), decl.clone(), &schema, &directory, lecteur)?),
                );
            }
            let mapping = JsonSchemaMapping::from_schema(&schema)?;
            let mut config = entity.config.clone();
            for (field, declared) in &config.fields {
                let mapped = mapping
                    .fields
                    .get(field)
                    .ok_or_else(|| format!("{name}.{field}: missing from payload schema"))?;
                if mapped.field_type != declared.field_type {
                    return Err(format!("{name}.{field}: conflicting field type"));
                }
            }
            for (field, definition) in &mapping.fields {
                config
                    .fields
                    .entry(field.clone())
                    .or_insert_with(|| definition.clone());
            }
            for transition in &entity.writes.transition_dates {
                if !config.fields.contains_key(&transition.field) {
                    return Err(format!("unknown transition field {}", transition.field));
                }
            }
            let generated: Vec<&String> = [
                &entity.writes.created_at,
                &entity.writes.updated_at,
                &entity.writes.revision,
            ]
            .into_iter()
            .flatten()
            .chain(
                entity
                    .writes
                    .transition_dates
                    .iter()
                    .map(|t| &t.timestamp_field),
            )
            .collect();
            if generated
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != generated.len()
            {
                return Err("duplicate generated field".into());
            }
            for field in generated {
                let f = config
                    .fields
                    .get(field)
                    .ok_or_else(|| format!("generated field {field} missing"))?;
                if !matches!(
                    f.field_type,
                    crate::config::FieldType::Int64 | crate::config::FieldType::Integer
                ) {
                    return Err(format!("generated field {field} must be an integer"));
                }
            }
            // L'erreur dit de quelle entité elle vient : sans le nom, « Field 'x' »
            // ne se retrouvait pas dans un manifeste de plusieurs entités.
            config.validate().map_err(|e| format!("{name}: {e}"))?;
            // **Une entité sans champ de contenu se refuse ici, au chargement**,
            // et non plus à l'ouverture (`register_entity`), où le refus ne
            // disait ni le manifeste ni quoi écrire. Tout ce que le chargement
            // peut vérifier se refuse au chargement (10-11 octobre 2026).
            if !config.has_simple_pipeline() && !config.has_kb_participation() {
                return Err(format!(
                    "{name}: aucun champ de contenu — marquez au moins un champ `isContent: true` \
                     (ou `contentFor` / `titleFor` vers une base de connaissances) ; sans contenu, \
                     l'entité n'est ni cherchable ni découpée"
                ));
            }
            if config.hashsafe.as_ref().is_none_or(|v| v.is_empty()) {
                return Err(format!("{name}: stable hashsafe identity required"));
            }
            schemas.insert(name.clone(), schema);
            mappings.insert(name.clone(), mapping);
            entities.insert(name.clone(), config);
        }
        if let Some(ui) = &manifest.ui_state {
            if !entities.contains_key(&ui.entity) {
                return Err(format!(
                    "ui_state : l'entité « {} » n'est pas déclarée dans \"entities\" ; c'est elle qui garde les cases d'écran de portée hôte",
                    ui.entity
                ));
            }
        }
        for (name, relation) in &manifest.relations {
            crate::schema::validate_identifier(name, "relation").map_err(|e| e.to_string())?;
            if !entities.contains_key(&relation.from) || !entities.contains_key(&relation.to) {
                return Err(format!("{name}: unknown relation endpoint"));
            }
            // Les états permis d'un bout : il faut un cycle de vie, et des
            // états qui y existent — sinon la garde ne pourrait jamais passer,
            // ou jamais refuser.
            for (cle, entite, permis) in [
                ("from_states", &relation.from, &relation.from_states),
                ("to_states", &relation.to, &relation.to_states),
            ] {
                let Some(permis) = permis else { continue };
                let Some(cycle) = entities[entite].lifecycle.as_ref() else {
                    return Err(format!(
                        "relation « {name} » : {cle} sur « {entite} », qui n'a pas de cycle de vie — \
                         retirez {cle}, ou déclarez un lifecycle sur l'entité"
                    ));
                };
                let connus = cycle.states();
                if permis.is_empty() {
                    return Err(format!("relation « {name} » : {cle} est vide — aucun lien ne pourrait se poser"));
                }
                for etat in permis {
                    if !connus.contains(&etat.as_str()) {
                        return Err(format!(
                            "relation « {name} » : {cle} nomme « {etat} », qui n'est pas un état de \
                             « {entite} » (états : {})",
                            connus.join(", ")
                        ));
                    }
                }
            }
        }
        let scripts = manifest
            .scripts
            .iter()
            .map(|(name, path)| {
                let text =
                    lire_texte(lecteur, &directory.join(path))?;
                if text.len() > 65536 {
                    return Err(format!("script {name} too large"));
                }
                Ok((name.clone(), text))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let prepare_hooks = |hooks: &[HookGraph]| -> Result<Vec<PreparedHook>, String> {
            hooks
                .iter()
                .map(|hook| {
                    let source = lire_texte(lecteur, &directory.join(&hook.graph))?;
                    let tool = GraphTool::from_mermaid(&source)
                        .and_then(|t| t.bind(&nodes))
                        .map_err(|e| e.to_string())?;
                    if tool.params().len() != 1 || tool.params()[0].name != "context" {
                        return Err("hook graph must declare exactly one context parameter".into());
                    }
                    let data = hook
                        .data
                        .iter()
                        .map(|(key, path)| {
                            let bytes =
                                lire_octets(lecteur, &directory.join(path))?;
                            Ok((
                                key.clone(),
                                serde_json::from_slice::<Value>(&bytes)
                                    .map_err(|e| e.to_string())?,
                            ))
                        })
                        .collect::<Result<serde_json::Map<_, _>, String>>()?;
                    Ok(PreparedHook {
                        tool,
                        data: Value::Object(data),
                    })
                })
                .collect()
        };
        let mut harnesses = HashMap::new();
        // Le schéma contre lequel un Plan déclaré se vérifie : les champs de
        // chaque entité (`_uuid` compris), les deux bouts de chaque relation.
        let schema_des_plans = rag3weaver_ir::PlanSchema {
            tables: entities
                .iter()
                .map(|(nom, e): (&String, &EntityConfig)| {
                    (nom.clone(), e.fields.keys().cloned().chain(["_uuid".to_string()]).collect())
                })
                .collect(),
            relations: manifest.relations.iter().map(|(nom, r)| (nom.clone(), (r.def.from.clone(), r.def.to.clone()))).collect(),
        };
        let mut after_hooks = BTreeMap::new();
        let mut tools = BTreeMap::new();
        let mut tool_schemas = BTreeMap::new();
        let mut tool_validators = HashMap::new();
        for (name, attachment) in &manifest.tools {
            crate::schema::validate_identifier(name, "tool").map_err(|e| e.to_string())?;
            if manifest.search_graphs && SEARCH_TOOLS.contains(&name.as_str()) {
                return Err(format!("reserved search tool {name}"));
            }
            if manifest.graph_tool && name == GRAPH_TOOL {
                return Err(format!("reserved tool {name} (graph_tool)"));
            }
            let source = lire_texte(lecteur, &directory.join(&attachment.graph))?;
            let mut tool = GraphTool::from_mermaid(&source)
                .and_then(|t| t.bind(&nodes))
                .map_err(|e| e.to_string())?;
            // **Les Plans déclarés** (`PlanNode(plan=@plans/…)`) : lus et vérifiés
            // ici, contre leur forme et contre le schéma du manifeste ; un Plan
            // faux refuse le gabarit.
            let plans: Vec<(String, String)> = tool
                .template()
                .nodes
                .iter()
                .filter(|n| n.node_type == crate::dataflow::plan_node::NODE_TYPE)
                .filter_map(|n| n.config.get("plan").and_then(Value::as_str).map(|c| (n.name.clone(), c.to_string())))
                .collect();
            for (noeud, cite) in plans {
                let lu = crate::dataflow::plan_node::lire_le_plan(
                    &cite,
                    |chemin| lire_texte(lecteur, &directory.join(chemin)),
                    &schema_des_plans,
                )
                .map_err(|e| format!("{name}: {e}"))?;
                tool.remplacer_config(&noeud, "plan", lu);
            }
            for key in attachment.bindings.keys() {
                if !tool.params().iter().any(|p| p.name == key.as_str()) {
                    return Err(format!("{name}: unknown binding {key}"));
                }
            }
            for avertissement in statuts_non_cables(name, tool.template()) {
                eprintln!("[backend] {avertissement}");
            }
            if let Some(hook) = &attachment.after {
                // Le graphe du crochet se charge et se valide ICI : un
                // crochet mal déclaré est une erreur de manifeste, pas un
                // silence — le silence couvre l'exécution, jamais le montage.
                let source = lire_texte(lecteur, &directory.join(&hook.graph))
                    .map_err(|e| format!("{name}: crochet after : {e}"))?;
                let hook_tool = GraphTool::from_mermaid(&source)
                    .and_then(|t| t.bind(&nodes))
                    .map_err(|e| format!("{name}: crochet after : {e}"))?;
                let permis = crate::backend_code::hook_nodes(&hook.policy);
                for node in &hook_tool.template().nodes {
                    if !permis.contains(&node.node_type.as_str())
                        && !scripted_types.contains(&node.node_type)
                    {
                        return Err(format!(
                            "{name}: crochet after : le nœud {} n'entre pas dans un \
                             crochet — un crochet n'écrit pas, ne bloque pas, ne lance \
                             rien ; pour lire des fichiers, ajoutez \"policy\": \
                             {{\"read_files\": true}} au crochet",
                            node.node_type
                        ));
                    }
                }
                if hook.max_lines == 0 {
                    return Err(format!("{name}: crochet after : max_lines doit être au moins 1"));
                }
                let veut_uuids = hook_tool.params().iter().any(|p| p.name == "result_uuids");
                if veut_uuids && hook.results_port.is_none() {
                    return Err(format!(
                        "{name}: crochet after : le gabarit déclare result_uuids — dites d'où \
                         ils viennent : \"results_port\": {{\"node\": \"render\", \
                         \"port\": \"results\"}}"
                    ));
                }
                if !veut_uuids && hook.results_port.is_some() {
                    return Err(format!(
                        "{name}: crochet after : results_port déclaré mais le gabarit ne \
                         demande pas result_uuids — retirez l'un ou ajoutez l'autre"
                    ));
                }
                after_hooks.insert(name.clone(), hook_tool);
            }
            let mut input = tool.tool_def().parameters;
            for (parameter, payload) in &attachment.input_payloads {
                if attachment.bindings.contains_key(parameter)
                    || input["properties"].get(parameter).is_none()
                {
                    return Err(format!(
                        "{name}: payload parameter {parameter} absent or bound"
                    ));
                }
                let schema = schemas
                    .get(&payload.entity)
                    .ok_or_else(|| format!("unknown payload entity {}", payload.entity))?;
                let entity = &manifest.entities[&payload.entity];
                input["properties"][parameter] = payload_schema(schema, entity, &payload.view)?;
            }
            if let Some(props) = input.get_mut("properties").and_then(Value::as_object_mut) {
                for key in attachment.bindings.keys() {
                    props.remove(key);
                }
            }
            if let Some(required) = input.get_mut("required").and_then(Value::as_array_mut) {
                required.retain(|v| {
                    v.as_str()
                        .is_none_or(|k| !attachment.bindings.contains_key(k))
                });
            }
            // **Les filtres et options de recherche, typés depuis l'entité.** Le
            // graphe dit quel paramètre alimente un filtre ou des options, et
            // sur quelle entité (littérale ou liée) : le schéma, la grammaire,
            // les champs, leur vocabulaire et un exemple en découlent, pour
            // n'importe quel backend. Le validateur plus bas les applique.
            for node in &tool.template().nodes {
                let (cle_param, cle_entite, options) = match node.node_type.as_str() {
                    "SelectRecordsNode" => ("filter", "entity", false),
                    "SearchSourceNode" => ("options", "target_name", true),
                    _ => continue,
                };
                let Some(param) = node.config.get(cle_param).and_then(Value::as_str).and_then(|v| v.strip_prefix('$')) else {
                    continue;
                };
                if input["properties"].get(param).is_none() {
                    continue;
                }
                let entite = match node.config.get(cle_entite).and_then(Value::as_str) {
                    Some(v) => match v.strip_prefix('$') {
                        Some(lie) => attachment.bindings.get(lie).and_then(Value::as_str).map(str::to_string),
                        None => Some(v.to_string()),
                    },
                    None => None,
                };
                let Some(entite) = entite else {
                    continue;
                };
                let Some(mapping) = mappings.get(&entite) else {
                    continue;
                };
                let mut description = crate::json_schema::filter_description(&mapping.filter_fields);
                if let Some(d) = manifest.entities.get(&entite).and_then(|b| b.description.as_deref()) {
                    description = format!("{d} — {description}");
                }
                input["properties"][param] = if options {
                    crate::json_schema::search_options_schema(&description)
                } else {
                    let mut filtre = crate::json_schema::filter_condition_schema();
                    filtre["description"] = Value::String(description);
                    filtre["default"] = json!({});
                    filtre
                };
            }
            if let Some(path) = &attachment.harness.input_schema {
                input = serde_json::from_slice(
                    &lire_octets(lecteur, &directory.join(path))?,
                )
                .map_err(|e| e.to_string())?;
                let declared = tool.tool_def().parameters;
                let props = input["properties"]
                    .as_object()
                    .ok_or("harness schema requires root properties")?;
                if props.keys().any(|k| {
                    declared["properties"].get(k).is_none() || attachment.bindings.contains_key(k)
                }) {
                    return Err("harness schema cannot expose unknown or bound parameters".into());
                }
            }
            harnesses.insert(
                name.clone(),
                PreparedHarness {
                    before: prepare_hooks(&attachment.harness.before)?,
                    after: prepare_hooks(&attachment.harness.after)?,
                    on_accept: prepare_hooks(&attachment.harness.on_accept)?,
                },
            );
            input["additionalProperties"] = json!(false);
            tool_validators.insert(
                name.clone(),
                jsonschema::validator_for(&input).map_err(|e| e.to_string())?,
            );
            tool_schemas.insert(name.clone(), input);
            tools.insert(name.clone(), tool);
        }
        // La politique de chaque outil se vérifie au chargement : un graphe
        // qui déborde sa politique, ou une politique sans le workspace
        // qu'elle exige, se refuse en disant quoi faire.
        for (name, attachment) in &manifest.tools {
            if let Some(tool) = tools.get(name) {
                crate::backend_code::validate_tool_policy(
                    name,
                    tool.template()
                        .nodes
                        .iter()
                        .map(|n| n.node_type.clone())
                        .filter(|t| !scripted_types.contains(t)),
                    &attachment.policy,
                    manifest.workspace.as_ref(),
                )?;
            }
        }
        // ── Les routes et les vues ──────────────────────────────────────
        let routes = crate::routes::Routes::load_avec(&manifest.routes, &directory, &|name| {
            tools.contains_key(name)
        }, lecteur)?;
        // ── Les graphes réactifs ─────────────────────────────────────────
        //
        // Lus, construits et liés ici, au même étage que les outils : un
        // manifeste invalide se refuse au **chargement**, jamais à la
        // première panne. Ce que le montage fera de ces graphes n'est pas
        // écrit ici — `Reactor::watch` n'a encore aucun appelant dans tout
        // `src/`, et il attend l'exécution asynchrone.
        //
        // **Deux faits du réacteur que le montage devra prendre en compte**,
        // relevés en lisant `dataflow/reactor.rs` et écrits ici parce qu'un
        // manifeste validé doit exécuter le graphe qu'il déclare :
        //
        // 1. `run_tool` instancie avec `json!({})` — les liaisons déclarées
        //    ici ne l'atteignent pas encore. C'est pourquoi on vérifie plus
        //    bas que la fiche s'instancie **avec ses liaisons** : sans ce
        //    passage, une réaction liée partirait avec d'autres valeurs que
        //    celles du manifeste, ou pas du tout ;
        // 2. il exécute sous `NodeTypePolicy::All`. La liste blanche
        //    ci-dessous est donc **le seul** garde-fou, et il est au
        //    chargement. À dire au montage plutôt qu'à découvrir.
        let mut reactions = BTreeMap::new();
        for (name, attachment) in &manifest.reactions {
            crate::schema::validate_identifier(name, "reaction").map_err(|e| e.to_string())?;
            let source = lecteur.lire_texte(&directory.join(&attachment.graph))
                .map_err(|e| format!("la réaction « {name} » : {e}"))?;
            let graphe = GraphTool::from_mermaid(&source)
                .and_then(|t| t.bind(&nodes))
                .map_err(|e| format!("la réaction « {name} » : {e}"))?;
            valider_une_reaction(
                name,
                &graphe,
                &attachment.bindings,
                manifest.tools.contains_key(name),
                &scripted_types,
            )?;
            reactions.insert(name.clone(), graphe);
        }
        // 8. **Une réaction qui remonte une relation qu'aucun outil n'écrit**
        //    tournerait à chaque événement sans jamais rien trouver — c'est
        //    ainsi que `review_anchored` a vécu dans le gabarit `memory` jusqu'au
        //    11 octobre, sans un mot. Lu sur le manifeste seul : les relations
        //    que les réactions lisent contre celles que les outils écrivent,
        //    d'après la marque des paramètres des nœuds. Un outil dont la
        //    relation reste ouverte (l'appelant la nomme) les écrit toutes.
        let mut ecrites: BTreeSet<String> = BTreeSet::new();
        let mut toutes_ecrites = false;
        for (nom, outil) in &tools {
            let (noms, toute) = relations_nommees(outil, &manifest.tools[nom].bindings, &nodes, |a| a.write);
            ecrites.extend(noms);
            toutes_ecrites |= toute;
        }
        if !toutes_ecrites {
            for (nom, graphe) in &reactions {
                let (lues, _) =
                    relations_nommees(graphe, &manifest.reactions[nom].bindings, &nodes, |a| a.read);
                if let Some(r) = lues.iter().find(|r| !ecrites.contains(*r)) {
                    return Err(format!(
                        "la réaction « {nom} » remonte la relation « {r} », qu'aucun outil du \
                         manifeste n'écrit : elle tournerait sans jamais rien trouver — déclarez un \
                         outil qui la pose (`templates/tools/link.mmd`, `relation` liée à « {r} »)"
                    ));
                }
            }
        }
        // Les entités décrites que chaque outil vise : par ses payloads, par
        // un binding qui nomme une entité, ou par ses nœuds de sélection et
        // de recherche (entité littérale ou liée). `describe()` les reprend.
        let mut tool_entities: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (name, attachment) in &manifest.tools {
            let mut vues: BTreeSet<String> = attachment
                .input_payloads
                .values()
                .map(|p| p.entity.clone())
                .collect();
            for v in attachment.bindings.values() {
                if let Some(e) = v.as_str() {
                    if manifest.entities.contains_key(e) {
                        vues.insert(e.to_string());
                    }
                }
            }
            if let Some(tool) = tools.get(name) {
                for node in &tool.template().nodes {
                    let cle = match node.node_type.as_str() {
                        "SelectRecordsNode" => "entity",
                        "SearchSourceNode" => "target_name",
                        _ => continue,
                    };
                    let entite = match node.config.get(cle).and_then(Value::as_str) {
                        Some(v) => match v.strip_prefix('$') {
                            Some(lie) => attachment.bindings.get(lie).and_then(Value::as_str).map(str::to_string),
                            None => Some(v.to_string()),
                        },
                        None => None,
                    };
                    if let Some(e) = entite {
                        if manifest.entities.contains_key(&e) {
                            vues.insert(e);
                        }
                    }
                }
            }
            vues.retain(|e| manifest.entities[e].description.is_some());
            if !vues.is_empty() {
                tool_entities.insert(name.clone(), vues);
            }
        }
        // Les copies locales qui diffèrent du gabarit livré du même nom : dites à
        // l'ouverture (l'état de l'hôte, la page), pas découvertes à une erreur
        // d'argument plus loin.
        let template_drift = serde_json::from_slice::<serde_json::Value>(&brut)
            .map(|v| crate::builtin_templates::drift_avec(&directory, &v, lecteur))
            .unwrap_or_default();
        Ok(Self {
            template_drift,
            manifest_path: path.canonicalize().map_err(|e| e.to_string())?,
            surcharges: Surcharges::default(),
            scripted_types,
            components,
            routes,
            relation_defs: manifest
                .relations
                .iter()
                .map(|(k, v)| (k.clone(), v.def.clone()))
                .collect(),
            manifest,
            embed,
            directory,
            entities,
            mappings,
            validators,
            miroirs,
            tools,
            after_hooks,
            reactions,
            nodes: Arc::new(nodes),
            tool_schemas,
            tool_validators,
            harnesses,
            scripts,
            tool_entities,
        })
    }
    /// Ce backend a-t-il besoin d'un service d'embarquement ? Oui dès
    /// qu'une entité déclare un signal vecteur ou sparse, ou qu'un
    /// workspace nomme le schéma de code (dont les scopes s'embarquent).
    /// Un backend en mots seuls démarre sans service — et son banc tourne
    /// sans carte.
    pub fn needs_embeddings(&self) -> bool {
        if self
            .manifest
            .workspace
            .as_ref()
            .and_then(|w| w.index.as_deref())
            .is_some()
        {
            return true;
        }
        self.entities
            .values()
            .any(|config| config.signals.vector() || config.signals.sparse())
    }

    /// Les composants d'interface et leurs bibliothèques, vérifiés.
    pub fn components(&self) -> Arc<crate::components::Components> {
        self.components.clone()
    }

    /// **Les composants tels qu'ils sont sur le disque**, relus et vérifiés
    /// contre le même écran qu'à l'ouverture : l'aperçu de l'IDE montre le
    /// disque, une déclaration éditée depuis l'ouverture comprise.
    pub fn components_on_disk(&self) -> Result<crate::components::Components, String> {
        self.components_on_disk_avec(&crate::lecteur::Disque)
    }

    /// [`Self::components_on_disk`], par un lecteur (une proposition).
    pub fn components_on_disk_avec(&self, lecteur: &dyn crate::lecteur::Lecteur) -> Result<crate::components::Components, String> {
        crate::components::Components::load_in_avec(&self.directory, &ecran_du_backend(&self.manifest, &self.manifest_path), lecteur)
            .map_err(|errors| format!("components/ : {}", errors.join(" ; ")))
    }

    pub fn path(&self, path: &Path) -> PathBuf {
        self.directory.join(path)
    }

    /// Le chemin canonique du manifeste chargé.
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// **Impose des surcharges** : la base et le mode de déclarations
    /// remplacent ceux du manifeste, et les rechargements les gardent.
    pub fn avec_surcharges(mut self, surcharges: Surcharges) -> Self {
        if let Some(base) = &surcharges.database {
            self.manifest.database = base.clone();
        }
        if let Some(mode) = surcharges.declarations {
            self.manifest.declarations = mode;
        }
        if let Some(racine) = &surcharges.files_root {
            self.miroirs = self
                .miroirs
                .iter()
                .map(|(e, m)| {
                    let mut m = (**m).clone();
                    m.racine = racine.clone();
                    (e.clone(), Arc::new(m))
                })
                .collect();
        }
        if let (Some((racine, ecrit)), Some(w)) = (&surcharges.workspace, self.manifest.workspace.as_mut()) {
            w.root = racine.clone();
            w.read_only = !ecrit;
        }
        self.surcharges = surcharges;
        self
    }

    /// Les surcharges imposées à ce backend.
    pub fn surcharges(&self) -> &Surcharges {
        &self.surcharges
    }

    /// **Recharger le même manifeste par un lecteur**, avec les mêmes
    /// surcharges : ce que font le rechargement (le disque) et la validation
    /// d'une proposition (un recouvrement).
    pub fn recharger(&self, lecteur: &dyn crate::lecteur::Lecteur) -> Result<PreparedBackend, String> {
        Ok(Self::load_avec(&self.manifest_path, lecteur)?.avec_surcharges(self.surcharges.clone()))
    }

    /// **Le fichier de l'extension vectorielle à charger.** Un chemin déclaré
    /// est un forçage, relatif au manifeste. Un nom (`vector`) est celle du
    /// bâti, cherchée là où le moteur se trouve (voir
    /// [`resolve_vector_extension`]). Un gabarit livré ne la cherche jamais
    /// relativement à lui-même (ticket du 11 octobre 2026 : copié ailleurs, il
    /// ne s'ouvrait plus).
    pub fn vector_extension_path(&self) -> Result<PathBuf, String> {
        resolve_vector_extension(
            &self.manifest.vector_extension,
            &self.directory,
            crate::build_check::loaded_lib().as_deref(),
            std::env::var_os("RAG3DB_ROOT"),
            std::env::var_os("RAG3DB_LIBRARY_DIR"),
            std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)),
        )
    }
    pub fn describe(&self) -> Value {
        let mut tools:Vec<Value>=self.tools.iter().map(|(name,t)|{
            // La surcharge du manifeste, sinon la ligne du gabarit ; puis la
            // description de chaque entité visée — c'est elle qui distingue
            // deux outils de même forme, et l'agent choisit sur elle.
            let attachment = &self.manifest.tools[name];
            let mut description = attachment
                .description
                .clone()
                .unwrap_or_else(|| t.tool_def().description);
            if let Some(vues) = self.tool_entities.get(name) {
                for e in vues {
                    if let Some(d) = self.manifest.entities[e].description.as_deref() {
                        description.push_str(&format!(" Cible « {e} » : {d}"));
                    }
                }
            }
            json!({"name":name,"description":description,"inputSchema":self.tool_schemas[name]})
        }).collect();
        if self.manifest.search_graphs {
            tools.extend(search_tool_definitions());
        }
        if self.manifest.graph_tool {
            tools.push(graph_tool_definition());
        }
        json!({"name":self.manifest.name,"version":self.manifest.version,"database":self.path(&self.manifest.database),"fts":"lucivy","embeddings":self.embed,"payloads":self.mappings,"tools":tools,"capabilities":["journal","journal_read","index_state","verify_files"]})
    }
    /// La déclaration de l'embarquement dense de ce backend.
    pub fn embed_source(&self) -> &crate::model_source::ModelSource {
        &self.embed
    }
    fn embed_dimensions(&self) -> usize {
        self.embed.dimensions.unwrap_or(0)
    }
    /// L'embarqueur que la déclaration désigne, et d'où il calcule.
    pub fn connect_embedder(&self) -> Result<(Box<dyn Embedder>, crate::model_source::Origin), String> {
        crate::model_source::connect_embedder(&self.embed)
    }
    /// **L'embarqueur de toutes les portes** (lignes JSON, `mcp`, `serve`,
    /// l'hôte du démon) : celui de la déclaration s'il répond ; sinon
    /// `None` **et la raison**, que [`Self::open_with_reason`] met dans le
    /// reçu avec l'embarqueur absent (plein texte seul, vecteurs en dette).
    /// La raison dit quoi faire, et ce n'est pas la même chose selon le
    /// cas : rien de déclaré (le donner), un service déclaré mais pas joint
    /// (le démarrer), un modèle local indisponible. Un backend en mots seuls
    /// n'en demande pas : `(None, None)`. Une identité de modèle différente
    /// reste un refus, dans `open`. Une seule méthode : le 11 octobre 2026,
    /// trois portes sur quatre refusaient encore un backend vectoriel sans
    /// service (le témoin « MCP de zéro » de l'archive alpha.3).
    pub fn embedder_or_absent(&self) -> (Option<Box<dyn Embedder>>, Option<String>) {
        if !self.needs_embeddings() {
            return (None, None);
        }
        match self.connect_embedder() {
            Ok((embedder, _)) => (Some(embedder), None),
            Err(erreur) => (None, Some(self.raison_sans_service(&erreur))),
        }
    }
    /// La raison du manque, et quoi faire, d'après la déclaration.
    fn raison_sans_service(&self, erreur: &str) -> String {
        use crate::model_source::{Capability, Provider};
        let adresses = self.embed.addresses(Capability::Embed, &|k| std::env::var(k).ok());
        match self.embed.provider {
            Provider::Local => format!("modèle local {} indisponible : {erreur}", self.embed.model),
            _ if adresses.is_empty() => format!(
                "aucun service déclaré — donnez models.embed.address (ou embeddings.address), ou \
                 RAG3WEAVER_SERVICE_EMBED (alias RAG3WEAVER_EMBED_SERVICE)"
            ),
            _ => format!(
                "service déclaré mais pas joint ({}) — démarrez-le ; l'index rattrapera la dette \
                 à la réouverture : {erreur}",
                adresses.join(", ")
            ),
        }
    }
    /// Ce que le backend dit quand il déclare du vecteur et démarre sans
    /// service d'embarquement — la même phrase sur stderr et dans le reçu.
    /// La phrase de fond est celle du catalogue
    /// (`AVERTISSEMENT_EMBARQUEUR_ABSENT`) ; s'y ajoute la raison, ou à
    /// défaut quoi faire.
    pub const AVERTISSEMENT_SANS_SERVICE: &str = "pas de service d'embarquement : index en \
        plein texte seul, vecteurs en dette (ce backend déclare des signaux vecteur ou sparse, \
        ou un workspace indexé)";
    const QUOI_FAIRE_SANS_SERVICE: &str = "pour les calculer, donnez models.embed.address (ou \
        embeddings.address), ou RAG3WEAVER_SERVICE_EMBED (alias RAG3WEAVER_EMBED_SERVICE)";

    pub fn open(
        self,
        conn: Box<dyn DbConnection>,
        embedder: Option<Box<dyn Embedder>>,
    ) -> Result<Backend, String> {
        self.open_with_reason(conn, embedder, None)
    }

    /// [`Self::open`], avec la raison du manque d'embarqueur quand la porte
    /// la connaît ([`Self::embedder_or_absent`]) : elle va dans le reçu.
    pub fn open_with_reason(
        self,
        conn: Box<dyn DbConnection>,
        embedder: Option<Box<dyn Embedder>>,
        raison: Option<String>,
    ) -> Result<Backend, String> {
        // Un backend en mots seuls démarre sans service d'embarquement. Un
        // backend qui déclare du vecteur démarre aussi (décision du
        // 10 octobre 2026, pour le paquet npm) : il le dit en le nommant —
        // sur stderr, et dans son reçu (`describe`, `index_state`), parce
        // que stderr n'est lu par personne quand il tourne en service. Le
        // plein texte indexe, les vecteurs restent en dette, rattrapée quand
        // un service apparaît : l'embarqueur absent ne pose aucun vecteur.
        // Toutes les portes passent par `embedder_or_absent` (11 octobre
        // 2026 : l'hôte du démon, `mcp` autonome et `serve` refusaient encore).
        let sans_service = embedder.is_none();
        let avertissement_sans_service = if sans_service && self.needs_embeddings() {
            let texte = format!(
                "{} — {}",
                Self::AVERTISSEMENT_SANS_SERVICE,
                raison.as_deref().unwrap_or(Self::QUOI_FAIRE_SANS_SERVICE)
            );
            eprintln!("[backend] {texte}");
            Some(texte)
        } else {
            None
        };
        let embedder = match embedder {
            Some(e) => {
                if e.is_mock()
                    || e.name() != self.embed.model
                    || e.dim() != self.embed_dimensions()
                {
                    return Err("embedding model identity differs from manifest".into());
                }
                e
            }
            // Sans service : l'embarqueur **absent** (arbre principal,
            // 10 octobre 2026), jamais un factice — il porte le nom et la
            // dimension du modèle attendu, n'écrit aucun vecteur, laisse la
            // dette se poser contre le vrai modèle, et la recherche dense se
            // replie en le disant.
            None => Box::new(crate::AbsentEmbedder::new(
                self.embed.model.clone(),
                self.embed_dimensions(),
            )),
        };
        let mut cat = Catalog::new(
            conn,
            embedder,
            CatalogConfig {
                name: Some(self.manifest.name.clone()),
                embedding_dim: self.embed_dimensions(),
                checkpoint_dir: Some(
                    self.path(&self.manifest.database)
                        .with_extension("checkpoints"),
                ),
                ..Default::default()
            },
        );
        cat.set_moteur_texte(MoteurTexte::Lucivy);
        cat.set_fts_positions(self.manifest.fts_positions.unwrap_or(true));
        // **Le creux, enfin branché.** Le backend comptait le signal `sparse`
        // pour exiger un service, puis ne donnait jamais d'embarqueur creux
        // au catalogue (trouvé le 3 octobre 2026) : le signal restait muet.
        // Déclaré sans `models.sparse`, c'est maintenant un refus qui dit
        // quoi écrire ; et le dual n'est pris que si c'est le même modèle des
        // deux côtés — sinon dense et creux viendraient de deux espaces.
        // Le relecteur et l'OCR : les nœuds existaient (`RerankNode`,
        // `OcrNode`), le backend ne leur donnait aucun service. Déclarés, ils
        // sont joints ici ; non déclarés, rien ne change.
        if let Some(source) = self.manifest.models.get(&crate::model_source::Capability::Rerank) {
            cat.set_reranker(crate::model_source::connect_reranker(source)?.0);
        }
        if let Some(source) = self.manifest.models.get(&crate::model_source::Capability::Ocr) {
            cat.set_ocr(crate::model_source::connect_ocr(source)?.0);
        }
        let veut_le_creux = self.entities.values().any(|config| config.signals.sparse())
            || self
                .manifest
                .workspace
                .as_ref()
                .is_some_and(|w| w.index_signals.values().flatten().any(|s| s == "sparse"));
        // Le sens inverse : un modèle creux que rien ne lit est une
        // déclaration morte — la famille « écrire ce que rien ne lira ».
        if self.manifest.models.contains_key(&crate::model_source::Capability::Sparse)
            && !veut_le_creux
        {
            return Err(
                "models.sparse est déclaré mais aucun signal sparse ne le lit — montez-le \
                 sur une entité (signals) ou sur le schéma nommé (workspace.index_signals)"
                    .into(),
            );
        }
        match (self.manifest.models.get(&crate::model_source::Capability::Sparse), veut_le_creux) {
            (None, true) if !sans_service => {
                return Err("ce backend déclare un signal sparse : `models.sparse` est requis \
                            (par exemple {\"provider\": \"service\", \"model\": \"bge-m3\"})"
                    .into());
            }
            (Some(source), _) => {
                let (clients, _) = crate::model_source::connect_sparse(source)?;
                cat.set_sparse_embedder(clients.sparse);
                if let Some(dual) = clients.dual.filter(|_| source.model == self.embed.model) {
                    cat.set_dual_embedder(dual);
                }
            }
            (None, _) => {}
        }
        // Le chemin cité pour Cypher (barres obliques inverses et apostrophes
        // échappées) : un `D:\a\…` nu cassait la requête sous Windows.
        let ext = self.vector_extension_path()?;
        cat.execute_raw(&format!(
            "LOAD EXTENSION {}",
            crate::connection::cypher_path_literal(&ext)
        ))
        .map_err(|e| e.to_string())?;
        cat.initialize().map_err(|e| e.to_string())?;
        for (name, config) in &self.entities {
            cat.register_entity(name, config.clone())
                .map_err(|e| e.to_string())?;
        }
        // Le schéma **nommé** du workspace, par le moteur : la vérité d'un
        // schéma ne se duplique pas en JSON (elle dériverait) — elle se
        // nomme, et le moteur l'enregistre.
        #[cfg(feature = "code")]
        if let Some("code") = self
            .manifest
            .workspace
            .as_ref()
            .and_then(|w| w.index.as_deref())
        {
            crate::code::register_code_schema(&mut cat, crate::code::default_scope_chunking())
                .map_err(|e| e.to_string())?;
            // Les fonctions de verrou déclarées pour ce dépôt.
            if let Some(w) = &self.manifest.workspace {
                cat.declare_lock_calls(w.locks_via.clone());
                let requetes = w.queries_via.as_ref().map(crate::queries_via::CompiledQueriesVia::compile).transpose()?;
                cat.declare_queries(requetes);
            }
            // Les signaux déclarés par le manifeste montent sur les entités
            // du schéma — la déclaration du moteur reste neutre, chaque
            // produit choisit. Une entité inconnue refuse en listant.
            if let Some(w) = &self.manifest.workspace {
                for (entite, signaux) in &w.index_signals {
                    let Some(config) = cat.entity_configs().get(entite.as_str()).cloned() else {
                        let connues: Vec<String> =
                            cat.entity_configs().keys().map(|k| k.to_string()).collect();
                        return Err(format!(
                            "workspace.index_signals : « {entite} » n'est pas une entité du \
                             schéma « {} » — les entités : {}",
                            w.index.as_deref().unwrap_or("?"),
                            connues.join(", ")
                        ));
                    };
                    let mut config = config;
                    let mut s = crate::search::SearchSignals::NONE;
                    for signal in signaux {
                        s = s | match signal.as_str() {
                            "bm25" => crate::search::SearchSignals::BM25,
                            "vector" => crate::search::SearchSignals::VECTOR,
                            "sparse" => crate::search::SearchSignals::SPARSE,
                            _ => unreachable!("validé au chargement"),
                        };
                    }
                    config.signals = s;
                    cat.register_entity(entite, config).map_err(|e| e.to_string())?;
                }
            }
        }
        for (name, r) in &self.manifest.relations {
            cat.register_relation_with(
                name,
                &r.from,
                &r.to,
                r.properties.clone().unwrap_or_default(),
            )
            .map_err(|e| e.to_string())?;
        }
        #[cfg(feature = "code")]
        let (file_source, garde) = match &self.manifest.workspace {
            Some(w) => (
                Some(crate::backend_code::build_source(w, &self.directory, &self.manifest.name)?),
                crate::backend_code::build_garde(w),
            ),
            None => (None, None),
        };
        // Un modèle de décision déclaré se joint à l'ouverture : un service
        // injoignable se dit ici, pas au premier outil qui s'en sert.
        let decider = match self.manifest.models.get(&crate::model_source::Capability::Decide) {
            Some(source) => Some(crate::decider::connect_decider(source)?.0),
            None => None,
        };
        let backend = Backend {
            decider,
            avertissement_sans_service,
            prepared: std::sync::RwLock::new(Arc::new(self)),
            version: std::sync::atomic::AtomicU64::new(0),
            // La vue de lecture, publiée une dernière fois le catalogue monté.
            read_view: {
                cat.publish_read_view();
                cat.read_view_cell()
            },
            catalog: Arc::new(Mutex::new(cat)),
            #[cfg(feature = "code")]
            file_source,
            #[cfg(feature = "code")]
            garde,
            reactions: Mutex::new(None),
            cell_reactions: Mutex::new(HashMap::new()),
            miroir_reacteur: Mutex::new(None),
            watches: Mutex::new(Vec::new()),
            components_watch: Mutex::new(None),
            propositions: Default::default(),
            reaction_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            runs_in_flight: crate::dataflow::RunsInFlight::new(),
            ecarts_des_fichiers: Mutex::new(None),
        };
        // **Une réaction déclarée est une réaction qui tourne.** Elle se monte
        // ici, pas sur un appel que l'hôte pourrait oublier : une section
        // `"reactions"` lue, validée, puis montée par personne serait
        // exactement la déclaration inerte qu'on écrit en boucle dans ce
        // dépôt. Sans réaction déclarée, aucun réacteur ne démarre.
        if !backend.prepared().surcharges.sans_reactions {
            backend.mount_reactions()?;
        }
        backend.seed()?;
        let regeneres = backend.regenerer_les_fichiers_composes()?;
        backend.comparer_les_fichiers_a_l_ouverture(regeneres);
        Ok(backend)
    }
}

/// Un répertoire sous le dossier temporaire du système, effacé à la sortie de
/// sa portée ; `tempfile` n'est qu'une dépendance de développement.
struct RepertoireJetable(PathBuf);

impl RepertoireJetable {
    fn nouveau(nom: &str) -> Result<Self, String> {
        let instant = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let chemin = std::env::temp_dir().join(format!("rag3weaver-{nom}-{}-{instant}", std::process::id()));
        std::fs::create_dir_all(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        Ok(Self(chemin))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for RepertoireJetable {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Copie les `.md` d'un dossier (sous-dossiers compris) sous `vers` : la copie
/// que relit la vérification à blanc.
fn copier_les_md(de: &Path, vers: &Path) -> Result<(), String> {
    std::fs::create_dir_all(vers).map_err(|e| format!("{} : {e}", vers.display()))?;
    if !de.exists() {
        return Ok(());
    }
    for e in std::fs::read_dir(de).map_err(|e| format!("{} : {e}", de.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let (source, cible) = (e.path(), vers.join(e.file_name()));
        if e.file_type().map_err(|e| e.to_string())?.is_dir() {
            copier_les_md(&source, &cible)?;
        } else if source.extension().is_some_and(|x| x == "md") {
            std::fs::copy(&source, &cible).map_err(|e| format!("{} : {e}", source.display()))?;
        }
    }
    Ok(())
}

/// **Ce qu'un outil à harnais rend à lire**, pour tous les transports (MCP,
/// chat, API). Sa sortie utile vit dans la livraison (`on_accept`) — le texte
/// d'import d'un deck — et aucun client ne savait la montrer : le modèle
/// recevait le reçu en JSON, avec une chaîne échappée qu'il ne recopiait pas
/// (27 septembre 2026). Accepté : les textes livrés tels quels ; refusé : les
/// erreurs ; les avertissements en clair dessous. Une présentation déjà posée
/// par le graphe de l'outil n'est pas touchée.
impl PreparedBackend {
    /// Le graphe déclaré d'un outil — pour le dessiner (la page vivante).
    pub fn tool_graph(&self, name: &str) -> Option<&GraphTool> {
        self.tools.get(name)
    }

    /// Une politique de nœuds, plus les nœuds scriptés déclarés.
    fn with_scripted<S: Into<String>>(&self, base: impl IntoIterator<Item = S>) -> NodeTypePolicy {
        NodeTypePolicy::only(
            base.into_iter()
                .map(Into::into)
                .chain(self.scripted_types.iter().cloned()),
        )
    }
}

/// Ce qui, d'une version des déclarations à l'autre, a été fixé à
/// l'ouverture de la base — nommé par sa clé de manifeste. Les entités sont
/// comparées après fusion de leur schéma JSON (un champ ajouté au schéma est
/// un changement d'entité), sans leur description.
pub(crate) fn opening_changes(old: &PreparedBackend, new: &PreparedBackend) -> Vec<String> {
    fn value<T: Serialize>(v: &T) -> Value {
        serde_json::to_value(v).unwrap_or(Value::Null)
    }
    fn entities(p: &PreparedBackend) -> Value {
        let mut all = value(&p.entities);
        if let Some(map) = all.as_object_mut() {
            for entity in map.values_mut() {
                if let Some(entity) = entity.as_object_mut() {
                    entity.remove("description");
                }
            }
        }
        all
    }
    let (a, b) = (&old.manifest, &new.manifest);
    let mut changed = Vec::new();
    for (key, before, after) in [
        ("database", value(&a.database), value(&b.database)),
        ("embeddings", value(&a.embeddings), value(&b.embeddings)),
        ("models", value(&a.models), value(&b.models)),
        ("vector_extension", value(&a.vector_extension), value(&b.vector_extension)),
        ("workspace", value(&a.workspace), value(&b.workspace)),
        ("fts_positions", value(&a.fts_positions), value(&b.fts_positions)),
        ("buffer_pool", value(&a.buffer_pool), value(&b.buffer_pool)),
        ("relations", value(&old.relation_defs), value(&new.relation_defs)),
        ("entities", entities(old), entities(new)),
    ] {
        if before != after {
            changed.push(format!("« {key} »"));
        }
    }
    changed
}

/// Une version vérifiée, pas encore en service ([`Backend::check_reload`]).
pub struct PendingReload {
    next: PreparedBackend,
    removed_reactions: Vec<String>,
}

impl PendingReload {
    /// Les réactions de la version en service qui n'existent plus dans la
    /// nouvelle : l'hôte oublie leur curseur sur le bus, **en le disant**
    /// (les événements en attente sont perdus), sinon il accumule sans
    /// lecteur.
    pub fn removed_reactions(&self) -> &[String] {
        &self.removed_reactions
    }
}

/// Ce qu'un rechargement accepté a mis en service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reloaded {
    pub version: u64,
    /// Les réactions retirées par ce rechargement (voir
    /// [`PendingReload::removed_reactions`]).
    pub removed_reactions: Vec<String>,
}

/// Ce qu'un rechargement a fait des réactions en service
/// ([`Backend::reload_reactions`]).
///
/// Tout est RENDU, rien n'est seulement journalisé : l'arrêt d'un réacteur
/// emporte ses erreurs avec lui, et un curseur oublié perd ses événements en
/// attente. Les taire ici en ferait deux pertes silencieuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionsReloaded {
    pub reloaded: Reloaded,
    /// Les erreurs des réactions arrêtées — personne d'autre ne les relira,
    /// leur réacteur est parti.
    pub errors_before: Vec<String>,
    /// Les curseurs oubliés, en clair (`<sujet>@<réaction>`) : **leurs
    /// événements en attente sont perdus**, c'est le prix de retirer une
    /// réaction d'un manifeste. Sans cet oubli, le bus garderait un curseur
    /// que plus rien ne vide.
    pub dropped_cursors: Vec<String>,
    /// Les réactions remontées.
    pub mounted: Vec<String>,
}

fn harness_presentation(response: &mut Value) {
    let Some(validation) = response.get("validation").cloned() else { return };
    if response.get("presentation").is_some_and(|p| p.is_string()) {
        return;
    }
    let issues = |key: &str, label: &str| -> Vec<String> {
        validation[key].as_array().into_iter().flatten()
            .map(|i| format!("- {label} {} : {}", i["code"].as_str().unwrap_or("?"), i["message"].as_str().unwrap_or("")))
            .collect()
    };
    let mut parts: Vec<String> = Vec::new();
    if validation["accepted"] == false {
        parts.push("Refusé par la validation : corriger chaque erreur et soumettre à nouveau.".into());
        parts.extend(issues("errors", "erreur"));
    } else if response["delivery"]["ok"] == false {
        parts.push(format!("Accepté, mais la livraison a échoué : {}", response["delivery"]["error"]));
    } else {
        let delivered: Vec<&str> = response["delivery"]["results"].as_array().into_iter().flatten()
            .filter_map(Value::as_str).collect();
        if delivered.is_empty() {
            return;
        }
        parts.push(delivered.join("\n\n"));
    }
    parts.extend(issues("warnings", "avertissement"));
    response["presentation"] = Value::String(parts.join("\n"));
}

// An explicit capability list: no SQL, script, write node or nested graph factory.
const SEARCH_NODES: &[&str] = &[
    "KBQuerySourceNode",
    "SelectRecordsNode",
    "SearchSourceNode",
    "BM25SearchNode",
    "VectorSearchNode",
    "SparseSearchNode",
    "RelatedResultsNode",
    "IntersectResultsNode",
    "LabelResultsNode",
    "FuseResultsNode",
    "RerankNode",
    "PaginateNode",
    "ResolveParentNode",
    "RenderResultsNode",
    "FetchRelatedNode",
    "ComposeNode",
    "GroupFrameNode",
];
const SEARCH_TOOLS: &[&str] = &["describe_backend", "validate_search", "run_search"];

/// L'outil fourni par l'hôte quand le manifeste dit `graph_tool`.
const GRAPH_TOOL: &str = "graph";

fn graph_tool_definition() -> Value {
    json!({"name":GRAPH_TOOL,
        "description":"The graph of one of this backend's tools, reactions or UI components, as JSON: nodes (name, node_type, config as written, declaring line, node schema with ports) and edges (from_node, from_port, to_node, to_port). Give exactly one of tool, reaction, component.",
        "inputSchema":{"type":"object","additionalProperties":false,"properties":{
            "tool":{"type":"string","description":"A tool of this backend."},
            "reaction":{"type":"string","description":"A reaction of this backend."},
            "component":{"type":"string","description":"A UI component of this backend (its click graph)."}
        }}})
}

/// Ce dont on demande le graphe (et ce qu'exécute un run) : la forme vit
/// avec les événements.
pub use crate::events::GraphTarget;

/// Le schéma d'un type de nœud, ses ports et ses paramètres, en JSON.
fn node_schema_json(s: &crate::dataflow::NodeSchema) -> Value {
    let port = |p: &crate::dataflow::PortDef| json!({"name":p.name,"type":format!("{:?}",p.port_type),"required":p.required});
    json!({"type":s.node_type,"description":s.description,
        "inputs":s.inputs.iter().map(port).collect::<Vec<_>>(),
        "outputs":s.outputs.iter().map(port).collect::<Vec<_>>(),
        "parameters":s.config_params.iter().map(|p| json!({"name":p.name,"type":format!("{:?}",p.param_type),"required":p.required,"default":p.default,"description":p.description,"schema":p.json_schema,"choices":p.choices.as_ref().map(|c| c.spec())})).collect::<Vec<_>>()})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchGraphRequest {
    mermaid: String,
    #[serde(default = "empty_object")]
    arguments: Value,
    #[serde(default)]
    metadata: Vec<OutputPort>,
}
fn empty_object() -> Value {
    json!({})
}
fn search_tool_definitions() -> Vec<Value> {
    let input = json!({"type":"object","additionalProperties":false,"required":["mermaid"],"properties":{
        "mermaid":{"type":"string","minLength":1,"maxLength":65536,"description":"GraphTool Mermaid, including %% tool, %% param and %% result headers. Discover nodes and examples first."},
        "arguments":{"type":"object","description":"Values for graph parameters; no text interpolation required."},
        "metadata":{"type":"array","maxItems":16,"items":{"type":"object","additionalProperties":false,"required":["node","port"],"properties":{"node":{"type":"string"},"port":{"type":"string"}}}}
    }});
    vec![
        json!({"name":"describe_backend","description":"Discover entity payloads, identities, relations, read-only nodes, filter grammar and attached Mermaid examples.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}),
        json!({"name":"validate_search","description":"Validate a read-only Mermaid search without executing queries. Same validation as run_search; does not estimate cost or prove data availability.","inputSchema":input}),
        json!({"name":"run_search","description":"Execute a composed read-only Mermaid search on the persistent backend. Returns structured result and requested terminal metadata; does not truncate intermediate branches.","inputSchema":input}),
    ]
}
impl PreparedBackend {
    /// **Le graphe d'un outil, d'une réaction ou d'un composant**, en JSON :
    /// la forme de `GraphDefinition` (`nodes` : `name`, `node_type`, `config`
    /// telle qu'écrite ; `edges` : `from_node`, `from_port`, `to_node`,
    /// `to_port`), plus la ligne de déclaration de chaque nœud dans son `.mmd`
    /// et, pour un nœud de flux, le schéma de son type (ses ports). Le nom d'un
    /// nœud est celui que portent les `NodeRun` du sujet `dataflow` : c'est ce
    /// qui allume un nœud dans un dessin vivant. Un composant donne son graphe
    /// de clic, relu sur le disque ; sans graphe de clic, un graphe vide.
    pub fn graph_json(&self, target: &GraphTarget) -> Result<Value, String> {
        self.graph_json_avec(target, &crate::lecteur::Disque)
    }

    /// [`Self::graph_json`], par un lecteur : le graphe d'une proposition se
    /// lit sur son recouvrement (le diff de graphe de l'IDE).
    pub fn graph_json_avec(&self, target: &GraphTarget, lecteur: &dyn crate::lecteur::Lecteur) -> Result<Value, String> {
        let dessin = |kind: &str, name: &str, source: &Path, def: &crate::dataflow::checkpoint::GraphDefinition, schemas: bool| {
            let texte = lecteur.lire_texte(&self.path(source)).unwrap_or_default();
            let lignes = crate::dataflow::clic::lignes_des_noeuds(&texte);
            let nodes: Vec<Value> = def
                .nodes
                .iter()
                .map(|n| {
                    let mut v = json!({"name":n.name,"node_type":n.node_type,"config":n.config,"line":lignes.get(&n.name)});
                    if schemas {
                        v["schema"] = self.nodes.schema(&n.node_type).map(|s| node_schema_json(&s)).unwrap_or(Value::Null);
                    }
                    v
                })
                .collect();
            json!({"kind":kind,"name":name,"source":source,"nodes":nodes,"edges":def.edges})
        };
        match target {
            GraphTarget::Tool(name) => {
                let t = self.tools.get(name).ok_or_else(|| format!("graph : pas d'outil « {name} » dans ce backend"))?;
                let mut v = dessin("tool", name, &self.manifest.tools[name].graph, t.template(), true);
                v["inputSchema"] = self.tool_schemas[name].clone();
                v["result"] = json!({"node":t.result().0,"port":t.result().1});
                Ok(v)
            }
            GraphTarget::Reaction(name) => {
                let t = self.reactions.get(name).ok_or_else(|| format!("graph : pas de réaction « {name} » dans ce backend"))?;
                let mut v = dessin("reaction", name, &self.manifest.reactions[name].graph, t.template(), true);
                v["result"] = json!({"node":t.result().0,"port":t.result().1});
                Ok(v)
            }
            GraphTarget::Component(name) => {
                let composants = self.components_on_disk_avec(lecteur)?;
                let decl = composants
                    .components
                    .get(name)
                    .ok_or_else(|| format!("graph : pas de composant « {name} » dans ce backend"))?;
                let Some(clic) = &decl.clic else {
                    return Ok(json!({"kind":"component","name":name,"source":Value::Null,"nodes":[],"edges":[]}));
                };
                let source = Path::new("components").join(clic);
                let texte = lecteur.lire_texte(&self.path(&source)).map_err(|e| format!("{} : {e}", source.display()))?;
                let def = crate::dataflow::mermaid::parse_mermaid(&texte).map_err(|e| format!("{} : {e}", source.display()))?;
                // Les genres d'un graphe de clic ne sont pas des nœuds de flux :
                // pas de schéma de registre.
                Ok(dessin("component", name, &source, &def, false))
            }
        }
    }

    fn search_description(&self) -> Value {
        let nodes: Vec<Value> = SEARCH_NODES.iter().filter_map(|name| self.nodes.schema(name)).map(|s| node_schema_json(&s)).collect();
        let examples: Vec<Value> = self.tools.iter().filter(|(_, t)| t.template().nodes.iter().all(|n| SEARCH_NODES.contains(&n.node_type.as_str()))).map(|(name,t)| json!({"name":name,"mermaid":t.to_mermaid(),"bindings":self.manifest.tools[name].bindings,"metadata":self.manifest.tools[name].metadata})).collect();
        json!({"backend":self.describe(),"entities":self.entities,"relations":self.manifest.relations,"nodes":nodes,"examples":examples,
            "filter_examples":{"all":{"must":[]},"scalar":{"path":{"path":["field"],"value":[{"op":"eq","value":true}]}},"array":{"path":{"path":["tags"],"value":[{"op":"has_any","value":["value"]}]}},"nested":{"nested":{"path":["abilities"],"condition":{"field":{"key":"text_en","value":[{"op":"contains","value":"graveyard"}]}}}}},
            "limits":{"mermaid_bytes":65536,"nodes":64,"edges":256,"metadata_ports":16},
            "notes":["Ad-hoc graphs are read-only; declared named tools may permit writes.","Validation checks graph structure and declared endpoints, not query cost or runtime service health.","No implicit top-k on intermediate selections; use PaginateNode explicitly for final presentation.","FuseResultsNode merges duplicates by entity and UUID by default; use its duplicates option to retain occurrences."]})
    }
    fn prepare_search(
        &self,
        request: &SearchGraphRequest,
    ) -> Result<(GraphTool, GraphDefinition), String> {
        if request.mermaid.len() > 65536 {
            return Err("Mermaid exceeds 65536 bytes".into());
        }
        let tool = GraphTool::from_mermaid(&request.mermaid)
            .and_then(|t| t.bind(&self.nodes))
            .map_err(|e| e.to_string())?;
        let mut schema = tool.tool_def().parameters;
        schema["additionalProperties"] = json!(false);
        jsonschema::validator_for(&schema)
            .map_err(|e| e.to_string())?
            .validate(&request.arguments)
            .map_err(|e| schema_refusal(&e))?;
        let def = tool
            .instantiate(&request.arguments)
            .map_err(|e| e.to_string())?;
        if def.nodes.len() > 64 || def.edges.len() > 256 {
            return Err("search graph exceeds 64 nodes or 256 edges".into());
        }
        for node in &def.nodes {
            for key in ["entity", "target_name", "source_entity"] {
                if let Some(name) = node
                    .config
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|n| !n.is_empty())
                {
                    if !self.entities.contains_key(name) {
                        return Err(format!("{}.{}: unknown entity {name}", node.name, key));
                    }
                }
            }
            let entity = node
                .config
                .get("entity")
                .or_else(|| node.config.get("target_name"))
                .and_then(Value::as_str);
            let filter = node.config.get("filter").or_else(|| {
                node.config
                    .get("options")
                    .and_then(|o| o.get("filter_condition"))
            });
            if let (Some(entity), Some(filter)) = (entity, filter) {
                if !filter.is_null() {
                    let condition: crate::filter::FilterCondition =
                        serde_json::from_value(filter.clone()).map_err(|e| e.to_string())?;
                    crate::json_schema::validate_structured_filter(
                        &condition,
                        &self.entities[entity].fields,
                    )?;
                }
            }
            if let Some(name) = node.config.get("relation").and_then(Value::as_str) {
                let relation = self
                    .manifest
                    .relations
                    .get(name)
                    .ok_or_else(|| format!("{}: unknown relation {name}", node.name))?;
                if let Some(source) = node
                    .config
                    .get("source_entity")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    let expected = if node.config["direction"].as_str() == Some("Incoming") {
                        &relation.to
                    } else {
                        &relation.from
                    };
                    if source != expected {
                        return Err(format!(
                            "{}: source_entity must be {expected} for this relation direction",
                            node.name
                        ));
                    }
                }
            }
        }
        build_definition(
            &def,
            &self.nodes,
            &NodeTypePolicy::only(SEARCH_NODES.iter().copied()),
        )
        .map_err(|e| e.to_string())?;
        for (node, port) in std::iter::once(tool.result()).chain(
            request
                .metadata
                .iter()
                .map(|p| (p.node.as_str(), p.port.as_str())),
        ) {
            let n = def
                .nodes
                .iter()
                .find(|n| n.name == node)
                .ok_or_else(|| format!("unknown output node {node}"))?;
            let schema = self.nodes.schema_for(&n.node_type, &n.config).unwrap();
            if !schema.outputs.iter().any(|p| p.name == port) {
                return Err(format!("unknown output {node}.{port}"));
            }
            if def
                .edges
                .iter()
                .any(|e| e.from_node == node && e.from_port == port)
            {
                return Err(format!(
                    "output {node}.{port} is consumed; expose a terminal port"
                ));
            }
        }
        Ok((tool, def))
    }
}

thread_local! {
    /// La cellule de l'appel en cours dans ce fil (`Backend::call_in`).
    static CELL_OF_CALL: std::cell::RefCell<Option<crate::scope::Scope>> = const { std::cell::RefCell::new(None) };
}

thread_local! {
    /// L'appel en cours est celui de l'humain (l'IDE), posé par
    /// [`Backend::call_humain`] le temps de l'appel, dans ce fil : la garde
    /// de `task_writes` se lit à l'entrée de l'appel, avant que le graphe ne
    /// parte sur d'autres fils.
    static APPEL_HUMAIN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Pose le drapeau de l'appel humain et le remet à sa valeur d'avant.
struct AppelHumain(bool);

impl AppelHumain {
    fn poser() -> Self {
        AppelHumain(APPEL_HUMAIN.with(|h| h.replace(true)))
    }
}

impl Drop for AppelHumain {
    fn drop(&mut self) {
        APPEL_HUMAIN.with(|h| h.set(self.0));
    }
}

pub struct Backend {
    /// **Les réactions en service** — le réacteur qui les exécute, ou rien si
    /// le manifeste n'en déclare aucune. Montées à l'ouverture, arrêtées par
    /// [`Backend::stop_reactions`] (un arrêt ATTENDU) avant que le catalogue
    /// ne soit lâché : une réaction qui finit d'écrire pendant qu'on ferme
    /// écrirait dans un catalogue en train de partir.
    ///
    /// **Premier champ, et c'est voulu** : les champs tombent dans l'ordre de
    /// leur déclaration, donc le `Drop` de la poignée — le filet, qui signale
    /// l'arrêt sans l'attendre — passe avant que l'`Arc` du catalogue ne soit
    /// lâché ici. Le contrat reste `stop_reactions`.
    reactions: Mutex<Option<ReactorHandle>>,
    /// Les montres de fichiers en service ; arrêtées **avant** les réactions
    /// qu'elles nourrissent.
    watches: Mutex<Vec<crate::watch::WatchHandle>>,
    /// La montre des composants, montée par l'aperçu de l'IDE
    /// ([`Backend::watch_components`]).
    components_watch: Mutex<Option<crate::components::a_chaud::ComponentsWatch>>,
    /// Les propositions de déclarations, gardées avec leur verdict
    /// ([`crate::propositions`]).
    pub(crate) propositions: crate::propositions::Propositions,
    /// **L'arrêt des réactions** : levé par `stop_reactions`, lu par une
    /// réaction longue entre deux unités de travail (`ReindexFileNode`, entre
    /// deux fichiers). Ce qui est fait reste ; le reste garde son retard et la
    /// réconciliation le reprend au prochain démarrage.
    reaction_stop: Arc<std::sync::atomic::AtomicBool>,
    /// **Les runs en vol** de ce backend (C4) : chaque run d'outil, de
    /// réaction ou de sous-graphe s'y inscrit le temps du run, et
    /// [`Backend::stop_run`] l'arrête.
    runs_in_flight: crate::dataflow::RunsInFlight,
    /// **Les réacteurs des autres cellules** (marche 1 bis, pas c) : un par
    /// cellule qu'une requête a nommée, monté à sa première requête, sur le bus
    /// et les services de cette cellule. Le bus préfixe ses sujets par la
    /// cellule : le réacteur de la cellule du catalogue n'entend pas les autres.
    cell_reactions: Mutex<HashMap<crate::scope::Scope, ReactorHandle>>,
    /// **La réaction de miroir** (lot M2c) : montée d'office dès qu'une entité
    /// déclare un fichier composé ; elle réécrit les fichiers touchés par chaque
    /// événement du catalogue.
    miroir_reacteur: Mutex<Option<ReactorHandle>>,
    /// **Les déclarations en service.** Un appel prend la version courante au
    /// départ et la garde jusqu'au bout : un rechargement qui la remplace
    /// pendant ce temps ne change rien à l'appel en cours.
    prepared: std::sync::RwLock<Arc<PreparedBackend>>,
    /// Le numéro de la version en service : 0 à l'ouverture, +1 par
    /// rechargement accepté.
    version: std::sync::atomic::AtomicU64,
    pub catalog: Arc<Mutex<Catalog>>,
    /// **La vue de lecture** du catalogue (`crate::read_view`) : les
    /// lectures d'outil et `build_services` la prennent sans le mutex.
    read_view: Arc<crate::read_view::ReadViewCell>,
    /// La source de fichiers du `workspace` déclaré — construite à
    /// l'ouverture (un instantané lit l'arbre une fois).
    #[cfg(feature = "code")]
    file_source: Option<Arc<dyn crate::code_tools::FileSource>>,
    /// La porte des commandes déclarée ; absente, `run` refuse tout.
    #[cfg(feature = "code")]
    garde: Option<Arc<crate::commande::Garde>>,
    /// Le modèle de décision déclaré (`models.decide`), donné aux nœuds sous
    /// la clé `crate::decider::DECIDER_SERVICE`.
    decider: Option<Arc<dyn crate::decider::Decider>>,
    /// Le backend déclare du vecteur et a démarré sans service : la phrase
    /// dite à l'ouverture, redite dans `describe` et `index_state`.
    avertissement_sans_service: Option<String>,
    /// **Ce que la comparaison d'ouverture a trouvé entre les dossiers et la
    /// base** (lot M3c) : chaque fichier propre contre sa ligne, dans les deux
    /// sens, et les fichiers composés régénérés. `index_state` le dit.
    ecarts_des_fichiers: Mutex<Option<Value>>,
}
impl Backend {
    /// Les avertissements d'ouverture, pour le reçu du backend ; vide quand
    /// tout ce qui est déclaré est servi.
    pub fn avertissements(&self) -> Vec<String> {
        self.avertissement_sans_service.iter().cloned().collect()
    }
    /// `describe()` du manifeste, plus les avertissements d'ouverture sous
    /// `warnings` — c'est le reçu que lit qui lance le backend en service.
    pub fn describe(&self) -> Value {
        let mut description = self.prepared().describe();
        let avertissements = self.avertissements();
        if !avertissements.is_empty() {
            description["warnings"] = json!(avertissements);
        }
        description
    }
    /// La version des déclarations en service.
    pub fn prepared(&self) -> Arc<PreparedBackend> {
        self.prepared
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Le numéro de la version des déclarations en service (0 à l'ouverture,
    /// +1 par rechargement) — à ne pas confondre avec `manifest.version`, le
    /// format du manifeste.
    pub fn declarations_version(&self) -> u64 {
        self.version.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// **Les réactions du manifeste, montées sur le bus du catalogue** — la
    /// première et seule porte par laquelle un backend exécute ses graphes
    /// réactifs. Jusqu'au 11 octobre 2026, `Reactor::watch` n'avait aucun
    /// appelant dans tout `src/` : la section `"reactions"` se lisait, se
    /// validait, et ne tournait jamais.
    ///
    /// Chaque réaction part avec **ses liaisons du manifeste**, telles quelles
    /// — le chargement a déjà refusé celle qui ne s'instancie pas avec elles
    /// seules, parce que personne ne complète un argument à l'exécution.
    ///
    /// **La politique de nœuds est écrite ICI, jamais dans le manifeste.**
    /// `ReactionAttachment` n'a pas de champ `policy`, et ce n'est pas un
    /// oubli : un gabarit qui nommerait sa propre politique s'accorderait une
    /// capacité de fichier ou de commande en l'écrivant. La liste blanche
    /// réactive était jusqu'ici le seul garde-fou et il était au chargement ;
    /// il est maintenant aux deux bouts, et le second refus nomme le nœud.
    ///
    /// Rend les noms montés. Sans réaction déclarée, aucun réacteur ne
    /// démarre et le résultat est vide.
    pub fn mount_reactions(&self) -> Result<Vec<String>, String> {
        let mut mounted = self.mount_reactors()?;
        mounted.extend(self.monter_le_miroir()?);
        // Les montres **après** les réactions : ce que leur réconciliation de
        // départ publie doit trouver la réaction à l'écoute.
        self.mount_watches()?;
        Ok(mounted)
    }

    /// **Monte la réaction de miroir** si une entité déclare un fichier composé
    /// (lot M2c ; `docs/18-octobre-2026-memoire-longue/04-…` §5 bis) : une
    /// fermeture sur le sujet `catalog`, sans graphe ni nœud, qui réécrit les
    /// fichiers des lignes que chaque événement touche, quelques millisecondes
    /// après la base, quel que soit l'écrivain. Déjà montée : rien.
    fn monter_le_miroir(&self) -> Result<Vec<String>, String> {
        const NOM: &str = "miroir_des_fichiers";
        let prepared = self.prepared();
        let composes: Vec<Arc<crate::fichiers_entite::Miroir>> =
            prepared.miroirs.values().filter(|m| m.est_compose()).cloned().collect();
        if composes.is_empty() {
            return Ok(Vec::new());
        }
        let mut slot = self.miroir_reacteur.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_some() {
            return Ok(Vec::new());
        }
        let bus = self.catalog.lock().unwrap_or_else(|e| e.into_inner()).event_bus();
        let catalogue = self.catalog.clone();
        let reacteur = Reactor::new(bus, prepared.nodes.clone(), Arc::new(ServiceRegistry::new())).on(
            NOM,
            ["catalog"],
            crate::dataflow::reactor::ReactPolicy::Debounce(50),
            move |evenements: Vec<Value>| {
                let cat = catalogue.lock().unwrap_or_else(|e| e.into_inner());
                for m in &composes {
                    // Une annonce de liens tronquée ne nomme pas tout ce qu'elle
                    // touche : les fichiers de l'entité se régénèrent tous.
                    if evenements.iter().any(|e| m.lit_une_annonce_tronquee(&cat, e)) {
                        if let Err(e) = m.regenerer_tout(&cat) {
                            eprintln!("[miroir] {} : {e}", m.entite);
                        }
                        continue;
                    }
                    let mut touchees: Vec<String> = evenements.iter().flat_map(|e| m.touchees(&cat, e)).collect();
                    touchees.sort();
                    touchees.dedup();
                    for u in touchees {
                        if let Err(e) = m.regenerer(&cat, &u) {
                            eprintln!("[miroir] {} {u} : {e}", m.entite);
                        }
                    }
                }
            },
        );
        *slot = Some(reacteur.spawn());
        Ok(vec![NOM.to_string()])
    }

    fn mount_reactors(&self) -> Result<Vec<String>, String> {
        let prepared = self.prepared();
        if prepared.reactions.is_empty() {
            return Ok(Vec::new());
        }
        let mut slot = self.reactions.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_some() {
            return Err("les réactions sont déjà en service : arrêtez-les par \
                        Backend::stop_reactions, ou remontez-les sur de nouvelles \
                        déclarations par Backend::reload_reactions"
                .into());
        }
        let bus = self
            .catalog
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .event_bus();
        let (handle, mounted) = self.monter_un_reacteur(&prepared, bus)?;
        *slot = Some(handle);
        Ok(mounted)
    }

    /// **Le bus où publier**, pris de la vue (le même `Arc` que le catalogue) :
    /// publier ne prend pas le mutex du catalogue, et n'attend pas l'écrivain
    /// qui le tient (L4). Le mutex seulement sans vue.
    pub(crate) fn bus_de_publication(&self) -> crate::events::EventBus {
        match self.read_view.current() {
            Some(vue) => vue.event_bus.in_scope(&vue.scope),
            None => self.catalog.lock().unwrap_or_else(|e| e.into_inner()).event_bus(),
        }
    }

    /// **Monte la montre des composants** (`components/`, `libs.json`) sur le
    /// bus du catalogue, si elle ne l'est pas : ses changements partent en
    /// `ComponentsChanged` (sujet `components`). L'aperçu de l'IDE la demande ;
    /// elle vit avec le backend.
    pub fn watch_components(&self) -> Result<(), String> {
        let mut slot = self.components_watch.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            let bus = self.catalog.lock().unwrap_or_else(|e| e.into_inner()).event_bus();
            *slot = Some(crate::components::a_chaud::spawn(&self.prepared().directory, bus)?);
        }
        Ok(())
    }

    /// **Monte les montres déclarées** (`watches`), sur le bus du catalogue.
    /// Déjà en service : rien. Une montre ne surveille qu'une copie de travail —
    /// une source chargée en mémoire ne change pas ; refusée en le disant.
    pub fn mount_watches(&self) -> Result<Vec<String>, String> {
        let prepared = self.prepared();
        if prepared.manifest.watches.is_empty() {
            return Ok(Vec::new());
        }
        #[cfg(not(feature = "code"))]
        {
            Err("watches : la montre de fichiers demande la feature `code`".to_string())
        }
        #[cfg(feature = "code")]
        {
            let mut slot = self.watches.lock().unwrap_or_else(|e| e.into_inner());
            if !slot.is_empty() {
                return Ok(Vec::new());
            }
            let workspace = prepared.manifest.workspace.as_ref().ok_or(
                "watches : il faut un workspace (la source surveillée)",
            )?;
            if !matches!(workspace.source, crate::backend_code::WorkspaceSource::WorkingTree) {
                return Err("watches : seule une copie de travail (`working_tree`) se surveille ; \
                            une source chargée en mémoire ne change pas"
                    .to_string());
            }
            let source = self.file_source.clone().ok_or("watches : pas de source de fichiers")?;
            let root = if workspace.root.is_absolute() {
                workspace.root.clone()
            } else {
                prepared.directory.join(&workspace.root)
            };
            // Le bus, le registre du retard et la cellule : la montre et sa
            // réaction travaillent dans la cellule du catalogue.
            // Et la référence de la réconciliation : l'empreinte que l'index
            // garde par fichier. Un index vide (jamais indexé) ne réconcilie
            // pas : la montre ne fait pas d'indexation entière à la place
            // d'`index`.
            let (bus, lag, cell, indexed) = {
                let cat = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
                let indexed = crate::watch::indexed_files(&cat, source.as_ref()).ok().filter(|i| !i.is_empty());
                (cat.event_bus(), cat.file_lag(), cat.scope().clone(), indexed)
            };
            let mut mounted = Vec::new();
            for (name, config) in &prepared.manifest.watches {
                if config.source != "workspace" {
                    return Err(format!(
                        "watches.{name} : source « {} » inconnue ; connue : workspace",
                        config.source
                    ));
                }
                slot.push(crate::watch::spawn(
                    name,
                    config,
                    root.clone(),
                    source.clone(),
                    bus.clone(),
                    lag.clone(),
                    cell.clone(),
                    indexed.clone(),
                    workspace.generated.clone(),
                )?);
                mounted.push(name.clone());
            }
            Ok(mounted)
        }
    }

    /// Arrête les montres et attend leurs fils (réveillés : pas d'attente de
    /// la période).
    pub fn stop_watches(&self) {
        let handles: Vec<_> = self.watches.lock().unwrap_or_else(|e| e.into_inner()).drain(..).collect();
        for h in handles {
            h.stop();
        }
    }

    /// **Les réactions d'une cellule**, montées à la première requête qui la
    /// nomme (`call_in`) : son bus, ses services. La cellule du catalogue a les
    /// siennes par [`mount_reactions`](Self::mount_reactions).
    fn reactions_de_la_cellule(&self, cell: &crate::scope::Scope) -> Result<(), String> {
        let prepared = self.prepared();
        if prepared.reactions.is_empty() || self.reactions.lock().unwrap_or_else(|e| e.into_inner()).is_none() {
            // Pas de réactions déclarées, ou pas en service (tests, arrêt) : rien à monter.
            return Ok(());
        }
        // **La cellule du catalogue se lit dans la vue, sans le mutex** : une
        // écriture qui le tient (la réaction de la montre) ne fait plus
        // attendre chaque appel d'une session de l'hôte, qui porte toujours
        // une cellule. Le cinquième rejeu de codeparsers a vu un usages
        // attendre 1 610 ms ici, toute la tenue de la montre.
        if self.read_view.current().is_some_and(|v| v.scope == *cell)
            || self.cell_reactions.lock().unwrap_or_else(|e| e.into_inner()).contains_key(cell)
        {
            return Ok(());
        }
        let bus = {
            let cat = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
            if *cat.scope() == *cell {
                return Ok(());
            }
            cat.event_bus_in(cell)
        };
        let mut cellules = self.cell_reactions.lock().unwrap_or_else(|e| e.into_inner());
        if cellules.contains_key(cell) {
            return Ok(());
        }
        let (handle, _) = self.monter_un_reacteur(&prepared, bus)?;
        cellules.insert(cell.clone(), handle);
        Ok(())
    }

    /// Un réacteur sur ce bus, avec les services de l'appel en cours (sa
    /// cellule, si `call_in` l'a posée).
    fn monter_un_reacteur(&self, prepared: &PreparedBackend, bus: crate::events::EventBus) -> Result<(ReactorHandle, Vec<String>), String> {
        let mut services = self.build_services(prepared);
        // Le bus en LECTURE, sous la clé que demande `EventSourceNode` : le
        // catalogue n'en donne que `"event_bus"`, par lequel on **publie**.
        // Le même bus des deux côtés — c'est sur lui que `watch` ouvre les
        // curseurs, et de lui que le graphe les relit.
        services.register(
            crate::dataflow::trace_nodes::EVENTS_SERVICE,
            Arc::new(bus.clone()),
        );
        services.register(crate::dataflow::REACTION_STOP_SERVICE, self.reaction_stop.clone());
        services.register(crate::dataflow::RUNS_IN_FLIGHT_SERVICE, self.runs_in_flight.clone());
        // Les runs des réactions se publient sur `reactions`, qu'aucune
        // réaction ne peut écouter : pas de boucle par construction.
        services.register("event_bus", Arc::new(bus.clone()));
        services.register("run_topic", crate::events::topic::REACTIONS.to_string());
        let mut reactor = Reactor::new(bus, prepared.nodes.clone(), Arc::new(services));
        let policy = NodeTypePolicy::only(crate::backend_code::reaction_nodes());
        let mut mounted = Vec::new();
        for (name, graphe) in prepared.reaction_graphs() {
            let bindings = prepared
                .manifest
                .reactions
                .get(name)
                .map(|a| a.bindings.clone())
                .unwrap_or_default();
            reactor = reactor
                .watch_bound(graphe.clone(), Value::Object(bindings), policy.clone())
                .map_err(|e| format!("la réaction « {name} » ne se monte pas : {e}"))?
                .named(GraphTarget::Reaction(name.clone()));
            mounted.push(name.clone());
        }
        Ok((reactor.spawn(), mounted))
    }

    /// **Arrête les réactions et rend leurs erreurs** — un arrêt ATTENDU
    /// (`ReactorHandle::stop`), pas espéré par un `Drop` : une réaction qui
    /// finit d'écrire pendant qu'on ferme écrirait dans un catalogue en train
    /// de partir.
    ///
    /// À appeler avant de lâcher le backend. Rien en service : rend vide.
    pub fn stop_reactions(&self) -> Vec<String> {
        // Les montres d'abord : plus rien n'arrive aux réactions qu'on arrête.
        self.stop_watches();
        // Puis le drapeau : une réaction longue s'arrête **au prochain lot** — un lot est une
        // transaction (`ReindexFileNode`, quatre fichiers d'abord, puis ce qui tient en
        // `HOLD_MS`) ; l'arrêt attend celui qui est en cours, pas le reste.
        self.reaction_stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let errors = self.stop_reactors();
        // Remontables ensuite.
        self.reaction_stop.store(false, std::sync::atomic::Ordering::SeqCst);
        errors
    }

    fn stop_reactors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        // **Borné** (18 octobre 2026) : une réaction figée au milieu d'un fichier ne
        // lit jamais le drapeau ; au-delà du délai, le réacteur est dit figé et lâché,
        // et `shutdown` ne ferme pas le catalogue qu'il tient peut-être.
        let delai = reaction_stop_wait();
        // Les réacteurs des autres cellules d'abord, puis celui du catalogue.
        let cellules: Vec<(crate::scope::Scope, ReactorHandle)> =
            self.cell_reactions.lock().unwrap_or_else(|e| e.into_inner()).drain().collect();
        for (cellule, handle) in cellules {
            errors.extend(handle.errors());
            if let Err(e) = handle.stop_within(delai) {
                errors.push(format!("{REACTION_FIGEE} (cellule {}/{}) : {e}", cellule.org, cellule.project));
            }
        }
        let mut slot = self.reactions.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(handle) = slot.take() {
            errors.extend(handle.errors());
            if let Err(e) = handle.stop_within(delai) {
                errors.push(format!("{REACTION_FIGEE} (catalogue) : {e}"));
            }
        }
        if let Some(handle) = self.miroir_reacteur.lock().unwrap_or_else(|e| e.into_inner()).take() {
            errors.extend(handle.errors());
            if let Err(e) = handle.stop_within(delai) {
                errors.push(format!("{REACTION_FIGEE} (miroir des fichiers) : {e}"));
            }
        }
        errors
    }

    /// **Copier la base sous la porte des écrivains** (une tâche, page 09) :
    /// le mutex du catalogue tenu — les écrivains attendent, les lectures
    /// passent par la vue —, un `CHECKPOINT` (la base est alors celle d'un
    /// point de reprise propre, son WAL vide, et le plein texte est validé
    /// sur le disque après chaque lot, sous ce même mutex), puis chaque fichier
    /// de la base (`code_tree::database_files`, sans le verrou `.readers`)
    /// copié dans `vers` par reflink, sinon en copie pleine, et la porte
    /// rendue. Refusée pendant une indexation. Rend la base copiée, si le
    /// reflink a pris, la taille et la durée de la porte.
    pub fn copier_la_base_sous_la_porte(&self, vers: &Path) -> Result<CopieDeBase, String> {
        #[cfg(feature = "code")]
        {
            let n = crate::dataflow::index_nodes::indexings_running(&self.catalog);
            if n > 0 {
                return Err(format!("{n} indexation(s) en cours : la base ne se copie pas pendant une indexation"));
            }
        }
        let prepared = self.prepared();
        let base = prepared.path(&prepared.manifest.database);
        std::fs::create_dir_all(vers).map_err(|e| format!("{} : {e}", vers.display()))?;
        let porte = std::time::Instant::now();
        let cat = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
        cat.conn().execute("CHECKPOINT").map_err(|e| format!("CHECKPOINT avant la copie : {e}"))?;
        let mut reflink = true;
        let mut fichiers = Vec::new();
        // Après le CHECKPOINT, le WAL n'existe plus : seuls les fichiers présents se copient.
        for f in crate::code_tree::database_files(&base).into_iter().filter(|f| f.exists()) {
            let cible = vers.join(f.file_name().ok_or("un fichier de base sans nom")?);
            reflink &= crate::code_tree::copy(&f, &cible, true)?;
            fichiers.push(cible);
        }
        drop(cat);
        let tenue_ms = porte.elapsed().as_millis() as u64;
        fn taille(p: &Path) -> u64 {
            match std::fs::metadata(p) {
                Ok(m) if m.is_dir() => std::fs::read_dir(p).map(|e| e.flatten().map(|e| taille(&e.path())).sum()).unwrap_or(0),
                Ok(m) => m.len(),
                Err(_) => 0,
            }
        }
        Ok(CopieDeBase {
            base: vers.join(base.file_name().ok_or("une base sans nom")?),
            octets: fichiers.iter().map(|f| taille(f)).sum(),
            reflink,
            tenue_ms,
            fichiers,
        })
    }

    /// **Ce qui retient ce backend de s'arrêter pour inactivité**, s'il y a
    /// quelque chose (18 octobre 2026) : une indexation en cours, des fichiers que
    /// la montre n'a pas encore réindexés, ou le catalogue tenu (un appel, une
    /// réaction, une ingestion). Ne bloque jamais : un catalogue tenu se dit tel.
    pub fn travail_en_cours(&self) -> Option<String> {
        #[cfg(feature = "code")]
        {
            let n = crate::dataflow::index_nodes::indexings_running(&self.catalog);
            if n > 0 {
                return Some(format!("{n} indexation(s) en cours"));
            }
        }
        let Ok(cat) = self.catalog.try_lock() else {
            return Some("le catalogue est tenu (un appel, une réaction ou une ingestion)".into());
        };
        let fichiers = cat.file_lag().pending_files();
        (fichiers > 0).then(|| format!("{fichiers} fichier(s) que la montre doit encore réindexer"))
    }

    /// Les erreurs des réactions en service, dans l'ordre — ce qu'un hôte
    /// affiche, et ce qu'un témoin lit. Une réaction qui échoue n'a aucun
    /// appelant à qui rendre son erreur : si personne ne demande ici, elle
    /// n'est nulle part.
    pub fn reaction_errors(&self) -> Vec<String> {
        self.reactions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(ReactorHandle::errors)
            .unwrap_or_default()
    }

    /// Le nombre d'exécutions de réaction depuis le montage.
    pub fn reaction_runs(&self) -> usize {
        self.reactions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(ReactorHandle::runs)
            .unwrap_or(0)
    }

    /// Attend que les réactions aient exécuté `n` fois, au plus `timeout` —
    /// pour un témoin, ou un hôte qui veut synchroniser.
    pub fn wait_reaction_runs(&self, n: usize, timeout: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        while self.reaction_runs() < n {
            if start.elapsed() > timeout {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        true
    }

    /// **Recharge les déclarations ET remonte les réactions**, dans le seul
    /// ordre qui tienne :
    ///
    /// 1. [`Backend::check_reload`] — un refus ici n'arrête rien, les
    ///    réactions en service continuent ;
    /// 2. l'arrêt ATTENDU des réactions — après la vérification, parce qu'un
    ///    refus ne doit pas coûter une interruption ;
    /// 3. [`Backend::apply_reload`] — d'un seul geste, et il ne peut plus
    ///    échouer ;
    /// 4. l'oubli des curseurs des réactions retirées, **en le disant** : ce
    ///    qui attendait dessus est perdu, et le bus n'accumule plus pour
    ///    personne ;
    /// 5. le remontage depuis la nouvelle version.
    ///
    /// Ce qui rend l'étape 2 sûre, c'est le refus de l'étape 1 : le schéma,
    /// la base et l'espace de travail ne se rechargent pas, donc une réaction
    /// de l'ancienne version qui finit d'écrire écrit dans un catalogue que
    /// la nouvelle ne change pas.
    pub fn reload_reactions(&self) -> Result<ReactionsReloaded, String> {
        self.apply_reload_with_reactions(self.check_reload()?)
    }

    /// **Les étapes 2 à 5 seules**, sur un [`PendingReload`] déjà vérifié —
    /// pour un hôte qui veut garder l'étape 1 à lui (un refus avec retour
    /// arrière doit pouvoir dire « rien n'a été arrêté », ce qu'il ne peut
    /// plus dire si la vérification et l'arrêt sont dans le même appel).
    /// Demandé par « everything declarative » pour `Backend::declare`.
    pub fn apply_reload_with_reactions(
        &self,
        pending: PendingReload,
    ) -> Result<ReactionsReloaded, String> {
        let before = self.prepared();
        let removed: Vec<String> = pending.removed_reactions().to_vec();
        let errors_before = self.stop_reactions();
        let reloaded = self.apply_reload(pending);
        let mut dropped_cursors = Vec::new();
        if !removed.is_empty() {
            let bus = self
                .catalog
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .event_bus();
            for name in &removed {
                let Some(graphe) = before.reaction_graphs().get(name) else { continue };
                for topic in graphe.on() {
                    // Le curseur du GRAPHE seulement. La sonnette du réacteur
                    // (`<nom>@reactor`) est un abonnement, pas un curseur
                    // enregistré sur le bus : elle part avec son récepteur.
                    bus.drop_cursor(topic, name);
                    dropped_cursors.push(format!("{topic}@{name}"));
                }
            }
        }
        let mounted = self.mount_reactions()?;
        Ok(ReactionsReloaded {
            reloaded,
            errors_before,
            dropped_cursors,
            mounted,
        })
    }

    /// **Relit les déclarations et les met en service d'un coup** :
    /// [`Backend::check_reload`] puis [`Backend::apply_reload`].
    ///
    /// Le manifeste et tout ce qu'il nomme (graphes, scripts, schémas,
    /// crochets, réactions) sont relus et revérifiés comme au chargement ; si
    /// la nouvelle version est valide et ne change rien de ce qui a été fixé
    /// à l'ouverture, elle remplace l'ancienne d'un seul geste. Sinon
    /// l'ancienne reste en service et l'erreur dit quoi corriger. Un appel en
    /// cours finit sur la version qu'il a prise à son départ.
    pub fn reload(&self) -> Result<Reloaded, String> {
        Ok(self.apply_reload(self.check_reload()?))
    }

    /// **Le premier temps : vérifier, sans rien toucher.** Un refus ici
    /// n'arrête rien — l'ancienne version et ses réactions continuent.
    ///
    /// Ce qui a été fixé à l'ouverture (la base, les modèles, l'extension
    /// vectorielle, l'espace de travail, les entités et relations inscrites
    /// en base) ne se recharge pas : un changement de schéma est une
    /// migration, pas un rechargement. **C'est ce refus qui rend sûr l'arrêt
    /// des réactions** entre les deux temps : une réaction de l'ancienne
    /// version qui finit d'écrire pendant ce temps écrit dans un catalogue
    /// que la nouvelle version ne change pas. L'assouplir demanderait de
    /// revoir cet ordre.
    pub fn check_reload(&self) -> Result<PendingReload, String> {
        let refused = |why: String| {
            format!("rechargement refusé, l'ancienne version reste en service : {why}")
        };
        let current = self.prepared();
        let next = current.recharger(&crate::lecteur::Disque).map_err(refused)?;
        let changed = opening_changes(&current, &next);
        if !changed.is_empty() {
            return Err(refused(format!(
                "{} change — fixé à l'ouverture de la base : c'est une migration, pas un \
                 rechargement",
                changed.join(", ")
            )));
        }
        let removed_reactions = current
            .reactions
            .keys()
            .filter(|name| !next.reactions.contains_key(*name))
            .cloned()
            .collect();
        Ok(PendingReload {
            next,
            removed_reactions,
        })
    }

    /// **Le second temps : mettre en service**, d'un seul geste ; ne peut plus
    /// échouer. L'hôte qui monte des réactions les arrête avant (arrêt
    /// attendu) et les remonte après, depuis [`Backend::prepared`].
    pub fn apply_reload(&self, pending: PendingReload) -> Reloaded {
        let mut slot = self.prepared.write().unwrap_or_else(|e| e.into_inner());
        *slot = Arc::new(pending.next);
        let version = self
            .version
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        drop(slot);
        Reloaded {
            version,
            removed_reactions: pending.removed_reactions,
        }
    }

    /// **Une adresse mène à un contrôleur, son résultat à une vue.** La
    /// requête devient les arguments de l'outil déclaré par la route (corps,
    /// requête, chemin), l'outil s'exécute comme pour l'agent — mêmes
    /// validateurs, même politique, une seule version des déclarations — et
    /// son résultat va à la vue. La page et le résultat sortent du même appel.
    pub fn route(&self, request: &crate::routes::RouteRequest) -> crate::routes::RouteResponse {
        use crate::routes::{arguments, RouteResponse};
        let prepared = self.prepared();
        let (tool, view, path) = match prepared.routes.find(&request.method, &request.path) {
            Ok((tool, view, path)) => (tool.to_string(), view.map(str::to_string), path),
            Err(refused) => return refused,
        };
        let args = match arguments(request, path, prepared.tool_schemas.get(&tool)) {
            Ok(args) => args,
            Err(refused) => return refused,
        };
        let mut response = match self.call_tool_on(&prepared, &tool, Value::Object(args.clone())) {
            Ok(response) => response,
            Err(error) => return RouteResponse::text(422, error),
        };
        harness_presentation(&mut response);
        let result = response["result"].clone();
        let (content_type, body) = match &view {
            Some(view) => {
                let context = json!({"result": result, "arguments": args, "response": response});
                match prepared.routes.render(view, &context) {
                    Ok(page) => ("text/html; charset=utf-8", page),
                    Err(error) => return RouteResponse::text(500, error),
                }
            }
            None => (
                "application/json",
                serde_json::to_string_pretty(&result).unwrap_or_default(),
            ),
        };
        RouteResponse {
            status: 200,
            content_type,
            body,
            result: Some(result),
        }
    }

    /// Library API shared by applications and future transports. No MCP dispatch.
    pub fn run_search_program(
        &self,
        program: &crate::dataflow::search_chain::SearchProgram,
    ) -> Result<Value, String> {
        let prepared = self.prepared();
        let plan = prepared.compile_search_program(program)?;
        let metadata = plan
            .metadata_nodes
            .iter()
            .map(|n| OutputPort {
                node: n.clone(),
                port: "meta".into(),
            })
            .collect::<Vec<_>>();
        let mut response = self.execute_plan(
            &prepared,
            (&plan.result_node, "results"),
            &plan.graph,
            &metadata,
            &NodeTypePolicy::only(SEARCH_NODES.iter().copied()),
            None,
        )?;
        response["graph_hash"] = json!(plan.graph.hash());
        response["candidate_bounded"] = json!(plan.candidate_bounded);
        response["diagnostics"] = json!(plan.diagnostics);
        Ok(response)
    }

    /// **Un appel dans une cellule** (marche 1 bis de la forme C) : la cellule
    /// de la session qui appelle, portée par les services de l'appel (`scope`)
    /// jusqu'aux nœuds, au lieu de la cellule du catalogue. `call` reste
    /// l'appel dans la cellule par défaut du catalogue.
    pub fn call_in(&self, cell: &crate::scope::Scope, name: &str, args: Value) -> Result<Value, String> {
        // La cellule vit le temps de cet appel, dans ce fil : les services se
        // construisent ici, avant que le graphe ne parte sur d'autres fils.
        struct Remettre(Option<crate::scope::Scope>);
        impl Drop for Remettre {
            fn drop(&mut self) {
                CELL_OF_CALL.with(|c| *c.borrow_mut() = self.0.take());
            }
        }
        let avant = CELL_OF_CALL.with(|c| c.replace(Some(cell.clone())));
        let _remettre = Remettre(avant);
        // Les réactions de cette cellule écoutent avant que l'appel n'écrive.
        self.reactions_de_la_cellule(cell)?;
        self.call(name, args)
    }

    /// **Arrête un run en vol** de ce backend (C4), au nom de `by` : un appel
    /// d'outil, une réaction, un sous-graphe. Le run finit `requested`, et
    /// `RunFinished.cancelled_by` dit `by`. Un run qui n'est pas en vol est
    /// un refus nommé, et rien ne bouge.
    pub fn stop_run(&self, run: &str, by: &str) -> Result<(), String> {
        self.runs_in_flight.stop(run, by)
    }

    /// Les runs en vol de ce backend.
    pub fn runs_in_flight(&self) -> Vec<String> {
        self.runs_in_flight.runs()
    }

    /// Le registre des runs en vol de ce backend : une boucle d'agent (le chat
    /// de l'IDE) s'y inscrit par ce qui l'arrête.
    pub fn runs_in_flight_registry(&self) -> &crate::dataflow::RunsInFlight {
        &self.runs_in_flight
    }

    /// [`call_in`](Self::call_in) au nom de l'humain (l'IDE).
    pub fn call_in_humain(&self, cell: &crate::scope::Scope, name: &str, args: Value) -> Result<Value, String> {
        let _humain = AppelHumain::poser();
        self.call_in(cell, name, args)
    }

    /// **Un appel d'outil**, au nom de l'agent : sur un workspace à
    /// `task_writes`, un outil qui écrit les fichiers est refusé sur le
    /// principal — l'agent écrit dans une tâche. L'IDE appelle
    /// [`call_humain`](Self::call_humain).
    pub fn call(&self, name: &str, args: Value) -> Result<Value, String> {
        if !APPEL_HUMAIN.with(|h| h.get()) {
            self.refus_de_l_agent_hors_tache(name)?;
        }
        let mut response = self.call_tool(name, args)?;
        harness_presentation(&mut response);
        Ok(response)
    }

    /// **Un appel d'outil au nom de l'humain** (l'IDE : l'édition de Lucie,
    /// B1) : `task_writes` ne vise que l'agent, l'humain écrit le principal
    /// tant que `read_only` ne l'interdit pas.
    pub fn call_humain(&self, name: &str, args: Value) -> Result<Value, String> {
        let _humain = AppelHumain::poser();
        self.call(name, args)
    }

    /// Le refus d'un outil d'écriture appelé par l'agent sur le principal
    /// d'un workspace à `task_writes` (une tâche, elle, a sa surcharge de
    /// workspace et écrit dans son worktree).
    fn refus_de_l_agent_hors_tache(&self, name: &str) -> Result<(), String> {
        let prepared = self.prepared();
        let task_writes = prepared.manifest.workspace.as_ref().is_some_and(|w| w.task_writes);
        let ecrit = prepared.manifest.tools.get(name).is_some_and(|t| t.policy.write_files);
        if task_writes && ecrit && prepared.surcharges().workspace.is_none() {
            return Err(format!(
                "« {name} » écrit les fichiers, et sur ce backend l'agent n'écrit pas l'arbre principal (task_writes) : \
                 ouvrez une tâche (POST /ide/taches {{backend, nom}}) — sa branche agent/<nom>, relue puis fusionnée — \
                 et appelez « {name} » sur le backend de la tâche"
            ));
        }
        Ok(())
    }

    fn call_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        self.call_tool_on(&self.prepared(), name, args)
    }

    /// Un appel sur **une** version des déclarations, de bout en bout.
    fn call_tool_on(
        &self,
        prepared: &PreparedBackend,
        name: &str,
        args: Value,
    ) -> Result<Value, String> {
        if prepared.manifest.graph_tool && name == GRAPH_TOOL {
            jsonschema::validator_for(&graph_tool_definition()["inputSchema"])
                .map_err(|e| e.to_string())?
                .validate(&args)
                .map_err(|e| schema_refusal(&e))?;
            return prepared.graph_json(&GraphTarget::from_args(&args)?);
        }
        if prepared.manifest.search_graphs && SEARCH_TOOLS.contains(&name) {
            let definition = search_tool_definitions()
                .into_iter()
                .find(|t| t["name"] == name)
                .unwrap();
            jsonschema::validator_for(&definition["inputSchema"])
                .map_err(|e| e.to_string())?
                .validate(&args)
                .map_err(|e| schema_refusal(&e))?;
            if name == "describe_backend" {
                return Ok(prepared.search_description());
            }
            let request: SearchGraphRequest =
                serde_json::from_value(args).map_err(|e| e.to_string())?;
            let (tool, def) = prepared.prepare_search(&request)?;
            if name == "validate_search" {
                return Ok(
                    json!({"valid":true,"read_only":true,"graph_hash":def.hash(),"plan":def,
                    "result":{"node":tool.result().0,"port":tool.result().1},
                    "validation":"structure, parameters, entities, relations and ports; no query executed"}),
                );
            }
            let mut response = self.execute_graph(
                prepared,
                &tool,
                &def,
                &request.metadata,
                &NodeTypePolicy::only(SEARCH_NODES.iter().copied()),
                Some(GraphTarget::Tool(name.to_string())),
            )?;
            response["graph_hash"] = json!(def.hash());
            return Ok(response);
        }
        let tool = prepared
            .tools
            .get(name)
            .ok_or_else(|| format!("unexposed tool {name}"))?;
        let attachment = &prepared.manifest.tools[name];
        let mut args = args
            .as_object()
            .ok_or("arguments must be an object")?
            .clone();
        for key in args.keys() {
            if attachment.bindings.contains_key(key) {
                return Err(format!("cannot override binding {key}"));
            }
            if !tool.params().iter().any(|p| p.name == key.as_str()) {
                return Err(format!("unknown argument {key}"));
            }
        }
        let errors: Vec<Value> = prepared.tool_validators[name].iter_errors(&Value::Object(args.clone())).map(|e|json!({"code":"schema","path":e.instance_path().to_string(),"message":schema_refusal(&e)})).collect();
        if !errors.is_empty() {
            if attachment.harness.input_schema.is_none()
                && attachment.harness.before.is_empty()
                && attachment.harness.after.is_empty()
                && attachment.harness.on_accept.is_empty()
            {
                return Err(errors[0]["message"]
                    .as_str()
                    .unwrap_or("invalid input")
                    .into());
            }
            return Ok(
                json!({"validation":{"accepted":false,"errors":errors,"warnings":[]},"stage":"input_schema","executed":false}),
            );
        }
        let mut context = json!({"tool":name,"arguments":args,"result":null});
        let harness = &prepared.harnesses[name];
        let before = self.validate_hooks(prepared, &harness.before, &context);
        if !before.accepted {
            return Ok(json!({"validation":before,"stage":"before","executed":false}));
        }
        args.extend(attachment.bindings.clone());
        let args_for_hook = args.clone();
        let def = tool
            .instantiate(&Value::Object(args))
            .map_err(|e| e.to_string())?;
        // La politique **déclarée** de l'outil — vérifiée au chargement, elle
        // tient ici le runtime : la base déclarative, plus ce que l'outil a
        // déclaré (fichiers, commandes).
        let policy = prepared.with_scripted(crate::backend_code::allowed_nodes(&attachment.policy));
        // Le crochet qui veut les uuids des résultats les capture au passage
        // de l'outil, par un port de métadonnées de plus — jamais un
        // re-calcul.
        let mut metadata_ports = attachment.metadata.clone();
        if let Some(port) = attachment.after.as_ref().and_then(|h| h.results_port.as_ref()) {
            if !metadata_ports.iter().any(|p| p.node == port.node && p.port == port.port) {
                metadata_ports.push(port.clone());
            }
        }
        let mut response = self.execute_graph(prepared, tool, &def, &metadata_ports, &policy, Some(GraphTarget::Tool(name.to_string())))?;
        context["result"] = response["result"].clone();
        let mut report = self.validate_hooks(prepared, &harness.after, &context);
        report.warnings.extend(before.warnings);
        if !report.accepted {
            response["validation"] = serde_json::to_value(report).unwrap();
            response["stage"] = json!("after");
            response["executed"] = json!(true);
            return Ok(response);
        }
        response["validation"] = serde_json::to_value(report).unwrap();
        let mut deliveries = Vec::new();
        for hook in &harness.on_accept {
            match self.call_hook(prepared, hook, &context) {
                Ok(value) => deliveries.push(value),
                Err(error) => {
                    response["delivery"] = json!({"ok":false,"error":error,"completed":deliveries});
                    return Ok(response);
                }
            }
        }
        response["delivery"] = json!({"ok":true,"results":deliveries});
        if attachment.after.is_some() {
            self.enrich_with_after_hook(prepared, name, &args_for_hook, &mut response);
        }
        if attachment.freshness {
            self.dire_la_fraicheur(&mut response);
        }
        Ok(response)
    }

    /// **La fraîcheur d'une réponse de lecture** (`ToolAttachment::freshness`) : le champ
    /// `freshness`, et la ligne en tête du texte rendu (le résultat en markdown, ou la
    /// présentation d'une recherche) quand le nœud ne l'y a pas déjà mise. Le catalogue se
    /// prend par un essai, jamais une attente : un écrivain qui le tient, et la réponse le dit
    /// (`writing`) sans retarder l'appel.
    fn dire_la_fraicheur(&self, response: &mut Value) {
        let cell = CELL_OF_CALL.with(|c| c.borrow().clone());
        let Ok(cat) = self.catalog.try_lock() else {
            response["freshness"] = json!({ "writing": true });
            return;
        };
        let cell = cell.unwrap_or_else(|| cat.scope().clone());
        let (Some(fraicheur), Some(ligne)) = (
            crate::dataflow::catalog_read::freshness_of(&cat, &cell),
            crate::dataflow::catalog_read::freshness_line(&cat, &cell),
        ) else {
            return;
        };
        drop(cat);
        response["freshness"] = fraicheur;
        for cle in ["result", "presentation"] {
            if let Some(texte) = response.get(cle).and_then(Value::as_str) {
                if !texte.starts_with("> ") {
                    response[cle] = Value::String(format!("> {ligne}\n\n{texte}"));
                }
            }
        }
    }

    /// **Le crochet après outil** : monte son graphe avec les mêmes arguments
    /// que l'outil (plus `threshold` s'il est déclaré), et ajoute sa section
    /// au rendu — ou se tait. Toute erreur se journalise et se tait aussi :
    /// le résultat de l'outil part toujours, entier, inchangé. Le journal du
    /// backend (stderr) rend le silence observable : déclenché / tu / en
    /// erreur — sans lui, le défaut sûr serait un défaut invisible.
    fn enrich_with_after_hook(
        &self,
        prepared: &PreparedBackend,
        tool_name: &str,
        args: &serde_json::Map<String, Value>,
        response: &mut Value,
    ) {
        let Some(hook) = prepared
            .manifest
            .tools
            .get(tool_name)
            .and_then(|a| a.after.as_ref())
        else {
            return;
        };
        let Some(graph) = prepared.after_hooks.get(tool_name) else {
            return;
        };
        if let Some(entite) = &hook.silent_unless_vectors_ready {
            let prets = self
                .catalog
                .try_lock()
                .ok()
                .and_then(|c| c.index_state_for(entite).ok())
                .is_some_and(|s| s.vectors == crate::catalog::Level::Ready);
            if !prets {
                eprintln!("[crochet {tool_name}] tu — les vecteurs de {entite} ne sont pas prêts");
                return;
            }
        }
        // Le crochet ne reçoit que ce que son gabarit déclare : les
        // arguments de l'outil qui l'intéressent, et le seuil s'il le
        // demande — `instantiate` refuse l'inconnu, à raison.
        let declares: std::collections::HashSet<&str> =
            graph.params().iter().map(|p| p.name.as_ref()).collect();
        let mut hargs: serde_json::Map<String, Value> = args
            .iter()
            .filter(|(k, _)| declares.contains(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if declares.contains("threshold") {
            if let Some(t) = hook.threshold {
                hargs.entry("threshold".to_string()).or_insert(json!(t));
            }
        }
        // Le chemin TEL QUE LA BASE L'INDEXE. Un crochet qui matche
        // l'égalité — l'impact d'un fichier — déclare `path_in_source`. Depuis
        // le 11 octobre 2026, `file_path` en base est relatif à la racine et
        // en « / » pour toute source (`workspace_path`) : c'est l'argument
        // `path` de l'outil, normalisé de la même façon. (Avant : racine +
        // chemin pour un arbre de travail, le chemin nu pour un instantané.)
        if declares.contains("path_in_source") {
            if let (Some(_), Some(Value::String(chemin))) =
                (prepared.manifest.workspace.as_ref(), args.get("path"))
            {
                hargs.insert("path_in_source".to_string(), json!(crate::workspace_path::slash(chemin)));
            }
        }
        if declares.contains("result_uuids") {
            if let Some(port) = &hook.results_port {
                let cle = format!("{}.{}", port.node, port.port);
                let uuids: Vec<String> = response["metadata"][&cle]
                    .as_array()
                    .map(|rs| {
                        rs.iter()
                            .filter_map(|r| r["uuid"].as_str())
                            .filter(|u| !u.starts_with("scan:"))
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                hargs.insert("result_uuids".to_string(), json!(uuids));
                // La métadonnée empruntée pour le crochet ne reste dans la
                // réponse que si l'outil la déclarait lui-même.
                let declaree = prepared.manifest.tools.get(tool_name).is_some_and(|a| {
                    a.metadata
                        .iter()
                        .any(|p| p.node == port.node && p.port == port.port)
                });
                if !declaree {
                    response["metadata"].as_object_mut().map(|m| m.remove(&cle));
                }
            }
        }
        // Un paramètre du gabarit absent des arguments garde son défaut ;
        // un requis manquant échoue ici — compté, jamais bloquant.
        let outcome = graph
            .instantiate(&Value::Object(hargs))
            .map_err(|e| e.to_string())
            .and_then(|def| {
                self.execute_graph(
                    prepared,
                    graph,
                    &def,
                    &[],
                    &prepared.with_scripted(crate::backend_code::hook_nodes(&hook.policy)),
                    None,
                )
            });
        match outcome {
            Ok(hresp) => {
                let texte = hresp["result"].as_str().unwrap_or_default().trim().to_string();
                if texte.is_empty() {
                    eprintln!("[crochet {tool_name}] tu — rien à dire");
                    return;
                }
                // Un état de la porte read_catalog n'est pas un contenu :
                // une section d'appoint le tait — l'outil principal porte
                // déjà sa ligne d'état. Trois formes : le partiel (une ligne
                // en blockquote devant le résultat), l'occupé et le
                // jamais-indexé (leurs constantes).
                if texte.starts_with("> ")
                    || texte.starts_with(crate::dataflow::catalog_read::BUSY)
                    || texte.starts_with(crate::dataflow::catalog_read::NEVER_INDEXED)
                {
                    eprintln!("[crochet {tool_name}] tu — un état, pas un contenu");
                    return;
                }
                let lignes: Vec<&str> = texte.lines().collect();
                let coupees = lignes.len().saturating_sub(hook.max_lines);
                let mut section = lignes
                    .into_iter()
                    .take(hook.max_lines)
                    .collect::<Vec<_>>()
                    .join("\n");
                if coupees > 0 {
                    // Tronqué, et avoué — jamais une troncature muette.
                    section.push_str(&format!("\n_… et {coupees} lignes de plus._"));
                }
                eprintln!("[crochet {tool_name}] déclenché — {} lignes", hook.max_lines.min(texte.lines().count()));
                match response.get_mut("result") {
                    // Le rendu markdown gagne sa section à la suite.
                    Some(Value::String(r)) => {
                        r.push_str(&format!("\n\n### {}\n{}", hook.title, section));
                    }
                    // Un résultat structuré la porte à côté, sans le toucher.
                    _ => {
                        response["after"] = json!({"title": hook.title, "text": section});
                    }
                }
            }
            Err(e) => {
                eprintln!("[crochet {tool_name}] en erreur (l'outil répond quand même) : {e}");
            }
        }
    }
    fn call_hook(
        &self,
        prepared: &PreparedBackend,
        hook: &PreparedHook,
        context: &Value,
    ) -> Result<Value, String> {
        let mut context = context.clone();
        context["data"] = hook.data.clone();
        let def = hook
            .tool
            .instantiate(&json!({"context":context}))
            .map_err(|e| e.to_string())?;
        // Hook graphs compute over supplied facts. They cannot invoke shell or mutate the DB.
        let response = self.execute_graph(
            prepared,
            &hook.tool,
            &def,
            &[],
            &prepared.with_scripted(["RhaiNode", "ValidationRuleNode", "ValidationMergeNode"]),
            None,
        )?;
        Ok(response["result"].clone())
    }
    fn validate_hooks(
        &self,
        prepared: &PreparedBackend,
        hooks: &[PreparedHook],
        context: &Value,
    ) -> crate::harness::ValidationReport {
        use crate::harness::{ValidationIssue, ValidationReport};
        let mut combined = ValidationReport {
            accepted: true,
            errors: vec![],
            warnings: vec![],
        };
        for hook in hooks {
            match self
                .call_hook(prepared, hook, context)
                .and_then(ValidationReport::from_value)
            {
                Ok(report) => {
                    combined.warnings.extend(report.warnings);
                    combined.errors.extend(report.errors);
                    if !report.accepted {
                        combined.accepted = false;
                    }
                }
                Err(message) => {
                    combined.accepted = false;
                    combined.errors.push(ValidationIssue {
                        code: "hook_failed".into(),
                        path: "".into(),
                        message,
                        node: None,
                        params: Default::default(),
                    });
                }
            }
        }
        combined
    }
    /// **Les services d'un graphe de ce backend, en un seul endroit.** Le
    /// chemin des outils ([`Backend::execute_plan`]) et le montage des
    /// réactions ([`Backend::mount_reactions`]) donnent exactement les mêmes :
    /// sans ce partage, une réaction validée au chargement échouerait à
    /// l'exécution sur un service absent, et le manifeste aurait menti.
    ///
    /// `"events"` (lecture du bus, ce que demande `EventSourceNode`) n'est
    /// pas ici : le catalogue donne `"event_bus"`, par lequel on **publie**.
    /// C'est le montage qui ajoute le premier, parce que lui seul sait sur
    /// quel bus les curseurs ont été ouverts.
    /// **L'instantané d'un appel de lecture** (L6) : un graphe dont tous les
    /// nœuds ne font que lire ([`crate::dataflow::NodeRegistry::snapshot_safe`]),
    /// sur la cellule de la vue, lit dans une transaction READ ONLY d'une
    /// connexion du pool : toutes ses requêtes voient le même état, et un lot
    /// validé entre deux d'entre elles ne se voit pas à moitié. `"conn"`,
    /// `"texte_natif"` et la vue de l'appel passent sur cette connexion ; la
    /// garde tombe avec les services de l'appel, qui valide et rend la
    /// connexion. Pool plein au-delà de l'attente : l'appel lit comme avant, et
    /// le dit.
    fn poser_l_instantane(&self, prepared: &PreparedBackend, def: &GraphDefinition, services: &mut ServiceRegistry) {
        if !prepared.nodes.snapshot_safe(def) {
            return;
        }
        let Some(vue) = self.read_view.current() else { return };
        let cell = CELL_OF_CALL.with(|c| c.borrow().clone());
        if cell.as_ref().is_some_and(|c| *c != vue.scope) {
            return;
        }
        let Some(pool) = vue.pool.clone() else { return };
        let Some(instantane) = pool.instantane(crate::read_pool::ATTENTE) else {
            services.register(
                crate::read_view::READ_VIEW_SERVICE,
                Arc::new(crate::read_view::ReadViewCell::avec(vue.sans_instantane())),
            );
            return;
        };
        let conn = instantane.conn();
        let appel = vue.sur_connexion(conn.clone());
        services.register("conn", crate::dataflow::services::ConnService(conn.clone()));
        if vue.texte_natif {
            let natif: Arc<dyn crate::search_backend::SearchBackend> =
                Arc::new(crate::rag3db_search_backend::Rag3dbSearchBackend::new(conn));
            services.register("texte_natif", natif);
        }
        services.register(crate::read_view::READ_VIEW_SERVICE, Arc::new(crate::read_view::ReadViewCell::avec(appel)));
        services.register(crate::read_pool::SERVICE, Arc::new(instantane));
    }

    fn build_services(&self, prepared: &PreparedBackend) -> ServiceRegistry {
        let mut services = ServiceRegistry::new();
        // La cellule de l'appel, si `call_in` l'a posée : ses index et sa
        // cellule remplacent ceux du catalogue pour les nœuds de cet appel.
        let cell = CELL_OF_CALL.with(|c| c.borrow().clone());
        let vue = self.read_view.current();
        match cell {
            // **La cellule de la vue** : ses services, sans le mutex, comme
            // un appel sans cellule — une session de l'hôte en porte une.
            Some(cell) if vue.as_ref().is_some_and(|v| v.scope == cell) => {
                services = ServiceRegistry::layered(vue.expect("la vue vient d'être lue").search_services.clone());
                services.register("scope", cell);
            }
            Some(cell) => {
                let mut cat = self.catalog.lock().unwrap();
                // Pas de repli sur la cellule du catalogue : ce serait chercher
                // ailleurs sans le dire. Les nœuds de recherche refuseront,
                // faute de leurs services.
                if let Err(e) = cat.register_search_services_in(&cell, &mut services) {
                    eprintln!("[backend] les services de recherche de la cellule {}/{} : {e}", cell.org, cell.project);
                }
                services.register("scope", cell);
            }
            // **Sans le mutex** (lot L2) : les services de recherche de la
            // vue, posés en parent. Le mutex seulement sans vue publiée.
            None => match vue {
                Some(vue) => services = ServiceRegistry::layered(vue.search_services.clone()),
                None => self.catalog.lock().unwrap().register_search_services(&mut services),
            },
        }
        services.register("catalog", self.catalog.clone());
        services.register(crate::read_view::READ_VIEW_SERVICE, self.read_view.clone());
        // Le modèle d'embarquement déclaré : l'estimation et le reçu de
        // l'index le nomment, c'est celui qu'emploie l'index.
        services.register(crate::embedding_choice::DECLARED_MODEL_SERVICE, prepared.embed_source().model.clone());
        if let Some(decider) = &self.decider {
            services.register(crate::decider::DECIDER_SERVICE, decider.clone());
        }
        // La surface de code déclarée : la source et la porte du manifeste,
        // montées comme sur le chemin agent — sans elles, les nœuds de code
        // refusent d'eux-mêmes.
        #[cfg(feature = "code")]
        {
            if let Some(source) = &self.file_source {
                services.register::<Arc<dyn crate::code_tools::FileSource>>(
                    crate::code_tools::FILE_SOURCE_SERVICE,
                    source.clone(),
                );
                if let Some(w) = &prepared.manifest.workspace {
                    services.register(
                        crate::generated::GENERATED_POLICY_SERVICE,
                        w.generated.clone(),
                    );
                }
            }
            if let Some(garde) = &self.garde {
                services.register(crate::dataflow::run_nodes::GARDE_SERVICE, garde.clone());
                if let Some(w) = &prepared.manifest.workspace {
                    let racine = if w.root.is_absolute() {
                        w.root.clone()
                    } else {
                        prepared.directory.join(&w.root)
                    };
                    if let Some(bac) = crate::backend_code::build_bac_a_sable(&w.sandbox, &racine) {
                        services.register(
                            crate::dataflow::run_nodes::BAC_A_SABLE_SERVICE,
                            std::sync::Arc::new(bac),
                        );
                    }
                }
            }
        }
        services.register("rhai_scripts", prepared.scripts.clone());
        services.register(
            "backend_relations",
            prepared.relation_defs.clone(),
        );
        // La politique d'écriture des relations (états permis), lue par les
        // nœuds qui posent des liens.
        services.register(
            crate::backend::RELATION_POLICIES_SERVICE,
            prepared.manifest.relations.clone(),
        );
        services.register("backend_mappings", prepared.mappings.clone());
        services.register("backend_validators", prepared.validators.clone());
        services.register(crate::fichiers_entite::SERVICE, prepared.miroirs.clone());
        services.register(
            "backend_write_policies",
            prepared
                .manifest
                .entities
                .iter()
                .map(|(k, v)| (k.clone(), v.writes.clone()))
                .collect::<HashMap<_, _>>(),
        );
        // **En vol** (C4), posé en dernier : une branche plus haut remplace le
        // registre par celui de la vue (une couche), et l'inscription s'y
        // perdait — l'arrêt depuis la page ne trouvait pas le run.
        services.register(crate::dataflow::RUNS_IN_FLIGHT_SERVICE, self.runs_in_flight.clone());
        services
    }
    /// **Sème les lignes déclarées par `seed`** qui manquent (voir
    /// [`BackendManifest::seed`]) et rend combien il en a posé. Chaque ligne
    /// passe par `EntityRecordNode` : d'abord `get` sur son identité, puis
    /// `put` si rien n'est là ; aucune autre sorte de nœud n'est permise.
    /// **Régénère tous les fichiers composés depuis la base** (à l'ouverture,
    /// lot M2c) : un arrêt brutal entre une écriture et sa réécriture laisse un
    /// fichier en retard ; c'est la base qui fait foi pour une vue. Rend combien
    /// ont été réécrits, et le dit.
    pub fn regenerer_les_fichiers_composes(&self) -> Result<usize, String> {
        let prepared = self.prepared();
        let cat = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
        let mut ecrits = 0;
        for m in prepared.miroirs.values().filter(|m| m.est_compose()) {
            ecrits += m.regenerer_tout(&cat)?;
        }
        if ecrits > 0 {
            eprintln!("[backend] {} : {ecrits} fichier(s) composé(s) régénéré(s) depuis la base", prepared.manifest.name);
        }
        Ok(ecrits)
    }

    /// **Reconstruit la base depuis les fichiers** (lot M3 de la mémoire,
    /// `docs/18-octobre-2026-memoire-longue/04-…` §5 et §5 bis). Une base neuve,
    /// ouverte sur les mêmes fichiers : chaque entité à fichier propre (la
    /// `Source`) est relue fichier par fichier ([`crate::fichiers_entite::Miroir::relire`])
    /// et chaque ligne posée par `EntityRecordNode` en **restauration** — dates
    /// et révision lues du fichier, uuid vérifié contre son identité. Une base
    /// qui porte autre chose que ses lignes semées est refusée, en le disant :
    /// une reconstruction ne fusionne pas.
    ///
    /// Rend, par entité : `restored`, `already_there` (une ligne semée à
    /// l'ouverture), `refused` (le fichier et la raison), `fields_not_carried`
    /// (ce que ses fichiers ne portent pas, donc ce qu'elle perdrait). Les
    /// entités à fichier composé sont dites (`composite`) : leurs fichiers sont
    /// des vues de la base, relus seulement quand la base est perdue (lot M3b).
    pub fn rebuild_from_files(&self) -> Result<Value, String> {
        let prepared = self.prepared();
        {
            let cat = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
            let mut pleines = Vec::new();
            let mut entites: Vec<&String> = prepared.manifest.entities.keys().collect();
            entites.sort();
            for e in entites {
                let n = cat.count(e).map_err(|e| e.to_string())?;
                let semees = prepared.manifest.seed.get(e).map(Vec::len).unwrap_or(0);
                if n > semees {
                    pleines.push(format!("{e} : {n} ligne(s), dont {semees} semée(s)"));
                }
            }
            if !pleines.is_empty() {
                return Err(format!(
                    "rebuild : la base n'est pas vide ({}) — une reconstruction part d'une base neuve ou perdue, \
                     elle ne fusionne pas",
                    pleines.join(" ; ")
                ));
            }
        }
        let policy = NodeTypePolicy::only(["EntityRecordNode"]);
        let mut rapport = serde_json::Map::new();
        let mut miroirs: Vec<(&String, &Arc<crate::fichiers_entite::Miroir>)> = prepared.miroirs.iter().collect();
        miroirs.sort_by(|a, b| a.0.cmp(b.0));
        // Les fichiers composés d'abord lus : leurs blocs machine portent des
        // lignes de plusieurs entités, posées après celles des fichiers propres.
        let mut du_bloc: BTreeMap<(String, String), crate::fichiers_entite::LigneDuBloc> = BTreeMap::new();
        let mut sans_bloc: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for (entite, m) in miroirs.iter().filter(|(_, m)| m.est_compose()) {
            for rel in m.lister()? {
                match m.lire(&rel).and_then(|t| crate::fichiers_entite::Miroir::relire_le_bloc(&t)) {
                    Ok(Some(lignes)) => {
                        for l in lignes {
                            du_bloc.entry((l.entity.clone(), l.uuid.clone())).or_insert(l);
                        }
                    }
                    Ok(None) => sans_bloc.entry((*entite).clone()).or_default().push(json!({"file": rel, "reason": "no machine block (written before M3b, or by hand): its rows cannot be read back"})),
                    Err(e) => sans_bloc.entry((*entite).clone()).or_default().push(json!({"file": rel, "reason": e})),
                }
            }
        }
        for (entite, m) in miroirs.iter().copied() {
            if m.est_compose() {
                continue;
            }
            let (mut restaurees, mut deja_la) = (0usize, 0usize);
            let mut refus = Vec::new();
            for rel in m.lister()? {
                let restaurer = || -> Result<(), String> {
                    let texte = m.lire(&rel)?;
                    let (uuid, champs) = m.relire(&texte)?;
                    let def = GraphDefinition {
                        nodes: vec![crate::dataflow::checkpoint::NodeDef {
                            name: "rebuild".into(),
                            node_type: "EntityRecordNode".into(),
                            config: json!({"entity": entite, "action": "restore", "record": champs, "uuid": uuid}),
                        }],
                        edges: vec![],
                    };
                    self.execute_plan_avec(&prepared, ("rebuild", "record"), &def, &[], &policy, None, true).map(|_| ())
                };
                match restaurer() {
                    Ok(()) => restaurees += 1,
                    Err(e) if e.contains("the row already exists") => deja_la += 1,
                    Err(e) => refus.push(json!({"file": rel, "reason": e})),
                }
            }
            rapport.insert(
                entite.clone(),
                json!({"restored": restaurees, "already_there": deja_la, "refused": refus, "fields_not_carried": m.champs_non_portes()}),
            );
        }
        // **Les lignes des blocs machine** (lot M3b), par entité, puis leurs liens.
        let mut par_entite: BTreeMap<String, (usize, usize, Vec<Value>)> = BTreeMap::new();
        for ((entite, uuid), l) in &du_bloc {
            let def = GraphDefinition {
                nodes: vec![crate::dataflow::checkpoint::NodeDef {
                    name: "rebuild".into(),
                    node_type: "EntityRecordNode".into(),
                    config: json!({"entity": entite, "action": "restore", "record": l.fields, "uuid": uuid}),
                }],
                edges: vec![],
            };
            let compte = par_entite.entry(entite.clone()).or_default();
            match self.execute_plan_avec(&prepared, ("rebuild", "record"), &def, &[], &policy, None, true) {
                Ok(_) => compte.0 += 1,
                Err(e) if e.contains("the row already exists") => compte.1 += 1,
                Err(e) => compte.2.push(json!({"uuid": uuid, "reason": e})),
            }
        }
        let mut poses = 0usize;
        let mut vers_absentes: BTreeMap<String, usize> = BTreeMap::new();
        let mut liens_refuses = Vec::new();
        {
            let mut cat = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
            let mut vus = std::collections::BTreeSet::new();
            for l in du_bloc.values() {
                for (relation, liens) in &l.links {
                    let Some(def) = cat.get_relation_def(relation).cloned() else {
                        liens_refuses.push(json!({"relation": relation, "reason": "relation not declared"}));
                        continue;
                    };
                    for lien in liens {
                        if !vus.insert((relation.clone(), l.uuid.clone(), lien.to.clone())) {
                            continue;
                        }
                        // Une cible qui n'est ni dans un fichier ni semée est
                        // perdue avec la base : comptée et dite, jamais devinée.
                        if cat.get(&def.to, &lien.to).map_err(|e| e.to_string())?.is_none() {
                            *vers_absentes.entry(def.to.clone()).or_default() += 1;
                            continue;
                        }
                        if cat.get(&l.entity, &l.uuid).map_err(|e| e.to_string())?.is_none() {
                            *vers_absentes.entry(l.entity.clone()).or_default() += 1;
                            continue;
                        }
                        let props: BTreeMap<String, crate::connection::CypherValue> =
                            serde_json::from_value(Value::Object(lien.props.clone())).map_err(|e| e.to_string())?;
                        match cat.link(relation, l.uuid.as_str(), lien.to.as_str(), props) {
                            Ok(_) => poses += 1,
                            Err(e) => liens_refuses.push(json!({"relation": relation, "from": l.uuid, "to": lien.to, "reason": e.to_string()})),
                        }
                    }
                }
            }
            cat.drain();
        }
        for (entite, refus) in &sans_bloc {
            let r = rapport.entry(entite.clone()).or_insert_with(|| json!({}));
            r["refused"] = json!(refus);
        }
        for (entite, (restaurees, deja_la, refus)) in par_entite {
            let r = rapport.entry(entite.clone()).or_insert_with(|| json!({"restored": 0, "already_there": 0, "refused": []}));
            r["restored"] = json!(r["restored"].as_u64().unwrap_or(0) as usize + restaurees);
            r["already_there"] = json!(r["already_there"].as_u64().unwrap_or(0) as usize + deja_la);
            let mut tous: Vec<Value> = r["refused"].as_array().cloned().unwrap_or_default();
            tous.extend(refus);
            r["refused"] = json!(tous);
            r["from_machine_blocks"] = json!(true);
        }
        for (entite, m) in miroirs.iter().filter(|(_, m)| m.est_compose()) {
            let r = rapport.entry((*entite).clone()).or_insert_with(|| json!({}));
            r["composite"] = json!(true);
            let _ = m;
        }
        rapport.insert(
            "links".into(),
            json!({"posed": poses, "to_absent_rows": vers_absentes, "refused": liens_refuses}),
        );
        Ok(Value::Object(rapport))
    }

    /// **Les lignes d'une entité**, par uuid : leurs champs, sans colonnes
    /// internes ni cellule — ce que comparent l'ouverture et la vérification.
    fn lignes_de(cat: &Catalog, entite: &str) -> Result<BTreeMap<String, serde_json::Map<String, Value>>, String> {
        crate::schema::validate_identifier(entite, "entity").map_err(|e| e.to_string())?;
        let uuids = cat.execute_raw(&format!("MATCH (n:`{entite}`) RETURN n._uuid")).map_err(|e| e.to_string())?;
        let mut out = BTreeMap::new();
        for row in &uuids.rows {
            let Some(u) = row.first().and_then(|v| v.as_str()) else { continue };
            let Some(ligne) = cat.get(entite, u).map_err(|e| e.to_string())? else { continue };
            let mut champs = serde_json::Map::new();
            for (k, v) in ligne {
                if k.starts_with('_') || crate::scope::is_scope_column(&k) || matches!(v, crate::connection::CypherValue::Null) {
                    continue;
                }
                champs.insert(k, serde_json::to_value(v).map_err(|e| e.to_string())?);
            }
            out.insert(u.to_string(), champs);
        }
        Ok(out)
    }

    /// **Chaque fichier propre contre sa ligne, dans les deux sens** (lot M3c) :
    /// un fichier sans ligne, une ligne sans fichier, un fichier illisible, un
    /// champ porté par le fichier qui diffère de la base. Rend les écarts par
    /// entité et leur total. Les fichiers composés n'y sont pas : la base les
    /// régénère (§5 bis), elle fait foi.
    pub fn compare_files_to_base(&self) -> Result<(Value, usize), String> {
        let prepared = self.prepared();
        let cat = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
        let mut par_entite = serde_json::Map::new();
        let mut total = 0usize;
        let mut miroirs: Vec<(&String, &Arc<crate::fichiers_entite::Miroir>)> = prepared.miroirs.iter().collect();
        miroirs.sort_by(|a, b| a.0.cmp(b.0));
        for (entite, m) in miroirs.into_iter().filter(|(_, m)| !m.est_compose()) {
            let lignes = Self::lignes_de(&cat, entite)?;
            let portes: Vec<&str> = m.decl.header.iter().map(String::as_str).chain(m.decl.body.as_deref()).collect();
            let mut ecarts = Vec::new();
            let mut vus = std::collections::BTreeSet::new();
            for rel in m.lister()? {
                let (uuid, champs) = match m.lire(&rel).and_then(|t| m.relire(&t)) {
                    Ok(x) => x,
                    Err(e) => {
                        ecarts.push(json!({"kind": "unreadable", "file": rel, "reason": e}));
                        continue;
                    }
                };
                vus.insert(uuid.clone());
                let Some(ligne) = lignes.get(&uuid) else {
                    ecarts.push(json!({"kind": "file_without_row", "file": rel, "uuid": uuid}));
                    continue;
                };
                for champ in &portes {
                    let (f, b) = (champs.get(*champ), ligne.get(*champ));
                    if f != b {
                        ecarts.push(json!({"kind": "field", "file": rel, "uuid": uuid, "field": champ, "file_value": f, "base_value": b}));
                    }
                }
            }
            for uuid in lignes.keys().filter(|u| !vus.contains(*u)) {
                ecarts.push(json!({"kind": "row_without_file", "uuid": uuid}));
            }
            total += ecarts.len();
            if !ecarts.is_empty() {
                par_entite.insert(entite.clone(), json!(ecarts));
            }
        }
        Ok((Value::Object(par_entite), total))
    }

    /// La comparaison de l'ouverture, gardée pour `index_state` : les fichiers
    /// composés régénérés depuis la base, et les écarts des fichiers propres.
    fn comparer_les_fichiers_a_l_ouverture(&self, regeneres: usize) {
        if self.prepared().miroirs.is_empty() {
            return;
        }
        let etat = match self.compare_files_to_base() {
            Ok((par_entite, total)) => {
                if total > 0 {
                    eprintln!("[backend] dossier ≠ base : {total} écart(s) — index_state les nomme");
                }
                json!({"regenerated": regeneres, "gaps": total, "by_entity": par_entite})
            }
            Err(e) => json!({"regenerated": regeneres, "error": e}),
        };
        *self.ecarts_des_fichiers.lock().unwrap_or_else(|e| e.into_inner()) = Some(etat);
    }

    /// **La vérification à blanc** (lot M3c, « memoire verifier ») : les
    /// dossiers copiés dans un répertoire temporaire, une base neuve à côté,
    /// ouverte sans réactions, reconstruite depuis la copie
    /// ([`Self::rebuild_from_files`]), puis comparée à la base vivante, entité
    /// par entité (les lignes, champ à champ) et relation par relation (les
    /// paires). Rien n'est écrit dans les vrais dossiers ni dans la vraie base.
    /// Une entité que les fichiers ne portent pas (ni fichier, ni bloc) rend
    /// ses lignes en `missing` : c'est ce qu'une base perdue perdrait.
    pub fn verify_files(&self) -> Result<Value, String> {
        let prepared = self.prepared();
        let Some(racine) = prepared.miroirs.values().next().map(|m| m.racine.clone()) else {
            return Ok(json!({"files": false, "ok": true}));
        };
        let tmp = RepertoireJetable::nouveau("verification")?;
        let copie = tmp.path().join("fichiers");
        copier_les_md(&racine, &copie)?;
        let surcharges = Surcharges {
            database: Some(tmp.path().join("verification.rag3db")),
            files_root: Some(copie),
            sans_reactions: true,
            ..prepared.surcharges.clone()
        };
        let p2 = PreparedBackend::load(&prepared.manifest_path)?.avec_surcharges(surcharges);
        let base = p2.path(&p2.manifest.database);
        let conn = crate::Rag3dbConnection::new(&base).map_err(|e| e.to_string())?;
        let mut b2 = p2.open(Box::new(conn), None)?;
        let reconstruction = b2.rebuild_from_files();
        let mut entites = serde_json::Map::new();
        let mut relations = serde_json::Map::new();
        let mut ok = reconstruction.is_ok();
        if reconstruction.is_ok() {
            let vivante = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
            let refaite = b2.catalog.lock().unwrap_or_else(|e| e.into_inner());
            let mut noms: Vec<&String> = prepared.manifest.entities.keys().collect();
            noms.sort();
            for e in noms {
                let (a, b) = (Self::lignes_de(&vivante, e)?, Self::lignes_de(&refaite, e)?);
                let manquantes: Vec<&String> = a.keys().filter(|u| !b.contains_key(*u)).collect();
                let en_trop: Vec<&String> = b.keys().filter(|u| !a.contains_key(*u)).collect();
                let mut differentes = Vec::new();
                for (u, champs) in &a {
                    let Some(autres) = b.get(u) else { continue };
                    let cles: std::collections::BTreeSet<&String> = champs.keys().chain(autres.keys()).collect();
                    for k in cles {
                        if champs.get(k) != autres.get(k) {
                            differentes.push(json!({"uuid": u, "field": k, "live": champs.get(k), "rebuilt": autres.get(k)}));
                        }
                    }
                }
                ok &= manquantes.is_empty() && en_trop.is_empty() && differentes.is_empty();
                entites.insert(e.clone(), json!({"rows": a.len(), "missing": manquantes, "extra": en_trop, "differing": differentes}));
            }
            let mut rels: Vec<String> = vivante.relation_names();
            rels.sort();
            for r in rels {
                let Some(def) = vivante.get_relation_def(&r).cloned() else { continue };
                if !prepared.manifest.entities.contains_key(&def.from) || !prepared.manifest.entities.contains_key(&def.to) {
                    continue;
                }
                let q = format!("MATCH (a:`{}`)-[:`{r}`]->(b:`{}`) RETURN a._uuid, b._uuid", def.from, def.to);
                let paires = |c: &Catalog| -> Result<std::collections::BTreeSet<(String, String)>, String> {
                    Ok(c.execute_raw(&q)
                        .map_err(|e| e.to_string())?
                        .rows
                        .iter()
                        .filter_map(|l| Some((l.first()?.as_str()?.to_string(), l.get(1)?.as_str()?.to_string())))
                        .collect())
                };
                let (a, b) = (paires(&vivante)?, paires(&refaite)?);
                let manquantes: Vec<&(String, String)> = a.difference(&b).collect();
                let en_trop: Vec<&(String, String)> = b.difference(&a).collect();
                ok &= manquantes.is_empty() && en_trop.is_empty();
                relations.insert(r, json!({"pairs": a.len(), "missing": manquantes, "extra": en_trop}));
            }
        }
        let _ = b2.shutdown();
        Ok(json!({
            "ok": ok,
            "rebuild": reconstruction.unwrap_or_else(|e| json!({"error": e})),
            "entities": entites,
            "relations": relations,
        }))
    }

    pub fn seed(&self) -> Result<usize, String> {
        let prepared = self.prepared();
        if prepared.manifest.seed.is_empty() {
            return Ok(0);
        }
        let policy = NodeTypePolicy::only(["EntityRecordNode"]);
        let mut poses = 0;
        for (entity, rows) in &prepared.manifest.seed {
            if !prepared.manifest.entities.contains_key(entity) {
                return Err(format!(
                    "seed.{entity} : l'entité n'est pas déclarée dans `entities` — semez une entité du gabarit"
                ));
            }
            for (rang, row) in rows.iter().enumerate() {
                let nom = |row: &Value| -> String {
                    let identite: Vec<String> = prepared
                        .entities
                        .get(entity)
                        .and_then(|c| c.hashsafe.clone())
                        .unwrap_or_default()
                        .iter()
                        .map(|f| format!("{f}={}", row.get(f).map(Value::to_string).unwrap_or_else(|| "?".into())))
                        .collect();
                    format!("seed.{entity}[{rang}] ({})", identite.join(", "))
                };
                if !row.is_object() {
                    return Err(format!("{} : une ligne de seed est un objet, pas {row}", nom(row)));
                }
                let pas = |action: &str| -> Result<Value, String> {
                    let def = GraphDefinition {
                        nodes: vec![crate::dataflow::checkpoint::NodeDef {
                            name: "seed".into(),
                            node_type: "EntityRecordNode".into(),
                            config: json!({"entity": entity, "action": action, "record": row}),
                        }],
                        edges: vec![],
                    };
                    self.execute_plan(&prepared, ("seed", "record"), &def, &[], &policy, None)
                        .map(|r| r["result"].clone())
                        .map_err(|e| format!("{} : {e}", nom(row)))
                };
                if pas("get")?["data"].is_null() {
                    pas("put")?;
                    poses += 1;
                }
            }
        }
        if poses > 0 {
            eprintln!("[backend] {} : {poses} ligne(s) semée(s) par `seed`", prepared.manifest.name);
        }
        Ok(poses)
    }

    fn execute_graph(
        &self,
        prepared: &PreparedBackend,
        tool: &GraphTool,
        def: &GraphDefinition,
        metadata_ports: &[OutputPort],
        policy: &NodeTypePolicy,
        target: Option<GraphTarget>,
    ) -> Result<Value, String> {
        self.execute_plan(prepared, tool.result(), def, metadata_ports, policy, target)
    }
    /// `target` : ce que le run exécute, qu'il dit dans son `RunStarted`
    /// (l'outil appelé) ; `None` pour un graphe sans nom déclaré.
    fn execute_plan(
        &self,
        prepared: &PreparedBackend,
        result_port: (&str, &str),
        def: &GraphDefinition,
        metadata_ports: &[OutputPort],
        policy: &NodeTypePolicy,
        target: Option<GraphTarget>,
    ) -> Result<Value, String> {
        self.execute_plan_avec(prepared, result_port, def, metadata_ports, policy, target, false)
    }

    /// [`Self::execute_plan`], et, si `restauration`, le service qui permet
    /// l'action `restore` d'`EntityRecordNode` : la reconstruction depuis les
    /// fichiers seulement (lot M3), jamais un outil.
    #[allow(clippy::too_many_arguments)]
    fn execute_plan_avec(
        &self,
        prepared: &PreparedBackend,
        result_port: (&str, &str),
        def: &GraphDefinition,
        metadata_ports: &[OutputPort],
        policy: &NodeTypePolicy,
        target: Option<GraphTarget>,
        restauration: bool,
    ) -> Result<Value, String> {
        let mut graph =
            build_definition(def, &prepared.nodes, policy).map_err(|e| e.to_string())?;
        // Des services neufs à chaque appel : les appels parallèles de l'hôte
        // (lot L5) ne partagent rien d'ici, et l'arrêt garde toutes les poignées.
        let mut services = self.build_services(prepared);
        if restauration {
            services.register(crate::backend_nodes::SERVICE_RESTAURATION, true);
        }
        self.poser_l_instantane(prepared, def, &mut services);
        // **Le run se publie** (`RunStarted`, `NodeRun`, `RunFinished`) sur
        // `dataflow`, dans la cellule de l'appel : ce qu'un appel exécute se
        // voit (la trace, le DAG vivant de l'IDE). Sans `run_topic`, chaque
        // événement va à son sujet.
        //
        // Le bus vient de la vue, qui en porte un clone (le même `Arc`) : un
        // appel ne prend pas le mutex du catalogue pour publier, et une lecture
        // n'attend pas l'écrivain qui le tient (L4). Le mutex seulement sans vue.
        let bus = match self.read_view.current() {
            Some(vue) => match services.get::<crate::scope::Scope>("scope") {
                Some(cell) => vue.event_bus.in_scope(cell),
                None => vue.event_bus.in_scope(&vue.scope),
            },
            None => {
                let cat = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
                match services.get::<crate::scope::Scope>("scope") {
                    Some(cell) => cat.event_bus_in(cell),
                    None => cat.event_bus(),
                }
            }
        };
        services.register("event_bus", Arc::new(bus));
        let run = crate::events::new_run_id("graph");
        if let Some(target) = target {
            services.register(crate::events::RUN_TARGET_SERVICE, crate::events::RunTarget { run: run.clone(), target });
        }
        let output = DataflowRuntime::with_services(100, services).execute_as(&mut graph, &run)?;
        let read = |node: &str, port: &str| -> Result<Value, String> {
            let value = output
                .get(node, port)
                .ok_or_else(|| format!("output {node}.{port} absent; expose a terminal port"))?;
            let cp = port_value_to_checkpoint(value)?;
            match cp.data_json {
                Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
                None => Ok(Value::Null),
            }
        };
        let result = read(result_port.0, result_port.1)?;
        let mut metadata = serde_json::Map::new();
        for port in metadata_ports {
            metadata.insert(
                format!("{}.{}", port.node, port.port),
                read(&port.node, &port.port)?,
            );
        }
        let mut response = json!({"result":result,"metadata":metadata});
        if result_port.1 == "results" && output.get(result_port.0, "text").is_some() {
            response["presentation"] = read(result_port.0, "text")?;
        }
        Ok(response)
    }
    /// **Le journal d'une conversation, écrit au fil de l'eau** (27 septembre
    /// 2026). L'hôte (pas l'agent : ce n'est pas un outil) envoie chaque
    /// événement dès qu'il arrive, avec l'instant où il est arrivé : `RunStarted`,
    /// `Message` — le format de [`crate::dataflow::trace_nodes::record_runs_and_messages`].
    /// Ils deviennent `Run`, `Message`, `Conversation`, `Participant` liés :
    /// cherchables comme le reste, et relisibles par un agent plus tard.
    /// **L'état de l'index**, et celui des dossiers d'entités (lot M3c) :
    /// `files` dit ce que la comparaison d'ouverture a trouvé, et
    /// « dossier ≠ base : N écarts » rejoint les avertissements.
    pub fn index_states(&self) -> Result<Value, String> {
        let mut etats = self.index_states_du_code()?;
        if let Some(fichiers) = self.ecarts_des_fichiers.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            if let Some(o) = etats.as_object_mut() {
                let ecarts = fichiers["gaps"].as_u64().unwrap_or(0);
                o.insert("files".into(), fichiers);
                if ecarts > 0 {
                    let ligne = format!("dossier ≠ base : {ecarts} écart(s) — files.by_entity les nomme");
                    match o.get_mut("warnings").and_then(Value::as_array_mut) {
                        Some(a) => a.push(json!(ligne)),
                        None => {
                            o.insert("warnings".into(), json!([ligne]));
                        }
                    }
                }
            }
        }
        Ok(etats)
    }

    /// **L'état de l'index, par entité du schéma nommé** — pour la ligne de
    /// statut de l'application : ce qu'un harnais sait, il le dit lui-même
    /// plutôt que de compter sur le modèle pour le répéter (décision du
    /// 4 octobre, après les rejeux T1). Une indexation qui tient le verrou
    /// au-delà de l'attente d'une lecture rend `"busy": true`, avec ce qui se
    /// lit hors du catalogue (montres, reprise, embarquement, avertissements).
    fn index_states_du_code(&self) -> Result<Value, String> {
        #[cfg(not(feature = "code"))]
        {
            return Ok(json!({}));
        }
        #[cfg(feature = "code")]
        {
        let entites: &[&str] = match self
            .prepared()
            .manifest
            .workspace
            .as_ref()
            .and_then(|w| w.index.as_deref())
        {
            Some("code") => &[
                crate::code::FILE,
                crate::code::SCOPE,
                crate::code::LIBRARY,
                crate::code::SYMBOL,
            ],
            // Pas d'index de code déclaré : rien à rapporter, sauf un retard.
            _ => &[],
        };
        // **L'embarquement en cours, lu hors du verrou** que la passe tient : le
        // lot qui part (combien, le premier nommé, depuis quand), et les
        // morceaux laissés dus faute de réponse du service.
        let embarquement = crate::dataflow::index_nodes::embed_watch_of(&self.catalog).map(|w| {
            let now = w.now.lock().unwrap_or_else(|p| p.into_inner()).clone();
            let muets: Vec<String> = w.silent.lock().unwrap_or_else(|p| p.into_inner()).iter().map(|(_, nom)| nom.clone()).collect();
            let echecs = w.errors.lock().unwrap_or_else(|p| p.into_inner()).len();
            let mut v = json!({});
            if let Some(n) = now {
                let depuis = crate::catalog::retard::now_ms().saturating_sub(n.since_ms) / 1000;
                v["in_progress"] = json!({"chunks": n.chunks, "first": n.first, "since_s": depuis});
            }
            if !muets.is_empty() {
                v["silent"] = json!({"chunks": muets.len(), "named": muets.iter().take(10).collect::<Vec<_>>()});
            }
            if echecs > 0 {
                v["failed_passes"] = json!(echecs);
            }
            v
        }).filter(|v| v.as_object().is_some_and(|o| !o.is_empty()));
        // Ce que coûtent les montres, par phase : où passe le temps au repos.
        // Lu sur le backend, pas dans le catalogue : rendu même occupé.
        let montres: serde_json::Map<String, Value> = self
            .watches
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|h| (h.name().to_string(), serde_json::to_value(h.stats()).unwrap_or_default()))
            .collect();
        // Une indexation entière en cours : combien de fichiers changés pendant
        // elle seront repris à sa fin. Hors du catalogue : rendu même occupé.
        let reprise = self
            .file_source
            .as_ref()
            .and_then(|s| crate::code_sync::pending_resume(s.as_ref()))
            .map(|n| json!({"indexing": true, "files": n}));
        // Les instantanés de lecture (L6) : les connexions ouvertes, le
        // plafond, et les appels passés sans (pool plein). Hors du catalogue.
        let lectures = self.read_view.current().and_then(|v| v.pool.clone()).map(|p| {
            let (ouvertes, plafond) = p.occupation();
            json!({"connections": ouvertes, "max": plafond, "without_snapshot": p.sans_instantane(), "shortened_snapshots": p.ecourtes()})
        });
        // Un verrou court (la réindexation d'un fichier) s'attend ; seule une
        // tenue longue rend « busy ».
        let Some(cat) = crate::dataflow::catalog_read::lock_for_read(&self.catalog) else {
            let mut occupe = json!({"busy": true});
            if let Some(l) = &lectures {
                occupe["reads"] = l.clone();
            }
            if let Some(r) = reprise {
                occupe["resume"] = r;
            }
            if let Some(e) = embarquement {
                occupe["embedding"] = e;
            }
            if !montres.is_empty() {
                occupe["watches"] = Value::Object(montres);
            }
            // Les avertissements d'ouverture, même occupé.
            let avertissements = self.avertissements();
            if !avertissements.is_empty() {
                occupe["warnings"] = json!(avertissements);
            }
            return Ok(occupe);
        };
        let mut etats = serde_json::Map::new();
        if let Some(e) = embarquement {
            etats.insert("embedding".to_string(), e);
        }
        let vides = cat.chunks_sans_texte_notes();
        if vides > 0 {
            etats.insert("chunks_without_text".to_string(), json!(vides));
        }
        let ambigues = cat.aretes_ambigues_notees();
        if ambigues > 0 {
            etats.insert("ambiguous_edges".to_string(), json!(ambigues));
        }
        for entite in entites {
            if let Ok(state) = cat.index_state_for(entite) {
                let mut v = serde_json::to_value(state).map_err(|e| e.to_string())?;
                // Les index de l'entité qui n'ont pas pu s'ouvrir ou se créer, et
                // pourquoi (19 octobre 2026) : jusque-là sur stderr seulement.
                let echecs = cat.index_failures_for(Some(entite));
                if !echecs.is_empty() {
                    v["failures"] = json!(echecs.iter().map(|(genre, raison)| json!({"index": genre, "reason": raison})).collect::<Vec<_>>());
                }
                etats.insert(entite.to_string(), v);
            }
        }
        // Le retard de l'index sur le disque : des fichiers changés pas encore
        // réindexés (la montre de fichiers).
        if let Some(lag) = cat.file_lag().summary_in(cat.scope()) {
            etats.insert("lag".to_string(), json!({"files": lag.files, "since_ms": lag.since_ms}));
        }
        if let Some(r) = reprise {
            etats.insert("resume".to_string(), r);
        }
        if !montres.is_empty() {
            etats.insert("watches".to_string(), Value::Object(montres));
        }
        if let Some(l) = lectures {
            etats.insert("reads".to_string(), l);
        }
        // Les avertissements, à côté des entités (leurs noms commencent par
        // une majuscule, `warnings` ne peut pas en être une) : sans service
        // d'embarquement, et une base d'avant la règle des chemins relatifs
        // (au lieu de doublons silencieux à la prochaine synchro, qui refuse).
        let mut avertissements = self.avertissements();
        if !entites.is_empty() && crate::code_sync::avant_la_regle(&cat).unwrap_or(false) {
            avertissements.push(crate::code_sync::AVANT_LA_REGLE.to_string());
        }
        if !avertissements.is_empty() {
            etats.insert("warnings".into(), json!(avertissements));
        }
        Ok(Value::Object(etats))
        }
    }

    pub fn journal(&self, events: &[Value]) -> Result<Value, String> {
        let mut cat = self.catalog.lock().map_err(|_| "catalog lock poisoned")?;
        crate::dataflow::trace_nodes::register_trace_schema(&mut cat).map_err(|e| e.to_string())?;
        let (mut runs, mut messages) = (0, 0);
        for event in events {
            // Chaque événement garde **son** instant : un lot ne les aplatit pas.
            let at_ms = event.get("at_ms").and_then(Value::as_i64).ok_or("journal event without at_ms")?;
            let (r, m) = crate::dataflow::trace_nodes::record_runs_and_messages(&mut cat, std::slice::from_ref(event), at_ms)?;
            runs += r;
            messages += m;
        }
        Ok(json!({"runs": runs, "messages": messages}))
    }

    /// Les messages d'un fil, dans l'ordre où ils sont arrivés (`at_ms`).
    pub fn journal_read(&self, conversation: &str, since_ms: i64) -> Result<Value, String> {
        // Par la vue de lecture, sans le mutex (lot L4) ; sinon le catalogue.
        let vue = self.read_view.current();
        let garde;
        let cat: &dyn crate::read_view::ReadCatalog = match &vue {
            Some(v) => &**v,
            None => {
                garde = self.catalog.lock().map_err(|_| "catalog lock poisoned")?;
                &garde
            }
        };
        if cat.entity_config(crate::dataflow::trace_nodes::MESSAGE_ENTITY).is_none() {
            return Ok(json!({"messages": []}));
        }
        use crate::dataflow::trace_nodes::{CONVERSATION_ENTITY, IN_CONVERSATION, MESSAGE_ENTITY};
        // La conversation par son uuid (son identifiant est son identité),
        // puis ses messages par un saut que dit le dialecte.
        let cle = std::collections::BTreeMap::from([("conversation_id".to_string(), crate::connection::CypherValue::String(conversation.to_string()))]);
        let uuid = cat.entity_uuid(CONVERSATION_ENTITY, &cle).map_err(|e| e.to_string())?;
        let mut saut = rag3weaver_ir::Hop::new(CONVERSATION_ENTITY, IN_CONVERSATION, MESSAGE_ENTITY, rag3weaver_ir::Direction::Incoming);
        saut.returns = ["at_ms", "at", "from", "to", "content"].map(|f| rag3weaver_ir::Column::Node(f.into())).to_vec();
        saut.filter = Some(rag3weaver_ir::Predicate::AtLeast { field: "at_ms".into(), param: "since".into() });
        saut.order_by = vec!["at_ms".into(), "seq".into()];
        let q = cat.dialect_arc().hop(&saut).map_err(|e| e.to_string())?;
        let rows = cat
            .execute_read_with_params(
                &q,
                &[
                    crate::dataflow::graph_walk::uuid_param(&[uuid]),
                    crate::connection::QueryParam::new("since", since_ms),
                ],
            )
            .map_err(|e| e.to_string())?;
        let messages: Vec<Value> = rows
            .rows
            .iter()
            .map(|r| {
                let v = |i: usize| r.get(i).map(|c| serde_json::to_value(c).unwrap_or(Value::Null)).unwrap_or(Value::Null);
                // La première colonne est l'uuid de la conversation.
                json!({"at_ms": v(1), "at": v(2), "from": v(3), "to": v(4), "content": v(5)})
            })
            .collect();
        Ok(json!({"messages": messages}))
    }

    /// La base doit-elle être rouverte ? Voir [`Catalog::must_reopen`] ; un
    /// hôte le demande après chaque requête.
    ///
    /// **Sans attendre l'écrivain** : un appel servi par la vue pendant que
    /// la montre tient le catalogue attendait ici, sa réponse prête, la fin
    /// de la tenue (sixième rejeu de codeparsers, 1 869 ms). Catalogue tenu,
    /// la raison se lit sur la connexion de la vue, qui partage l'état de
    /// réouverture de la base ; seul le compte des opérations en file manque
    /// alors à la phrase, et l'appel suivant la dit entière.
    pub fn must_reopen(&self) -> Option<String> {
        match self.catalog.try_lock() {
            Ok(cat) => cat.must_reopen(),
            Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner().must_reopen(),
            Err(std::sync::TryLockError::WouldBlock) => self.read_view.current().and_then(|v| v.conn.must_reopen()),
        }
    }

    /// **Arrête ce backend : les réactions d'abord, le catalogue ensuite.**
    ///
    /// L'ordre n'est pas négociable, et il manquait au premier jet du montage
    /// (11 octobre 2026, trouvé en jouant la passe MCP) : une réaction qui
    /// finit d'écrire pendant que le catalogue se ferme écrirait dans un
    /// catalogue en train de partir. L'arrêt est ATTENDU, pas espéré par un
    /// `Drop`.
    ///
    /// Les erreurs des réactions arrêtées sont **dites** ici : leur réacteur
    /// est parti avec elles, et personne d'autre ne les relira.
    pub fn shutdown(&mut self) -> Result<(), String> {
        let mut figees = Vec::new();
        for erreur in self.stop_reactions() {
            eprintln!("[réaction] {erreur}");
            if erreur.starts_with(REACTION_FIGEE) {
                figees.push(erreur);
            }
        }
        // **Un réacteur figé tient peut-être le catalogue** (un verrou du moteur, une
        // ingestion à mi-chemin) : le fermer sous lui bloquerait l'arrêt à son tour, ou
        // pire. On ne le ferme pas ; ce qui est validé est dans le journal du moteur,
        // rejoué à la prochaine ouverture (ticket « l'arrêt de l'hôte attend sans
        // borne une réaction figée »).
        if !figees.is_empty() {
            return Err(format!(
                "{} réacteur(s) figé(s) — catalogue laissé ouvert, rejoué à la prochaine ouverture : {}",
                figees.len(),
                figees.join(" ; ")
            ));
        }
        // **Les indexations en cours s'arrêtent au prochain paquet validé**, et
        // on les attend avant de fermer la base : la relance reprendra là
        // (marche 0 de la forme C). Le fil d'indexation tient le catalogue
        // pendant tout son plein texte, d'où l'arrêt avant le verrou.
        // L'indexation n'existe qu'avec la feature `code`.
        #[cfg(feature = "code")]
        {
            let attendues = crate::dataflow::index_nodes::stop_indexings_of(&self.catalog);
            if attendues > 0 {
                eprintln!("[index] {attendues} indexation(s) arrêtée(s) ; relancer index reprend là");
            }
        }
        self.catalog
            .lock()
            .unwrap()
            .shutdown()
            .map_err(|e| e.to_string())
    }
}

/// Le début du message d'un réacteur qui ne s'arrête pas dans le délai.
pub const REACTION_FIGEE: &str = "réaction figée";

/// Le délai d'arrêt d'un réacteur : `RAG3WEAVER_STOP_WAIT_S` secondes, 30 par défaut.
fn reaction_stop_wait() -> std::time::Duration {
    let s = std::env::var("RAG3WEAVER_STOP_WAIT_S").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(30);
    std::time::Duration::from_secs(s)
}

fn payload_schema(
    schema: &Value,
    entity: &BackendEntity,
    view: &PayloadView,
) -> Result<Value, String> {
    let mut result = schema.clone();
    let props = result
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .ok_or("input payload views require root object properties")?;
    let generated: Vec<&String> = [
        &entity.writes.created_at,
        &entity.writes.updated_at,
        &entity.writes.revision,
    ]
    .into_iter()
    .flatten()
    .chain(
        entity
            .writes
            .transition_dates
            .iter()
            .map(|t| &t.timestamp_field),
    )
    .collect();
    let identity = entity.config.hashsafe.as_ref().ok_or("identity missing")?;
    props.retain(|key, _| match view {
        PayloadView::Identity => identity.contains(key),
        PayloadView::Editable => !generated.contains(&key),
    });
    let names: Vec<String> = props.keys().cloned().collect();
    if let Some(required) = result.get_mut("required").and_then(Value::as_array_mut) {
        required.retain(|v| v.as_str().is_some_and(|k| names.iter().any(|n| n == k)));
    }
    result["additionalProperties"] = json!(false);
    // Entity-local $refs must resolve inside this payload even when embedded in a tool schema.
    fn inline_refs(value: &mut Value, root: &Value, depth: usize) -> Result<(), String> {
        if depth > 32 {
            return Err("payload schema reference nesting too deep".into());
        }
        if let Some(reference) = value.get("$ref").and_then(Value::as_str).map(str::to_owned) {
            let pointer = reference
                .strip_prefix('#')
                .ok_or("only local schema references supported")?;
            let resolved = root
                .pointer(pointer)
                .ok_or("unresolved payload schema reference")?
                .clone();
            let mut siblings = value.as_object().cloned().unwrap_or_default();
            siblings.remove("$ref");
            *value = if siblings.is_empty() {
                resolved
            } else {
                json!({"allOf":[resolved,siblings]})
            };
        }
        match value {
            Value::Object(map) => {
                for child in map.values_mut() {
                    inline_refs(child, root, depth + 1)?;
                }
            }
            Value::Array(items) => {
                for child in items {
                    inline_refs(child, root, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    result.as_object_mut().unwrap().remove("$defs");
    inline_refs(&mut result, schema, 0)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    /// **L'extension se cherche d'abord à côté de la lib réellement chargée**
    /// (lot 3e du magasin), puis à côté de `RAG3DB_LIBRARY_DIR`, puis dans
    /// l'arbre ; un même dossier n'est cherché qu'une fois.
    #[test]
    fn the_vector_extension_is_looked_up_next_to_the_loaded_lib_first() {
        let c = super::vector_extension_candidates(
            "vector",
            Some(Path::new("/magasin/rag3db-build-L")),
            Some("/arbre".into()),
            Some("/magasin/rag3db-build-A".into()),
            Some(PathBuf::from("/bin-dir")),
        );
        assert_eq!(c[0], PathBuf::from("/magasin/rag3db-build-L/libvector.rag3db_extension"));
        assert_eq!(c[1], PathBuf::from("/magasin/rag3db-build-A/libvector.rag3db_extension"));
        assert_eq!(c[2], PathBuf::from("/arbre/extension/vector/build/libvector.rag3db_extension"));
        assert_eq!(c[3], PathBuf::from("/bin-dir/libvector.rag3db_extension"));
        let sans_lib = super::vector_extension_candidates("vector", None, Some("/arbre".into()), None, None);
        assert_eq!(sans_lib[0], PathBuf::from("/arbre/extension/vector/build/libvector.rag3db_extension"));
        let meme = super::vector_extension_candidates("vector", Some(Path::new("/m/L")), None, Some("/m/L".into()), None);
        assert_eq!(meme.iter().filter(|p| p.starts_with("/m/L")).count(), 1, "{meme:?}");
    }

    /// Une extension factice qui porte (ou non) un identifiant de bâti, comme
    /// la chaîne que rend sa fonction `build_id`.
    fn extension_factice(dir: &Path, id: Option<&str>) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let p = dir.join("libvector.rag3db_extension");
        let mut octets = b"\x7fELF factice\0".to_vec();
        if let Some(id) = id {
            octets.extend_from_slice(id.as_bytes());
            octets.push(0);
        }
        std::fs::write(&p, octets).unwrap();
        p
    }

    /// **L'identifiant décide** (lot 3e) : sans aucune variable, l'extension à
    /// côté de la lib chargée ; `RAG3DB_LIBRARY_DIR` sur un autre bâti ne la
    /// supplante pas ; une extension d'un autre bâti est passée quand elle est
    /// cherchée, refusée quand elle est forcée, et le refus nomme les deux.
    #[test]
    fn an_extension_of_another_build_is_passed_when_searched_and_refused_when_forced() {
        use crate::build_check::{tests::elf, LIB_SECTION};
        let d = tempfile::tempdir().unwrap();
        let (l, x) = ("rag3db-build-L", "rag3db-build-X");
        let dossier_l = d.path().join("lib-L");
        std::fs::create_dir_all(&dossier_l).unwrap();
        let lib = elf(&dossier_l, "librag3db.so", LIB_SECTION, Some(l));
        let bonne = extension_factice(&dossier_l, Some(l));
        let autre = extension_factice(&d.path().join("autre"), Some(x));
        let nom = Path::new("vector");
        let resoudre = |declare: &Path, lib: &Path, lib_dir: Option<&Path>| {
            super::resolve_vector_extension(declare, d.path(), Some(lib), None, lib_dir.map(|p| p.as_os_str().to_owned()), None)
        };

        assert_eq!(resoudre(nom, &lib, None).unwrap(), bonne, "sans aucune variable : à côté de la lib chargée");
        assert_eq!(resoudre(nom, &lib, Some(&d.path().join("autre"))).unwrap(), bonne, "la lib chargée passe avant la variable");

        let seule = d.path().join("lib-seule");
        std::fs::create_dir_all(&seule).unwrap();
        let lib_seule = elf(&seule, "librag3db.so", LIB_SECTION, Some(l));
        let err = resoudre(nom, &lib_seule, Some(&d.path().join("autre"))).unwrap_err();
        assert!(err.contains(l) && err.contains(x) && err.contains("écartées"), "{err}");

        let err = resoudre(Path::new("autre/libvector.rag3db_extension"), &lib, None).unwrap_err();
        assert!(err.contains("refusée") && err.contains(l) && err.contains(x), "{err}");
        assert_eq!(resoudre(Path::new("lib-L/libvector.rag3db_extension"), &lib, None).unwrap(), bonne);

        let sans_id = extension_factice(&d.path().join("ancienne"), None);
        assert_eq!(resoudre(Path::new("ancienne/libvector.rag3db_extension"), &lib, None).unwrap(), sans_id, "sans identifiant : le moteur dira");
        assert_eq!(crate::build_check::extension_build_id(&autre).unwrap().as_deref(), Some(x));
    }

    /// **De bout en bout, sans aucune variable** (lot 3e) : la lib que ce
    /// processus a réellement chargée désigne son extension, du même
    /// identifiant, et le moteur la charge. Sous le magasin (le défaut de
    /// `cargo_magasin.sh`), c'est le dossier de l'identifiant.
    #[cfg(feature = "rag3db-native")]
    #[test]
    fn the_extension_next_to_the_loaded_lib_is_loaded_without_any_variable() {
        let lib = crate::build_check::loaded_lib().expect("le moteur est lié en partagé : sa lib est dans /proc/self/maps");
        let d = tempfile::tempdir().unwrap();
        let resolue = super::resolve_vector_extension(Path::new("vector"), d.path(), Some(&lib), None, None, None);
        let id_lib = crate::build_check::read_section(&lib, crate::build_check::LIB_SECTION).unwrap();
        let Some(dossier) = lib.parent().filter(|p| p.join("libvector.rag3db_extension").is_file()) else {
            // Un dossier de bâti (`build/lecteurs-csv/src`) n'a pas l'extension à
            // côté : sans variable, ce qui est trouvé (l'arbre de ce crate) n'est
            // jamais d'un autre bâti.
            if let Ok(ext) = &resolue {
                let id_ext = crate::build_check::extension_build_id(ext).unwrap();
                assert!(id_ext.is_none() || id_ext == id_lib, "{} : {id_ext:?} contre {id_lib:?}", ext.display());
            }
            eprintln!("[témoin] la lib chargée ({}) n'a pas d'extension à côté (hors magasin) : {resolue:?}", lib.display());
            return;
        };
        let ext = resolue.unwrap();
        assert_eq!(ext, dossier.join("libvector.rag3db_extension"));
        assert_eq!(crate::build_check::extension_build_id(&ext).unwrap(), id_lib, "le même identifiant");
        let conn = crate::Rag3dbConnection::in_memory().expect("base en mémoire");
        conn.execute(&format!("LOAD EXTENSION {}", crate::connection::cypher_path_literal(&ext))).expect("le moteur charge l'extension");
        eprintln!("[témoin] {} chargée, bâti {:?}", ext.display(), id_lib);
    }

    /// **Les tests chargent l'extension de leur lib, pas celle de l'arbre**
    /// (lot 3f) : même avec `RAG3DB_ROOT` posé sur l'arbre que l'arbre
    /// principal rebâtit, l'aide des tests rend l'extension à côté de la lib
    /// réellement chargée quand elle y est (le magasin), du même identifiant.
    /// Le rouge du 12 octobre, 12 h 58 : un rebâti en place de l'arbre avait
    /// fait refuser au moteur l'extension d'un autre bâti.
    #[cfg(feature = "rag3db-native")]
    #[test]
    fn the_test_helper_loads_the_extension_of_the_loaded_lib_not_of_the_tree() {
        let lib = crate::build_check::loaded_lib().expect("le moteur est lié en partagé");
        let ext = PathBuf::from(extension_vector_de_test());
        let id_lib = crate::build_check::read_section(&lib, crate::build_check::LIB_SECTION).unwrap();
        match lib.parent().map(|d| d.join("libvector.rag3db_extension")).filter(|p| p.is_file()) {
            Some(a_cote) => assert_eq!(ext, a_cote, "RAG3DB_ROOT = {:?}", std::env::var_os("RAG3DB_ROOT")),
            None => eprintln!("[témoin] hors magasin : {} (la lib chargée {})", ext.display(), lib.display()),
        }
        let id_ext = crate::build_check::extension_build_id(&ext).unwrap();
        assert!(id_ext.is_none() || id_ext == id_lib, "{} : {id_ext:?} contre {id_lib:?}", ext.display());
    }

    /// **L'extension vector des tests** : celle de ce processus
    /// ([`super::vector_extension_of_this_process`]), à côté de la lib
    /// réellement chargée d'abord. Elle lisait `RAG3DB_ROOT` seule : un rebâti
    /// en place de l'arbre faisait tomber un rejeu qui tournait sur la lib du
    /// magasin (« Extension VECTOR was built from… », 12 octobre, 12 h 58).
    /// Introuvable, le refus nomme chaque endroit cherché.
    #[allow(dead_code)]
    fn extension_vector_de_test() -> String {
        super::vector_extension_of_this_process().unwrap_or_else(|e| panic!("{e}")).display().to_string()
    }
    /// Une fiche réactive en mémoire, pour éprouver les refus sans manifeste.
    ///
    /// Elle est calquée sur la vraie
    /// (`templates/backends/memory/graphs/review_anchored.mmd`) : le port
    /// s'appelle `events`, la clé `topics`. Une fiche de test qui parle une
    /// syntaxe supposée éprouve la supposition, pas le code.
    fn fiche(entete: &str, corps: &str, result: &str) -> crate::dataflow::GraphTool {
        crate::dataflow::GraphTool::from_mermaid(&format!(
            "%% tool: r\n%% description: d\n{entete}%% result: {result}\n\ngraph LR\n{corps}"
        ))
        .expect("fiche lisible")
    }

    /// La fiche réactive de référence : une source d'événements, une
    /// transition, et le port `events` entre les deux.
    fn fiche_reactive(entete: &str) -> crate::dataflow::GraphTool {
        fiche(
            entete,
            "    events[\"EventSourceNode(topics='catalog', cursor='r', limit=1000)\"]\n    \
             react[\"ReactTransitionNode(target=$target)\"]\n    events -->|events| react\n",
            "react.report",
        )
    }

    /// **Les sept refus d'une réaction disent chacun quoi faire.**
    ///
    /// Ils sont la valeur de la section `"reactions"` : sans eux, un manifeste
    /// faux se découvre à la première panne, dans un flux que personne ne lit.
    /// Le test regarde donc le **texte** du refus, pas seulement qu'il y en
    /// ait un — un refus qui ne nomme pas le remède ne sert ni à une personne
    /// ni à un modèle.
    #[test]
    fn les_refus_d_une_reaction_nomment_le_remede() {
        use serde_json::Map;
        let vide = Map::new();
        let bonne =
            fiche_reactive("%% on: catalog\n%% policy: debounce 500\n%% param: target string = \"Memory\" -- la cible\n");

        // Vert d'abord : la forme attendue passe, sinon les rouges ci-dessous
        // ne prouveraient rien.
        super::valider_une_reaction("r", &bonne, &vide, false, &BTreeSet::new()).expect("la forme attendue");

        // 1. Homonyme d'un outil — le curseur du réacteur EST ce nom.
        let e = super::valider_une_reaction("r", &bonne, &vide, true, &BTreeSet::new()).unwrap_err();
        assert!(e.contains("curseur"), "{e}");

        // 2. Un nœud hors de la liste réactive, avec sa capacité : le refus
        //    renvoie vers `tools` ET nomme la politique à déclarer.
        let lit = fiche(
            "%% on: catalog\n",
            "    lu[\"ReadFileNode(path='x')\"]\n",
            "lu.content",
        );
        let e = super::valider_une_reaction("r", &lit, &vide, false, &BTreeSet::new()).unwrap_err();
        assert!(e.contains("read_files") && e.contains("tools"), "{e}");

        // 3. Pas de `%% on:` — et l'autre porte de sortie est nommée.
        let muette = fiche_reactive("%% param: target string = \"Memory\" -- la cible\n");
        let e = super::valider_une_reaction("r", &muette, &vide, false, &BTreeSet::new()).unwrap_err();
        assert!(e.contains("%% on:") && e.contains("tools"), "{e}");

        // 4. Une liaison que la fiche ne déclare pas.
        let mut inconnue = Map::new();
        inconnue.insert("cible".into(), json!("Memory"));
        let e = super::valider_une_reaction("r", &bonne, &inconnue, false, &BTreeSet::new()).unwrap_err();
        assert!(e.contains("liaison inconnue cible"), "{e}");

        // 5. Non déterminée par ses liaisons : un paramètre obligatoire que
        //    personne ne remplira à l'exécution.
        let exigeante = fiche_reactive("%% on: catalog\n%% param: target string! -- la cible\n");
        let e = super::valider_une_reaction("r", &exigeante, &vide, false, &BTreeSet::new()).unwrap_err();
        assert!(e.contains("n'est pas déterminée par ses liaisons"), "{e}");
        // Liée, la même fiche passe.
        let mut liee = Map::new();
        liee.insert("target".into(), json!("Memory"));
        super::valider_une_reaction("r", &exigeante, &liee, false, &BTreeSet::new()).expect("liée");

        // 6. **La clé n'est pas le nom de la fiche** — et le curseur, c'est le
        //    nom de la fiche. Le refus donne le nom à écrire.
        let e = super::valider_une_reaction("revoir", &bonne, &vide, false, &BTreeSet::new())
            .unwrap_err();
        assert!(e.contains("« r »") && e.contains("curseur"), "{e}");

        // 7. **Le curseur du graphe nomme autre chose que la réaction.** Le
        //    réacteur ouvre le curseur au `watch`, avant ce qu'on observe :
        //    deux noms, et la réaction manque son premier lot sans rien dire.
        let ailleurs = fiche(
            "%% on: catalog\n%% param: target string = \"Memory\" -- la cible\n",
            "    events[\"EventSourceNode(topics='catalog', cursor='autre', limit=1000)\"]\n    \
             react[\"ReactTransitionNode(target=$target)\"]\n    events -->|events| react\n",
            "react.report",
        );
        let e = super::valider_une_reaction("r", &ailleurs, &vide, false, &BTreeSet::new())
            .unwrap_err();
        assert!(e.contains("cursor='r'") && e.contains("avant"), "{e}");
    }

    /// **Le compte de chaque liste blanche est épinglé.**
    ///
    /// Une liste de sécurité s'élargit par accident : on ajoute un nœud à
    /// `BASE_NODES` « pour un cas », et la surface de tous les outils et de
    /// toutes les réactions grandit sans que personne l'ait voulu. Ce test ne
    /// juge pas le contenu — il exige seulement qu'un ajout soit **décidé**,
    /// en faisant rougir la lib au lieu de passer.
    #[test]
    fn les_listes_blanches_ne_s_elargissent_pas_en_silence() {
        let outil = crate::backend_code::allowed_nodes(&Default::default());
        let reactif = crate::backend_code::reaction_nodes();
        assert_eq!(
            outil.len(),
            38,
            "la liste des nœuds d'outil a changé ({} entrées) : si c'est voulu, corrigez \
             ce compte dans le même commit — et dites pourquoi dans son message",
            outil.len()
        );
        assert_eq!(
            reactif.len(),
            outil.len() + 3,
            "la liste réactive est la base plus EventSourceNode, ReactTransitionNode et \
             ReindexFileNode (écrire l'index d'un fichier changé sous une montre, W2), \
             soit trois entrées — elle en a {}",
            reactif.len() - outil.len()
        );
    }

    #[test]
    fn un_status_non_cable_s_avertit_au_chargement() {
        use crate::dataflow::GraphTool;
        // Câblé : aucun avertissement.
        let cable = GraphTool::from_mermaid(
            "%% tool: t\n%% description: d\n%% result: fuse.results\n\ngraph LR\n    v[\"VectorSearchNode\"]\n    fuse[\"FuseResultsNode\"]\n    v -->|results:vector| fuse\n    v -->|status| fuse\n",
        )
        .expect("gabarit câblé");
        assert!(super::statuts_non_cables("t", cable.template()).is_empty());
        // Non câblé : l'avertissement nomme le nœud et la conséquence.
        let nu = GraphTool::from_mermaid(
            "%% tool: t\n%% description: d\n%% result: fuse.results\n\ngraph LR\n    v[\"VectorSearchNode\"]\n    fuse[\"FuseResultsNode\"]\n    v -->|results:vector| fuse\n",
        )
        .expect("gabarit nu");
        let avert = super::statuts_non_cables("t", nu.template());
        assert_eq!(avert.len(), 1, "{avert:?}");
        assert!(avert[0].contains("status non câblé"), "{}", avert[0]);
        // Sans fusion : un signal seul n'a rien à câbler, pas d'avertissement.
        let seul = GraphTool::from_mermaid(
            "%% tool: t\n%% description: d\n%% result: v.results\n\ngraph LR\n    v[\"VectorSearchNode\"]\n",
        )
        .expect("gabarit à signal seul");
        assert!(super::statuts_non_cables("t", seul.template()).is_empty());
    }


    use super::*;

    /// Le refus d'un tableau trop long dit sa taille et la limite, sans le
    /// recopier (ticket de G : 2 Mo de JSON dans le message d'`ingest_snapshot`).
    #[test]
    fn un_refus_de_schema_dit_la_taille_sans_recopier_la_donnee() {
        let schema = json!({"type":"object","properties":{"records":{"type":"array","maxItems":512}}});
        let records: Vec<Value> = (0..20_000).map(|i| json!({"id": i, "texte": "une ligne assez longue"})).collect();
        let args = json!({ "records": records });
        let validator = jsonschema::validator_for(&schema).unwrap();
        let e = validator.validate(&args).unwrap_err();
        let message = schema_refusal(&e);
        assert!(message.len() < 200, "le refus recopie la donnée ({} octets) : {}", message.len(), &message[..message.len().min(300)]);
        assert!(message.contains("20000") && message.contains("512"), "{message}");
        // Une petite valeur reste dite telle quelle : elle aide à comprendre.
        let petite = json!({"records": "pas un tableau"});
        let e = validator.validate(&petite).unwrap_err();
        assert!(schema_refusal(&e).contains("pas un tableau"), "{}", schema_refusal(&e));
    }
    #[test]
    fn search_verbs_run_through_library_without_a_transport() {
        use crate::connection::{CallbackConnection, CypherValue, QueryResult};
        use crate::dataflow::search_chain::{Chain, SearchProgram};
        use crate::embedder::HashEmbedder;
        let (prepared, _) = search_fixture();
        let program = SearchProgram {
            branches: BTreeMap::new(),
            pipeline: Chain::select(
                "Note",
                serde_json::from_value(json!({
                    "field":{"key":"text","value":"$literal"}
                }))
                .unwrap(),
            )
            .page(5, 0)
            .render("tree"),
        };
        let plan = prepared.compile_search_program(&program).unwrap();
        assert_eq!(plan.entity, "Note");
        let conn = CallbackConnection::new(|q, _| {
            Ok(if q.starts_with("MATCH (m:`Note`)") {
                QueryResult {
                    columns: vec![],
                    rows: vec![vec![CypherValue::Map(
                        [
                            ("_uuid".into(), CypherValue::String("note-1".into())),
                            ("text".into(), CypherValue::String("$literal".into())),
                        ]
                        .into(),
                    )]],
                }
            } else {
                QueryResult::default()
            })
        });
        let mut catalog = Catalog::new(
            Box::new(conn),
            Box::new(HashEmbedder::new(8)),
            CatalogConfig {
                allow_mock_embedder: true,
                embedding_dim: 8,
                ..Default::default()
            },
        );
        catalog.initialize().unwrap();
        for (name, config) in &prepared.entities {
            let mut config = config.clone();
            config.signals = crate::search::SearchSignals::NONE;
            catalog.register_entity(name, config).unwrap();
        }
        let backend = Backend {
            decider: None,
            avertissement_sans_service: None,
            prepared: std::sync::RwLock::new(Arc::new(prepared)),
            version: std::sync::atomic::AtomicU64::new(0),
            read_view: {
                catalog.publish_read_view();
                catalog.read_view_cell()
            },
            catalog: Arc::new(Mutex::new(catalog)),
            #[cfg(feature = "code")]
            file_source: None,
            #[cfg(feature = "code")]
            garde: None,
            reactions: Mutex::new(None),
            cell_reactions: Mutex::new(HashMap::new()),
            miroir_reacteur: Mutex::new(None),
            watches: Mutex::new(Vec::new()),
            components_watch: Mutex::new(None),
            propositions: Default::default(),
            reaction_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            runs_in_flight: crate::dataflow::RunsInFlight::new(),
            ecarts_des_fichiers: Mutex::new(None),
        };
        let response = backend.run_search_program(&program).unwrap();
        assert_eq!(response["result"][0]["uuid"], "note-1");
        assert_eq!(response["result"][0]["data"]["text"], "$literal");
        assert_eq!(response["candidate_bounded"], false);
        assert!(response["presentation"].is_string());
        assert_eq!(response["graph_hash"], plan.graph.hash());
    }
    #[test]
    fn notebook_describes_typed_inputs_without_opening_storage() {
        let prepared = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        let descriptor = prepared.describe();
        let tools = descriptor["tools"].as_array().unwrap();
        let put = tools.iter().find(|t| t["name"] == "put_note").unwrap();
        let schema = &put["inputSchema"];
        assert!(schema["properties"].get("entity").is_none());
        assert!(schema["properties"]["record"]["properties"]
            .get("created_at")
            .is_none());
        let validator = jsonschema::validator_for(schema).unwrap();
        let valid =
            json!({"record":{"key":"id","text":"hello","labels":["Blue"],"stage":"working"}});
        assert!(validator.is_valid(&valid));
        let mut invalid = valid.clone();
        invalid["record"]["labels"] = json!("Blue");
        assert!(!validator.is_valid(&invalid));
        invalid = valid.clone();
        invalid["record"]["created_at"] = json!(0);
        assert!(!validator.is_valid(&invalid));
        invalid = valid;
        invalid["entity"] = json!("Snapshot");
        assert!(!validator.is_valid(&invalid));
    }
    fn search_fixture() -> (PreparedBackend, SearchGraphRequest) {
        let p = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        let request = SearchGraphRequest { mermaid: "%% tool: select\n%% description: Select records\n%% param: entity string! -- Entity\n%% param: filter json! -- Filter\n%% result: select.results\ngraph LR\n select[\"SelectRecordsNode(entity=$entity, filter=$filter)\"]".into(), arguments: json!({"entity":"Note","filter":{"must":[]}}), metadata:vec![] };
        (p, request)
    }

    /// Le backend notebook copié dans un dossier jetable, avec les
    /// descriptions du pas produit : l'entité `Note` en gagne une, l'outil
    /// `get_note` une surcharge — `put_note` garde le texte de son gabarit.
    fn fixture_avec_descriptions() -> (tempfile::TempDir, PreparedBackend) {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
        let dir = tempfile::tempdir().unwrap();
        fn copier(src: &Path, dst: &Path) {
            std::fs::create_dir_all(dst).unwrap();
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let cible = dst.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copier(&entry.path(), &cible);
                } else {
                    std::fs::copy(entry.path(), &cible).unwrap();
                }
            }
        }
        copier(&src, dir.path());
        // Le gabarit partagé `../../tools/search_structured.mmd` sort du
        // dossier copié : on le rapatrie et on pointe dessus.
        let partage = src.join("../../tools/search_structured.mmd");
        std::fs::copy(&partage, dir.path().join("search_structured.mmd")).unwrap();
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
        manifest["entities"]["Note"]["description"] = json!(
            "Une note de travail : un texte court sous une clé stable. À prendre quand la question porte sur ce que l'utilisateur a écrit."
        );
        manifest["tools"]["get_note"]["description"] = json!("Relit une note précise par sa clé exacte.");
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let p = PreparedBackend::load(&chemin).unwrap();
        (dir, p)
    }

    /// **La description d'entité se reprend dans les outils** (la leçon du
    /// deck builder : deux outils de même forme ne se distinguent que par
    /// elle) ; la surcharge d'attachement prime sur le gabarit ; sans
    /// surcharge, le gabarit garde son texte ; et le schéma des options de
    /// recherche commence par l'entité.
    #[test]
    fn la_description_d_entite_se_reprend_dans_les_outils() {
        let (_garde, p) = fixture_avec_descriptions();
        let d = p.describe();
        let outil = |nom: &str| -> Value {
            d["tools"].as_array().unwrap().iter().find(|t| t["name"] == nom).unwrap().clone()
        };
        let put = outil("put_note");
        let texte = put["description"].as_str().unwrap();
        assert!(texte.contains("note de travail"), "l'entité visée se dit : {texte}");
        assert!(texte.contains("révision attendue"), "le texte du gabarit reste : {texte}");
        let get = outil("get_note");
        let texte = get["description"].as_str().unwrap();
        assert!(texte.starts_with("Relit une note précise"), "la surcharge prime : {texte}");
        assert!(texte.contains("note de travail"), "et l'entité se dit aussi : {texte}");
        let search = outil("search_notes");
        let texte = search["description"].as_str().unwrap();
        assert!(texte.contains("note de travail"), "le binding `target` suffit à viser : {texte}");
        let filtre = &search["inputSchema"]["properties"]["options"]["properties"]["filter_condition"]["description"];
        assert!(
            filtre.as_str().unwrap().starts_with("Une note de travail"),
            "le schéma du filtre commence par l'entité : {filtre}"
        );
    }

    /// Le notebook copié avec un workspace et les outils de code attachés —
    /// deux manifestes du même moteur, la politique seule les distingue.
    #[cfg(feature = "code")]
    fn fixture_code(
        source: &str,
        read_only: bool,
        outils: Value,
    ) -> (tempfile::TempDir, Result<PreparedBackend, String>) {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
        let dir = tempfile::tempdir().unwrap();
        fn copier(src: &Path, dst: &Path) {
            std::fs::create_dir_all(dst).unwrap();
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let cible = dst.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copier(&entry.path(), &cible);
                } else {
                    std::fs::copy(entry.path(), &cible).unwrap();
                }
            }
        }
        copier(&src, dir.path());
        let outils_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
        for mmd in ["read.mmd", "edit.mmd", "run.mmd"] {
            std::fs::copy(outils_dir.join(mmd), dir.path().join(mmd)).unwrap();
        }
        std::fs::copy(src.join("../../tools/search_structured.mmd"), dir.path().join("search_structured.mmd")).unwrap();
        let sources = dir.path().join("sources");
        std::fs::create_dir(&sources).unwrap();
        std::fs::write(sources.join("main.rs"), "fn main() { depart(); }\n").unwrap();
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
        manifest["workspace"] = json!({"source": source, "root": "sources", "read_only": read_only});
        for (nom, attachement) in outils.as_object().unwrap() {
            manifest["tools"][nom] = attachement.clone();
        }
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let p = PreparedBackend::load(&chemin);
        (dir, p)
    }

    /// [`fixture_code`], plus un patch libre du manifeste.
    #[cfg(feature = "code")]
    fn fixture_code_avec(
        source: &str,
        read_only: bool,
        outils: Value,
        patch: &dyn Fn(&mut Value),
    ) -> (tempfile::TempDir, Result<PreparedBackend, String>) {
        let (dir, _) = fixture_code(source, read_only, outils);
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        patch(&mut manifest);
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let p = PreparedBackend::load(&chemin);
        (dir, p)
    }

    /// Un Backend sans base réelle, comme `search_verbs_…` : le catalogue
    /// répond à vide, la source vient du manifeste — ce qu'on éprouve est la
    /// politique et le chemin des fichiers, pas la persistance.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn backend_de_code(dir: &Path, prepared: PreparedBackend) -> Backend {
        use crate::embedder::HashEmbedder;
        // Une VRAIE base en mémoire : depuis que `edit` réindexe par la
        // synchronisation déclarée, la session fine s'écrit puis se RELIT en
        // base — une connexion factice qui rend des résultats vides la
        // trouvait « fermée » (le rouge du 3 octobre au soir ; le scénario
        // de tuyauterie, sur une vraie base, passait). Trouvé par ce test.
        let conn = crate::Rag3dbConnection::in_memory().expect("base en mémoire");
        conn.execute(&format!("LOAD EXTENSION '{}'", extension_vector_de_test()))
        .unwrap();
        let mut catalog = Catalog::new(
            Box::new(conn),
            Box::new(HashEmbedder::new(8)),
            CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() },
        );
        catalog.initialize().unwrap();
        // Les nœuds de code consultent File/Scope : le schéma de code
        // s'enregistre, la connexion à vide absorbe les DDL.
        crate::code::register_code_schema(&mut catalog, crate::code::default_scope_chunking()).unwrap();
        let workspace = prepared.manifest.workspace.clone().unwrap();
        let file_source =
            crate::backend_code::build_source(&workspace, dir, &prepared.manifest.name).unwrap();
        Backend {
            decider: None,
            avertissement_sans_service: None,
            prepared: std::sync::RwLock::new(Arc::new(prepared)),
            version: std::sync::atomic::AtomicU64::new(0),
            read_view: {
                catalog.publish_read_view();
                catalog.read_view_cell()
            },
            catalog: Arc::new(Mutex::new(catalog)),
            file_source: Some(file_source),
            garde: crate::backend_code::build_garde(&workspace),
            reactions: Mutex::new(None),
            cell_reactions: Mutex::new(HashMap::new()),
            miroir_reacteur: Mutex::new(None),
            watches: Mutex::new(Vec::new()),
            components_watch: Mutex::new(None),
            propositions: Default::default(),
            reaction_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            runs_in_flight: crate::dataflow::RunsInFlight::new(),
            ecarts_des_fichiers: Mutex::new(None),
        }
    }


    /// **Les outils de lecture rendent du JSON sur demande, et le même
    /// markdown sans elle** (l'IDE, page 05 §3.1, lot A1). Six graphes de
    /// lecture déclarent `format` (défaut `markdown`) ; la recherche rendait
    /// déjà sa liste structurée. Sans le paramètre et
    /// avec `format=markdown`, la réponse est la même octet pour octet — ce
    /// que lisent les agents ne bouge pas ; avec `format=json`, elle est
    /// structurée et porte ce dont une interface fait des liens (chemins,
    /// lignes, uuid).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn the_read_tools_render_json_on_request_and_the_same_markdown_without() {
        let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
        let graphes = ["read.mmd", "list.mmd", "grep.mmd", "usages.mmd", "impact.mmd", "callees.mmd", "search_workspace.mmd"];
        let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
            for (outil, graphe) in [
                ("list_files", "list.mmd"),
                ("grep_files", "grep.mmd"),
                ("usages", "usages.mmd"),
                ("impact", "impact.mmd"),
                ("callees", "callees.mmd"),
            ] {
                m["tools"][outil] = json!({"graph": graphe, "policy": {"read_files": true}});
            }
            m["tools"]["read_file"] = json!({"graph": "read.mmd", "policy": {"read_files": true}});
            m["tools"]["search_code"] =
                json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
        });
        drop(p);
        for g in graphes {
            std::fs::copy(tools.join(g), dir.path().join(g)).unwrap();
        }
        std::fs::write(dir.path().join("sources/outils.rs"), "pub fn depart() {}\n").unwrap();
        let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("le manifeste se charge");
        let backend = backend_de_code(dir.path(), prepared);
        {
            let source = backend.file_source.clone().unwrap();
            let mut cat = backend.catalog.lock().unwrap();
            crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).expect("l'index");
        }
        let appels: [(&str, Value, &[&str]); 6] = [
            ("read_file", json!({"path": "outils.rs"}), &["\"scopes\"", "\"uuid\"", "\"start_line\""]),
            ("list_files", json!({}), &["\"path\"", "outils.rs", "\"indexed\""]),
            ("grep_files", json!({"pattern": "depart"}), &["\"path\"", "\"line\"", "\"uuid\""]),
            ("usages", json!({"name": "depart"}), &["\"uuid\"", "\"path\"", "outils.rs"]),
            // `main.rs` n'appelle `depart` que par le nom : impact et callees
            // le comptent sans le lister ; leur départ, lui, est structuré.
            ("impact", json!({"name": "depart"}), &["\"starts\"", "\"uuid\"", "outils.rs"]),
            ("callees", json!({"name": "main"}), &["\"starts\"", "\"uuid\"", "main.rs"]),
        ];
        for (outil, args, attendus) in appels {
            let avec = |format: Option<&str>| {
                let mut a = args.clone();
                if let Some(f) = format {
                    a["format"] = json!(f);
                }
                backend.call(outil, a).unwrap_or_else(|e| panic!("{outil} {format:?} : {e}"))
            };
            let nu = avec(None);
            let markdown = avec(Some("markdown"));
            assert!(nu["result"].is_string(), "{outil} sans format rend du markdown : {nu}");
            assert_eq!(nu, markdown, "{outil} : sans format = format=markdown, la réponse entière");
            let structure = avec(Some("json"));
            assert!(
                structure["result"].is_object() || structure["result"].is_array(),
                "{outil} en json rend une structure : {}",
                structure["result"]
            );
            let texte = structure["result"].to_string();
            for a in attendus {
                assert!(texte.contains(a), "{outil} en json porte {a} : {}", &texte[..texte.len().min(600)]);
            }
        }
        // La recherche n'a pas besoin du paramètre : son `result` est déjà la
        // liste structurée (uuid, chemin, lignes), son markdown va à
        // `presentation`.
        let r = backend.call("search_code", json!({"query": "depart", "options": {}})).unwrap();
        assert!(r["result"].is_array() && r["presentation"].is_string(), "{r}");
        let texte = r["result"].to_string();
        assert!(texte.contains("\"uuid\"") && texte.contains("\"file_path\"") && texte.contains("\"start_line\""), "{texte}");
        // Un format inconnu est refusé, pas rendu en markdown sans le dire.
        assert!(backend.call("read_file", json!({"path": "outils.rs", "format": "html"})).is_err());
    }

    /// **Chaque nœud atteint dit d'où il vient** (`from`, l'IDE, lot A5) :
    /// une chaîne `a → b → c` dans un fichier (des appels résolus), et le JSON
    /// d'impact et de callees relie chaque niveau au précédent — ce qui fait un
    /// graphe du voisinage plutôt que des anneaux. Le markdown ne bouge pas.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn each_reached_node_says_where_it_comes_from() {
        let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
        let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
            m["tools"]["impact"] = json!({"graph": "impact.mmd", "policy": {"read_files": true}});
            m["tools"]["callees"] = json!({"graph": "callees.mmd", "policy": {"read_files": true}});
        });
        drop(p);
        for g in ["impact.mmd", "callees.mmd"] {
            std::fs::copy(tools.join(g), dir.path().join(g)).unwrap();
        }
        std::fs::write(dir.path().join("sources/chaine.rs"), "fn c() {}\nfn b() { c(); }\nfn a() { b(); }\n").unwrap();
        let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("le manifeste se charge");
        let backend = backend_de_code(dir.path(), prepared);
        {
            let source = backend.file_source.clone().unwrap();
            let mut cat = backend.catalog.lock().unwrap();
            crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).expect("l'index");
        }
        // Un nom vers son uuid, d'après un rapport (départs et atteints).
        let uuid_de = |r: &Value, nom: &str| -> String {
            r["starts"].as_array().unwrap().iter().chain(r["reached"].as_array().unwrap())
                .find(|x| x["title"] == nom).map(|x| x["uuid"].as_str().unwrap().to_string())
                .unwrap_or_else(|| panic!("{nom} absent : {r}"))
        };
        let niveau = |r: &Value, nom: &str| -> (u64, String) {
            let x = r["reached"].as_array().unwrap().iter().find(|x| x["title"] == nom).unwrap_or_else(|| panic!("{nom} non atteint : {r}"));
            (x["level"].as_u64().unwrap(), x["from"].as_str().unwrap_or_else(|| panic!("{nom} sans from : {x}")).to_string())
        };
        for (outil, depart, premier, second) in [("callees", "a", "b", "c"), ("impact", "c", "b", "a")] {
            let r = backend.call(outil, json!({"name": depart, "format": "json"})).unwrap()["result"].clone();
            let (n1, de1) = niveau(&r, premier);
            let (n2, de2) = niveau(&r, second);
            assert_eq!((n1, n2), (1, 2), "{outil} : {r}");
            assert_eq!(de1, uuid_de(&r, depart), "{outil} : {premier} vient du départ");
            assert_eq!(de2, uuid_de(&r, premier), "{outil} : {second} vient de {premier}");
            let md = backend.call(outil, json!({"name": depart})).unwrap();
            assert!(md["result"].is_string() && !md["result"].as_str().unwrap().contains("\"from\""), "le markdown ne change pas");
        }
    }

    /// Un backend de code qui **surveille son workspace** : la montre `code`
    /// (période courte), la réaction `reindex_on_change` du gabarit code et
    /// l'outil `usages`. `sources/outils.rs` définit `depart`, `main.rs`
    /// l'appelle.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn backend_qui_surveille() -> (tempfile::TempDir, Backend) {
        let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
            m["watches"] = json!({"code": {"source": "workspace", "every_ms": 20, "settle_ms": 0}});
            m["reactions"] = json!({"reindex_on_change": {"graph": "reindex_on_change.mmd"}});
            m["tools"]["usages"] = json!({"graph": "usages.mmd"});
        });
        drop(p);
        std::fs::copy(tools.join("backends/code/graphs/reindex_on_change.mmd"), dir.path().join("reindex_on_change.mmd")).unwrap();
        std::fs::copy(tools.join("tools/usages.mmd"), dir.path().join("usages.mmd")).unwrap();
        std::fs::write(dir.path().join("sources/outils.rs"), "pub fn depart() {}\n").unwrap();
        let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("le manifeste qui surveille se charge");
        let backend = backend_de_code(dir.path(), prepared);
        {
            let source = backend.file_source.clone().unwrap();
            let mut cat = backend.catalog.lock().unwrap();
            crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).expect("l'index de départ");
        }
        (dir, backend)
    }

    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn usages_de(backend: &Backend, name: &str) -> String {
        backend.call("usages", json!({"name": name})).map(|v| v.to_string()).unwrap_or_else(|e| e)
    }

    /// Attend (10 s au plus) que `ok` tienne sur ce que rend `usages`.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn attendre_les_usages(backend: &Backend, name: &str, ok: impl Fn(&str) -> bool) -> String {
        // Sondé toutes les 200 ms : une lecture partage désormais le verrou avec
        // la réaction (elle lui cède), une sonde trop serrée la ralentirait.
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let u = usages_de(backend, name);
            if ok(&u) || std::time::Instant::now() > fin {
                return u;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    /// **L'index suit le disque** (W2) : un fichier édité à la main — hors de
    /// nos outils — et `usages` voit le nouvel appel sans `index` manuel ; un
    /// fichier supprimé, et ses usages disparaissent. La montre et la
    /// réaction sont montées par le backend, comme à l'ouverture.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn the_index_follows_files_edited_by_hand() {
        let (dir, backend) = backend_qui_surveille();
        let montees = backend.mount_reactions().expect("montre et réaction se montent");
        assert_eq!(montees, ["reindex_on_change"]);
        let avant = usages_de(&backend, "depart");
        assert!(avant.contains("main") && !avant.contains("ailleurs"), "{avant}");

        // Une édition à la main : un second appelant.
        std::fs::write(dir.path().join("sources/main.rs"), "fn main() { depart(); }\nfn ailleurs() { depart(); }\n").unwrap();
        let apres = attendre_les_usages(&backend, "depart", |u| u.contains("ailleurs"));
        assert!(apres.contains("ailleurs"), "l'édition n'a pas été réindexée : {apres}");

        // **Le disque décide, pas l'événement** : un « removed » forgé pour
        // `outils.rs`, qui existe, ne le retire pas. Une édition qui le suit
        // prouve que le lot qui le portait est passé.
        backend.catalog.lock().unwrap().event_bus().emit(crate::events::Event::FileChanged {
            watch: "code".into(),
            path: "outils.rs".into(),
            change: "removed".into(),
            hash: String::new(),
            at_ms: 0,
        });
        std::fs::write(dir.path().join("sources/main.rs"), "fn main() { depart(); }\nfn ailleurs() { depart(); }\nfn encore() { depart(); }\n").unwrap();
        let suivi = attendre_les_usages(&backend, "depart", |u| u.contains("encore"));
        assert!(suivi.contains("encore") && suivi.contains("outils.rs"), "un removed forgé a retiré un fichier présent : {suivi}");

        // Le fichier supprimé : ses usages partent.
        std::fs::remove_file(dir.path().join("sources/main.rs")).unwrap();
        let sans = attendre_les_usages(&backend, "depart", |u| !u.contains("ailleurs") && !u.contains("main.rs"));
        assert!(!sans.contains("ailleurs") && !sans.contains("main.rs"), "le fichier supprimé est resté : {sans}");

        let erreurs = backend.stop_reactions();
        assert!(erreurs.is_empty(), "{erreurs:?}");
    }


    /// **Le retard se dit** (W3) : la montre en service, sans sa réaction —
    /// une édition à la main n'est pas réindexée. L'état de l'index dit le
    /// retard, et `usages` le dit en tête de sa réponse au lieu de rendre
    /// l'index d'avant comme sûr. La réaction montée, une nouvelle édition
    /// est réindexée et le retard tombe.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn the_lag_is_said_between_an_edit_and_its_reindexing() {
        let (dir, backend) = backend_qui_surveille();
        assert_eq!(backend.mount_watches().unwrap(), ["code"]);
        assert!(!usages_de(&backend, "depart").contains("pas encore réindexé"));

        std::fs::write(dir.path().join("sources/main.rs"), "fn main() { depart(); }\nfn ailleurs() { depart(); }\n").unwrap();
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let etat = loop {
            let e = backend.index_states().unwrap();
            if e["lag"]["files"] == 1 || std::time::Instant::now() > fin {
                break e;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        assert_eq!(etat["lag"]["files"], 1, "l'état ne dit pas le retard : {etat}");
        let u = usages_de(&backend, "depart");
        assert!(u.contains("1 fichier changé") && u.contains("pas encore réindexé"), "{u}");
        assert!(!u.contains("ailleurs"), "rien n'est réindexé sans la réaction : {u}");

        // Une cellule voisine n'a pas ce retard, et le sien ne déborde pas ici.
        let voisine = crate::scope::Scope { org: "voisine".into(), project: "p".into() };
        let lag = backend.catalog.lock().unwrap().file_lag();
        assert_eq!(lag.summary_in(&voisine), None);
        lag.note(&voisine, "autre.rs", 0);
        let u = usages_de(&backend, "depart");
        assert!(u.contains("1 fichier changé"), "seul le retard de la cellule de l'appel compte : {u}");
        lag.done(&voisine, "autre.rs", u64::MAX);

        // La réaction montée : l'édition suivante est réindexée, le retard tombe.
        backend.mount_reactions().unwrap();
        std::fs::write(dir.path().join("sources/main.rs"), "fn main() { depart(); }\nfn ailleurs() { depart(); }\nfn encore() { depart(); }\n").unwrap();
        let apres = attendre_les_usages(&backend, "depart", |u| u.contains("encore") && !u.contains("pas encore réindexé"));
        assert!(apres.contains("encore") && !apres.contains("pas encore réindexé"), "{apres}");
        assert!(backend.index_states().unwrap().get("lag").is_none());
        assert!(backend.stop_reactions().is_empty());
    }

    /// **Chaque outil de lecture dit sa fraîcheur** (graine de l'hôte d'équipe) : un dépôt propre
    /// donne « index : … @ <commit> de <branche>, indexé à l'instant, à jour » en tête et le champ
    /// `freshness` ; un fichier changé à la main (la montre le voit, sans réaction) apparaît en
    /// retard ; la réaction montée, il est rattrapé et le retard tombe.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn les_outils_de_lecture_disent_leur_fraicheur() {
        let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
            m["watches"] = json!({"code": {"source": "workspace", "every_ms": 20, "settle_ms": 0}});
            m["reactions"] = json!({"reindex_on_change": {"graph": "reindex_on_change.mmd"}});
            m["tools"]["usages"] = json!({"graph": "usages.mmd", "freshness": true});
            m["tools"]["read_file"] = json!({"graph": "read.mmd", "policy": {"read_files": true}, "freshness": true});
        });
        drop(p);
        std::fs::copy(tools.join("backends/code/graphs/reindex_on_change.mmd"), dir.path().join("reindex_on_change.mmd")).unwrap();
        std::fs::copy(tools.join("tools/usages.mmd"), dir.path().join("usages.mmd")).unwrap();
        std::fs::copy(tools.join("tools/read.mmd"), dir.path().join("read.mmd")).unwrap();
        let racine = dir.path().join("sources");
        std::fs::write(racine.join("outils.rs"), "pub fn depart() {}\n").unwrap();
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(&racine)
                .args(["-c", "user.email=t@t", "-c", "user.name=t", "-c", "init.defaultBranch=principale"])
                .args(args)
                .output()
                .unwrap();
            assert!(ok.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&ok.stderr));
            String::from_utf8_lossy(&ok.stdout).trim().to_string()
        };
        git(&["init", "-q"]);
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "départ"]);
        let commit = git(&["rev-parse", "--short", "HEAD"]);
        let branche = git(&["rev-parse", "--abbrev-ref", "HEAD"]);
        let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("le manifeste se charge");
        let backend = backend_de_code(dir.path(), prepared);
        {
            let source = backend.file_source.clone().unwrap();
            let mut cat = backend.catalog.lock().unwrap();
            crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).expect("l'index de départ");
            let cell = cat.scope().clone();
            cat.note_indexed_tree(&cell, &crate::catalog::indexed_tree_of(&racine.to_string_lossy())).unwrap();
        }
        let lire = |format: &str| backend.call("read_file", json!({"path": "outils.rs", "format": format})).unwrap();

        // Un dépôt propre.
        let md = lire("markdown");
        let texte = md["result"].as_str().unwrap();
        let attendu = format!("> index : {} @ {commit} de {branche}, indexé à l'instant, à jour\n\n", racine.to_string_lossy());
        assert!(texte.starts_with(&attendu), "en tête attendu {attendu:?} : {texte}");
        assert_eq!(md["freshness"]["commit"], commit);
        assert_eq!(md["freshness"]["branch"], branche);
        assert_eq!(md["freshness"]["lag_files"], 0);
        let js = lire("json");
        assert!(js["result"].is_object() || js["result"].is_array(), "le résultat JSON reste structuré : {js}");
        assert_eq!(js["freshness"]["dirty"], false);
        // `usages` a déjà sa ligne : pas de doublon.
        let u = backend.call("usages", json!({"name": "depart"})).unwrap();
        let u = u["result"].as_str().unwrap_or_default().to_string();
        assert_eq!(u.matches("> index :").count(), 1, "{u}");
        assert!(u.contains(&format!("@ {commit} de {branche}")), "{u}");

        // Un fichier modifié : la montre le voit, il est en retard.
        assert_eq!(backend.mount_watches().unwrap(), ["code"]);
        std::fs::write(racine.join("outils.rs"), "pub fn depart() {}\npub fn arrivee() {}\n").unwrap();
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let en_retard = loop {
            let r = lire("markdown");
            if r["freshness"]["lag_files"] == 1 || std::time::Instant::now() > fin {
                break r;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        assert_eq!(en_retard["freshness"]["lag_files"], 1, "{en_retard}");
        assert!(en_retard["result"].as_str().unwrap().contains("1 fichier changé"), "{en_retard}");

        // Rattrapé : la réaction réindexe, le retard tombe.
        backend.mount_reactions().unwrap();
        std::fs::write(racine.join("outils.rs"), "pub fn depart() {}\npub fn arrivee() {}\npub fn encore() {}\n").unwrap();
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let rattrape = loop {
            let r = lire("markdown");
            let fait = r["freshness"]["lag_files"] == 0 && r["result"].as_str().unwrap_or_default().contains("encore");
            if fait || std::time::Instant::now() > fin {
                break r;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        assert_eq!(rattrape["freshness"]["lag_files"], 0, "{rattrape}");
        assert!(rattrape["result"].as_str().unwrap().contains(", à jour"), "{rattrape}");
        assert!(backend.stop_reactions().is_empty());
    }

    /// **La synchronisation efface le retard en relisant** : un retard noté
    /// avant une synchronisation de la source tombe quand elle relit le
    /// fichier — c'est elle qui reprend ce qu'une réaction a mis en reprise.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_sync_clears_the_lag_of_the_files_it_reads() {
        let (_dir, backend) = backend_qui_surveille();
        let (lag, cell) = {
            let cat = backend.catalog.lock().unwrap();
            (cat.file_lag(), cat.scope().clone())
        };
        lag.note(&cell, "outils.rs", crate::catalog::retard::now_ms());
        assert_eq!(lag.summary_in(&cell).map(|s| s.files), Some(1));
        let source = backend.file_source.clone().unwrap();
        let mut cat = backend.catalog.lock().unwrap();
        crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).unwrap();
        assert_eq!(lag.summary_in(&cell), None, "relu par la synchronisation, le retard tombe");
    }

    /// **Un fichier créé est vu** : la montre relit la liste de la source à
    /// chaque balayage, pas celle de l'index — un fichier neuf, avec une
    /// définition et son appelant, et `usages` les voit sans `index`.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_file_created_by_hand_is_indexed() {
        let (dir, backend) = backend_qui_surveille();
        backend.mount_reactions().unwrap();
        assert!(usages_de(&backend, "arrivee").contains("Définitions (0)"));
        std::fs::write(dir.path().join("sources/nouveau.rs"), "pub fn arrivee() {}\nfn appelant() { arrivee(); }\n").unwrap();
        let u = attendre_les_usages(&backend, "arrivee", |u| u.contains("appelant"));
        assert!(u.contains("nouveau.rs") && u.contains("appelant"), "le fichier créé n'a pas été indexé : {u}");
        assert!(backend.stop_reactions().is_empty());
    }

    /// **La montre réconcilie au démarrage** : l'index est fait, des fichiers
    /// changent pendant que rien ne surveille (la montre n'est pas encore
    /// montée) ; au démarrage de la montre, le disque est comparé à
    /// l'empreinte que l'index garde par fichier, et `usages` voit l'édition
    /// et la suppression sans `index` (la base de Lucie, le 11 octobre).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn the_watch_reconciles_what_moved_before_it_started() {
        let (dir, backend) = backend_qui_surveille();
        std::fs::write(dir.path().join("sources/main.rs"), "fn main() { depart(); }\nfn pendant() { depart(); }\n").unwrap();
        std::fs::write(dir.path().join("sources/neuf.rs"), "fn neuf() { depart(); }\n").unwrap();
        backend.mount_reactions().unwrap();
        let u = attendre_les_usages(&backend, "depart", |u| u.contains("pendant") && u.contains("neuf"));
        assert!(u.contains("pendant") && u.contains("neuf"), "ce qui a bougé avant la montre n'est pas repris : {u}");
        // Ce que la montre coûte se lit dans l'état.
        let montre = &backend.index_states().unwrap()["watches"]["code"];
        assert!(montre["scans"].as_u64().unwrap_or(0) > 0, "{montre}");
        assert!(backend.stop_reactions().is_empty());
        // Et une suppression faite pendant l'arrêt : reprise au redémarrage.
        std::fs::remove_file(dir.path().join("sources/neuf.rs")).unwrap();
        backend.mount_reactions().unwrap();
        let u = attendre_les_usages(&backend, "depart", |u| !u.contains("neuf"));
        assert!(!u.contains("neuf"), "la suppression faite à l'arrêt n'est pas reprise : {u}");
        assert!(backend.stop_reactions().is_empty());
    }

    /// **Un changement localisé ne fait pas refuser une lecture** : le
    /// catalogue tenu le temps d'une réindexation (ici 300 ms), `usages` attend
    /// et répond — « catalogue occupé » est réservé à une tenue longue.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_short_hold_of_the_catalog_does_not_refuse_a_read() {
        let (_dir, backend) = backend_qui_surveille();
        let catalog = backend.catalog.clone();
        let (pris, prends) = std::sync::mpsc::channel();
        let tenu = std::thread::spawn(move || {
            let _g = catalog.lock().unwrap();
            pris.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(300));
        });
        prends.recv().unwrap();
        let u = usages_de(&backend, "depart");
        tenu.join().unwrap();
        assert!(!u.contains("Catalogue occupé") && u.contains("main"), "{u}");
    }

    /// **Le reçu « occupé » dit encore ce qui se lit sans le catalogue** : une
    /// indexation qui tient le catalogue plus longtemps que l'attente d'une
    /// lecture rend `busy`, mais le coût des montres se lit quand même — un
    /// état qui le perdait faisait rougir une fois sur cinq la réconciliation
    /// (ticket du 11 octobre, trouvé par G).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_busy_index_state_still_tells_the_watches() {
        let (_dir, backend) = backend_qui_surveille();
        backend.mount_reactions().unwrap();
        let catalog = backend.catalog.clone();
        let (pris, prends) = std::sync::mpsc::channel();
        let tenu = std::thread::spawn(move || {
            let _g = catalog.lock().unwrap();
            pris.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2500));
        });
        prends.recv().unwrap();
        let etat = backend.index_states().unwrap();
        tenu.join().unwrap();
        assert_eq!(etat["busy"], true, "le catalogue tenu ne rend pas busy : {etat}");
        assert!(etat["watches"]["code"].is_object(), "le reçu occupé perd les montres : {etat}");
        assert!(backend.stop_reactions().is_empty());
    }

    /// **La ligne [montre] dit l'embarquement sans la variable du profil** :
    /// les durées des nœuds ne s'écoutaient que sous `RAG3WEAVER_INGEST_PROFILE`,
    /// et « embarquer » manquait à toute ligne de production (relecture de la
    /// recherche, 12 octobre).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_recorded_reingestion_tells_the_embedding_without_the_profile() {
        assert!(!crate::ingest_profile::enabled(), "le témoin se joue sans RAG3WEAVER_INGEST_PROFILE");
        let (dir, backend) = backend_qui_surveille();
        let source = backend.file_source.clone().unwrap();
        let contenu = "pub fn neuve() { depart(); }\npub fn partante() { depart(); }\n";
        std::fs::write(dir.path().join("sources/neuve.rs"), contenu).unwrap();
        let mut cat = backend.catalog.lock().unwrap();
        let (r, etapes) = crate::ingest_profile::record(|| {
            crate::code_tools::reingest_file(&mut cat, source.as_ref(), "neuve.rs", contenu)
        });
        r.unwrap();
        assert!(etapes.iter().any(|(e, _)| *e == "nœud embed"), "pas d'embarquement dans les étapes : {etapes:?}");
        // Une fonction retirée supprime son scope : la suppression du drain se
        // chronomètre aussi, sans la variable.
        let modifie = "pub fn neuve() { depart(); }\n";
        std::fs::write(dir.path().join("sources/neuve.rs"), modifie).unwrap();
        let (r, etapes) = crate::ingest_profile::record(|| {
            crate::code_tools::reingest_file(&mut cat, source.as_ref(), "neuve.rs", modifie)
        });
        r.unwrap();
        assert!(etapes.iter().any(|(e, _)| *e == "drain · suppressions"), "pas de suppression du drain dans les étapes : {etapes:?}");
        // Ce qui courait sans nom : la fin d'instantané et les symboles.
        for attendue in [
            "instantané · mettre de côté",
            "instantané · noter la fin",
            "symboles · mise en file DEFINES/MENTIONS",
            "drain · construction du graphe",
            // Ce que la relecture des arêtes fait, instruction par instruction.
            "symboles · lire les DEFINES",
            "symboles · lire les MENTIONS",
            "symboles · parents et fichiers",
            "symboles · choisir et lier",
        ] {
            assert!(etapes.iter().any(|(e, _)| *e == attendue), "« {attendue} » manque aux étapes : {etapes:?}");
        }
    }

    /// **La fin d'un fichier atteint ses scopes par l'arête** : depuis la clé
    /// du `File`, par `DEFINED_IN`, et non en balayant `Scope` sur
    /// `file_path` — un balayage qui grandit avec la base (546 ms pour un
    /// fichier sur 102 469 morceaux, 12 octobre). Le plan trouve exactement les
    /// lignes que le filtre sur les champs trouverait.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn the_fine_plan_reaches_the_scopes_of_a_file_by_its_edge() {
        use crate::catalog::SnapshotFinishOptions;
        use crate::connection::{CypherValue, QueryParam};
        let (_dir, backend) = backend_qui_surveille();
        let source = backend.file_source.clone().unwrap();
        let mut cat = backend.catalog.lock().unwrap();
        let (source_id, chemin) = crate::code_tools::indexed_name(source.as_ref(), "main.rs");
        let grain = crate::code_sync::grain_du_fichier(&source_id, &chemin);
        let open = cat.begin_snapshot(crate::code::SCOPE, &grain, true).unwrap();
        let plan = cat
            .plan_snapshot_finish(crate::code::SCOPE, &grain, &open.session, SnapshotFinishOptions { allow_empty: true, force: true })
            .unwrap();
        cat.abort_snapshot(crate::code::SCOPE, &grain, &open.session).unwrap();
        assert_eq!(plan.by_relation.as_deref(), Some("DEFINED_IN"), "le plan n'est pas passé par l'arête");
        let par_champs = cat
            .execute_raw_with_params(
                "MATCH (n:Scope) WHERE n.source = $s AND n.file_path = $p RETURN count(n)",
                &[QueryParam::new("s", CypherValue::String(source_id.clone())), QueryParam::new("p", CypherValue::String(chemin.clone()))],
            )
            .unwrap()
            .rows[0][0]
            .as_i64()
            .unwrap() as usize;
        assert!(par_champs > 0, "le fichier n'a pas de scope : le témoin ne prouverait rien");
        assert_eq!(plan.in_scope, par_champs, "le plan par l'arête ne trouve pas les lignes du filtre");
    }

    /// **Un lot de fichiers se réindexe en un passage**, et l'index en sort
    /// tel que des réindexations une à une l'auraient laissé : deux fichiers
    /// modifiés (une fonction retirée, une ajoutée), un supprimé, vus ensemble.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_batch_of_files_is_reindexed_in_one_pass() {
        use crate::connection::{CypherValue, QueryParam};
        let (dir, backend) = backend_qui_surveille();
        let s = dir.path().join("sources");
        std::fs::write(s.join("a.rs"), "pub fn a1() { depart(); }\npub fn a2() { depart(); }\n").unwrap();
        std::fs::write(s.join("b.rs"), "pub fn b1() { depart(); }\n").unwrap();
        std::fs::write(s.join("c.rs"), "pub fn c1() { depart(); }\n").unwrap();
        let source = backend.file_source.clone().unwrap();
        let mut cat = backend.catalog.lock().unwrap();
        crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).unwrap();
        let a = "pub fn a1() { depart(); }\n";
        let b = "pub fn b1() { depart(); }\npub fn b2() { depart(); }\n";
        std::fs::write(s.join("a.rs"), a).unwrap();
        std::fs::write(s.join("b.rs"), b).unwrap();
        std::fs::remove_file(s.join("c.rs")).unwrap();
        let lot = [("a.rs".to_string(), Some(a.to_string())), ("b.rs".to_string(), Some(b.to_string())), ("c.rs".to_string(), None)];
        let rapport = crate::code_sync::reingest_files(&mut cat, source.as_ref(), &lot, crate::disponibilite::Disponibilites::TOUT).unwrap();
        assert!(rapport.deferred_to.is_none(), "{rapport:?}");
        let (source_id, _) = crate::code_tools::indexed_name(source.as_ref(), "a.rs");
        let noms = |chemin: &str| -> Vec<String> {
            let mut v: Vec<String> = cat
                .execute_raw_with_params(
                    "MATCH (n:Scope) WHERE n.source = $s AND n.file_path = $p RETURN n.name",
                    &[QueryParam::new("s", CypherValue::String(source_id.clone())), QueryParam::new("p", CypherValue::String(chemin.into()))],
                )
                .unwrap()
                .rows
                .iter()
                .filter_map(|r| r[0].as_str().map(str::to_string))
                .collect();
            v.sort();
            v
        };
        assert_eq!(noms("a.rs"), ["a1"], "a2 retirée");
        assert_eq!(noms("b.rs"), ["b1", "b2"], "b2 ajoutée");
        assert!(noms("c.rs").is_empty(), "c.rs supprimé : plus de scope");
        let fichiers = cat
            .execute_raw_with_params(
                "MATCH (f:File) WHERE f.source = $s AND f.path = 'c.rs' RETURN count(f)",
                &[QueryParam::new("s", CypherValue::String(source_id.clone()))],
            )
            .unwrap()
            .rows[0][0]
            .as_i64();
        assert_eq!(fichiers, Some(0), "c.rs supprimé : plus de ligne File");
        assert_eq!(rapport.scopes_deleted, 2, "a2 et c1 retirées : {rapport:?}");
    }

    /// **La résolution d'un lot ne relit que ce qui le concerne** : trente
    /// fichiers appellent `commun()`, un seul est réingéré. Une mention ne
    /// devient une arête que si sa cible est un scope du lot ou son
    /// mentionneur en est un ; `commun` est défini hors du lot, donc seules
    /// les mentions du fichier réingéré comptent, pas les trente (sur la base
    /// entière, relire toutes les mentions des noms courants coûtait ~0,7 s).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_batch_resolution_reads_only_the_mentions_that_concern_it() {
        let (dir, backend) = backend_qui_surveille();
        let s = dir.path().join("sources");
        std::fs::write(s.join("commun.rs"), "pub fn commun() {}\n").unwrap();
        for i in 0..30 {
            std::fs::write(s.join(format!("f{i}.rs")), format!("pub fn f{i}() {{ commun(); }}\n")).unwrap();
        }
        let source = backend.file_source.clone().unwrap();
        let mut cat = backend.catalog.lock().unwrap();
        crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).unwrap();
        let contenu = "pub fn f0() { commun(); commun(); }\n";
        let cursor = source.cursor();
        let root = crate::workspace_path::worktree_root(&cursor).unwrap().to_string();
        let projet = source.list().ok();
        let mut analyse = crate::code::analyze_in_project(&root, vec![("f0.rs".to_string(), contenu.to_string())], &cursor, projet.as_deref());
        for f in &mut analyse.files {
            f.cursor = cursor.clone();
        }
        let rapport = cat.ingest_code_jusqu_a(&analyse, crate::disponibilite::Disponibilites::TOUT).unwrap();
        assert!(rapport.mentions_read <= 3, "la résolution a relu {} mentions pour un fichier : {rapport:?}", rapport.mentions_read);
        // Et l'arête de f0 vers commun est là, comme celles des autres.
        drop(cat);
        let u = usages_de(&backend, "commun");
        assert!(u.contains("Usages (30)") && u.contains("f0.rs"), "{u}");
    }

    /// Les fichiers que l'index entier écarte, et un fichier admis qui sert de
    /// témoin de passage : un dossier `generated`, un marqueur « DO NOT EDIT »
    /// en tête, un binaire (des octets nuls, sous une extension qu'aucune règle
    /// ne connaît : les `.wal` de l'hôte de Lucie).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn poser_des_exclus_et_un_admis(sources: &Path) {
        std::fs::create_dir_all(sources.join("generated")).unwrap();
        std::fs::write(sources.join("generated/genere.rs"), "pub fn genere() { depart(); }\n").unwrap();
        std::fs::write(sources.join("marque.rs"), "// DO NOT EDIT\npub fn marque() { depart(); }\n").unwrap();
        std::fs::write(sources.join("journal.wal"), b"\x00\x01wal\x00binaire\x00").unwrap();
        std::fs::write(sources.join("admis.rs"), "pub fn admis() { depart(); }\n").unwrap();
    }

    /// Les lignes `File` de ces chemins, en base.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn fichiers_en_base(backend: &Backend, chemins: &[&str]) -> Vec<String> {
        use crate::connection::{CypherValue, QueryParam};
        let cat = backend.catalog.lock().unwrap();
        chemins
            .iter()
            .filter(|c| {
                cat.execute_raw_with_params(
                    "MATCH (f:File) WHERE f.path = $p RETURN count(f)",
                    &[QueryParam::new("p", CypherValue::String(c.to_string()))],
                )
                .unwrap()
                .rows[0][0]
                .as_i64()
                    > Some(0)
            })
            .map(|c| c.to_string())
            .collect()
    }

    /// **La montre n'indexe pas ce que l'index écarte** : un dossier généré,
    /// un marqueur en tête, un binaire, créés sous une montre qui tourne. Un
    /// fichier admis, créé en même temps, passe (720 fichiers réintégrés au
    /// redémarrage de l'hôte de Lucie, le 12 octobre : seize minutes, le
    /// verrou tenu jusqu'à 55 s).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn the_watch_does_not_index_what_the_index_excludes() {
        let (dir, backend) = backend_qui_surveille();
        backend.mount_reactions().unwrap();
        poser_des_exclus_et_un_admis(&dir.path().join("sources"));
        let u = attendre_les_usages(&backend, "depart", |u| u.contains("admis"));
        assert!(u.contains("admis"), "le fichier admis n'est pas passé : {u}");
        assert!(backend.stop_reactions().is_empty());
        assert!(!u.contains("genere") && !u.contains("marque"), "un exclu est passé : {u}");
        let passes = fichiers_en_base(&backend, &["generated/genere.rs", "marque.rs", "journal.wal"]);
        assert!(passes.is_empty(), "des exclus sont en base : {passes:?}");
    }

    /// **La réconciliation de départ ne reprend pas ce que l'index écarte** :
    /// les mêmes fichiers, créés entre l'index et le démarrage de la montre.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn the_startup_reconciliation_does_not_index_what_the_index_excludes() {
        let (dir, backend) = backend_qui_surveille();
        poser_des_exclus_et_un_admis(&dir.path().join("sources"));
        backend.mount_reactions().unwrap();
        let u = attendre_les_usages(&backend, "depart", |u| u.contains("admis"));
        assert!(u.contains("admis"), "le fichier admis n'est pas repris : {u}");
        assert!(backend.stop_reactions().is_empty());
        let passes = fichiers_en_base(&backend, &["generated/genere.rs", "marque.rs", "journal.wal"]);
        assert!(passes.is_empty(), "la réconciliation a repris des exclus : {passes:?}");
    }

    /// **Une réingestion par nos outils passe la porte aussi** : un agent qui
    /// édite un fichier généré (`edit_file`) ne le fait pas entrer dans
    /// l'index. La réaction n'est pas le seul appelant de la réingestion
    /// (relecture de l'arbre principal, 12 octobre).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_reingested_generated_file_does_not_enter_the_index() {
        let (dir, backend) = backend_qui_surveille();
        let source = backend.file_source.clone().unwrap();
        let contenu = "// DO NOT EDIT\npub fn marque() { depart(); }\n";
        std::fs::write(dir.path().join("sources/marque.rs"), contenu).unwrap();
        {
            let mut cat = backend.catalog.lock().unwrap();
            crate::code_tools::reingest_file(&mut cat, source.as_ref(), "marque.rs", contenu).unwrap();
            let lot = [("marque.rs".to_string(), Some(contenu.to_string()))];
            crate::code_sync::reingest_files(&mut cat, source.as_ref(), &lot, crate::disponibilite::Disponibilites::TOUT).unwrap();
        }
        let passes = fichiers_en_base(&backend, &["marque.rs"]);
        assert!(passes.is_empty(), "un fichier généré est entré par la réingestion : {passes:?}");
    }

    /// **La réaction paie la dette de vecteurs de son lot**, hors du verrou du
    /// catalogue (`Catalog::embarquer_le_retard_hors_verrou`) : après elle, les
    /// morceaux ne doivent plus rien. Le garde-fou du branchement ; le témoin
    /// « le service n'est pas appelé sous le verrou » est au catalogue.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_reaction_pays_the_vector_debt_of_its_batch() {
        let (dir, backend) = backend_qui_surveille();
        backend.mount_reactions().unwrap();
        std::fs::write(dir.path().join("sources/vecteurs.rs"), "pub fn vectorise() { depart(); }\n").unwrap();
        let u = attendre_les_usages(&backend, "depart", |u| u.contains("vectorise"));
        assert!(u.contains("vectorise"), "{u}");
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let du = loop {
            let n = {
                let cat = backend.catalog.lock().unwrap();
                let marqueur = cat.vector_storage("Scope_Chunk").unwrap().marker;
                cat.execute_raw(&format!(
                    "MATCH (c:Scope_Chunk) WHERE c._text <> '' AND (c.`{marqueur}` IS NULL OR c.`{marqueur}` = '') RETURN count(c)"
                ))
                .unwrap()
                .rows[0][0]
                .as_i64()
                .unwrap_or(-1)
            };
            if n == 0 || std::time::Instant::now() > fin {
                break n;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        assert!(backend.stop_reactions().is_empty());
        assert_eq!(du, 0, "des morceaux doivent encore leur vecteur après la réaction");
    }

    /// **L'arrêt coupe une réaction longue au prochain lot** : cent fichiers
    /// supprimés font une longue réaction ; l'arrêt rend la main avant sa fin,
    /// ce qui est fait reste, et au redémarrage la réconciliation reprend le
    /// reste.
    ///
    /// Le témoin juge l'**ordre**, pas une durée : l'arrêt est demandé
    /// pendant la réaction (un lot déjà soldé), et il revient avant qu'elle
    /// ne finisse — du retard reste. Une borne de 3 s rougissait sous la
    /// charge de la lib entière (4,5 s : un premier lot de quatre fichiers à
    /// plus d'une seconde chacun), et dans la plupart des passes l'arrêt
    /// tombait avant le premier fichier, sans rien couper (le diagnostic du
    /// 19 octobre : `docs/tickets/closed/2026-10-19-…`).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn stopping_cuts_a_long_reaction_and_the_restart_resumes_it() {
        let (dir, backend) = backend_qui_surveille();
        let lot = dir.path().join("sources/lot");
        std::fs::create_dir(&lot).unwrap();
        // Cent fichiers : la réaction de retrait dure assez pour être coupée
        // (un retrait coûte des centaines de millisecondes sur ce moteur ; à
        // mesurer, chantier suivant), et la reprise tient dans le délai.
        for i in 0..100 {
            std::fs::write(lot.join(format!("f{i}.rs")), format!("fn f{i}() {{ depart(); }}\n")).unwrap();
        }
        {
            let source = backend.file_source.clone().unwrap();
            let mut cat = backend.catalog.lock().unwrap();
            crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).unwrap();
        }
        backend.mount_reactions().unwrap();
        std::fs::remove_dir_all(&lot).unwrap();
        // La réaction est en cours : son retard est noté (cent fichiers) et
        // au moins un lot en est soldé.
        let retard = || backend.index_states().unwrap()["lag"]["files"].as_u64().unwrap_or(0);
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(60);
        loop {
            let r = retard();
            if r > 0 && r < 100 {
                break;
            }
            assert!(std::time::Instant::now() < fin, "la réaction n'a soldé aucun lot en 60 s (retard {r})");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(backend.stop_reactions().is_empty());
        // **L'arrêt revient avant la fin de la réaction** : du retard reste.
        let reste = retard();
        assert!(reste > 0, "l'arrêt a attendu la fin de la réaction : plus aucun retard");
        // Au redémarrage, la réconciliation reprend ce qui restait. Le retard
        // seul ne suffit pas à le dire : la montre ne note la réconciliation
        // qu'à son premier tour, et si la réaction coupée avait soldé tout ce
        // qu'elle avait noté, aucun retard n'est visible juste après le
        // remontage (rouge sous charge, des deux côtés de 113e388fd :
        // docs/tickets/closed/2026-10-18-la-reprise-apres-arret-rougit-sous-charge.md).
        // On attend donc ce que le test garde : plus aucun usage dans `lot/`,
        // lu seulement quand le retard est tombé, dans le même délai.
        backend.mount_reactions().unwrap();
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(120);
        let u = loop {
            if backend.index_states().unwrap().get("lag").is_none() {
                let u = usages_de(&backend, "depart");
                if !u.contains("lot/") || std::time::Instant::now() >= fin {
                    break u;
                }
            } else if std::time::Instant::now() >= fin {
                break usages_de(&backend, "depart");
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        };
        assert!(!u.contains("lot/"), "le reste n'est pas repris : {}", &u[..u.len().min(400)]);
        assert!(backend.stop_reactions().is_empty());
    }

    /// **Une racine disparue met la montre en pause** : rien n'est retiré de
    /// l'index, l'état le dit, et la liste de la source nomme la racine.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_vanished_root_pauses_the_watch_and_keeps_the_index() {
        let (dir, backend) = backend_qui_surveille();
        backend.mount_reactions().unwrap();
        let racine = dir.path().join("sources");
        let ailleurs = dir.path().join("ailleurs");
        std::fs::rename(&racine, &ailleurs).unwrap();
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let etat = loop {
            let e = backend.index_states().unwrap();
            if e["watches"]["code"]["paused"].is_string() || std::time::Instant::now() > fin {
                break e;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert!(etat["watches"]["code"]["paused"].as_str().unwrap_or("").contains("racine absente"), "{etat}");
        std::thread::sleep(std::time::Duration::from_millis(300));
        let u = usages_de(&backend, "depart");
        assert!(u.contains("outils.rs") && u.contains("main"), "l'index a été vidé : {u}");
        let err = backend.file_source.clone().unwrap().list().unwrap_err();
        assert!(err.contains("racine du workspace absente") && err.contains("sources"), "{err}");
        assert!(backend.stop_reactions().is_empty());
    }

    /// **Une lecture passe pendant une réaction qui enchaîne les fichiers** :
    /// la réaction reprend le verrou du catalogue fichier après fichier, et un
    /// verrou qui n'est pas équitable laisserait le lecteur perdre à chaque
    /// fois — « catalogue occupé » sans aucune tenue longue. Trois cents
    /// fichiers créés d'un coup ; pendant leur réindexation, `usages` répond.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_read_gets_through_a_reaction_that_chains_files() {
        let (dir, backend) = backend_qui_surveille();
        backend.mount_reactions().unwrap();
        let lot = dir.path().join("sources/lot");
        std::fs::create_dir(&lot).unwrap();
        for i in 0..300 {
            std::fs::write(lot.join(format!("g{i}.rs")), format!("fn g{i}() {{ depart(); }}\n")).unwrap();
        }
        // La réaction a commencé : une partie est réindexée, pas tout.
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            let u = usages_de(&backend, "depart");
            assert!(!u.contains("Catalogue occupé"), "une lecture refusée pendant la réaction : {}", &u[..u.len().min(300)]);
            if u.contains("lot/") || std::time::Instant::now() > fin {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        // Et encore pendant qu'elle continue.
        for _ in 0..10 {
            let u = usages_de(&backend, "depart");
            assert!(!u.contains("Catalogue occupé"), "une lecture refusée pendant la réaction : {}", &u[..u.len().min(300)]);
        }
        assert!(backend.stop_reactions().is_empty());
    }

    /// **Une lecture pendant un lot réindexé voit le lot d'avant, entier**
    /// (L3, page 12-octobre-2026-arbre-principal/02). Deux fichiers arrivent
    /// d'un seul coup (un dossier renommé) : l'un définit `arrivee`, l'autre
    /// l'appelle et tient le lot ouvert 2 s (le crochet `__pause_du_lot__`).
    /// Pendant ce temps, `usages arrivee` (par la vue de lecture, sans le
    /// mutex) voit soit rien, soit tout : jamais la définition sans son appel.
    /// Ses requêtes (le pivot, les définitions, les rendez-vous) lisent dans
    /// l'instantané de l'appel (L6) : un COMMIT du lot entre deux d'entre elles
    /// ne se voit plus (environ 1 rouge sur 10 sous charge avant L6).
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_read_during_a_reindexed_batch_sees_the_batch_before_whole() {
        let (dir, backend) = backend_qui_surveille();
        backend.mount_reactions().unwrap();
        attendre_les_usages(&backend, "depart", |u| u.contains("outils.rs"));
        let prep = dir.path().join("prep");
        std::fs::create_dir(&prep).unwrap();
        std::fs::write(prep.join("def.rs"), "pub fn arrivee() {}\n").unwrap();
        std::fs::write(prep.join("__pause_du_lot__.rs"), "fn h() { arrivee(); }\n").unwrap();
        std::fs::rename(&prep, dir.path().join("sources/neuf")).unwrap();
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let (mut avant, mut apres) = (0, false);
        while std::time::Instant::now() < fin {
            let u = usages_de(&backend, "arrivee");
            let (def, appel) = (u.contains("neuf/def.rs"), u.contains("__pause_du_lot__"));
            assert_eq!(def, appel, "un lot vu à moitié : {}", &u[..u.len().min(400)]);
            if def {
                apres = true;
                break;
            }
            avant += 1;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(apres, "le lot finit par être visible, entier");
        assert!(avant >= 10, "des lectures ont eu lieu pendant le lot ouvert : {avant}");
        assert!(backend.stop_reactions().is_empty());
    }

    /// **Un lot en échec est défait sans empoisonner le catalogue** (L3) : le
    /// crochet `__echec_du_lot__` fait échouer le lot avant le COMMIT, puis
    /// chaque fichier repris seul. Ni le lot ni le fichier fautif ne restent
    /// dans l'index, le catalogue sert encore, et l'édition suivante est
    /// réindexée.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_failed_batch_is_undone_without_poisoning_the_catalog() {
        let (dir, backend) = backend_qui_surveille();
        backend.mount_reactions().unwrap();
        attendre_les_usages(&backend, "depart", |u| u.contains("outils.rs"));
        std::fs::write(dir.path().join("sources/__echec_du_lot__.rs"), "fn e() { depart(); }\n").unwrap();
        // Le lot échoue, puis le fichier seul échoue encore : le temps de deux
        // passes de la réaction.
        std::thread::sleep(std::time::Duration::from_secs(3));
        let u = usages_de(&backend, "depart");
        assert!(!u.contains("__echec_du_lot__"), "le lot défait ne laisse rien : {u}");
        assert!(!u.contains("rouvert") && !u.contains("Catalogue occupé"), "le catalogue sert encore : {u}");
        std::fs::write(dir.path().join("sources/apres.rs"), "fn k() { depart(); }\n").unwrap();
        let u = attendre_les_usages(&backend, "depart", |u| u.contains("apres.rs"));
        assert!(u.contains("apres.rs"), "l'édition suivante est réindexée : {u}");
        assert!(backend.stop_reactions().is_empty());
    }

    /// **Une base copiée se réconcilie par chemin relatif** : l'index fait sur
    /// une racine, la source ouverte sur une copie de cet arbre qui en diffère
    /// d'un fichier — la carte de l'index se lit à la racine que chaque ligne a
    /// enregistrée, et la réconciliation ne rend que le fichier changé.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_copied_base_reconciles_by_relative_path() {
        let (dir, backend) = backend_qui_surveille();
        let copie = dir.path().join("copie");
        std::fs::create_dir(&copie).unwrap();
        for f in ["main.rs", "outils.rs"] {
            std::fs::copy(dir.path().join("sources").join(f), copie.join(f)).unwrap();
        }
        std::fs::write(copie.join("main.rs"), "fn main() { depart(); }\nfn branche() { depart(); }\n").unwrap();
        let source: Arc<dyn crate::code_tools::FileSource> = Arc::new(crate::code_tools::WorkingTree::new(&copie));
        let indexed = crate::watch::indexed_files(&backend.catalog.lock().unwrap(), source.as_ref()).unwrap();
        assert_eq!(indexed.keys().cloned().collect::<Vec<_>>(), ["main.rs", "outils.rs"], "lu par chemin relatif");
        let scanner = crate::watch::Scanner::new(copie.clone(), source, &crate::watch::WatchConfig::default()).unwrap();
        let changes: Vec<_> = scanner.reconcile(&indexed).into_iter().map(|c| (c.path, c.change)).collect();
        assert_eq!(changes, [("main.rs".to_string(), crate::watch::Change::Modified)]);
    }

    /// **La mesure, phase par phase, de la réindexation d'un fichier** (les
    /// « 6 s » de Lucie). Joué à la demande : `RAG3WEAVER_INGEST_PROFILE=1
    /// cargo test … measure_one_file_reindexing -- --ignored --nocapture`.
    /// Les sources de ce crate indexées, puis un petit fichier réindexé cinq
    /// fois et un autre retiré : la durée de chacun, et les totaux par phase
    /// (le profileur d'ingestion). Sans la montre : ce qui est mesuré est le
    /// chemin du moteur, pas l'attente de l'événement.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    #[ignore]
    fn measure_one_file_reindexing() {
        fn copier(src: &Path, dst: &Path) -> usize {
            std::fs::create_dir_all(dst).unwrap();
            let mut n = 0;
            for e in std::fs::read_dir(src).unwrap() {
                let e = e.unwrap();
                let cible = dst.join(e.file_name());
                if e.file_type().unwrap().is_dir() {
                    n += copier(&e.path(), &cible);
                } else if e.path().extension().is_some_and(|x| x == "rs") {
                    std::fs::copy(e.path(), &cible).unwrap();
                    n += 1;
                }
            }
            n
        }
        let (dir, backend) = backend_qui_surveille();
        let arbre = dir.path().join("sources/arbre");
        let n = copier(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &arbre);
        let source = backend.file_source.clone().unwrap();
        let t = std::time::Instant::now();
        {
            let mut cat = backend.catalog.lock().unwrap();
            crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).unwrap();
        }
        eprintln!("[mesure] index de {n} fichiers : {:?}", t.elapsed());
        let _ = crate::ingest_profile::take();
        let petit = "arbre/scope.rs";
        let original = std::fs::read_to_string(dir.path().join("sources").join(petit)).unwrap();
        for i in 0..5 {
            let contenu = format!("{original}\n// mesure {i}\nfn mesure_{i}() {{ depart(); }}\n");
            std::fs::write(dir.path().join("sources").join(petit), &contenu).unwrap();
            let t = std::time::Instant::now();
            let mut cat = backend.catalog.lock().unwrap();
            // La même ligne que la montre écrit, pour la lire ici aussi.
            let (r, etapes) = crate::ingest_profile::record(|| crate::code_tools::reingest_file(&mut cat, source.as_ref(), petit, &contenu));
            r.unwrap();
            let tenu = t.elapsed();
            eprintln!("[mesure] réindexer {petit} ({} octets), essai {i} : {tenu:?}", contenu.len());
            eprintln!(
                "{}",
                crate::dataflow::reindex_nodes::journal_line(petit, &Ok(crate::dataflow::reindex_nodes::Outcome::Reindexed), std::time::Duration::ZERO, tenu, &etapes)
            );
        }
        let retire = "arbre/meter.rs";
        std::fs::remove_file(dir.path().join("sources").join(retire)).unwrap();
        let t = std::time::Instant::now();
        {
            let mut cat = backend.catalog.lock().unwrap();
            crate::code_sync::remove_file(&mut cat, source.as_ref(), retire, crate::disponibilite::Disponibilites::TOUT).unwrap();
        }
        let retrait = t.elapsed();
        eprintln!("[mesure] retirer {retire} : {retrait:?}");
        for (etape, duree, fois) in crate::ingest_profile::take() {
            eprintln!("[mesure]   {etape} : {duree:?} ({fois} fois)");
        }
        // Le seul seuil de la mesure, large : retirer ce fichier (45 morceaux)
        // prenait 112 s quand chaque morceau supprimé portait encore son
        // vecteur indexé, 0,3 s quand on le vide d'abord (12 octobre 2026).
        assert!(retrait < std::time::Duration::from_secs(20), "retirer {retire} : {retrait:?}");
    }

    /// Une montre ne surveille qu'une copie de travail : sur un `snapshot`
    /// (une source chargée en mémoire), elle est refusée en le disant.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn a_watch_on_a_snapshot_is_refused() {
        let (dir, p) = fixture_code_avec("snapshot", false, json!({}), &|m| {
            m["watches"] = json!({"code": {}});
        });
        let backend = backend_de_code(dir.path(), p.unwrap());
        let err = backend.mount_watches().unwrap_err();
        assert!(err.contains("working_tree"), "{err}");
    }

    /// **La politique refuse au chargement, en disant quoi faire** — la
    /// leçon du deck builder : nommer le paramètre à changer, pas la règle.
    // Un espace de travail de code (`fixture_code`) : la clé `workspace` exige la feature.
    #[cfg(feature = "code")]
    #[test]
    fn la_politique_refuse_en_disant_quoi_faire() {
        // edit attaché sans write_files : le refus nomme la capacité.
        let (_d, p) = fixture_code("working_tree", false, json!({
            "edit_file": {"graph": "edit.mmd", "policy": {"read_files": true}}
        }));
        let erreur = p.err().unwrap();
        assert!(erreur.contains("write_files"), "{erreur}");
        assert!(erreur.contains("ajoutez"), "le refus dit quoi faire : {erreur}");

        // write_files sur un workspace read_only : incohérence dite au load.
        let (_d, p) = fixture_code("working_tree", true, json!({
            "edit_file": {"graph": "edit.mmd", "policy": {"write_files": true}}
        }));
        let erreur = p.err().unwrap();
        assert!(erreur.contains("read_only"), "{erreur}");

        // run sans porte : la clé à changer est nommée.
        let (_d, p) = fixture_code("working_tree", false, json!({
            "run_command": {"graph": "run.mmd", "policy": {"run_commands": true}}
        }));
        let erreur = p.err().unwrap();
        assert!(erreur.contains("workspace.commands"), "{erreur}");
    }

    /// **Le même moteur, deux politiques** : l'une lit un instantané sans
    /// rien pouvoir écrire, l'autre édite un arbre de travail — et le disque
    /// le prouve.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn le_meme_moteur_sous_deux_politiques() {
        // La politique cloud : un instantané, lecture seule.
        let (dir, p) = fixture_code("snapshot", true, json!({
            "read_file": {"graph": "read.mmd", "policy": {"read_files": true}}
        }));
        let backend = backend_de_code(dir.path(), p.unwrap());
        let lu = backend
            .call_tool("read_file", json!({"path": "main.rs"}))
            .unwrap();
        assert!(
            lu["result"].as_str().unwrap_or_default().contains("depart()"),
            "l'instantané se lit : {lu}"
        );

        // La politique de poste : l'arbre réel, l'édition permise.
        let (dir, p) = fixture_code("working_tree", false, json!({
            "read_file": {"graph": "read.mmd", "policy": {"read_files": true}},
            "edit_file": {"graph": "edit.mmd", "policy": {"write_files": true}}
        }));
        let backend = backend_de_code(dir.path(), p.unwrap());
        backend
            .call_tool("edit_file", json!({"path": "main.rs", "old": "depart()", "new": "arrivee()"}))
            .unwrap();
        assert!(
            std::fs::read_to_string(dir.path().join("sources/main.rs"))
                .unwrap()
                .contains("arrivee()"),
            "l'arbre de travail est édité sur le disque"
        );
    }

    /// **L'édition humaine par l'outil** (B1, 18 octobre 2026) : `read_file` en
    /// `format: "brut"` rend le fichier exact et son empreinte ; `edit_file` avec
    /// `content` seul écrit (les défauts vides du graphe ne le refusent plus), avec
    /// `expected_hash` ; une empreinte périmée refuse sans rien écrire.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn l_edition_humaine_lit_en_brut_et_ecrit_sur_l_empreinte_lue() {
        let (dir, p) = fixture_code("working_tree", false, json!({
            "read_file": {"graph": "read.mmd", "policy": {"read_files": true}},
            "edit_file": {"graph": "edit.mmd", "policy": {"write_files": true}}
        }));
        let backend = backend_de_code(dir.path(), p.unwrap());
        let fichier = dir.path().join("sources/main.rs");
        let lu = backend.call_tool("read_file", json!({"path": "main.rs", "format": "brut"})).unwrap();
        let lu = &lu["result"];
        assert_eq!(lu["content"].as_str().unwrap(), std::fs::read_to_string(&fichier).unwrap(), "{lu}");
        let empreinte = lu["content_hash"].as_str().unwrap().to_string();
        let neuf = format!("{}// ajouté par l'IDE\n", lu["content"].as_str().unwrap());
        backend
            .call_tool("edit_file", json!({"path": "main.rs", "content": neuf, "expected_hash": empreinte}))
            .unwrap();
        assert_eq!(std::fs::read_to_string(&fichier).unwrap(), neuf, "content seul, par l'outil, écrit tel quel");
        // L'ancienne empreinte est périmée : refus, rien d'écrit.
        let err = backend
            .call_tool("edit_file", json!({"path": "main.rs", "content": "écrasé\n", "expected_hash": empreinte}))
            .unwrap_err();
        assert!(err.contains(crate::code_tools::STALE_EDIT), "{err}");
        assert_eq!(std::fs::read_to_string(&fichier).unwrap(), neuf, "rien n'est écrit sur une empreinte périmée");
        // Hors du worktree : refus.
        assert!(backend
            .call_tool("edit_file", json!({"path": "../hors.rs", "content": "x", "expected_hash": empreinte}))
            .is_err());
        assert!(!dir.path().join("hors.rs").exists());
        // content et old ensemble : l'appel est refusé à l'exécution, avec le nom
        // des deux, et rien n'est écrit (le graphe, lui, se construit : cbe99e380).
        let err = backend
            .call_tool("edit_file", json!({"path": "main.rs", "content": "écrasé\n", "old": "arrivee", "new": "x", "expected_hash": empreinte}))
            .unwrap_err();
        assert!(err.contains("'content'") && err.contains("'old'") && err.contains("exclusive"), "{err}");
        assert_eq!(std::fs::read_to_string(&fichier).unwrap(), neuf, "rien n'est écrit sur un appel refusé");
    }

    /// **Un backend en mots seuls démarre sans service d'embarquement** —
    /// signalé par la session mémoire : connect était appelé à l'ouverture
    /// sans regarder les signaux, et une adresse injoignable empêchait un
    /// backend bm25-only de démarrer. Et l'inverse : un backend qui déclare
    /// du vecteur refuse l'absence de service en la nommant.
    #[cfg(feature = "rag3db-native")]
    #[test]
    fn un_backend_en_mots_seuls_demarre_sans_service() {
        // Le notebook, ramené aux mots seuls, avec une adresse injoignable.
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
        let dir = tempfile::tempdir().unwrap();
        fn copier(src: &Path, dst: &Path) {
            std::fs::create_dir_all(dst).unwrap();
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let cible = dst.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copier(&entry.path(), &cible);
                } else {
                    std::fs::copy(entry.path(), &cible).unwrap();
                }
            }
        }
        copier(&src, dir.path());
        std::fs::copy(
            src.join("../../tools/search_structured.mmd"),
            dir.path().join("search_structured.mmd"),
        )
        .unwrap();
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
        manifest["entities"]["Note"]["config"]["signals"] = json!(["bm25"]);
        manifest["embeddings"]["address"] = json!("127.0.0.1:9");
        manifest["vector_extension"] = json!(extension_vector_de_test());
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let prepared = PreparedBackend::load(&chemin).unwrap();
        assert!(!prepared.needs_embeddings(), "aucun signal vecteur déclaré");
        let conn = crate::Rag3dbConnection::in_memory().expect("base en mémoire");
        let backend = prepared
            .open(Box::new(conn), None)
            .expect("un backend en mots seuls démarre sans service");
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k1", "text": "le four ne chauffe plus", "labels": [], "stage": "working"}}))
            .unwrap();
        assert!(r.get("ok").map(|v| v != false).unwrap_or(true), "{r}");

        // Le notebook d'origine (vecteur déclaré) démarre aussi sans service
        // (10 octobre 2026), mais le dit dans son reçu, en nommant quoi
        // faire : `describe` et `index_state` portent l'avertissement, parce
        // que stderr n'est lu par personne quand le backend tourne en service.
        // Une seconde copie, ses signaux vecteur gardés : ses graphes pris au
        // gabarit par chemin absolu, et l'extension de l'arbre bâti — le
        // gabarit la cherche relativement à lui-même (ticket ouvert).
        let dir_vecteur = tempfile::tempdir().unwrap();
        copier(&src, dir_vecteur.path());
        let chemin = dir_vecteur.path().join("backend.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        for outil in manifest["tools"].as_object_mut().unwrap().values_mut() {
            if let Some(graphe) = outil["graph"].as_str().filter(|g| g.starts_with("..")) {
                outil["graph"] = json!(src.join(graphe).to_string_lossy());
            }
        }
        manifest["vector_extension"] = json!(extension_vector_de_test());
        // Un service déclaré où personne n'écoute (port 1) : la raison dit
        // « pas joint » et l'adresse, donc « démarrez-le », pas « déclarez-le ».
        manifest["embeddings"]["address"] = json!("127.0.0.1:1");
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        let prepared = PreparedBackend::load(&chemin).unwrap();
        assert!(prepared.needs_embeddings());
        let (embedder, raison) = prepared.embedder_or_absent();
        assert!(embedder.is_none(), "personne n'écoute au port 1");
        let raison = raison.expect("la porte connaît la raison");
        assert!(raison.contains("pas joint") && raison.contains("127.0.0.1:1"), "{raison}");
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let backend = prepared
            .open_with_reason(Box::new(conn), None, Some(raison))
            .expect("du vecteur déclaré sans service démarre en le disant");
        let avertissements = backend.avertissements();
        assert_eq!(avertissements.len(), 1, "{avertissements:?}");
        assert!(avertissements[0].contains("pas de service d'embarquement"), "{avertissements:?}");
        assert!(
            avertissements[0].contains("démarrez-le") && avertissements[0].contains("127.0.0.1:1"),
            "le reçu dit la raison et quoi faire, pas seulement stderr : {avertissements:?}"
        );
        assert_eq!(backend.describe()["warnings"], json!(avertissements), "le reçu de describe porte l'avertissement");
    }

    /// **Un gabarit copié hors de l'arbre trouve l'extension du bâti** (ticket
    /// « un gabarit livré cherche l'extension vector relativement à
    /// lui-même ») : le notebook copié ailleurs, comme un paquet installé ou
    /// un worktree sans moteur, son `vector_extension` **tel que livré**,
    /// s'ouvre en trouvant l'extension du bâti déclaré (RAG3DB_ROOT).
    #[cfg(feature = "rag3db-native")]
    #[test]
    fn un_gabarit_copie_hors_de_l_arbre_trouve_l_extension_du_bati() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
        let dir = tempfile::tempdir().unwrap();
        fn copier(src: &Path, dst: &Path) {
            std::fs::create_dir_all(dst).unwrap();
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let cible = dst.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copier(&entry.path(), &cible);
                } else {
                    std::fs::copy(entry.path(), &cible).unwrap();
                }
            }
        }
        copier(&src, dir.path());
        std::fs::copy(src.join("../../tools/search_structured.mmd"), dir.path().join("search_structured.mmd")).unwrap();
        let chemin = dir.path().join("backend.json");
        let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
        manifest["entities"]["Note"]["config"]["signals"] = json!(["bm25"]);
        manifest["embeddings"]["address"] = json!("127.0.0.1:9");
        std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        // Le bâti est déclaré par RAG3DB_ROOT, comme dans toute passe.
        let _ = extension_vector_de_test();
        let prepared = PreparedBackend::load(&chemin).unwrap();
        let conn = crate::Rag3dbConnection::in_memory().expect("base en mémoire");
        let backend = prepared
            .open(Box::new(conn), None)
            .unwrap_or_else(|e| panic!("le gabarit copié doit trouver l'extension du bâti : {e}"));
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k1", "text": "le four ne chauffe plus", "labels": [], "stage": "working"}}))
            .unwrap();
        assert!(r.get("ok").map(|v| v != false).unwrap_or(true), "{r}");
        assert_eq!(backend.prepared().manifest.vector_extension, Path::new(DEFAULT_VECTOR_EXTENSION), "le gabarit livré nomme l'extension");

        // Les refus nomment ce qui manque : un chemin forcé, puis un nom que
        // rien ne porte (la liste des endroits cherchés, et la variable).
        for (declare, attendus) in [
            ("ailleurs/libvector.rag3db_extension", vec!["introuvable", "ailleurs/libvector.rag3db_extension", "relatif"]),
            ("inexistante", vec!["« inexistante » introuvable", "libinexistante.rag3db_extension", "RAG3DB_ROOT"]),
        ] {
            manifest["vector_extension"] = json!(declare);
            std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            let erreur = PreparedBackend::load(&chemin).unwrap().vector_extension_path().unwrap_err();
            for a in attendus {
                assert!(erreur.contains(a), "« {a} » absent du refus : {erreur}");
            }
        }
    }

    /// **Le crochet après outil** : la section s'ajoute au rendu, une
    /// erreur du crochet laisse l'outil entier, et un crochet qui demande
    /// un nœud interdit est refusé au chargement, en disant quoi faire.
    #[cfg(feature = "rag3db-native")]
    #[test]
    fn le_crochet_apres_outil_enrichit_se_tait_et_se_valide() {
        // Le montage notebook en mots seuls, comme le test des embarquements.
        fn montage(after: Value, gabarit: &str) -> (tempfile::TempDir, Result<PreparedBackend, String>) {
            let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
            let dir = tempfile::tempdir().unwrap();
            fn copier(src: &Path, dst: &Path) {
                std::fs::create_dir_all(dst).unwrap();
                for entry in std::fs::read_dir(src).unwrap() {
                    let entry = entry.unwrap();
                    let cible = dst.join(entry.file_name());
                    if entry.file_type().unwrap().is_dir() {
                        copier(&entry.path(), &cible);
                    } else {
                        std::fs::copy(entry.path(), &cible).unwrap();
                    }
                }
            }
            copier(&src, dir.path());
            std::fs::copy(
                src.join("../../tools/search_structured.mmd"),
                dir.path().join("search_structured.mmd"),
            )
            .unwrap();
            std::fs::write(dir.path().join("apres_essai.mmd"), gabarit).unwrap();
            let chemin = dir.path().join("backend.json");
            let mut manifest: Value =
                serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
            manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
            manifest["entities"]["Note"]["config"]["signals"] = json!(["bm25"]);
            manifest["embeddings"]["address"] = json!("127.0.0.1:9");
            manifest["tools"]["put_note"]["after"] = after;
            manifest["vector_extension"] = json!(extension_vector_de_test());
            std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            let prepared = PreparedBackend::load(&chemin);
            (dir, prepared)
        }
        let carte = "%% tool: apres_essai\n%% description: essai\n%% result: schema.result\n\ngraph LR\n    schema[\"SchemaNode\"]\n";

        // (a) La section s'ajoute au rendu de l'outil.
        let (_d, prepared) = montage(json!({"graph": "apres_essai.mmd", "title": "À voir aussi"}), carte);
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let backend = prepared.unwrap().open(Box::new(conn), None).unwrap();
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k1", "text": "essai", "labels": [], "stage": "working"}}))
            .unwrap();
        let rendu = serde_json::to_string(&r).unwrap();
        assert!(rendu.contains("À voir aussi"), "la section du crochet manque : {rendu}");

        // (b) Un gabarit qui exige un paramètre jamais fourni : l'outil
        // répond entier, sans section — l'erreur se journalise et se tait.
        let exigeant = "%% tool: apres_essai\n%% description: essai\n%% param: inconnu string! -- jamais fourni\n%% result: schema.result\n\ngraph LR\n    schema[\"SchemaNode(target=$inconnu)\"]\n";
        let (_d, prepared) = montage(json!({"graph": "apres_essai.mmd"}), exigeant);
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let backend = prepared.unwrap().open(Box::new(conn), None).unwrap();
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k2", "text": "essai", "labels": [], "stage": "working"}}))
            .unwrap();
        let rendu = serde_json::to_string(&r).unwrap();
        assert!(r.get("delivery").is_some(), "l'outil répond entier : {rendu}");
        assert!(!rendu.contains("À voir aussi"), "pas de section sur une erreur : {rendu}");

        // (c) Un nœud interdit dans le crochet : refus au chargement, qui
        // dit quoi faire.
        let interdit = "%% tool: apres_essai\n%% description: essai\n%% result: run.result\n\ngraph LR\n    run[\"RunCommandNode(command='ls')\"]\n";
        let (_d, prepared) = montage(json!({"graph": "apres_essai.mmd"}), interdit);
        let erreur = prepared.err().expect("un crochet qui lance se refuse au chargement");
        assert!(erreur.contains("crochet"), "{erreur}");
        assert!(erreur.contains("RunCommandNode"), "le refus nomme le nœud : {erreur}");

        // (d) Un gabarit qui demande les uuids des résultats sans dire d'où
        // ils viennent : refus au chargement, qui nomme results_port.
        let avide = "%% tool: apres_essai\n%% description: essai\n%% param: result_uuids json! -- les identités résolues\n%% result: src.query\n\ngraph LR\n    src[\"KBQuerySourceNode(kb_name='k', query='q', options=$result_uuids)\"]\n";
        let (_d, prepared) = montage(json!({"graph": "apres_essai.mmd"}), avide);
        let erreur = prepared.err().expect("result_uuids sans results_port se refuse");
        assert!(erreur.contains("results_port"), "le refus dit quoi déclarer : {erreur}");

        // (e) La porte des vecteurs : rien n'est indexé pour Note, le
        // crochet se tait — même graphe que (a), qui parlait.
        let (_d, prepared) = montage(
            json!({"graph": "apres_essai.mmd", "silent_unless_vectors_ready": "Note"}),
            carte,
        );
        let conn = crate::Rag3dbConnection::in_memory().unwrap();
        let backend = prepared.unwrap().open(Box::new(conn), None).unwrap();
        let r = backend
            .call_tool("put_note", json!({"record": {"key": "k3", "text": "essai", "labels": [], "stage": "working"}}))
            .unwrap();
        let rendu = serde_json::to_string(&r).unwrap();
        assert!(
            !rendu.contains("À voir aussi"),
            "vecteurs pas prêts : le crochet se tait : {rendu}"
        );
    }

    /// **Les signaux du schéma nommé se valident dans les deux sens** : le
    /// signal sans modèle, la clé sans index, le signal inconnu — chacun
    /// refuse au chargement en disant quoi faire.
    // Un espace de travail de code (`fixture_code`) : la clé `workspace` exige la feature.
    #[cfg(feature = "code")]
    #[test]
    fn index_signals_se_valide_dans_les_deux_sens() {
        let (_d, p) = fixture_code("working_tree", false, json!({}));
        assert!(p.is_ok(), "le montage de base passe");

        // index_signals sans index.
        let patch = |m: &mut Value| {
            m["workspace"]["index_signals"] = json!({"Scope": ["sparse"]});
        };
        let (_d, p) = fixture_code_avec("working_tree", false, json!({}), &patch);
        let e = p.err().unwrap();
        assert!(e.contains("workspace.index"), "{e}");

        // sparse déclaré sans models.sparse.
        let patch = |m: &mut Value| {
            m["workspace"]["index"] = json!("code");
            m["workspace"]["index_signals"] = json!({"Scope": ["bm25", "vector", "sparse"]});
        };
        let (_d, p) = fixture_code_avec("working_tree", false, json!({}), &patch);
        let e = p.err().unwrap();
        assert!(e.contains("models.sparse"), "{e}");

        // signal inconnu.
        let patch = |m: &mut Value| {
            m["workspace"]["index"] = json!("code");
            m["workspace"]["index_signals"] = json!({"Scope": ["fuzzy"]});
        };
        let (_d, p) = fixture_code_avec("working_tree", false, json!({}), &patch);
        let e = p.err().unwrap();
        assert!(e.contains("fuzzy"), "{e}");
    }

    /// **Une entité sans champ de contenu est refusée au chargement**, en la
    /// nommant, et plus seulement à l'ouverture (relevé du lot 4 d'everything
    /// declarative). Et une erreur de la configuration d'une entité dit de
    /// quelle entité elle vient.
    #[test]
    fn une_entite_sans_contenu_est_refusee_au_chargement_en_la_nommant() {
        fn copier(src: &Path, dst: &Path) {
            std::fs::create_dir_all(dst).unwrap();
            for entry in std::fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let cible = dst.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copier(&entry.path(), &cible);
                } else {
                    std::fs::copy(entry.path(), &cible).unwrap();
                }
            }
        }
        let charger = |patch: &dyn Fn(&mut Value)| {
            let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook");
            let dir = tempfile::tempdir().unwrap();
            copier(&src, dir.path());
            std::fs::copy(src.join("../../tools/search_structured.mmd"), dir.path().join("search_structured.mmd")).unwrap();
            let chemin = dir.path().join("backend.json");
            let mut manifest: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
            manifest["tools"]["search_notes"]["graph"] = json!("search_structured.mmd");
            patch(&mut manifest);
            std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            let p = PreparedBackend::load(&chemin);
            (dir, p)
        };
        // Plein texte seul, aucun champ de contenu : refusée au chargement.
        let (_d, p) = charger(&|m: &mut Value| {
            m["entities"]["Note"]["config"]["fields"]["text"]["isContent"] = json!(false);
            m["entities"]["Note"]["config"]["signals"] = json!(["bm25"]);
        });
        let e = p.err().expect("refusée au chargement");
        assert!(e.starts_with("Note: aucun champ de contenu"), "{e}");
        assert!(e.contains("isContent"), "le refus dit quoi écrire : {e}");
        // Avec un signal vecteur, c'est `validate()` qui refuse : son erreur
        // porte maintenant le nom de l'entité.
        let (_d, p) = charger(&|m: &mut Value| {
            m["entities"]["Note"]["config"]["fields"]["text"]["isContent"] = json!(false);
        });
        let e = p.err().expect("refusée au chargement");
        assert!(e.starts_with("Note: "), "l'erreur nomme l'entité : {e}");
        // Le gabarit tel quel se charge.
        let (_d, p) = charger(&|_m: &mut Value| {});
        p.expect("le carnet tel quel se charge");
    }

    /// **Les fonctions de verrou déclarées** (`workspace.locks_via`) se
    /// lisent à l'ingestion du code : sans `index: "code"`, refusées au
    /// chargement ; avec, lues telles quelles.
    // Un espace de travail de code (`fixture_code`) : la clé `workspace` exige la feature.
    #[cfg(feature = "code")]
    #[test]
    fn locks_via_exige_le_schema_de_code() {
        let patch = |m: &mut Value| {
            m["workspace"]["locks_via"] = json!(["acquireLock"]);
        };
        let (_d, p) = fixture_code_avec("working_tree", false, json!({}), &patch);
        let e = p.err().unwrap();
        assert!(e.contains("workspace.locks_via"), "{e}");

        let patch = |m: &mut Value| {
            m["workspace"]["index"] = json!("code");
            m["workspace"]["locks_via"] = json!(["acquireLock", "lockRowForWrite"]);
        };
        let (_d, p) = fixture_code_avec("working_tree", false, json!({}), &patch);
        let p = p.expect("déclarées avec le schéma de code");
        assert_eq!(p.manifest.workspace.as_ref().unwrap().locks_via, vec!["acquireLock", "lockRowForWrite"]);
    }

    /// **`models.embed` et `embeddings` disent la même chose** : l'une ou
    /// l'autre, jamais les deux, et l'absence des deux se dit au chargement.
    #[test]
    fn l_embarquement_se_declare_par_models_embed_ou_par_embeddings() {
        use crate::model_source::{Capability, Provider};
        let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json");
        let base: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();

        // La section d'avant : un démon, à l'adresse écrite.
        let avant: BackendManifest = serde_json::from_value(base.clone()).unwrap();
        let s = avant.embed_source().unwrap();
        assert_eq!((s.provider, s.model.as_str(), s.dimensions), (Provider::Service, "bge-m3", Some(1024)));
        assert_eq!(s.address.0, ["127.0.0.1:7878"]);

        // La même chose sous `models.embed`, sans adresse : elle viendra de l'environnement.
        let mut neuf = base.clone();
        neuf.as_object_mut().unwrap().remove("embeddings");
        neuf["models"] = json!({"embed": {"provider": "service", "model": "granite-278m", "dimensions": 768}});
        let neuf: BackendManifest = serde_json::from_value(neuf).unwrap();
        let s = neuf.embed_source().unwrap();
        assert_eq!((s.provider, s.model.as_str(), s.dimensions, s.address.is_empty()), (Provider::Service, "granite-278m", Some(768), true));
        assert!(neuf.models.contains_key(&Capability::Embed));

        // Les deux à la fois, ou aucune : une erreur qui dit laquelle.
        let mut deux = base.clone();
        deux["models"] = json!({"embed": {"model": "granite-278m", "dimensions": 768}});
        let e = serde_json::from_value::<BackendManifest>(deux).unwrap().embed_source().unwrap_err();
        assert!(e.contains("gardez-en un"), "{e}");
        let mut aucune = base.clone();
        aucune.as_object_mut().unwrap().remove("embeddings");
        let e = serde_json::from_value::<BackendManifest>(aucune).unwrap().embed_source().unwrap_err();
        assert!(e.contains("models.embed"), "{e}");
        // Une capacité inconnue est une faute de frappe, pas un modèle.
        let mut faute = base;
        faute["models"] = json!({"embedd": {"model": "x"}});
        assert!(serde_json::from_value::<BackendManifest>(faute).is_err());
    }

    /// Sans aucune description déclarée, rien ne change : les textes des
    /// gabarits se rendent tels quels — une base d'avant se relit et se
    /// décrit comme avant.
    #[test]
    fn sans_description_les_gabarits_gardent_leur_texte() {
        let p = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        let d = p.describe();
        let put = d["tools"].as_array().unwrap().iter().find(|t| t["name"] == "put_note").unwrap();
        let texte = put["description"].as_str().unwrap();
        assert!(texte.contains("révision attendue"), "{texte}");
        assert!(!texte.contains("Cible «"), "pas d'entité à dire : {texte}");
    }
    #[test]
    fn ad_hoc_search_validates_without_storage() {
        let (p, r) = search_fixture();
        let (_, def) = p.prepare_search(&r).unwrap();
        assert_eq!(def.nodes.len(), 1);
        let description = p.search_description();
        assert!(description["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["type"] != "EntityRecordNode"));
    }
    #[test]
    fn ad_hoc_search_rejects_unknown_entities_parameters_fields_and_outputs() {
        let (p, mut r) = search_fixture();
        r.arguments["entity"] = json!("Missing");
        assert!(p.prepare_search(&r).unwrap_err().contains("unknown entity"));
        r.arguments["entity"] = json!("Note");
        r.arguments["typo"] = json!(1);
        assert!(p.prepare_search(&r).is_err());
        r.arguments.as_object_mut().unwrap().remove("typo");
        r.arguments["filter"] = json!({"path":{"path":["missing"],"value":true}});
        assert!(p.prepare_search(&r).is_err());
        r.arguments["filter"] = json!({"must":[]});
        r.metadata.push(OutputPort {
            node: "select".into(),
            port: "missing".into(),
        });
        assert!(p.prepare_search(&r).unwrap_err().contains("unknown output"));
    }
    #[test]
    fn ad_hoc_search_rejects_writes_and_oversized_graphs() {
        let (p, mut r) = search_fixture();
        r.arguments = json!({});
        // Use the existing known-valid write graph: rejection must be policy, not parsing.
        let tool = &p.tools["put_note"];
        r.mermaid = tool.to_mermaid();
        r.arguments =
            json!({"entity":"Note","record":{"key":"x","text":"x","labels":[],"stage":"working"}});
        let err = p.prepare_search(&r).unwrap_err();
        assert!(err.contains("EntityRecordNode"), "{err}");
        r.mermaid = " ".repeat(65537);
        assert!(p.prepare_search(&r).unwrap_err().contains("65536"));
    }
    #[test]
    fn local_refs_remain_valid_inside_derived_tool_input() {
        let prepared = PreparedBackend::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/backend.json"),
        )
        .unwrap();
        let entity = &prepared.manifest.entities["Note"];
        let schema = json!({"type":"object","properties":{"key":{"$ref":"#/$defs/Id"}},"required":["key"],"$defs":{"Id":{"type":"string","minLength":1}}});
        let derived = payload_schema(&schema, entity, &PayloadView::Identity).unwrap();
        let outer = json!({"type":"object","properties":{"record":derived}});
        let validator = jsonschema::validator_for(&outer).unwrap();
        assert!(validator.is_valid(&json!({"record":{"key":"one"}})));
        assert!(!validator.is_valid(&json!({"record":{"key":""}})));
    }
}

#[cfg(test)]
#[path = "backend_reload_tests.rs"]
mod reload_tests;

#[cfg(test)]
#[path = "backend_reactions_tests.rs"]
mod reactions_tests;

/// **Où chercher l'extension vectorielle, dans l'ordre.** D'abord **à côté de
/// la lib** (`RAG3DB_LIBRARY_DIR`) : sous le magasin de libs, le dossier d'un
/// identifiant porte la lib et l'extension du même bâti (page
/// 07-le-magasin-de-libs, §2.1). Un dossier de bâti sans extension
/// (`build/lecteurs-csv/src`) retombe sur l'arbre (`RAG3DB_ROOT`), comme avant.
pub(crate) fn vector_extension_candidates(
    name: &str,
    loaded_dir: Option<&Path>,
    root: Option<std::ffi::OsString>,
    lib_dir: Option<std::ffi::OsString>,
    exe_dir: Option<PathBuf>,
) -> Vec<PathBuf> {
    let fichier = format!("lib{name}.rag3db_extension");
    let mut cherches: Vec<PathBuf> = Vec::new();
    // **À côté de la lib réellement chargée, d'abord** (lot 3e du magasin) :
    // un binaire lié par RUNPATH au dossier d'un identifiant y trouve
    // l'extension du même bâti, sans aucune variable.
    if let Some(dossier) = loaded_dir {
        cherches.push(dossier.join(&fichier));
    }
    if let Some(lib) = lib_dir {
        cherches.push(PathBuf::from(lib).join(&fichier));
    }
    if let Some(racine) = root {
        cherches.push(PathBuf::from(racine).join("extension").join(name).join("build").join(&fichier));
    }
    if let Some(dossier) = exe_dir {
        cherches.push(dossier.join(&fichier));
    }
    cherches.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extension").join(name).join("build").join(&fichier));
    let mut vus = std::collections::HashSet::new();
    cherches.retain(|p| vus.insert(p.clone()));
    cherches
}

/// **L'extension vector de ce processus** : la règle de
/// [`resolve_vector_extension`] (à côté de la lib réellement chargée d'abord,
/// puis `RAG3DB_LIBRARY_DIR`, `RAG3DB_ROOT`…) appliquée à l'état du processus.
/// C'est aussi l'aide des tests et des sondes (lot 3f du magasin) : un rebâti
/// en place de l'arbre ne change plus l'extension d'une passe qui tourne sur
/// la lib du magasin, quelle que soit la variable posée.
pub fn vector_extension_of_this_process() -> Result<PathBuf, String> {
    resolve_vector_extension(
        Path::new(DEFAULT_VECTOR_EXTENSION),
        Path::new("."),
        crate::build_check::loaded_lib().as_deref(),
        std::env::var_os("RAG3DB_ROOT"),
        std::env::var_os("RAG3DB_LIBRARY_DIR"),
        std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)),
    )
}

/// **L'extension vectorielle à charger**, de ce qu'on lui donne : la valeur
/// du manifeste et son dossier, la lib du moteur chargée dans le processus,
/// et les variables. Un chemin déclaré est un forçage, relatif au manifeste.
/// Un nom est cherché, dans cet ordre : à côté de la lib chargée,
/// `RAG3DB_LIBRARY_DIR`, `RAG3DB_ROOT/extension/<nom>/build/`, le dossier de
/// l'exécutable (le paquet npm), l'arbre dont ce crate vient.
///
/// **L'identifiant doit être celui de la lib chargée** (lot 3e) : une
/// extension d'un autre bâti est passée quand elle est cherchée, refusée
/// quand elle est forcée, et le refus nomme les deux identifiants. Le moteur
/// la refuserait aussi au `LOAD`, mais après coup et sans dire d'où elle
/// venait. Une extension sans identifiant lisible est laissée au moteur.
pub(crate) fn resolve_vector_extension(
    declared: &Path,
    manifest_dir: &Path,
    loaded_lib: Option<&Path>,
    root: Option<std::ffi::OsString>,
    lib_dir: Option<std::ffi::OsString>,
    exe_dir: Option<PathBuf>,
) -> Result<PathBuf, String> {
    let loaded_id = loaded_lib.and_then(|l| crate::build_check::read_section(l, crate::build_check::LIB_SECTION).ok().flatten());
    // L'identifiant d'une extension qui n'est pas celui de la lib chargée.
    let autre = |ext: &Path| -> Option<String> {
        let id = loaded_id.as_deref()?;
        crate::build_check::extension_build_id(ext).ok().flatten().filter(|e| e != id)
    };
    let texte = declared.to_string_lossy();
    let est_un_chemin = texte.contains('/') || texte.contains('\\') || texte.ends_with(".rag3db_extension");
    if est_un_chemin {
        let chemin = manifest_dir.join(declared);
        if !chemin.is_file() {
            return Err(format!(
                "extension vectorielle introuvable : {} (`vector_extension` du manifeste, un chemin relatif à lui) — \
                 donnez un chemin qui existe, ou le nom « {DEFAULT_VECTOR_EXTENSION} » pour celle du bâti",
                chemin.display()
            ));
        }
        if let Some(sien) = autre(&chemin) {
            return Err(format!(
                "extension vectorielle refusée : {} (`vector_extension` du manifeste) est du bâti {sien}, \
                 la lib chargée ({}) du bâti {} — donnez le nom « {DEFAULT_VECTOR_EXTENSION} » pour celle de la lib, \
                 ou un chemin vers l'extension de ce bâti",
                chemin.display(),
                loaded_lib.map(|l| l.display().to_string()).unwrap_or_default(),
                loaded_id.as_deref().unwrap_or_default()
            ));
        }
        return Ok(chemin);
    }
    let cherches = vector_extension_candidates(&texte, loaded_lib.and_then(Path::parent), root, lib_dir, exe_dir);
    let mut ecartees = Vec::new();
    for p in cherches.iter().filter(|p| p.is_file()) {
        match autre(p) {
            None => return Ok(p.clone()),
            Some(sien) => ecartees.push(format!("{} (bâti {sien})", p.display())),
        }
    }
    let liste: Vec<String> = cherches.iter().map(|p| p.display().to_string()).collect();
    if !ecartees.is_empty() {
        return Err(format!(
            "extension vectorielle « {texte} » : aucune n'est du bâti de la lib chargée ({}) ; écartées : {}",
            loaded_id.as_deref().unwrap_or_default(),
            ecartees.join(" ; ")
        ));
    }
    Err(format!(
        "extension vectorielle « {texte} » introuvable ; cherchée dans : {} — posez RAG3DB_ROOT sur l'arbre où le moteur est bâti, \
         ou `vector_extension` sur le chemin du fichier",
        liste.join(" ; ")
    ))
}
