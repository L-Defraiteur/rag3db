# Ce que la semaine du 11 au 14 novembre a appris — session « mémoire » (96)

Ce fichier complète `docs/memoire/11-11-2026/02-knowledge-dump.md` (le vocabulaire, les parties que
je tiens, les procédures), sans le répéter. L'état et la reprise sont dans
`01-rapport-de-session.md`, dans le même dossier.

## 1. Les trois blancs et leurs nombres

Les trois blancs ont joué le même corpus : les 52 documents de `extension/rag3weaver/docs/0[3-9]-novembre-2026-*/`,
découpés en 432 sections. Ils ont tous pris la même copie de départ (l'instantané de G du
13 novembre à 15 h 54, 19 fiches), avec `gemini-3.8-flash` par Vertex et `allow_create: true` sur la
copie seulement.

| | blanc-96 | blanc-96-2 | blanc-96-3 |
|---|---|---|---|
| l'archiviste de | `207701bb2` | `cd51ab5cd` (carry, name_as) | `25c6b63d5` (le nom de sujet) |
| lib | 207701bb2 | 835d5298f | e93710f36 |
| durée | 110,5 min | 75,8 min | 65,9 min |
| fiches créées | 326 | 113 | 115 |
| dont à une section | 274 | 45 | 45 |
| dont même document | 0 | 31 | 32 |
| dont plusieurs documents | 52 | 37 | 38 |
| noms de moins de 25 caractères | 95 | 33 | 29 |
| sections sous une fiche existante | 10 | 9 | 19 |
| none | 24 | 16 | 18 |
| paires de noms voisins | — | 7 | 0 |

Les pages qui les expliquent sont 02 et 03 de ce dossier. Les dossiers sont à luciepc, sous
`~/.cache/rag3weaver-build/blanc-96*`.

## 2. L'archiviste, tel qu'il est maintenant

`archive.mmd` enchaîne les étages suivants :

- `SplitSections` découpe le document.
- `ForEachNode(archive_decide.mmd, carry='proposed')` décide section par section. On y trouve la
  recherche des candidates par le texte, `archive_candidates` (les candidates et les fiches
  proposées plus haut), `Decide(name_as='subject')` et `archive_decided`.
- `Route(key='stage')` ne laisse passer que les éléments décidés.
- `ForEachNode(archive_subject.mmd)` cherche par le nom de sujet parmi les fiches en base.
- `Route(key='kind')` envoie vers les écritures `write`, `create` et `link`.

- **Le voisinage des noms** (`sujets_voisins`, la même fonction dans `archive_decided.rhai` et
  `archive_subject.rhai`). On compare en minuscules, sans ponctuation et sans mots vides. Deux noms
  sont voisins si leurs jeux de mots sont égaux, ou si l'un contient l'autre et que le plus petit a
  au moins deux mots. Il ne voit que les mots : deux noms voisins par le sens restent séparés.
- **Le nom daté** (`nom_date`) : une fiche neuve nommée par un mois, ou un jour de la semaine, avec un
  nombre est refusée. Le refus se dit dans le rapport, et la section reste dans le L0.
- **L'ordre des étages compte.** Une écriture pendant une recherche rend celle-ci indisponible :
  l'étage de recherche par le nom tourne avant toute écriture. Un `ForEachNode` lit tout son flux
  avant de commencer et n'émet qu'à la fin. Un `Route` émet au fil des éléments.
- **Les échecs d'un `ForEachNode` en `on_error='report'`** passent dans le flux sous la forme
  `{error, element, index}`, sans les champs de l'élément. Un étage suivant qui lit
  `$element.<champ>` refuserait ces éléments ; d'où le `Route(key='stage')`.
- **Un `RhaiNode` n'a qu'un port d'entrée** (`value`). Il ne sait pas réunir deux rapports.

## 3. L'outillage des tours (luciepc)

- **`TOUR_LIB=<lib>`**, ajouté le 13 novembre à `tour.sh`, `archfil-temoin.sh` et `bin-blanc.sh`,
  sur le modèle de `TOUR_TARGET`. Sans la variable, ces scripts lisent `lib-des-passes`, comme avant.
  Ce fichier est lu par une douzaine de scripts du poste, banc compris : on ne le change pas pour
  son propre lot, c'est G qui le bascule à une pose.
- **La garde de fraîcheur de la lib.** Un témoin refuse une lib plus vieille que le dernier commit
  moteur de master. On prend la lib du magasin nommée par ce commit
  (`ls -dt ~/.cache/rag3db-libs/rag3db-build-*-Release-p12*`). Le drapeau `moteur-ancien` est à c7,
  on ne le pose pas soi-même.
- **Les fichiers d'un lot** : `<lot>-tout.diff`, `<lot>-commit.txt`, `<lot>-suites.txt`,
  `<lot>-court.txt`. Le tour écrit lui-même `<lot>-base-jouee.txt`. Un diff qui déplace un fichier
  se tire par `git diff HEAD`, pour avoir l'index et l'arbre ensemble.
- **La base testée.** Au dernier pas, `pousser.sh` refuse sous le verrou dès que master diffère de la
  base gardée par le tour (`JOUE`). C'est plus strict que `push_garde.sh --code-apporte`, et cela
  évite le piège relevé par archi : les refs sont communes aux worktrees, et un fetch d'une autre
  session fait bouger `origin/master`.
- **`instables.txt`** : un test qui rougit sous la charge et passe seul n'y entre qu'avec sa raison
  et son ticket, sur le mot de c7.
- **Le shell de luciepc est fish.** Une commande `ssh luciepc '…'` qui pose `D=…` ou lance une
  boucle `for` échoue : il faut passer par `ssh luciepc bash -s <<'EOF'`.

## 4. Mesurer un blanc

1. Un appel `select_apports` filtré sur `via = archiviste` (`limit: 0`), par le binaire du blanc
   avec son manifeste. L'environnement est celui de `lancer.py` : `GOOGLE_APPLICATION_CREDENTIALS`
   en chemin vers `.vault`, jamais copié, `RAG3DB_LIBRARY_DIR` et `LD_LIBRARY_PATH` sur la lib.
2. Les fiches d'avant (`copie-origine/memoire-longue/fiches`) et d'après (`copie/…/fiches`), lues
   par leur en-tête `name:`.
3. Les comptes : fiches neuves, à une section, même document, plusieurs documents, noms courts,
   sections sous une fiche existante, et les paires voisines par la règle de `sujets_voisins`.
   Le script est dans le transcript de la session. Il fait une vingtaine de lignes de Python et se
   réécrit depuis cette liste.

## 5. Les pièges appris

- **Les remises git sont communes à tous les worktrees du dépôt.** Le 13 novembre, un `git stash
  pop` de G dans son worktree a sorti la remise de 96. Une remise se cherche par son message, jamais
  par son index, se reprend par `apply` puis `drop` de celle-là seule, après avoir vérifié qu'elle
  ne touche rien d'autre.
- **Un worktree où le tour a posé le lot** garde ses fichiers après le push. On compare chaque
  fichier à `origin/master` (`cmp`) avant `reset --hard`, puis on rebascule.
- **Un test instable sous la charge** : `native_process_death_recovers_same_program_and_lost_ack_once`
  (le raccord physique) a rougi aux deux rejeux d'archjour, sous la lib entière, et passé seul
  14 fois sur 14. Il récidive à chaque passe complète, ce n'est donc pas un hasard. Le ticket est
  pour arbre principal, et la fixture est gardée hors de `/tmp`, qui est de la mémoire vive.
- **Un verrou n'est pas un rouge.** Un garde-fou qui tient déjà se fixe par un témoin vert dès sa
  pose, et se dit ainsi, sans le présenter comme un correctif.
- **Le relais ne vaut pas le mot de Lucie sur ses données.** Une décision relayée par c7 suffit pour
  coder. Toucher sa base demande son mot direct : l'endroit, le moment, l'instantané.
