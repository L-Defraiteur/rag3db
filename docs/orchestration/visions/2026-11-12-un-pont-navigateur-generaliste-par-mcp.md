# Un pont navigateur généraliste pour les agents, par MCP

12 novembre 2026. Piste demandée par Lucie **pour plus tard** : regarder s'il
existe un équivalent open source de l'expérience d'une extension d'assistant
dans Chrome, utilisable par différents agents via MCP.
[Ticket de recherche](../../rag3weaver-archi/tickets/2026-11-12-etudier-un-pont-navigateur-ouvert-par-mcp.md).
Lucie a ensuite retenu la direction d'un **futur fork d'un projet existant**,
avec trois candidats ci-dessous. Aucun dépôt retenu définitivement, fork créé,
logiciel installé ou raccordement intégré à ce stade.

## L'expérience recherchée

Depuis le chat de l'atelier, l'agent pourrait observer une page, comprendre son
état, naviguer et interagir avec elle, avec un retour lisible de ses actions.
Le besoin mérite d'abord d'examiner les projets existants : une extension de
navigateur pourrait servir de pont vers les onglets d'une session humaine ;
un navigateur piloté séparément pourrait couvrir d'autres usages. Ces deux
parcours doivent être distingués dans la comparaison.

Le contrat doit être généraliste : un serveur MCP réutilisable par plusieurs
clients agents, sans imposer un fournisseur de modèle. Si un projet convient,
composer ses capacités avec les outils de l'atelier avant d'écrire notre propre
extension ou un second moteur de navigation.

## Ce que la recherche devra vérifier

- Licence réellement open source du pont, de l'extension éventuelle et des
  composants nécessaires ; maintenance, versions et sources datées.
- Accès aux onglets existants ou lancement d'une session distincte ; comportement
  avec plusieurs fenêtres, profils, agents et connexions simultanées.
- Observation et actions disponibles : structure de page, captures, navigation,
  clic, saisie, attentes et erreurs explicites. Distinguer possibilités annoncées
  et outils effectivement accessibles par MCP.
- Exécution locale ou dépendance à un service tiers ; circulation des pages et
  des données de session. Le rattachement à un profil humain doit être un choix
  explicite, sans acquisition implicite de ses identifiants ou de ses onglets.
- Intégration aux accords, à l'annulation et aux journaux de l'atelier : politique
  commune pour les actions et leurs effets, sans second décideur caché dans un
  client. Relier cette étude à la [vision du mode auto](2026-11-12-mode-auto-contextuel-et-accords.md).

La première sortie attendue est une comparaison sourcée et une recommandation
du socle à forker, avec le périmètre de nos adaptations et le suivi de l'amont.
L'étude et un éventuel prototype seront planifiés ensuite ; cette note garde
l'idée, elle ne lance pas de nouveau service.

## Trois candidats au futur fork

Projets proposés par Lucie et repérés le 12 novembre 2026. Vérification limitée
aux sources publiées ; aucune compilation ni qualification en exécution.
Les capacités décrites restent celles annoncées par les projets.

| Candidat identifié | Base annoncée et intérêt à examiner | Référence figée |
|---|---|---|
| [EmilLindfors/rust-browser-mcp](https://github.com/EmilLindfors/rust-browser-mcp) | Rust, WebDriver Chrome/Firefox, MCP stdio ou HTTP, interactions et formulaires, gestion de sessions et métriques. README sous MIT. Vérifier l'isolation effective des sessions et ce que recouvre l'observation réseau. | [README, 36f65ae95b](https://github.com/EmilLindfors/rust-browser-mcp/blob/36f65ae95b693ec69249c2efdb578d4539ad1c53/README.md) |
| [BB-fat/browser-use-rs](https://github.com/BB-fat/browser-use-rs) | Rust/CDP, Chrome ou Chromium, binaire `mcp-server`, extraction DOM avec éléments interactifs numérotés et ciblage par index ou sélecteur. README sous MIT. Ce dépôt correspond à la description fournie ; distinguer les autres projets homonymes. | [README, ed7b523bfe](https://github.com/BB-fat/browser-use-rs/blob/ed7b523bfe6935dbddac3fded23e49218f585e9e/README.md) |
| [yfedoseev/browser_oxide — browser_oxide_mcp](https://github.com/yfedoseev/browser_oxide) | Serveur MCP du moteur BrowserOxide, licence annoncée MIT OR Apache-2.0. **Écart avec la description initiale : le projet actuel annonce un moteur propre, sans Chromium sous-jacent**, pas une base chromiumoxide. Les profils Chrome/Firefox/iPhone/Pixel sont des identités simulées, pas quatre moteurs pilotés. | [README, 9cd4af36b7](https://github.com/yfedoseev/browser_oxide/blob/9cd4af36b744bc8f9047bcceeb341610a6f32109/README.md), [crate MCP](https://github.com/yfedoseev/browser_oxide/blob/9cd4af36b744bc8f9047bcceeb341610a6f32109/crates/browser_oxide_mcp/Cargo.toml) |

L'attribution de `browser_oxide_mcp` repose sur le nom de crate retrouvé ; si
Lucie visait un autre dépôt fondé sur chromiumoxide, conserver cette distinction
et ajouter son URL à l'étude. Une compatibilité CDP annoncée n'implique pas que
le moteur soit Chromium ni qu'il sache s'attacher aux onglets d'un Chrome humain.

Pour le choix : comparer la surface MCP utile à l'atelier et la fidélité aux
vraies pages, puis le coût du fork, les licences complètes et la maintenabilité.
Ne pas embarquer une boucle d'agent autonome simplement parce que le socle en
propose une : l'atelier possède déjà son agent et ses contrats d'exécution.

## Exécution JavaScript à la demande

Lucie souhaite garder cette capacité parmi les critères du futur fork.
Sur `rust-browser-mcp` au SHA ci-dessus, la surface n'est pas seulement annoncée :
[`automation.rs`](https://github.com/EmilLindfors/rust-browser-mcp/blob/36f65ae95b693ec69249c2efdb578d4539ad1c53/src/tools/automation.rs)
déclare `execute_script` avec `script` obligatoire et `session_id` optionnel ;
[`server.rs`](https://github.com/EmilLindfors/rust-browser-mcp/blob/36f65ae95b693ec69249c2efdb578d4539ad1c53/src/server.rs)
le route vers
[`handle_execute_script`](https://github.com/EmilLindfors/rust-browser-mcp/blob/36f65ae95b693ec69249c2efdb578d4539ad1c53/src/handlers/page.rs),
qui appelle `client.execute(script, vec![]).await` et renvoie le résultat dans
un contenu texte `Script result: …` accompagné de la session. Aucun argument
JavaScript séparé n'est transmis par cette façade.

Exemple de demande possible, non exécutée :

```json
{"script":"return { title: document.title, url: location.href };","session_id":"chrome_demo"}
```

La présence d'un `.await` Rust ne suffit pas à qualifier un mode JavaScript
asynchrone distinct. La façade relue ne propose pas de commande dédiée
`execute_async` ; résolution des promesses, délais, erreurs et forme des valeurs
retournées devront être vérifiés avant de promettre ce contrat à l'agent.
Pour l'atelier, cette capacité rejoint les mêmes accords et journaux d'effets
que les autres interactions avec les pages.
