# Un pont navigateur généraliste pour les agents, par MCP

12 novembre 2026. Piste demandée par Lucie **pour plus tard** : regarder s'il
existe un équivalent open source de l'expérience d'une extension d'assistant
dans Chrome, utilisable par différents agents via MCP.
[Ticket de recherche](../../rag3weaver-archi/tickets/2026-11-12-etudier-un-pont-navigateur-ouvert-par-mcp.md).
Lucie a ensuite retenu la direction d'un **futur fork d'un projet existant**.
Les trois premiers candidats Rust sont conservés ci-dessous, puis complétés
par des ponts avec extension répondant mieux au besoin précisé. Aucun dépôt retenu définitivement, fork créé,
logiciel installé ou raccordement intégré à ce stade.

## L'expérience recherchée

Depuis le chat de l'atelier, l'agent pourrait observer une page, comprendre son
état, naviguer et interagir avec elle, avec un retour lisible de ses actions.
Précision de Lucie : **le parcours principal doit permettre de travailler dans
un Chrome visible, avec une extension ou un pont équivalent et un passage de
main humain**. Les connexions, doubles facteurs et pages de clés API ne doivent
pas finir en impasse faute d'accès de Lucie à la page. L'agent doit pouvoir lui
demander d'intervenir puis reprendre dans le même onglet et la même session.
Les onglets déjà authentifiés sont donc un critère central du choix.

Un navigateur lancé séparément peut aussi être visible ; le headless n'est pas
une obligation de WebDriver ou de CDP. Ce parcours reste utile, mais il ne prouve
pas le raccordement au Chrome déjà utilisé par Lucie. Le headless peut rester
une option pour les tâches qui s'y prêtent.

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

## Ponts avec extension à comparer en priorité

Relecture documentaire du 12 novembre 2026, sans installation ni essai réel.
Les sources identifient bien une extension à installer, au-delà d'un pilote de
navigateur. La fidélité du passage de main reste à qualifier en pratique.

| Projet | Correspondance avec le besoin | Sources figées |
|---|---|---|
| **Playwright MCP et Playwright Extension** | Extension Chrome/Edge/Chromium, serveur en mode `--extension`, choix de l'onglet et réutilisation de l'état du profil existant. Sources de l'extension dans Playwright ; le manifeste relu indique la version0.5.0. Ce candidat ne dépend pas d'une réécriture en Rust. | [Guide de l'extension](https://github.com/microsoft/playwright/blob/bac1f9f67e9cff2a8e7ecbac25d56ec3bb1d15c6/packages/extension/README.md), [manifeste](https://github.com/microsoft/playwright/blob/bac1f9f67e9cff2a8e7ecbac25d56ec3bb1d15c6/packages/extension/manifest.json) |
| **BrowserMCP/mcp** | Serveur local et extension dans le même dépôt, Chrome déjà connecté et onglets par client. La source actuelle décrit la construction du serveur et de l'extension ; ne pas reprendre sans vérification l'ancien constat d'un dépôt incomplet. Licence Apache-2.0 indiquée. | [README](https://github.com/BrowserMCP/mcp/blob/33da4f6a3582139a377725e6932aeeb3daaedbda/README.md), [licence](https://github.com/BrowserMCP/mcp/blob/33da4f6a3582139a377725e6932aeeb3daaedbda/LICENSE) |
| **jackwener/browsermcp** | Projet distinct malgré le nom proche : extension BrowserMCP Connector, hôte local, session Chrome existante et outils MCP dont un espace JavaScript persistant. Licence Apache-2.0 indiquée. Intéressant pour l'emprunt puis la restitution d'un onglet humain. | [README](https://github.com/jackwener/browsermcp/blob/d6597aacbac2f8cb1094b523852b8f53cd15b0a7/README.md), [licence](https://github.com/jackwener/browsermcp/blob/d6597aacbac2f8cb1094b523852b8f53cd15b0a7/LICENSE) |

Les liens d'installation sont dans ces guides ; aucune extension n'est installée
par cette étude. Le parcours humain et la composabilité avec notre code Rust
sont deux critères à satisfaire ensemble. Un pont existant ou une adaptation
ciblée peut être préférable à reconstruire le navigateur.

## Intégration Rust : d'abord le pont vers l'extension

Lucie précise le but : conserver les synergies avec notre code, et pouvoir
composer les capacités du navigateur dans rag3weaver. La seule disponibilité
d'un serveur MCP externe ne remplit pas ce besoin de réutilisation interne.

**Piste privilégiée à étudier : garder l'extension existante et transcrire en
Rust seulement la partie qui lui parle.** Si cette partie est petite et son
protocole clair, une transcription ciblée peut suffire. Le portage de tout le
serveur Node/TypeScript, de Playwright ou de sa boucle d'agent n'est pas le but.
L'extension peut conserver son implémentation JavaScript/TypeScript.

Frontière proposée, pas encore implémentée :

```text
outils et traitements rag3weaver ──┐
                                 ├── bibliothèque Rust de pont navigateur
façade MCP pour les autres agents ┘               ⇅ protocole de l'extension
                                         extension Chrome existante
                                                 ⇅
                                         onglets visibles de Lucie
```

Le cœur Rust doit être appelable comme bibliothèque : sessions et références
d'onglets, observation, actions, exécution JavaScript, événements et erreurs
utilisables sans faire passer nos appels internes par MCP. La façade MCP expose
ces mêmes capacités ; elle ne doit pas contenir une seconde logique métier.
Les outils déclaratifs de l'atelier composent ce cœur, tandis que leurs accords
et leur orchestration restent portés par les contrats communs.

L'étude de chaque candidat doit donc isoler le protocole réel entre extension
et hôte : messages, transport, appairage, versions, sélection des onglets,
déconnexion et cycle de vie. Mesurer le volume de code à transcrire et ce qu'il
délègue à ses dépendances : un relais très court peut dépendre d'un moteur
d'automatisation beaucoup plus vaste. Les fonctions fournies directement par
l'extension et celles calculées par l'hôte doivent être distinguées.

Si un petit pont suffit, proposer cette tranche avec des échanges de référence
et des preuves de compatibilité avec la même extension : observation/action,
JavaScript et résultats, passage de main humain, reprise et déconnexion. Si des
comportements indispensables résident dans le serveur amont, nommer lesquels
avant de choisir un portage plus large. Préserver licences et suivi de versions
du protocole. Aucune transcription ni création de fork engagée par cette note.

## Passer la main, puis reprendre

Parcours à concevoir sur les contrats communs de l'atelier :

1. L'agent identifie une étape nécessitant Lucie et désigne l'onglet ainsi que
   l'action attendue : connexion, validation, choix de compte ou intervention
   sur une page de configuration.
2. Il suspend ses actions sur cet onglet pendant l'intervention. Lucie peut voir
   et manipuler la page ; la demande doit permettre de rejoindre ce navigateur.
3. Après sa réponse, l'agent observe à nouveau la page et confirme l'état avant
   de reprendre. Une notification « terminé » ne remplace pas cette observation.
4. Une annulation ou déconnexion garde un état explicite et récupérable. Les
   journaux distinguent actions de l'agent et intervention humaine ; une étape
   inconnue ne devient pas une réussite implicite.

Le but est de pouvoir saisir une authentification dans le navigateur sans devoir
la dicter au chat. Si la tâche porte sur une clé API à créer ou configurer,
préciser la destination attendue de cette clé plutôt que rendre obligatoire sa
copie dans la conversation. Aucun accès à un profil ou à ses secrets n'est
effectué par cette note.

La topologie compte : si l'agent s'exécute sur luciepc et Chrome sur le portable,
le pont doit atteindre le navigateur que Lucie voit. Un serveur déclaré local
ne résout pas à lui seul ce cas ; transport, sélection du profil et arrêt de
session seront à qualifier sans supposer les deux programmes sur le même poste.

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
