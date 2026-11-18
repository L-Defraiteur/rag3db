# Évaluation du travail de Codex dans le périmètre d'everything declarative

18 novembre 2026, session ED. Demande de c7 sur le mot de Lucie : chaque session évalue la pertinence de
ce que Codex a fait dans son périmètre, avant tout code. Relevé fait sur `origin/master` e8a90bc88f.

## 1. Le verdict en cinq lignes

1. Les deux entrées que je lui avais laissées, 2.A (l'ordre des ex æquo) et 2.B (le jeton Vertex), sont sur
   master, et le code de chacune est bon : **utile, à garder**.
2. Les ex æquo sont **mon lot 1834bf111 à l'identique**, trois commits cueillis sans retouche. Codex a rejoué la
   passe complète sur la lib 59ac : rien n'y est inventé.
3. Le jeton Vertex suit le plan de ma page : la source de jetons est conservée, un 401 avant le flux invalide une
   fois puis rejoue une fois, et le témoin rouge a été vu rouge. Tout est prouvé sur un **faux serveur HTTP
   local** ; un vrai Vertex n'a jamais été joué. Le ticket reste ouvert à juste titre.
4. Pour arriver là : environ 2 000 lignes de pages, des scripts Python et des manifestes dans
   `docs/everything-declarative/16-11-2026/`. C'est beaucoup de papier pour quatre fichiers de code, et
   presque tout ce papier sert son propre harnais de qualification, pas le produit.
5. Rien n'est nuisible sur master. Restent deux commentaires périmés dans le code Vertex, à corriger par un
   petit lot, et une passation difficile à lire.

## 2. Les commits de master dans mon périmètre

Il y en a 26 entre 927941a67a et e8a90bc88f. Tous portent l'adresse gmail, et aucun n'a de ligne d'attribution
(contrôlé sur les 79 commits de la fenêtre).

### L'ordre des ex æquo (2.A)

| Commit | Ce que c'est | Verdict |
|---|---|---|
| 032d273942 | mon témoin rouge `ties_keep_the_same_order_from_a_rewritten_base_to_a_fresh_one`, octet pour octet celui d'ea4704bf6 | utile, à garder |
| e9febedb0e | mon correctif `stable_order` / `stable_identity` (search.rs, generic_search_nodes.rs), identique à bb38c2cde | utile, à garder |
| 3e771bc260 | ma remise en place de `stable_order` avant les attributs, identique à a4d1c59db | utile, à garder |
| 5b8ac75039, 9f51c97993 | page « candidat » réécrite sur la lib neuve | utile mais à requalifier : redit ma page 2.A |
| ab0e877375, f6b8b4d454, ac30def3e4 | `driver.py`, `launch.py`, `admit.py` et deux tests Python d'un lanceur privé, rangés **dans docs/** | inutile : outillage de Codex hors de `tools/`, que rien n'appelle |
| bcd7ec1ae5, 7ea272f8c1, 6c41445708 (en partie) | deux tickets sur ce lanceur (« le pilote isole le cache… », « le watcher confond un journal… »), fermés depuis dans `docs/tickets/closed/` | inutile : ce sont des défauts de son harnais. « Famille : produit » y est faux, ils disent eux-mêmes « atteignable en service : non » |
| a3d0d6291d, 056c23079a, 6c41445708, d395cb1f03 | rapports de passe, manifestes JSON, audit et range-diff | utile pour la preuve (voir ci-dessous), bien trop long pour ce qu'il dit |

**La preuve, contrôlée.** La passe canonique v3 (page 09) a été rejouée par `passe_epinglee.sh` sur la lib 59ac.
Résultats : les deux témoins ciblés 1/0/0, e2e_code 36, e2e_lectures 27, check_features 8/8, lib 2595/0/72,
navigateur 45. Ce sont les mêmes nombres que ma passe du 16 sur 373b5477b, et un rouge de son harnais n'a pas
été repeint en vert.

**Suites exigées.** search.rs et generic_search_nodes.rs ne figurent dans aucune ligne de
`tools/poste/required_suites.txt`, donc seules les suites par défaut sont dues : e2e_code et e2e_lectures. Elles
ont été jouées.

**Rouge d'abord.** Le témoin a été vu rouge sur ea4704bf6 (ma passe du 16), avant le correctif.

**La nouveauté.** Ma ligne de 1834bf111 dans `docs/nouveautes.md` n'est pas dans les trois commits cueillis,
mais elle est sur master par ailleurs (ligne 3).

### Le jeton Vertex (2.B)

| Commit | Ce que c'est | Verdict |
|---|---|---|
| 38146af549, 166a5dbc23, cefdd3fb2c, cb0ea38b52 | préparation : la page 03, le ticket du banc complété | utile mais à requalifier : quatre états successifs d'un plan, illisibles bout à bout |
| 936f13daa2 (+ 5ed1961d6b) | `openai_llm/vertex_refresh_tests.rs` : un faux serveur `tiny_http` qui rend 401 au jeton périmé, et cinq témoins | utile, à garder |
| efb9aba736 | raccord des témoins à `vertex_with_source`, et deux juges de plus : connecteur déclaré et `Debug` sans jeton | utile, à garder |
| 4b32256693 | **le correctif** : trait `AccessTokenSource` (gcp_auth.rs), champ `vertex_tokens` et `vertex_with_source` (openai_llm.rs), 401 → `invalidate()` puis un seul rejeu, `connect_llm_with_vertex` (model_source.rs) | utile, à garder |
| b472035a4e | audit de l'union, rapport CPU, manifeste de 297 lignes | utile pour la preuve, trop long |
| 99fef92ce6 | « candidat Markdown seul » : sept pages de rapport | voir la question (c) ci-dessous |

Le correctif tient ce que ma page 2.B demandait :

- `connect_llm` ne prend plus de jeton à la connexion. Il garde `Arc<dyn AccessTokenSource>`, et le juge
  `declared_vertex_connector_retains_injected_source_and_refreshes` vérifie qu'aucun jeton n'est acquis à la
  connexion ;
- `token()` est appelé à chaque tentative ;
- un 401 reçu **avant le flux** invalide le jeton une fois, et la requête repart une fois ;
- un second 401 est une erreur nommée (`vertex_persistent_unauthorized_is_named_after_one_refresh_only`) ;
- un 401 reçu après un fragment de flux n'est jamais rejoué (`vertex_never_refreshes_or_replays_after_a_stream_fragment`) ;
- une clé d'API fixe ne se renouvelle pas (`a_fixed_api_bearer_is_not_refreshed_after_unauthorized`).

`Auth` n'a pas été touché : Anthropic le partage, et la capacité reste un champ privé d'`OpenAiLlm`. C'est le bon
choix.

**Rouge d'abord.** Le témoin a été écrit contre l'ancien code, puis vu rouge : source fe40b640eb, pilote
eb026f0a3c, preuves 5d6e3f873f. « Trois positifs puis le 401 rouge » ; le correctif est venu après.

**Les passes.** Les sept juges ont d'abord été joués seuls (page 19). Puis la passe canonique a été jouée sur le
produit uni b472035a4e (page 23) : témoins 7/0/0, e2e_lectures 27, e2e_code 36, check_features 8/8,
lib 2612/0/72, navigateur 45, rc 0. Le « contrôleur rouge » de cette page est un défaut de **son** observateur
(`library_summary` appelé sur un journal à deux résumés), pas du produit.

**Suites exigées.** Aucun des fichiers touchés n'est dans `required_suites.txt` ; les suites par défaut ont été
jouées.

**Langue.** Identifiants, témoins et messages de test sont en anglais. Les commentaires de doc nouveaux sont en
français, ce qui est de la prose et reste permis.

**Ce qui n'est pas prouvé**, et que Codex dit lui-même :

- `TokenSource::from_env` n'a pas été exercé ;
- aucun échange OAuth réel n'a eu lieu ;
- aucune passe de plus d'une heure contre un vrai Vertex n'a tourné (la F300 du banc).

**Deux défauts restent sur master** (à corriger par un petit lot, sans effet sur l'analyse) :

1. `model_source.rs:776-778` dit encore « **Limite** : il est pris à la connexion et dure une heure ». C'est faux
   depuis 4b32256693. Codex l'a consigné au ticket sans le corriger, pour ne pas changer le produit qualifié.
2. L'en-tête de `vertex_refresh_tests.rs` dit encore « Prepared only: no execution or compilation verdict yet ».
   C'est faux : les témoins sont joués dans la lib.

### Hors de mes deux entrées

- 4cc6caeb3a, e982b13e60 et ecea15b074 (`docs(ops)`) concernent le ticket « la garde pile ne juge aucun en-tête
  d'un range-diff court ». Il a été ouvert dans `docs/everything-declarative/tickets/`, puis fermé dans
  `docs/tickets/closed/`. C'est la garde de G, pas mon code : **utile, à garder**, et le ticket est bien fermé.
- De 2.1 à 2.13, rien n'a été touché : aucun commit, aucune branche.

## 3. Les branches `cx-ed-*`

### Les branches du 16 et du 17 novembre

| Branche(s) | État | Verdict |
|---|---|---|
| cx-ed-stable-tie-identity-r1/r2, -qualifier-r1/r2/r3, -preuves-r1 | étapes du lot ties, en avance de 5 à 14 commits, dont trois fichiers Rust identiques à master | historique, à laisser : rien à reprendre |
| cx-ed-stable-tie-integration-r1 (d395cb1f03) | sur master | posée |
| cx-ed-stable-tie-landing-docs-r1 | +7 commits, docs seulement | historique, rien à reprendre |
| cx-ed-vertex-token-refresh-r1/r2/r3, -red-pilot-r1, -red-preuves-r1, -fix-r1, -green-pilot-r1, -green-preuves-r1 | étapes du lot Vertex, y compris le rouge fe40 et ses preuves | à laisser : ce sont les preuves du rouge d'abord |
| cx-ed-vertex-token-union-r1 (b472035a4e), cx-ed-vertex-integration-r2 (99fef92ce6) | sur master | posées |
| cx-ed-vertex-token-union-pilot-r1, -union-observation-r1, -raw-classifier-r1 | +2 à +15 commits, docs et scripts de son contrôleur | inutile pour le produit, à laisser |
| cx-ed-vertex-landing-docs-r1 | +37 commits, docs seulement : la passation du 17 et les relectures CLI99/Q9 faites pour Arbre | à lire par Arbre et par c7, pas par ED |

### Les branches du 11 au 13 novembre

Il y en a une cinquantaine : relais, accord, bge, frontières, parcours-agent-ide, read-scope, ouverture,
réclamations-rendues, carte-index-fixture, reste-explicite… Elles datent de la journée Codex précédente. Celles
dont l'avance est nulle sont posées. Les autres sont des essais successifs (`-2`, `-3`, `-4`) dont le dernier
numéro est posé.

Trois restent en avance avec du code :

- cx-ed-reste-explicite-3 : +6, dont 3 fichiers hors docs ;
- cx-ed-carte-index-fixture : +19, dont 5 fichiers hors docs ;
- cx-ed-ci-writer-diagnostic-r1 : +2, docs seulement.

Aucune n'est dans la passation comme travail à reprendre, et je ne les ai pas relues ici. Ma page du 16 les
tenait déjà pour closes. Si c7 veut un verdict sur ces trois, c'est une demi-heure de plus.

**Aucune branche n'est nuisible, et aucune n'est à effacer.** On n'efface pas une branche distante.

## 4. Ce que cela change à ma page du 16 novembre

- **2.A est fait.** Le lot est sur master, à l'identique, et la passe a été rejouée sur la lib 59ac. Le ticket de lucivy (`les-ex-aequo-de-la-recherche-n-ont-pas-d-ordre-stable`)
  est dans `docs/tickets/closed/`. Dans ma page, 2.A passe en « posé par 032d273942..3e771bc260 ».
- **2.B est fait pour le code**, pas pour la preuve en service. Il reste :
  - les deux commentaires périmés, en un petit lot de mon côté : rien que des commentaires ; pas de témoin rouge
    possible, et je le dis dans le message ;
  - la fermeture du ticket du banc, après une passe réelle de plus d'une heure sur un `models.llm` Vertex. Elle
    revient au banc (la F300) ou à la première passe longue de l'archiviste.

  Je ne ferme pas le ticket sur un faux serveur.
- **De 2.1 à 2.13, rien n'a changé.** Ma file reprend dans l'ordre de la page : le genre d'une méthode (2.6), le
  scope de namespace (2.12), le redécoupage forcé (2.10), puis le reste.

## 5. La passation de Codex : lisible ?

**Mal.** `cx-ed-vertex-landing-docs-r1:docs/everything-declarative/17-11-2026/99-passation-pour-claude.md` est
exacte dans ses hashes, mais presque illisible :

- des mots collés (« Canonique2150 rc0, F8huit jeux », « 72ignoredNONJUGÉS ») ;
- un vocabulaire privé jamais défini : GO, coupon, END, RO, APP, pilote, Q/W/pins, B/C, CLI99, Q9 ;
- des instants en secondes monotones ;
- trois listes de worktrees et de preuves, alors qu'un seul hash compte par lot.

Ce qu'il fallait savoir tient en trois lignes :

- « 2.A et 2.B sont sur master, passe canonique verte » ;
- « Vertex non prouvé contre le vrai service » ;
- « mes relectures en cours concernent Arbre, pas ED ».

Il a fallu les reconstruire depuis le git. Les pages 09, 19, 20 et 23 ont le même défaut, dans une moindre
mesure. Les faits y sont, et ils se sont révélés vrais à chaque contrôle.

Pour la suite, je n'adopte ni ses worktrees, ni ses targets, ni ses dossiers de preuves sous
`~/.cache/rag3weaver-build/cx-*`. Ils restent à lui ou à c7.
