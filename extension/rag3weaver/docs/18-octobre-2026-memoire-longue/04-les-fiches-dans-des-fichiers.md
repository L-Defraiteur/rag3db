# La mémoire en fichiers : la fiche est le sujet, ses apports, son état

*Étude du 18 octobre 2026 (mémoire longue), réécrite le même jour sur le modèle que Lucie
a retenu (relayé par l'orchestration, en termes génériques ; rien d'un autre dépôt n'entre
ici). Elle remplace la première version (une fiche atomique par fait, rangée sous des
sujets, un classeur L1/L2/L3). Son inquiétude reste le fil : « il ne faut pas qu'on prenne
l'habitude, et qu'au dernier moment on dise : l'ancien index est corrompu, on doit
reconstruire ». Rien n'est codé ; `seed` et `cited_by` (le Plan) passent avant.*

## 1. Le modèle : la fiche est le sujet

- **Une fiche = un sujet**, avec ses **ancres** (les Refs `path`, `symbol`, et les autres
  genres) : « le moteur de reprise du WAL », « les préférences de communication de
  Lucie ». Elle a un nom, une description courte, un état (`current`, `to_review`,
  `retired`).
- **Un L1 par apport qui la touche** : ce qu'un document a dit de ce sujet, à une date,
  par un auteur. Le L1 **cite sa source** : le Markdown d'origine et sa section, par un
  genre `doc:` à ancre de titre (`doc:<chemin du source>#<titre>`). Un L1 ne change plus
  une fois posé : c'est un fait d'archive (« ce jour-là, ce document a dit ceci »).
- **Un L2 par fiche : son état global**, ce qu'on sait du sujet aujourd'hui. **Toujours
  recalculé depuis ses L1, jamais depuis l'ancien L2** : un L2 ne dérive pas de
  lui-même, une erreur ne s'y accumule pas. Il a une partie structurée, **les faits**,
  une ligne par fait, chacune marquée :
  - **dit** : des mots cités **tels quels** d'un L1 (donc de sa source) ;
  - **déduit** : ce que le modèle en a conclu, avec les L1 d'où il le tire.

  **Une déduction n'écrase jamais une parole** : un fait « dit » contredit par un
  « déduit » reste, et le désaccord se dit.
- **Un L1 bis consolide les vieux L1** pour borner le recalcul : au-delà d'un budget, les
  L1 les plus anciens d'une fiche sont résumés en un L1 bis, qui les cite ; le recalcul du
  L2 lit le L1 bis à leur place. Les L1 consolidés restent (la provenance), ils ne sont
  plus relus.
- **Ce qui disparaît** : les fiches atomiques d'aujourd'hui deviennent les **lignes de
  faits** des L2 ; le L2 du jour et le L3 du classeur n'existent plus. La vue d'un sujet
  (la précision de Lucie de ce matin), c'est désormais **la fiche elle-même** : son L2 en
  tête, ses L1 dessous.

**Une garde qui se vérifie** : une ligne « dit » doit se retrouver **mot pour mot** dans le
texte d'un L1 cité, et ce L1 dans sa source. C'est déterministe ; le recalcul du L2 refuse
une ligne « dit » qui ne s'y retrouve pas, au lieu de la croire.

## 2. Le flux : Claude écrit, l'archiviste range

1. **Claude écrit un Markdown** (un rapport, une note de fin de tâche) quand il le juge
   cohérent, et l'**envoie à l'archiviste** — un agent de rag3weaver, par un outil MCP
   (`archive(document)`).
2. **Le source est gardé tel quel** en fichier : c'est le **L0**, et la provenance de tout
   ce qui suit. Son **auteur est posé par l'hôte** (§6), jamais par l'outil.
3. **L'archiviste découpe** le document par sujets (ses sections, puis le modèle quand une
   section en touche plusieurs).
4. **Pour chaque morceau, une classification fermée** choisit la fiche : l'une des fiches
   candidates (rappelées par la recherche sur les fiches), **aucune**, ou **proposer d'en
   créer une**. C'est exactement la forme de la tentative 4 des fiches classées
   (`labels_from` sur les candidates, plus des étiquettes fixes) : 36 fiches justes sur
   39, aucun faux rangement (une décision par morceau).
5. **Il écrit le L1** sur la fiche choisie, avec sa Ref `doc:` vers le source et la
   section, puis **recalcule le L2** de cette fiche depuis ses L1.

Une proposition de création n'est pas posée seule : elle reste « proposée » tant qu'elle
n'est pas confirmée (par l'agent qui a envoyé le document, ou par Lucie), comme une
transition du cycle de vie.

## 3. Le filet : les crochets de Claude Code

Si Claude n'a rien envoyé, **les crochets de Claude Code** (`PreCompact`, la fin de
session) envoient ce qu'il y a : la note de fin si elle existe, sinon un résumé de la
partie de conversation qui va être compactée, marqué comme tel (`origin: filet`). Le
filet n'invente rien : il ne fait que garantir qu'aucune session ne se perd sans trace.

## 4. Les fichiers

```text
<dossier de la mémoire>/
  fiches/<uuid>.md         une fiche = un fichier : en-tête YAML, puis son L2, puis ses L1
  sources/<date>-<uuid>.md les Markdown reçus, tels quels (les L0)
```

- **L'en-tête est en YAML, entre `---`**, comme dans Obsidian (choix de Lucie) ; listes en
  tirets ; **le harnais écrit toujours la même forme**, si bien qu'un fichier écrit par lui
  se relit et se réécrit à l'identique, octet pour octet. Un fichier écrit à la main qui ne
  se réécrirait pas à l'identique est **validé d'abord**, puis remis en forme canonique
  **seulement s'il est valide**.
- **L'en-tête de la fiche** : `uuid`, `name`, `about`, `state`, `refs` (les ancres, en forme
  normale), `author` (de la création), `created_at`, `updated_at`, `revision`.
- **Le corps** : une section `## État` (le L2 : un résumé, puis `### Faits`, une ligne par
  fait avec `dit` ou `déduit` et ses L1), marquée générée ; puis `## Apports`, les L1 dans
  l'ordre chronologique, chacun avec sa date, son auteur, sa Ref `doc:` (un lien relatif
  vers `../sources/…#titre`) et son texte ; les L1 bis à leur place, avec les L1 qu'ils
  consolident.
- **Les fiches à plat, par uuid** : le nom de fichier ne bouge jamais.
- Les L1 et les L2 sont produits par un modèle, coûteux et non déterministes : un index
  reconstruit les **relit**, il ne les **régénère** jamais.

## 5. La base est l'index, et la reconstruction tourne tout le temps

- **Le fichier d'abord, la base ensuite**, par une déclaration générique sur l'entité
  (`"files": {"dir", "name", "template", "header", "body"}`), pas un cas pour la mémoire :
  rendre (jinja), écrire atomiquement, puis ingérer la ligne. Une ingestion qui échoue
  laisse le fichier, l'index est en retard, la réconciliation le rattrape.
- **`rebuild`** : une base neuve, les dossiers relus (fiches, leurs L1 et L2, les sources),
  chaque ligne posée par le chemin de `seed` (`EntityRecordNode`) en mode **restauration**
  (dates et révisions lues du fichier, la seule exception au refus des champs du serveur,
  et pas un outil).
- **La preuve de parité, champ à champ** : écrire par les outils (des sources, des L1, des
  L2, des ancres, un L1 bis), puis comparer la base vivante et la base reconstruite (les
  lignes, leurs champs déclarés, les relations avec leur multiplicité, le patron
  d'`e2e_code_sync`) ; les vecteurs se recalculent et se comparent à l'écart borné du
  service.
- **Contre l'inquiétude de Lucie** :
  - le témoin de parité est dans la batterie ;
  - l'hôte compare au démarrage dossiers et base (empreinte de chaque fichier contre
    `_content_hash` de sa ligne), et `index_state` dit « dossier ≠ base : N écarts » ;
  - `memoire verifier` reconstruit à blanc dans une base temporaire et rend les écarts.

## 6. L'édition à la main, l'auteur, le partage

- **La montre sur les dossiers** (la montre d'aujourd'hui est liée au schéma de code ; il
  faut la même pour des entités déclarées). Ce qu'un humain édite :
  - **les ancres et l'en-tête** d'une fiche : relus et indexés ;
  - **un apport** : il **ajoute un L1** (`dit` par lui, auteur posé par l'hôte), il ne
    réécrit pas un L1 existant — un L1 est une archive ;
  - **la section `## État`** est générée : une édition à la main y serait écrasée au
    prochain recalcul ; la montre la refuse en le disant, et propose d'en faire un L1.
- **Un fichier invalide** (en-tête illisible, champ hors schéma, marqueurs de conflit git)
  : la ligne indexée reste celle d'avant, le fichier n'est jamais réécrit par l'hôte,
  `index_state` dit le fichier et la raison.
- **L'agent et l'humain sur la même fiche** : l'`expected_hash` du proto, côté fichier —
  l'écriture d'un L1 ou d'un L2 reprend l'empreinte du fichier lu, et est refusée si le
  fichier a changé depuis ; l'agent relit, puis réécrit.
- **L'auteur** est de la provenance (qui l'apport vient de : la personne pour qui l'agent
  travaillait), sur chaque L1 et sur la création d'une fiche. **Posé par l'hôte**, comme
  `created_at`, et refusé s'il est donné par l'outil : `author` = l'identité git lue
  **dans le dépôt de la mémoire** (pas le global) ; `via` = le client de la session MCP
  (`--keys`). Le commit git les recoupe. Cela empêche l'agent de mentir ; une signature de
  commit viendrait ensuite, si une équipe le demande. `cited_by` gagne `authors?`.
- **Le partage par git** : un `pull` change des fichiers, la montre réindexe. Un conflit
  n'arrive que si la même fiche a changé des deux côtés ; comme les L1 s'ajoutent et ne se
  réécrivent pas, la plupart se résolvent en gardant les deux ; la section `## État` en
  conflit se recalcule.

## 7. `cited_by`, avec ce modèle

`cited_by` (la forme du Plan fixée avec l'arbre principal) vise désormais **les fiches et
leurs L1** : une Ref `path` ou `symbol` est une ancre de fiche (« ce sujet parle de ce
fichier ») ; une Ref `doc:` est la source d'un L1. Les fiches qui citent un chemin, triées
par la date de leur dernier L1, et pour chacune les L1 qui l'ont cité — le même Plan, avec
une relation de plus (`Fiche -[:A_POUR_APPORT]-> L1`).

## 8. Les lots, en ordre de grandeur relatif

| lot | contenu | taille relative |
|---|---|---|
| M1 | le schéma du gabarit `memory` : Fiche (le sujet), L1, L1 bis, L2 avec ses faits `dit`/`déduit`, Source (L0), et leurs relations ; la migration des fiches d'aujourd'hui (la mémoire de Lucie en a une) | moyen |
| M2 | les fichiers : la déclaration `files` générique, le rendu canonique (YAML, `## État`, `## Apports`), l'écriture avant la ligne | moyen |
| M3 | `rebuild` en restauration, et **le témoin de parité champ à champ** | moyen |
| M4 | l'archiviste : `archive(document)`, le L0 gardé, le découpage, la classification fermée (la tentative 4, plus « créer »), l'écriture du L1 avec sa Ref `doc:` | moyen à gros |
| M5 | le recalcul du L2 depuis les L1 : sortie structurée, la garde « dit = mot pour mot » | moyen |
| M6 | le L1 bis : la consolidation au-delà d'un budget | petit à moyen |
| M7 | l'auteur posé par l'hôte (identité git du dépôt, client de la session) et `authors?` dans `cited_by` | petit à moyen |
| M8 | la montre sur les dossiers d'entités déclarées, les éditions à la main (un L1 ajouté, `## État` protégé), les fichiers invalides dits, `expected_hash` | moyen à gros |
| M9 | les crochets de Claude Code (`PreCompact`, fin de session) comme filet | petit |
| M10 | le partage par git : deux clones, un conflit, une fiche créée des deux côtés (témoins) | moyen |

**L'ordre** : M1 puis M2 et M3 d'abord (sans la preuve de parité, le reste repose sur une
promesse) ; M4 et M5 ensuite, c'est le flux qui sert Lucie ; M7 avec M4 (un L1 sans auteur
ne doit pas exister) ; M6 quand une fiche dépasse le budget ; M8, M9, M10. Ces lots
remplacent F1 à F7 et F2 bis de la première version.
