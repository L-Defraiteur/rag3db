//! **Un serveur MCP par manifeste** — les outils que le backend déclare,
//! exposés à un client MCP sur stdio.
//!
//! Page de conception : `docs/10-octobre-2026-18h00/02-un-serveur-mcp-par-manifeste.md`.
//!
//! **Rien n'est écrit à la main ici.** La liste et les schémas viennent de
//! [`crate::backend::PreparedBackend::describe`], donc de `NodeSchema` par
//! `tools.rs` — la même source que les formes OpenAI et Anthropic. Un gabarit
//! qui déclare sept outils en expose sept sans une ligne de plus ; si un
//! gabarit demandait du code dans ce fichier, le principe serait déjà cassé.
//!
//! **Le protocole est à la main, et synchrone, pour une raison qui vient de
//! notre code** : le catalogue est un `Arc<Mutex<Catalog>>` et le fan-out de
//! cellules **bascule** la cellule courante le temps d'un appel
//! (`Catalog::rechercher` nomme lui-même la limite : « pas plus sûr qu'avant
//! face à une recherche concurrente sur le même catalogue »). Une boucle qui
//! traite **un appel à la fois** rend cette limite inatteignable par
//! construction. Un serveur async la rendrait atteignable, et il faudrait
//! sérialiser à la main ce qu'on a gratuitement.
//!
//! **Sur la version du protocole, ce que je sais et ce que je ne sais pas.**
//! La révision que j'ai pu lire (sources secondaires, pas la spécification
//! elle-même) annonce `2026-07-28`, un `server/discover` qui rend tout d'un
//! coup, et `initialize` déprécié mais **encore exigé** pendant une transition
//! de dix-huit mois. Les deux sont donc servis ici. Et la négociation est
//! permissive : si le client demande une version, on la lui **rend** — pour un
//! serveur qui ne fait que des outils, le dénominateur commun est le même d'une
//! révision à l'autre, et refuser une version qu'on aurait mal devinée coûte
//! plus que l'accepter. **À confirmer contre la spécification avant qu'un
//! produit en dépende** ; le test de ce fichier épingle notre forme, pas la
//! leur.

use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// La révision servie quand le client n'en demande aucune.
pub const PROTOCOLE_DEFAUT: &str = "2026-07-28";

/// Ce que le serveur sait faire. Déclaré honnêtement : ni `resources`, ni
/// `prompts`, ni `sampling`, ni `roots` — **absents et dits absents**, plutôt
/// que silencieux, pour qu'un client n'aille pas les demander.
fn capacites() -> Value {
    json!({ "tools": { "listChanged": false } })
}

/// Ce que le protocole demande à son hôte, et rien de plus.
///
/// Cette frontière existe pour que le protocole s'éprouve **sans base, sans
/// manifeste et sans moteur** : le test de ce fichier branche un hôte de
/// fantaisie, et le binaire y branche un vrai backend. Un protocole qu'on ne
/// peut éprouver qu'en ouvrant une base est un protocole qu'on éprouve mal.
pub trait Hote {
    /// Le nom du backend, tel qu'il paraît au client.
    fn nom(&self) -> String;
    /// La version du manifeste.
    fn version(&self) -> u32;
    /// Les outils, déjà sous la forme `{name, description, inputSchema}`.
    fn outils(&self) -> Vec<Value>;
    /// Joue l'outil par le même chemin que l'agent.
    fn appeler(&self, nom: &str, arguments: Value) -> Result<Value, String>;
    /// La base demande-t-elle une réouverture ? `None` : tout va bien.
    fn doit_rouvrir(&self) -> Option<String> {
        None
    }
}

/// Ce que la boucle veut que l'appelant fasse ensuite.
#[derive(Debug, PartialEq)]
pub enum Suite {
    /// L'entrée est fermée : le client est parti.
    Fini,
    /// **La base doit être rouverte** — et le client l'a appris par un refus
    /// nommé avant qu'on en arrive là. Un client MCP ne doit jamais voir son
    /// serveur disparaître sans explication (décision du 10 octobre 2026) :
    /// l'appelant rouvre et continue si c'est possible, sort après l'avoir dit
    /// sinon.
    DoitRouvrir(String),
}

/// Le texte lisible d'une réponse d'outil — celui que l'agent reçoit.
///
/// `Backend::call` pose `presentation` quand le graphe rend un port `text`, et
/// `harness_presentation` la pose pour un refus de validation. Sans elle, le
/// JSON du résultat, indenté : un modèle lit mieux un objet formé qu'une ligne.
fn texte_de(reponse: &Value) -> String {
    match reponse.get("presentation").and_then(Value::as_str) {
        Some(t) => t.to_string(),
        None => serde_json::to_string_pretty(reponse.get("result").unwrap_or(reponse))
            .unwrap_or_else(|e| format!("résultat illisible : {e}")),
    }
}

/// Un refus de validation **est** une erreur d'outil pour le modèle : il doit
/// la voir et corriger, pas la confondre avec un succès vide.
fn est_un_refus(reponse: &Value) -> bool {
    reponse["validation"]["accepted"] == false || reponse["delivery"]["ok"] == false
}

fn erreur(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message.into()}})
}

fn resultat(id: Value, valeur: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":valeur})
}

/// **Une requête, une réponse — ou aucune.**
///
/// `None` veut dire « rien à répondre », et c'est le cas d'une **notification**
/// (une requête sans `id`). Le protocole l'exige : répondre à une notification
/// est une faute, et certains clients ferment la connexion là-dessus.
pub fn traiter(requete: &Value, hote: &dyn Hote, cles: &[String]) -> Option<Value> {
    let id = requete.get("id").cloned();
    let methode = requete.get("method").and_then(Value::as_str).unwrap_or_default();

    // Une notification : on agit s'il y a lieu, on ne répond jamais.
    let Some(id) = id else {
        return None;
    };

    match methode {
        // `initialize` reste exigé pendant la transition.
        "initialize" => {
            let version = requete["params"]["protocolVersion"]
                .as_str()
                .unwrap_or(PROTOCOLE_DEFAUT)
                .to_string();
            Some(resultat(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": capacites(),
                    "serverInfo": info_du_serveur(hote, cles),
                }),
            ))
        }
        // La forme qui remplace `initialize` : tout d'un coup, outils compris.
        "server/discover" => Some(resultat(
            id,
            json!({
                "protocolVersion": requete["params"]["protocolVersion"]
                    .as_str().unwrap_or(PROTOCOLE_DEFAUT),
                "capabilities": capacites(),
                "serverInfo": info_du_serveur(hote, cles),
                "tools": hote.outils(),
            }),
        )),
        "tools/list" => Some(resultat(id, json!({ "tools": hote.outils() }))),
        "tools/call" => {
            let Some(nom) = requete["params"]["name"].as_str() else {
                return Some(erreur(id, -32602, "params.name manquant"));
            };
            let arguments = requete["params"]
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            // **Un outil absent est inconnu, pas interdit.** Le manifeste est
            // la seule liste : un nom qu'il ne porte pas n'existe pas pour ce
            // serveur, et le dire ainsi ne révèle rien de ce qui existe
            // ailleurs (ce sera la même réponse pour un outil écarté faute de
            // clés d'exposition).
            if !hote.outils().iter().any(|t| t["name"] == nom) {
                return Some(erreur(id, -32602, format!("outil inconnu : {nom}")));
            }
            match hote.appeler(nom, arguments) {
                Ok(reponse) => Some(resultat(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": texte_de(&reponse) }],
                        "isError": est_un_refus(&reponse),
                    }),
                )),
                // **Une erreur d'outil n'est pas une erreur de protocole.**
                // Elle part dans `content` avec `isError`, pour que le modèle
                // la lise et corrige ; une erreur JSON-RPC, lui, ne la voit
                // pas.
                Err(e) => Some(resultat(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": e }],
                        "isError": true,
                    }),
                )),
            }
        }
        "ping" => Some(resultat(id, json!({}))),
        autre => Some(erreur(id, -32601, format!("méthode inconnue : {autre}"))),
    }
}

/// Ce que le serveur dit de lui-même — et **pourquoi sa liste peut être vide**.
///
/// L'exposition par clés (vision §7) écartera au chargement ce que les clés ne
/// rendent pas vrai. Aucun gabarit livré n'en porte encore, donc rien n'est
/// écarté aujourd'hui — mais le jour où ça change, une liste vide **sans
/// explication** serait un défaut et non une sécurité. Les clés présentées
/// sont donc dites ici, et le binaire dit ce qu'il a écarté.
fn info_du_serveur(hote: &dyn Hote, cles: &[String]) -> Value {
    json!({
        "name": hote.nom(),
        "version": hote.version().to_string(),
        "keys": cles,
    })
}

/// **La boucle** : une requête par ligne, une réponse par ligne.
///
/// MCP sur stdio est du JSON délimité par retours à la ligne — pas d'en-têtes
/// `Content-Length`, qui sont ceux de LSP. C'est exactement la forme que
/// `rag3weaver-backend` tient déjà pour son propre vocabulaire.
pub fn servir(
    hote: &dyn Hote,
    cles: &[String],
    entree: impl BufRead,
    sortie: &mut impl Write,
) -> Result<Suite, String> {
    for ligne in entree.lines() {
        let ligne = ligne.map_err(|e| e.to_string())?;
        if ligne.trim().is_empty() {
            continue;
        }
        let reponse = match serde_json::from_str::<Value>(&ligne) {
            Ok(requete) => traiter(&requete, hote, cles),
            // Une ligne illisible : le protocole veut un `id` nul.
            Err(e) => Some(erreur(Value::Null, -32700, format!("JSON illisible : {e}"))),
        };
        if let Some(reponse) = reponse {
            writeln!(sortie, "{reponse}").map_err(|e| e.to_string())?;
            sortie.flush().map_err(|e| e.to_string())?;
        }
        // **Le refus nommé avant la disparition.** La base peut demander une
        // réouverture après n'importe quel appel ; le client l'apprend par une
        // notification lisible, puis l'appelant décide.
        if let Some(raison) = hote.doit_rouvrir() {
            let avis = json!({
                "jsonrpc": "2.0",
                "method": "notifications/message",
                "params": {
                    "level": "error",
                    "data": format!(
                        "la base doit être rouverte ({raison}) : le serveur la rouvre et \
                         continue si c'est possible, sinon il s'arrête — relancez-le"
                    ),
                },
            });
            writeln!(sortie, "{avis}").map_err(|e| e.to_string())?;
            sortie.flush().map_err(|e| e.to_string())?;
            return Ok(Suite::DoitRouvrir(raison));
        }
    }
    Ok(Suite::Fini)
}

// ─── L'hôte réel : un backend ouvert ────────────────────────────────────────
//
// L'adaptateur vit ici et non dans le binaire : `Hote` et `Backend` sont tous
// deux de cette crate, un `impl` ailleurs serait orphelin. Il reste mince
// exprès — tout ce qui décide est au-dessus, et s'éprouve sans base.

impl Hote for crate::backend::Backend {
    fn nom(&self) -> String {
        self.prepared().manifest.name.clone()
    }

    fn version(&self) -> u32 {
        self.prepared().manifest.version
    }

    /// **La liste vient de `describe()`**, donc de `NodeSchema` par `tools.rs` :
    /// la même source que les formes OpenAI et Anthropic, et déjà sous la forme
    /// `{name, description, inputSchema}` que MCP attend. Rien à traduire.
    fn outils(&self) -> Vec<Value> {
        self.prepared()
            .describe()
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    fn appeler(&self, nom: &str, arguments: Value) -> Result<Value, String> {
        self.call(nom, arguments)
    }

    fn doit_rouvrir(&self) -> Option<String> {
        self.must_reopen()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un hôte de fantaisie : deux outils, aucune base, aucun moteur.
    struct HoteDeTest {
        refuse: bool,
        rouvrir: Option<String>,
    }

    impl Hote for HoteDeTest {
        fn nom(&self) -> String {
            "carnet".into()
        }
        fn version(&self) -> u32 {
            1
        }
        fn outils(&self) -> Vec<Value> {
            vec![json!({
                "name": "search",
                "description": "Chercher",
                "inputSchema": {"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}
            })]
        }
        fn appeler(&self, nom: &str, arguments: Value) -> Result<Value, String> {
            assert_eq!(nom, "search");
            if self.refuse {
                return Err("le workspace n'autorise pas ce chemin".into());
            }
            Ok(json!({
                "result": [{"uuid":"u1"}],
                "presentation": format!("un résultat pour {}", arguments["query"]),
            }))
        }
        fn doit_rouvrir(&self) -> Option<String> {
            self.rouvrir.clone()
        }
    }

    fn hote() -> HoteDeTest {
        HoteDeTest { refuse: false, rouvrir: None }
    }

    #[test]
    fn une_notification_ne_recoit_jamais_de_reponse() {
        // Le protocole l'exige, et certains clients ferment la connexion sur
        // une réponse à une notification — c'est donc un cas à éprouver, pas
        // une politesse.
        let n = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        assert_eq!(traiter(&n, &hote(), &[]), None);
    }

    #[test]
    fn initialize_rend_la_version_demandee_et_des_capacites_honnetes() {
        let r = traiter(
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize",
                    "params":{"protocolVersion":"2025-06-18"}}),
            &hote(),
            &["dev".to_string()],
        )
        .expect("une réponse");
        assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(r["result"]["serverInfo"]["name"], "carnet");
        // Les clés présentées sont dites : une liste vide devra pouvoir
        // s'expliquer.
        assert_eq!(r["result"]["serverInfo"]["keys"][0], "dev");
        // Déclarer ce qu'on n'a pas serait pire que de ne rien déclarer.
        assert!(r["result"]["capabilities"]["tools"].is_object());
        assert!(r["result"]["capabilities"].get("resources").is_none());
        assert!(r["result"]["capabilities"].get("prompts").is_none());
        // Sans version demandée, la nôtre.
        let d = traiter(
            &json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}),
            &hote(),
            &[],
        )
        .expect("une réponse");
        assert_eq!(d["result"]["protocolVersion"], PROTOCOLE_DEFAUT);
    }

    #[test]
    fn discover_rend_les_outils_du_meme_coup_que_tools_list() {
        // Les deux chemins doivent rendre **la même** liste : deux sources
        // divergentes seraient le défaut que cette architecture existe pour
        // éviter.
        let d = traiter(
            &json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}),
            &hote(),
            &[],
        )
        .expect("une réponse");
        let l = traiter(
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            &hote(),
            &[],
        )
        .expect("une réponse");
        assert_eq!(d["result"]["tools"], l["result"]["tools"]);
        assert_eq!(l["result"]["tools"][0]["inputSchema"]["required"][0], "query");
    }

    #[test]
    fn un_appel_rend_le_texte_lisible_que_l_agent_recoit() {
        let r = traiter(
            &json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
                    "params":{"name":"search","arguments":{"query":"four"}}}),
            &hote(),
            &[],
        )
        .expect("une réponse");
        assert_eq!(r["result"]["content"][0]["type"], "text");
        assert_eq!(r["result"]["content"][0]["text"], "un résultat pour \"four\"");
        assert_eq!(r["result"]["isError"], false);
    }

    #[test]
    fn un_refus_de_l_outil_part_dans_content_pas_en_erreur_de_protocole() {
        // **La distinction qui compte pour un modèle** : une erreur JSON-RPC
        // ne lui est pas montrée, un `content` avec `isError` l'est. Un refus
        // nommé ne sert à rien si le modèle ne le lit pas.
        let h = HoteDeTest { refuse: true, rouvrir: None };
        let r = traiter(
            &json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
                    "params":{"name":"search","arguments":{"query":"x"}}}),
            &h,
            &[],
        )
        .expect("une réponse");
        assert!(r["result"].is_object(), "pas une erreur de protocole : {r}");
        assert_eq!(r["result"]["isError"], true);
        assert!(
            r["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("workspace"),
            "le refus doit arriver au modèle : {r}"
        );
    }

    #[test]
    fn un_outil_hors_manifeste_est_inconnu_et_rien_n_est_joue() {
        let r = traiter(
            &json!({"jsonrpc":"2.0","id":5,"method":"tools/call",
                    "params":{"name":"rm_rf","arguments":{}}}),
            &hote(),
            &[],
        )
        .expect("une réponse");
        // `appeler` aurait paniqué sur un autre nom : la garde est bien avant.
        assert_eq!(r["error"]["code"], -32602);
        assert!(r["error"]["message"].as_str().unwrap().contains("rm_rf"));
    }

    #[test]
    fn une_ligne_illisible_et_une_methode_inconnue_se_refusent_par_leur_code() {
        let mut sortie: Vec<u8> = Vec::new();
        let entree = std::io::Cursor::new("ceci n'est pas du json\n");
        let suite = servir(&hote(), &[], entree, &mut sortie).expect("la boucle");
        assert_eq!(suite, Suite::Fini);
        let r: Value = serde_json::from_slice(&sortie).expect("une réponse JSON");
        assert_eq!(r["error"]["code"], -32700);
        assert!(r["id"].is_null(), "un id nul quand la requête est illisible");

        let r = traiter(
            &json!({"jsonrpc":"2.0","id":6,"method":"resources/list"}),
            &hote(),
            &[],
        )
        .expect("une réponse");
        assert_eq!(r["error"]["code"], -32601);
    }

    #[test]
    fn la_base_a_rouvrir_se_dit_au_client_avant_que_le_serveur_parte() {
        // Décision du 10 octobre : un client ne doit jamais voir son serveur
        // disparaître sans explication.
        let h = HoteDeTest {
            refuse: false,
            rouvrir: Some("point de reprise échoué".into()),
        };
        let mut sortie: Vec<u8> = Vec::new();
        let entree = std::io::Cursor::new(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\
             \"params\":{\"name\":\"search\",\"arguments\":{\"query\":\"x\"}}}\n",
        );
        let suite = servir(&h, &[], entree, &mut sortie).expect("la boucle");
        assert_eq!(suite, Suite::DoitRouvrir("point de reprise échoué".into()));
        let lignes: Vec<&str> = std::str::from_utf8(&sortie)
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        assert_eq!(lignes.len(), 2, "la réponse, puis l'avis : {lignes:?}");
        let avis: Value = serde_json::from_str(lignes[1]).unwrap();
        assert_eq!(avis["method"], "notifications/message");
        assert!(avis["params"]["data"].as_str().unwrap().contains("rouverte"));
        // Un avis est une notification : pas d'`id`, jamais.
        assert!(avis.get("id").is_none());
    }
}
