# Les propositions : proposer, montrer, appliquer (la marche suivante de l'IDE)

19 octobre 2026, everything declarative, sur la demande de c7. La page 09 de codeparsers
(`../10-octobre-2026-18h20/codeparsers/09-l-agent-qui-edite-des-gabarits.md`, §2) donne la forme.
Son lot 1 (`tsx`/`jsx` dans `KINDS`, `declare_set`) est sur master (`c5358704f`, ab), son lot 2
est C1 (`/ide/graphe`, l'outil `graph`). Ici, son lot 3.

## Ce qui manque : valider sans écrire

`declare_set` écrit, recharge (`check_reload`), puis défait au moindre refus. Une proposition doit
être validée **sans rien écrire dans l'arbre**, par la même validation que le chargement :
`PreparedBackend::load`. Or le chargement lit le disque par deux aides de `backend.rs`
(`lire_octets`, `lire_texte`) et par une quinzaine d'appels directs à `std::fs`, répartis dans six
modules :

- `backend.rs` : le graphe d'une réaction, la racine du workspace, les `canonicalize` ;
- `dataflow/scripted_node.rs` : la liste de `nodes/`, les déclarations, les scripts ;
- `components.rs` : `libs.json`, l'existence des objets de `.libs/`, la liste de `components/`,
  les déclarations, la présence des sources, les graphes de clic ;
- `routes.rs` : la liste de `views/` et les vues ;
- `fichiers_entite.rs` : le gabarit d'un miroir ;
- `builtin_templates.rs` : la dérive des copies locales.

## Décisions

1. **Un lecteur de déclarations, passé en paramètre** (`declarations::Lecteur`) : lire des octets
   ou du texte, dire si un chemin est un fichier ou un dossier, lister un dossier. Deux
   implémentations :
   - `Disque`, qui fait ce que fait le code d'aujourd'hui ;
   - `Recouvrement`, le disque sous des fichiers en mémoire (chemin absolu → contenu), qui
     ajoutent ou remplacent ; un dossier listé voit les fichiers du recouvrement qui y sont.

   Pas d'état caché dans un fil : le lecteur se lit dans la signature.
2. **`PreparedBackend::load` garde sa signature** et appelle
   `PreparedBackend::load_avec(path, &Disque)`. Chaque lecture du chargement, directe ou dans les
   six modules, passe par le lecteur. Les fonctions publiques touchées (`Components::load_in`,
   `scripted_node::discover`, `Routes::load`, `Miroir::nouveau`) gardent la leur, et gagnent une
   variante `…_avec(…, lecteur)`.
3. **`Backend::valider_sur(files)`** fait la validation d'une proposition :
   - `load_avec` sur le recouvrement ;
   - puis la même règle que `check_reload` (`opening_changes` : un changement de base, de
     modèles ou d'entités est une migration, pas une proposition) ;
   - il rend le verdict, accepté ou refus avec chemin et ligne quand le refus les dit.

   Il n'écrit rien, et un témoin le prouve : l'arbre est inchangé, octet pour octet, après une
   proposition acceptée comme après une refusée.
4. **La proposition est un état de l'hôte.** L'hôte la garde : un id, les fichiers avant et
   après, la raison, l'auteur (la session), le verdict, l'état (`proposée`, `appliquée`,
   `rejetée` et pourquoi). Il la publie sur `/ide/flux`, sujet `declarations` :
   `Proposed`, puis `Applied` ou `Rejected`. La page ne garde rien qu'elle ne relise de l'hôte.
5. **`appliquer(id)`** revalide sur l'arbre du moment, qui a pu bouger. Si c'est accepté, il passe
   par `declare_set`, ce qui donne l'écriture, le rechargement et le commit sous le verrou
   `DECLARING`. **`rejeter(id, pourquoi)`** garde le pourquoi.
6. **Le mode `approbation` par défaut** (c7) : l'agent propose, une personne applique dans l'IDE.
   Le mode `auto` se déclare par backend (`declarations: "auto"`, sur le modèle de
   `workspace.commands`) ; l'agent peut alors appliquer ses propres propositions.
7. **Les outils de l'agent** : `propose`, `apply`, `reject` (identifiants en anglais), à côté des
   `DeclarationTools`. `apply` n'est donné à l'agent qu'en mode `auto`.

## La vue dans l'IDE

8. `/ide/code/<id>/propositions/` liste les propositions du backend et leur état.
   `/ide/code/<id>/propositions/<pid>` montre une proposition :
   - la différence textuelle, fichier par fichier ;
   - pour un graphe (`.mmd`, `.clic.mmd`), **le diff de graphe nœud à nœud** : le graphe d'avant et
     celui d'après, dans la forme de `/ide/graphe`, et les nœuds et arêtes ajoutés, retirés ou
     changés, surlignés sur le dessin ;
   - pour un composant, **l'aperçu du module régénéré sans l'appliquer** : l'aperçu se charge sur
     le recouvrement de la proposition (`/ide/apercu/<id>/<Comp>?proposition=<pid>`).

   Deux boutons, Appliquer et Rejeter (avec son pourquoi), en mode `approbation`.

## Les lots

- **P1** : le lecteur, le recouvrement, `load_avec` à travers les six modules,
  `Backend::valider_sur`. Témoins : une proposition acceptée, une refusée avec sa ligne, l'arbre
  inchangé, la parité de `load` et de `load_avec(Disque)` sur les gabarits livrés.
- **P2** : les propositions gardées par l'hôte, le sujet `declarations`, les routes de l'IDE
  (`POST /ide/propositions`, `…/<pid>/appliquer`, `…/<pid>/rejeter`, `GET …`), et les outils de
  l'agent.
- **P3** : la vue, avec le diff textuel, le diff de graphe et l'aperçu sur le recouvrement.
- **P4** : le témoin de bout en bout dans le navigateur. Un composant est proposé, sa proposition
  paraît, le graphe changé est surligné, l'aperçu joue le module proposé, et Appliquer l'écrit et
  le commite.

## État (19 octobre 2026)

- **P1** sur master (`95b6bccf3`) : le lecteur, le recouvrement, `load_avec`, `valider_sur`, et la
  garde du recouvrement (chaque déclaration empoisonnée sur le disque, proposée valide).
- **P2** sur master (`7b544bfb7`) : `proposer`, `appliquer`, `rejeter`, `PropositionChanged` sur
  `declarations`, le mode `approbation` par défaut et les outils de l'agent selon le mode.
- **P3 et P4**, dans le même commit :
  - les routes de l'IDE ;
  - le détail d'une proposition, avec les graphes touchés avant et après, leur diff nœud à
    nœud et arête à arête, et les composants touchés ;
  - l'aperçu et `/ide/graphe` sur le recouvrement d'une proposition (`?proposition=<n>` ;
    `module_graph_avec`, `graph_json_avec`, `components_on_disk_avec`) ;
  - les vues de la page, et le dessin de graphe partagé avec le DAG vivant (`dessinerGraphe`) ;
  - le témoin de bout en bout `une_proposition_se_voit_se_joue_puis_s_applique_dans_l_ide`.

Reste le lot 4 de la page 09 de codeparsers : le worktree de tâche (une branche, une seconde
instance du backend, la relecture et la fusion dans l'IDE). Il attend la décision de Lucie sur la
base d'une tâche, que c7 a tranchée : copiée, jamais partagée en écriture.
