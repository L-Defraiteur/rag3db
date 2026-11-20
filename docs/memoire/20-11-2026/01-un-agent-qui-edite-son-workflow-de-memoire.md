# Un agent qui édite son propre workflow de mémoire

Session mémoire (96), 20 novembre 2026. Page de conception, sans code. Demandée par c7, d'après la graine de Lucie du
19 novembre : « tout doit être déclaratif en général ; quelqu'un doit pouvoir se dire : c'est juste un DAG comme un
autre, j'aurais pu en faire un moi-même ; et pareil si on veut des boucles auto-amélioratives plus tard, un agent qui
édite son propre workflow de mémoire ».

La page part de l'archiviste tel qu'il sera après les lots B1 à B6 (page du 19 novembre,
[le lot B](../19-11-2026/01-le-lot-b-tout-declaratif.md)), l'extraction coupée et B4.

## 1. Ce qui est éditable, une fois B1 à B6 posés

Tout le travail de l'archiviste est dans le gabarit `memory` : des fichiers texte, sans Rust propre à la mémoire.

| quoi | où | forme |
|---|---|---|
| le chemin d'un rangement | `graphs/*.mmd` (archive, archive_slice, archive_slice_core, archive_decide…) | un graphe mermaid : des nœuds génériques et leurs arêtes |
| les réglages | les `%% param:` de chaque graphe, avec leur défaut | une valeur par paramètre |
| les consignes | `instruction`, `extract_instruction`, `candidates_template` | une ligne ou un gabarit, en paramètre |
| le sens d'une réponse | les descriptions des champs de `schema` (extraction, découpage) | du JSON Schema |
| la présentation d'un type | `x-presentation` dans `schemas/*.json` | un gabarit ou une forme nommée |
| l'ordre du contexte | les entrées d'un `Collect` | une liste de noms |
| les motifs de secrets | `patterns` de `put_turn` | une liste de motifs |
| la logique entre deux nœuds | `scripts/*.rhai` | des fonctions pures, sans état |

Ce qui n'est pas éditable, et ne doit pas le devenir par cette voie : les entités et relations inscrites en base (un
changement de schéma est une migration), les nœuds du moteur, et le client du modèle.

## 2. Ce qui existe déjà pour changer un gabarit en service

- **Le rechargement à chaud** : `Backend::check_reload` relit le manifeste et tout ce qu'il nomme (graphes, scripts,
  schémas, crochets, réactions), et les revérifie comme au chargement, sans rien toucher. `apply_reload` met ensuite la
  nouvelle version en service d'un seul geste. Une version a un numéro (`Reloaded.version`). Un appel en cours finit
  sur la version qu'il a prise à son départ. Ce qui a été fixé à l'ouverture (la base, les modèles, les entités et les
  relations) est refusé au rechargement, en le disant.
- **La vérification au chargement** couvre déjà une bonne part des fautes d'un éditeur : une politique d'outil
  (`call_models`, `response_schema` exigé d'un `LlmNode`), les types de nœuds admis, les ports, les paramètres
  inconnus, une présentation mal formée, un gabarit qui ne compile pas.
- **L'écriture de fichiers par un agent** existe dans l'atelier, hors de la mémoire.

## 3. La porte : une tâche de l'hôte, jamais un outil de la mémoire

Un agent ne doit pas pouvoir changer son workflow par un appel d'outil ordinaire, sinon une décision mal rangée
pourrait réécrire le rangement lui-même. La porte proposée est **une tâche de l'hôte**, du même genre que celles de
l'équipier : « proposer une version du gabarit memory ». Elle suit quatre temps :

1. **Proposer.** L'agent écrit sa version dans une copie du gabarit, jamais dans celui qui est en service. La
   proposition dit ce qu'elle change, et le diff est son contenu.
2. **Vérifier.** L'hôte appelle `check_reload` sur la copie. Un refus s'arrête ici, en le disant.
3. **Juger.** Le banc rejoue la parité sur les mêmes tranches, l'ancienne version contre la nouvelle : sujets, P1, P2,
   jetons d'invite et part servie par le cache. C'est ce qu'il a fait pour B3 et B4. La règle à décider est le seuil :
   pas de baisse de P1 au-delà du bruit mesuré (± 0,07 d'un jeu à l'autre le 20 novembre), et pas de coût plus haut
   sans gain.
4. **Mettre en service.** Après un jugement vert, et le oui de Lucie tant que la boucle n'a pas fait ses preuves,
   l'hôte copie la version dans le gabarit en service, puis appelle `apply_reload`. La version précédente est gardée.

`define_ref` n'est pas cette porte. Il change une définition, une donnée de la mémoire, pas le workflow.

## 4. Ce qui garde l'édition

- **La version** : chaque version mise en service est un commit du dépôt du gabarit, avec son numéro de rechargement
  et la page de parité qui l'a jugée. Le dépôt git du gabarit est la mémoire de ses versions.
- **Le retour arrière** : remettre la version d'avant, c'est le même chemin en sens inverse (`check_reload`, puis
  `apply_reload` sur le commit précédent), sans jugement, puisqu'elle a déjà été jugée.
- **Ce qui ne bouge pas en service** : une version mise en service ne s'édite plus. Une nouvelle proposition en part.
- **La trace** : chaque rangement dit la version du gabarit qui l'a fait, pour qu'une baisse se relie à sa version.

## 5. Ce qui manque encore

1. **La version du gabarit dans la trace d'un rangement.** Le rapport d'`archive` et d'`archive_slice` ne dit pas
   quelle version du gabarit l'a produit. Il faut exposer `Reloaded.version` (ou le commit du gabarit) aux graphes, et
   le poser sur les Sources et les L1.
2. **Le rechargement par l'hôte, en deux temps séparés.** Aujourd'hui, `reload` enchaîne la vérification et la mise
   en service. La porte demande une vérification sur une copie, qui ne touche pas le gabarit en service.
3. **La parité du banc, appelable sans la main du banc.** Le banc la joue par ses scripts. La boucle demande une
   tâche qui la lance sur deux versions et rend un verdict lisible : vert, ou rouge avec ses chiffres.
4. **Le reste du lot B** :
   - `compose_restart` en ingrédients (B5) ;
   - la présentation des types de la mémoire (Fiche, Source, Turn), encore jamais déclarée dans le gabarit.
5. **Les scripts Rhai dupliqués** (`sujet_mots`, présent dans deux scripts), parce que Rhai n'importe rien. Un agent
   qui en corrige un laisserait l'autre faux. Il faut soit un module de scripts partagé que le moteur charge, soit
   une seule fonction dans un nœud.
6. **Une limite à ce qu'une proposition peut toucher.** Une liste déclarée des fichiers que la tâche peut écrire :
   les graphes, scripts et schémas de présentation, mais pas `backend.json` (entités, relations, modèles).

## 6. Ce qui reste à trancher

- le seuil de la parité : P1, P2, coût, et leur bruit ;
- si le oui de Lucie reste exigé, ou s'il ne l'est que pour certaines parties (les consignes oui, les graphes non) ;
- l'ordre des lots : la version dans la trace (5.1) d'abord, puisqu'elle sert aussi le banc aujourd'hui, puis le
  rechargement en deux temps (5.2), puis la tâche de proposition.
