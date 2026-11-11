# codeparsers : ce qu'il faut savoir pour reprendre (11 novembre 2026)

*Par la session « codeparsers » de Claude Code, pour un agent qui reprend sans mémoire de nos échanges.
L'état du travail en cours est dans [01-rapport-de-session.md](01-rapport-de-session.md). Tout chemin
`~/…` est relatif au compte `lucied` ; « le portable » est le poste de Lucie, « luciepc » le second poste
(`ssh lucied@luciepc`).*

## 1. Le dépôt et son sous-module

- **codeparsers** est une bibliothèque Rust (crate `codeparsers`) : elle analyse du code source avec
  tree-sitter et rend, par fichier, des **scopes** (un morceau nommé du code : fonction, méthode, classe,
  variable, ou **zone de fichier** `file_scope_NN`, le texte entre deux scopes) et des **relations**
  entre eux (`CONSUMES`, `PARENT_OF`, `INHERITS_FROM`, `USES_LIBRARY`…). Dépôt :
  `github.com/L-Defraiteur/codeparsers`, branche principale `master`.
- Dans rag3db, il est le **sous-module git** `extension/rag3weaver/codeparsers`. Le commit que rag3db
  enregistre s'appelle **le pointeur**. Le faire avancer est un lot de rag3db : on pousse d'abord le
  commit sur le `master` de codeparsers, puis le pointeur sur le `master` de rag3db, dans le même
  dernier pas.
- **Pointeur et Cargo.lock** : si le nouveau commit de codeparsers ajoute ou change une dépendance
  (`Cargo.toml`), le `Cargo.lock` de rag3weaver change aussi ; il part **dans le même lot** que le
  pointeur, jamais à part. Vérifier par `git -C extension/rag3weaver/codeparsers diff <ancien> <neuf> --
  Cargo.toml`.
- Après un `rebase` ou un `checkout` d'un worktree de rag3db : `git submodule update
  extension/rag3weaver/codeparsers`, sinon le sous-module reste à l'ancien commit et apparaît « modifié ».
- **Un commit de dossier emporte le sous-module** : `git commit -- extension/rag3weaver` commite aussi
  un pointeur déplacé par mégarde. Commiter par chemins précis.

### Où est quoi dans codeparsers

- `src/scope_extraction/` : un extracteur par langage (`rust_…`, `python_…`, `cpp_…`, `c_…`,
  `go_…`, `c_sharp_…`) et `base_scope_extraction_parser.rs`, qui porte JavaScript/TypeScript et ce qui est
  commun (zones de fichier, imports par regex, références lues dans le texte d'une zone, locaux).
  `types.rs` : `ScopeInfo` (un scope : nom, sorte, lignes, références, imports, `parent`,
  `visible_until_line`…), `ScopeFileAnalysis` (un fichier), `ImportReference` (un import : `source`,
  `imported`, `alias`, `kind` Default/Named/Namespace/SideEffect/Dynamic, `is_local`, `line`),
  `IdentifierReference` (une mention d'un nom, avec son `kind` : `LocalScope`, `Import`, `Unknown`).
- `src/relationship_resolution/relationship_resolver.rs` : le résolveur. Il construit une table nom →
  scopes (`scope_mapping`) puis, par scope, résout ses références : locales (`resolve_local_scope_references`),
  par import (`resolve_import_references`, seulement en mode « entre fichiers »), inconnues par le nom
  (`resolve_unknown_references`). `types.rs` : `ResolvedRelationship` et ses `RelationshipMetadata`
  (`by_name`, `external_binding`, `sites` avec la ligne de chaque usage).
- `src/parallel/project_parser.rs` : `ProjectParser::parse_project`, l'entrée des tests (avec
  `content_map` pour des fichiers virtuels).
- `src/syntax_highlighting/` : la coloration (`highlight(texte, langue)`) sur `tree-sitter-highlight`.
- `src/import_resolution/` : ancien, **jamais appelé** (« resolvers are not yet implemented in Rust »).
- `tests/` : un fichier par sujet ; le harnais commun est `ProjectParser` + `content_map` (voir
  `tests/fausses_aretes.rs`, `tests/locales_js.rs`, `tests/require_js.rs`).

### Le mode « fichier seul » de rag3weaver (important)

rag3weaver appelle codeparsers avec `resolve_cross_file: Some(false)` (`src/code.rs`, vers la ligne
1041) et `project_files` (tous les fichiers du projet). Dans ce mode, **codeparsers ne relie rien entre
fichiers** : `resolve_import_references` n'est appelé que pour poser les liaisons JS/TS (voir §4). Les
liens entre fichiers se font dans **le rendez-vous** de `code.rs`. Un correctif de résolution entre
fichiers écrit seulement dans codeparsers **ne change pas l'index** : le vérifier avant de mesurer.

## 2. Les grammaires tree-sitter 0.27, et clangd en oracle

- codeparsers dépend de `tree-sitter` 0.27 (qui lit l'**ABI 15** des grammaires) et des crates de
  grammaires (rust 0.24, python 0.25, go 0.25, c 0.24, cpp 0.23, typescript 0.23, c-sharp 0.23, bash 0.25,
  css 0.25, html 0.23, scss en copie locale `vendor/`). **C'est l'ABI qui décide, pas Cargo** : une
  grammaire compilée pour une autre ABI s'installe mais panique à l'exécution. `tests/grammaires.rs`
  vérifie que chaque grammaire se charge.
- Passer à tree-sitter 0.27 (25 octobre) a fait reculer un cas C++ : une macro `fmt/format.h` lue comme
  un corps de fonction (ticket ouvert, recul accepté par Lucie).
- **Aucun compilateur dans le produit** (décision de Lucie, 25 octobre : trop lourd). clangd et les
  autres serveurs de langage ne servent que **d'oracle de mesure** (§6).

## 3. L'IR et les gabarits

- **Les gabarits** (`extension/rag3weaver/templates/backends/code/` et les copies dans
  `~/.config/rag3weaver/<base>/templates/`) déclarent, en JSON (`backend.json`), les entités et les
  relations de la base de code : `Scope`, `File`, `Symbol`, et les arêtes `CONSUMES`, `CONSUMED_BY`
  (la réciproque), `DEFINES`, `MENTIONS`, `DEFINED_IN`, `INCLUDES`… Règle de Lucie : une organisation
  nouvelle se décrit dans le gabarit (`EntityConfig`, découpe, relations), **jamais en dur** dans le code.
- **L'IR** (`extension/rag3weaver/ir/`, crate `rag3weaver-ir`) est la forme intermédiaire que les
  gabarits produisent pour le moteur. Un lot qui touche `code.rs`, l'IR ou un gabarit joue
  **`e2e_postgres`** (le même index vers PostgreSQL) en plus d'`e2e_code`.

## 4. La résolution des arêtes

### Dans codeparsers (par fichier)

- Une mention est classée par `classify_scope_references` : un nom importé devient `Import` (avec sa
  `source`), un nom d'un scope du fichier devient `LocalScope`, le reste `Unknown`.
- Les locaux d'une fonction (paramètres, `let`, motifs) sont exclus des références de cette fonction.
  Rust ajoute les liaisons de motifs (`collect_pattern_bindings`) ; la **garde** d'un bras `match`
  (`Motif if f(x)`) n'est pas un motif mais du code (7 nov.).
- **Les zones de fichier** lisent leurs références dans le texte, ligne à ligne, par regex
  (`extract_identifier_references_from_text`) ; les lignes de commentaire, d'attribut, de préprocesseur
  et (depuis le lot `require`) d'import ne nomment rien.
- **La portée d'une locale JS/TS** (8 nov., sur master) : `ScopeInfo::visible_until_line`, la dernière
  ligne où une variable se voit (la fin du bloc pour `const`/`let`, de la fonction pour `var`, `None`
  pour une globale). Le repli du résolveur prend la déclaration la plus proche qui couvre la mention.
- **La liaison de module externe** (lot `require`, pas encore sur master) : un import JS/TS qui ne se
  résout pas dans le projet envoie la mention à la zone de sa ligne d'import, avec
  `external_binding = true`.

### Dans rag3weaver (`src/code.rs`)

- Pour une relation de codeparsers, la marque est `fichier` (dans le fichier), `nom` (`by_name` : un
  appel sur un receveur non typé relié par le nom), `import` (`USES_LIBRARY`), et `liaison`
  (`external_binding`, lot `require`).
- **Le rendez-vous** relie une mention d'un nom à un scope d'un autre fichier : un scope DEFINES le
  `Symbol` de son nom, une mention MENTIONS le `Symbol` du nom qu'elle écrit, et `choose_target` /
  `departager` choisissent parmi les définisseurs. Un import de la mention (`import_modules`, tiré de
  `import_origin`) désigne un fichier (`module_designe_fichier`) : un seul définisseur désigné → marque
  `import` ; un seul définisseur sans import → `nom` ; plusieurs sans départage → `ambigu` (l'arête vers
  chacun) ou abstention. **Le rendez-vous ne connaît que le nom écrit** : `import X from "./m"` ne trouve
  que les scopes nommés `X`, jamais un export par défaut d'un autre nom.
- **C/C++ (INCLUDES, 7 nov.)** : la visibilité d'un définisseur passe par les `#include` (propriété
  `visibility` : `inclusion`, `entete`, `declaration`) ; les homonymes sont comptés avant le filtre (un
  filtre **ôte** des candidats, il ne **promeut** jamais), et l'outil `usages` dit les usages invisibles
  (`invisible_usages`).

## 5. Comment on mesure une règle : A/B sur une vraie base

Une règle de résolution se mesure **sur code-lourd** (la base de code de rag3db lui-même, l'arbre
`~/git_workspaces/rag3db-lourd` à luciepc, commit a72acfeef), avant le push :

1. **Deux index** de la même base : A avec le code d'avant, B avec le lot. Chacun par un **hôte isolé**
   (le binaire `rag3weaver-backend daemon --address 127.0.0.1:7978`, son propre dossier), une copie du
   gabarit de code-lourd qui pointe vers une base neuve, l'appel `index` (`mcp_appel.py`), l'attente de
   « indexation terminée » dans le journal, l'arrêt par SIGTERM. **Toute la séquence sous un seul
   `poste lourd`**.
2. **La sonde** (`tests/sonde_aretes_par_relation.rs`, variable `SONDE_REQUETE`) sort les arêtes
   `CONSUMES` de chaque base (sur une **copie** de la base, faite par reflink).
3. **La différence** : les arêtes ajoutées et ôtées, comparées **par mention et cible, sans l'appelant**
   (ôter des scopes renumérote les `file_scope_NN`, et une même arête changerait d'appelant).
4. **L'oracle** juge les arêtes (§6) : l'échantillon de référence (les mêmes mentions qu'avant), puis
   les ajoutées et les ôtées.

Le modèle complet est `~/.cache/codeparsers-notes/mesure-require-2.sh` à luciepc. Une mesure se fait
**sous une seule lib** du moteur, épinglée par son identifiant complet. Une durée d'index ne vaut que
sur un poste calme (§8).

## 6. L'oracle (`extension/rag3weaver/scripts/oracle_lsp.py`)

- Pour un langage, il tire au hasard (**graine 25**) 300 arêtes, demande au serveur
  (`textDocument/definition` puis `declaration`) où va le nom à la ligne de la mention, et rend un
  verdict par arête dans un `.tsv`.
- **Juge strict** (`ORACLE_STRICT=1`, toujours) : le fichier de la cible est ouvert aussi ; « juste »
  seulement si une position rendue tombe dans la cible même.
- Verdicts : **juste** ; **faux** ; **décl** (le serveur désigne la mention elle-même) ; **muet** (rien) ;
  **en-tête seul** (C/C++ : seulement une déclaration d'en-tête) ; **liaison d'import** (notre cible lie
  le nom à un module hors de l'arbre, et le serveur ne rend que ce module) ; **réexportation** (le serveur
  rend la ligne qui lie le nom dans le fichier de la mention, notre arête suit l'import).
- Variables : `ORACLE_EXIGER=1` exige une indexation annoncée par le serveur (`$/progress`) ; à
  n'utiliser que pour rust-analyzer et clangd (pyright et typescript-language-server n'annoncent rien,
  ils calculent à la demande). `ORACLE_BG_INDEX=1` + `ORACLE_PLAFOND` : l'index d'arrière-plan de clangd.
  `ORACLE_ARBRE_CC` : l'arbre d'où vient `compile_commands.json`. `ORACLE_TSSERVER` : le `lib/` d'un
  TypeScript 5 (TypeScript 7 n'a plus de `tsserver.js`).
- À luciepc : l'arbre jugé `~/.cache/codeparsers-notes/oracle/arbre` (rag3db à a72acfeef), les arêtes et
  scopes de référence `oracle/aretes-<langage>.txt`, `oracle/scopes-<langage>.txt`, TypeScript 5 dans
  `oracle/ts5`. Page des chiffres :
  `extension/rag3weaver/docs/10-octobre-2026-18h20/codeparsers/11-oracles-par-langage.md`.
- Chiffres du 8 novembre (juge strict, 300 arêtes tirées) : Rust 252 justes, Python 269, TypeScript 243.
  Après le correctif des locales JS, sur les mêmes mentions TypeScript (324 arêtes) : justes 259 → 279,
  faux 30 → 10.

## 7. Les procédures

- **Le push d'un lot** : `docs/procedures/push-d-un-lot.md`. En bref :
  - la lib du moteur **épinglée** par son identifiant complet (`RAG3DB_LIB=rag3db-build-…`), jamais
    `courant` ;
  - `scripts/check_features.sh` ;
  - la **lib entière** (`cargo test --lib`) sous le jeu large
    `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm` ;
  - les témoins navigateur (`scripts/temoins_navigateur.sh`, avec `RAG3WEAVER_PAQUETS_REACT`) ;
  - les e2e du changement, **une suite par appel** de `run_e2e.sh` (seul le dernier nom filtre), et
    `e2e_postgres` si `code.rs`, l'IR ou un gabarit change ;
  - le push dans un `if grep -qx VERT` : vert = **aucun rouge**, jamais un compte de verts.
- **Le verrou du poste** : `~/.cache/rag3weaver-build/poste`. `poste lourd timeout N <commande>` pour
  tout travail lourd (au plus N à la fois, file dans l'ordre ; `poste --qui` pour voir) ; `poste push`
  ne couvre **que le dernier pas** (fetch, rebase, push, contrôles). Si le rebase sous le verrou apporte
  du code : s'arrêter, rendre le verrou, rejouer dehors, reprendre le verrou.
- **« Poussé » après trois contrôles lus** : `git status` propre, aucun rebase en cours,
  `git log -1 origin/master` après un `fetch` égal à la tête poussée.
- **L'ordre des pushes** est donné par l'orchestration (« à toi ») ; un lot attend son tour, sa place
  lourde rendue.
- **Git** : chaque commit se pousse sur sa branche dans la minute ; après un rebase, un nouveau nom de
  branche (`-2`, `-3`…) ; jamais de push en force ; jamais de `git push --delete` sans Lucie ; commit par
  chemins (`git commit -- <chemins>`) ; identité `luciedefraiteur@gmail.com` (jamais l'adresse pro) ;
  aucune ligne d'attribution d'IA.
- **L'arbre de Lucie** (`~/git_workspaces/rag3db` sur le portable) : on n'y commite rien et on n'y
  bascule rien ; lire seulement.
- **Un défaut connu non corrigé** = un ticket dans `docs/tickets/` (champs d'en-tête `État`, `Famille`,
  `Gravité`, `Atteignable en service`, `Touche rag3weaver` ; fermer = `git mv` dans `closed/` et un
  `État` qui commence par « corrigé » ; contrôle : `extension/rag3weaver/scripts/check_tickets.py`, avec
  `--lot origin/master` pour un lot).
- **Une nouveauté visible** = une ligne dans `docs/nouveautes.md`.

## 8. Les pièges appris

- **Un rouge de 60 s** (« n'est pas indexé en 60 s ») à luciepc est souvent **le disque** : le NVMe
  occupé par des TRIM (`discard=async`, ou le `fstrim` hebdomadaire du lundi vers 1 h 35). Lire
  `/proc/pressure/io` (`some avg60` sous 5 à 8 pour un poste calme) avant d'accuser le code ; rejouer
  le test **seul**. Un rouge qui tient seul, disque calme, est une régression.
- **Départager avant d'accuser** : rejouer le même test sur le même arbre sans le changement (par
  exemple le sous-module ramené à son commit d'avant). Si c'est rouge aussi, ce n'est pas le lot.
- **Un témoin vert se lit par son chemin** : vérifier que l'arête attendue part de la vraie mention (une
  sonde qui imprime les relations et leurs métadonnées), pas d'un chemin de hasard. Le témoin du
  `require` passait d'abord par la ligne du `require` elle-même.
- **Un script distant survit à son ssh** : un `timeout` local ne tue pas le bash de luciepc. Arrêter
  par les pids lus (`ps --ppid`, `/proc/<pid>/cmdline`), **jamais par motif** (`pkill -f` se trouve
  lui-même dans la ligne de commande ssh).
- **Arrêter un hôte rag3weaver par SIGTERM**, jamais `kill -9` : sa base (le WAL) casse sur arrêt
  brutal.
- **Jamais de substitution de processus infinie** (`<(yes)`) : le processus survit orphelin et brûle
  un cœur.
- **Le shell de l'outil est zsh** : une variable non citée ne se découpe pas ; `$c:e` est un
  modificateur (écrire `${c}:…`) ; pas de `/dev/tcp`.
- **Le target** cargo d'une passe pèse 19 à 26 Gio sous le jeu large ; `run_e2e.sh` refuse au-delà de
  `RAG3WEAVER_TARGET_MAX_GO` (32 par défaut).
- **La mesure tient compte de la charge** : une durée prise pendant un TRIM, ou à côté d'un autre lourd,
  ne se compare à rien. Les résultats (les verdicts) restent valables.
- **Les données mtga** à luciepc (`~/.cache/codeparsers-notes/hors-arbre-18-oct/`,
  `archive-mtga-18-oct/`, environ 24 Gio, le projet Magic de Lucie, sans doute la seule copie) : ne
  jamais les supprimer ni les déplacer, même lors d'un ménage du dossier.

## 9. Mes tickets ouverts (une ligne chacun)

- `docs/codeparsers/tickets/2026-11-08-un-require-relatif-s-arrete-a-la-liaison.md` — un `require` relatif s'arrêtait
  à la ligne de liaison ; le lot `require` le traite (non poussé).
- `docs/codeparsers/tickets/2026-11-07-la-marque-nom-relie-un-appel-de-methode-standard-a-un-homonyme.md` — un appel
  sur un type de la bibliothèque standard (`Vec::split_off`) relié par le nom à un homonyme du dépôt.
- `docs/codeparsers/tickets/2026-10-25-une-definition-cpp-se-lie-a-sa-declaration-d-en-tete.md` — C++ : une
  définition reliée à sa propre déclaration d'en-tête.
- `docs/codeparsers/tickets/2026-10-25-fmt-format-h-une-macro-lue-comme-un-corps-de-fonction-depuis-tree-sitter-0-27.md`
  — une macro lue comme une fonction depuis tree-sitter 0.27 (recul accepté).
- `docs/codeparsers/tickets/2026-10-18-des-scopes-sans-defined-in-dans-une-reindexation-a-neuf.md` — des scopes sans
  arête vers leur fichier après une réindexation à neuf.
- `docs/codeparsers/tickets/2026-10-11-un-fichier-rust-s-analyse-plus-lentement-a-la-ligne.md` — le coût d'analyse
  d'un fichier Rust croît avec sa longueur plus que linéairement.
- `docs/codeparsers/tickets/2026-10-11-new-est-un-mot-vide-en-rust.md` — `new` est traité comme un mot vide.
- `docs/codeparsers/tickets/2026-10-04-des-receveurs-restent-sans-type.md` — des receveurs d'appel restent sans type
  lu (en cours, partagé).
- `docs/codeparsers/tickets/2026-10-04-une-arete-ne-dit-pas-comment-elle-a-ete-resolue.md` — les marques
  `resolution` (en cours, partagé ; la marque `liaison` du lot `require` s'y ajoute).

## Fait d'export et preuve LSP — complément de reprise

Sur la branche cx-codeparsers-require-2 (pas master), ScopeFileAnalysis expose
un fait de fichier optionnel default_export : nom ou ligne de l'instance.
Le consommateur ne doit pas relire le texte propre d'un scope pour retrouver
un préfixe export retiré par la découpe. L'import et le symbole d'export gardent
le fichier exact avec son extension ; le contenu partagé reste distinct du
contexte de résolution d'occurrence. Voir la page 03 pour hashes et preuves.

L'oracle historique utilise le nom cible pour retrouver le token source : une
zone ou un alias peuvent être « muets » sans interrogation. Séparer question
non envoyée, serveur sans réponse, réponse visible hors corpus et verdict de
cible. Les 415 liaisons historiques n'ont pas déclenché de questions ; aucune
conclusion de précision n'en découle. Le ticket du contrat mention/cible et
le lecteur de TSV préservent les preuves sans changer cet oracle.

## Livraison du lot require et exports — état après revue finale

L'état de reprise ci-dessus est historique. Le lot initial et son complément
exports sont réunis sur cx-codeparsers-require-3 ; code d92c0f4b7, producteur
a730a9b publié sur origin/cx-codeparsers-default-metadata. Passe intégrée sur
50ebbb760/base68b9/lib29a7 relue : features5/5, lib2114 réussis/64 ignorés
non revendiqués (39 repris navigateur), navigateur39, code36, usages19,
PostgreSQL privé34, sans échec ni ignoré dans ces étapes séparées.
Le contrat navigation vers le symbole/valeur exportée est accepté ; les cinq
strict-faux restent dans le bilan, sans score global. Tour de push accordé.
Ticket corrigé déplacé vers
`docs/tickets/closed/2026-11-08-un-require-relatif-s-arrete-a-la-liaison.md`.
Les actions non livrées restent aux tickets codeparsers de provenance oracle,
homonyme platform, import_binding et receveurs/résolution ; aucune nouvelle
campagne LSP ni capture n'est lancée par cette livraison.

Les worktrees et branches locales du lot, le target-codeparsers-clic et la base
PG privée sont à supprimer immédiatement après les trois contrôles du push.
Les branches distantes et les preuves sous ~/.cache/codeparsers-notes/ restent
conservées ; ne pas effacer les données des deux A/B avec le target. Le résultat
effectif du nettoyage est transmis à l'orchestration après exécution.

## Livraison et nettoyage effectifs

Master reçoit 2e910930dbb9684fdfeb1f387fc2a430c9604880 sous poste push après
verdict relu. Trois contrôles lus : status propre, aucun rebase en cours,
origin/master égal à HEAD. Le sous-module a730a9ba89ef233423bb7063f85181de26d98ce0
est accessible sur origin/cx-codeparsers-default-metadata.

Sur luciepc : worktree de passe cx-wt-codeparsers-require, worktrees des deux
mesures (cx-wt-codeparsers-mesure-v2, rag3db-require-default-mesure) et ancien
rag3db-brancher supprimés ; branches locales -2/-3/-4 et require-temoins retirées.
Target unique target-codeparsers-clic et base PG privée
cx_codeparsers_20261111_1698_test supprimés. Sur le portable : worktrees require,
témoins et métadonnées, branches locales require et variantes, ainsi que les
branches locales du sous-module associées, supprimés. Les branches distantes
restent intactes. Mesures, bases A/B, binaires figés et journaux sont conservés
sous ~/.cache/codeparsers-notes/, hors du target supprimé ; derniers journaux
du wrapper copiés dans cx-passe-require-3/wrapper-final.

Incident de nettoyage : codeparsers-garde distant était un worktree rattaché
au sous-module de rag3db-brancher. Le retrait du parent invalidait son lien Git.
Il a été rétabli en clone autonome au commit historique 672a0d5, sans écraser
ses fichiers ; status propre après rétablissement. Ce clone ancien est conservé,
ainsi que la trace dans cx-passe-require-3/reparation-ancien-codeparsers-garde.log.
Aucun travail restant pour cet incident ; les suites ouvertes sont les tickets
oracle/provenance et homonyme (et les chantiers antérieurs distincts).
