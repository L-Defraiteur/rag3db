# Le contrat de la version de scope (lot 1a des vues)

13 novembre 2026, arbre principal, à la demande de c7. Page à relire par ED et recherche avant toute
ligne de Rust. Les références de code sont celles d'`origin/master` d59e99ed7, relativement à
`extension/rag3weaver/`. Rien n'a été compilé ni joué pour l'écrire.

## Ce qui existe déjà, et ce qui manque

**Le mécanisme est livré, générique.** Une entité dérivée déclare `derived.identity =
{"mode":"content","format":…}` (README, « Shared indexed content »). Le partage, la projection
vers les occurrences et le masque avant le top-k sont sur master depuis le 12 novembre (cinq
tickets fermés, `d7d2a6434`). Les occurrences dont le rendu est identique partagent alors une
version, ses morceaux et ses vecteurs.

**La forme pour le code est déjà écrite, dans un test.** `tests/e2e_scope_versions.rs` part du
schéma de code réel et le surcharge par un manifeste privé (l. 28-65) :
- `Scope` est routé par `searchIndex: "ScopeContent"`, avec `chunked: false` et aucun signal ;
- `ScopeContent` est dérivé en mode contenu, format `code-scope-v1`.

Ses trois tests, tous `#[ignore]`, ne se jouent que par `run_e2e.sh`. Ils couvrent :
- un seul embarquement pour deux occurrences, et pas de nouvel embarquement après un déplacement ;
- deux docstrings distinctes qui restent deux versions ;
- les arêtes par occurrence ;
- la recherche publique sur `Scope`, dense et BM25.

**Ce qui manque : le schéma de production.** `register_code_schema` (`src/code.rs:646`, appelé par
`src/backend.rs:2389-2396`) enregistre encore `Scope` seul, embarqué et découpé directement. Le
README le dit : les gabarits de code de production n'activent pas l'option. C'est cela, le lot 1a.

## 1. L'identité d'une version

- **L'adresse est celle du mécanisme**, sans règle propre au code. `derived_content_uuid`
  (`src/dataflow/derive_nodes.rs:103-125`) hache, sous le domaine de l'entité dérivée :
  - le format (`code-scope-v1`) ;
  - la cellule `org × projet` ;
  - les champs rendus, octet pour octet ;
  - les rôles de contenu et de titre ;
  - `chunked` et la recette de découpage.
- **Les champs rendus pour le code** sont ceux du test : `name` (le titre), `signature`,
  `content`, `docstring`, `scope_type`, `language`.
  - Deux corps identiques sous deux noms font deux versions : le nom fait partie de ce qui
    s'embarque.
  - Un même texte en Rust et en TypeScript fait aussi deux versions : le langage est rendu.
  - Le découpage (`default_scope_chunking`, `src/code.rs:220-233`, 30 lignes, 1 500 caractères,
    recouvrement de 5) ne dépend pas du langage.
- **N'y entrent pas** : la position (lignes, octets), le chemin, la clé stable, le dépôt, la
  branche, ni le modèle d'embarquement.
- **Aucune normalisation** au départ : les octets sont gardés. Les surlignages (offsets lucivy)
  restent justes, et la recherche refuse un rendu qui exigerait un remappage
  (`src/search.rs:1555-1560`).
- **Question pour la relecture** : faut-il rendre aussi `parent_name` ? Deux méthodes
  identiques de deux classes partageraient sinon une version. C'est voulu pour l'embarquement, mais
  le titre affiché viendrait de l'occurrence, pas de la version.

## 2. Ce qu'une occurrence référence

- **L'occurrence, c'est le `Scope` d'aujourd'hui**, inchangé. Il garde sa clé stable
  (`stable_scope_keys`, `src/code.rs:1801`), ses lignes, son fichier et ses relations (`CONSUMES`,
  `CALLS`, etc.).
- **Une seule référence**, de la version vers la racine : `ScopeContent_DERIVED_FROM`
  (`src/schema.rs:379-380`). Elle porte l'état de rendu propre à l'occurrence (`_render_hash` sur
  la relation, `derive_nodes.rs:326-330`). Aucune relation `OF_VERSION` en plus : la conception du
  8 novembre la proposait, la composition existante la rend inutile.
- **Une édition** repointe la référence de cette occurrence seule, dans la transaction de
  l'ingestion. Les autres occurrences gardent leur version.
- **Les relations restent sur les occurrences.** Une version ne porte aucune analyse : un scope
  inchangé peut changer de comportement, on réutilise son embarquement, jamais ses arêtes.
- **Les versions orphelines restent** : aucun ramasse-miettes dans ce lot. La politique attend la
  mesure M5.

## 3. Ce que l'embarquement partage

- **Les morceaux et les vecteurs vivent sur la version** (`ScopeContent_Chunk`). `Scope` n'a plus de
  morceaux à lui (`Scope_Chunk = 0`, `tests/e2e_scope_versions.rs:89`).
- **Par modèle** : une colonne et un index par modèle, avec la dimension dans le type
  (`src/embedding_storage.rs:148-150`). Deux modèles ne se mélangent pas.
- **Par cellule** : la cellule entre dans l'adresse. Deux projets ne partagent rien, et ce n'est pas
  un cache global de textes.
- **Les signaux** :
  - dense et BM25 sont prouvés par le test ;
  - **le creux ne l'est pas**. Le lot doit le prouver sur `ScopeContent` (un témoin creux, comme
    les deux autres) ou le refuser en le disant. Il ne doit jamais retomber en silence sur un
    index vide.
- **La recherche publique reste sur `Scope`**, par `searchIndex`. Une recherche directe sur
  `ScopeContent` est refusée (`src/read_view.rs:213-218`). Les outils (`search_code`, `usages`,
  `outline`) ne changent pas d'entrée ; ils rendent des occurrences avec leur chemin et leurs lignes.

## 4. Les bases existantes : le point à trancher avant le code

**Activer 1a par défaut rendrait les bases de Lucie inouvrables.** Elles ont un `Scope` peuplé sans
`searchIndex`. Le ré-enregistrement refuse de changer l'index public d'une entité peuplée
(`src/catalog.rs:2092-2103`), et le changement de mode d'identité aussi (`src/catalog.rs:2087-2089`).
Avec un binaire neuf et le schéma neuf par défaut, chaque base de code existante refuserait de
s'ouvrir tant que la migration 1b n'existe pas.

Je propose, à faire trancher par c7 et Lucie :

- **(A, proposé) Le schéma suit la base.** Une base vide, ou neuve, reçoit le schéma à versions. Une
  base existante (son `Scope` peuplé sans `searchIndex`) garde l'ancien schéma, ouvre comme
  aujourd'hui, et le dit (`daemon_status`, `inspect`) : « base de code sans versions de scope, à
  migrer (1b) ». La forme se lit sur la base elle-même ; aucun choix silencieux. Rien ne casse,
  et 1b migre ensuite chaque base sur une copie, à son mot.
- **(B) 1a et 1b ensemble.** Le schéma neuf par défaut n'atterrit qu'avec la migration. Le lot est
  plus gros, et l'économie n'arrive qu'avec lui.
- **(C) Une option du manifeste**, désactivée par défaut. C'est le plus sûr, mais un nouvel index
  n'en profite pas sans y penser.

## 5. La migration 1b, en bref (hors de ce lot)

Elle suit le plan du 12 novembre (`docs/arbre-principal/12-11-2026/01-plan-de-migration-de-l-ancre.md`)
et le ticket parent (`docs/arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md`).

- **Ce qu'elle ne fait pas** : pas d'`index confirm`, pas de réindexation complète, aucun appel à
  l'embarqueur.
- **Ce qu'elle fait, une base à la fois** :
  - elle ajoute `ScopeContent` et sa relation ;
  - elle calcule l'adresse de chaque `Scope` à partir de ses champs ;
  - elle re-rattache les morceaux et les vecteurs existants à la version, sans les recalculer ;
  - elle laisse les relations de code telles quelles ;
  - elle publie le schéma neuf, avec journal et reprise.
- **Le profil admis** (décision de c7 du 12) : dense et BM25. Le creux est refusé avant tout effet
  tant que son transfert n'est pas prouvé.
- **Toujours sur une copie d'abord**, puis `tools/check_rel_directions`, la couverture des
  occurrences et un jeu fixe de requêtes. Aucune base de Lucie sans son mot.

## 6. Les témoins du lot 1a, rouges d'abord

1. **Sur le schéma de production** (sans manifeste privé) : la même fonction dans `a.rs` et `b.rs`
   donne un seul appel à l'embarqueur, deux `Scope`, chacun avec son chemin et ses lignes. Rouge
   aujourd'hui. Les témoins de `dd5f30465` (branche `cx-arbre-scope-version`) visent exactement
   cela ; ils sont à reprendre, sans leur `#[ignore]` s'ils peuvent tourner dans la lib.
2. **Deux docstrings distinctes**, et deux noms distincts sur un même corps : deux versions.
3. **Le déplacement** : une fonction insérée au-dessus ne réembarque pas sa voisine. Le garde-fou
   `a_scope_that_only_moves_is_not_embedded_again` passe sur le nouveau schéma.
4. **Le creux** : prouvé sur `ScopeContent`, ou refusé en le disant (§ 3).
5. **Les outils** : `search_code`, `usages` et `outline` rendent les mêmes occurrences sur une
   petite base, avant et après. Les e2e `e2e_code`, `e2e_code_sync` et `e2e_scope_versions` passent.
6. **Selon la décision du § 4**, l'une ou l'autre :
   - (A) une base existante ouvre comme avant et se dit « à migrer » ;
   - (B ou C) son refus, ou son option, est témoigné.

Lib `207701bb2`, 32 Gio, un target neuf. Aucune base de Lucie n'est lue ni ouverte.

## Ce que cette page ne décide pas

- La vue elle-même : la table chemin → version et le masque d'une vue. Ce sont les lots 2 et
  suivants.
- Le ramasse-miettes des versions (M5).
- L'adoption du raccord physique par l'index réel, qui attend Lucie.
