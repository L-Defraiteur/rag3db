# Rapport de session — mémoire longue, 11 octobre 2026 (matin, midi, après-midi, soir et nuit)

*Pour Lucie et pour l'orchestration. Ce qui est sur master, ce qui reste, ce qui attend.*

## Sur master

**La page C, avec l'arbre principal** (`docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`).
Mes parties : le client mince (§3), quand le démon n'est pas là (§4), ma part des
marches (§7), puis l'alignement sur tes treize décisions. Deux faits de la carte
portent ma partie : le protocole client ↔ backend existe déjà (le chat le parle, il
est donc déjà un client mince), et `mcp.rs` a déjà sa couture (`trait Hote`) — le
client de C est un autre `impl Hote`. Et un défaut que C rend atteignable, nommé et
réglé par ta décision 12 : la bascule de cellule du catalogue entier, devenue une
course dès que plusieurs clients partagent un backend.

**Le lien générique, quatre marches sur cinq** (`01-un-lien-entre-deux-identites-declarees.md`) :

| | commit | ce que c'est |
|---|---|---|
| témoin | `49142d58f` | la garde de `RelationBatchNode` ne protégeait aucune garantie déclarée — établi champ à champ |
| 1 | `6d212551e`, `cde65dc9d` | `RelationBatchNode` devient le nœud de lien général ; existence vérifiée au catalogue (base ou file) ; propriétés déclarées et typées ; `add_ref` ne prétend plus avoir lié une fiche absente |
| 2 | `13c6d950f` | `from_states` / `to_states`, une politique d'écriture du backend, appliquée par les deux nœuds qui lient |
| 3 | `a6c2a61dd` | `link.mmd` ; `anchor` et `supersede_link` dans le gabarit `memory` ; la réaction ancre par l'outil |
| 4 | `ad51a6630` | le huitième refus, lu sur la marque « relation lue / écrite » des paramètres |

**Ta mémoire branchée, rafraîchie** (`9734e75e0`) : neuf outils, et la chaîne entière
éprouvée par MCP sur une copie de ta base — ancrer, changer le sujet, la mémoire passe
« à revoir » d'elle-même. Ta base n'est pas touchée, l'ancien est gardé.

## L'après-midi, sur master

| commit | ce que c'est |
|---|---|
| `59718e806` | **SIGTERM vaut la fermeture du tube** dans `servir_mcp` : l'appel en cours finit, les indexations s'arrêtent au prochain paquet, la base se ferme ; témoin `e2e_mcp_sigterm` (sortie 0, la base se rouvre) |
| `2d0687a71` | **le rapport des liens dit la vérité sur `atomic`** : une instruction, atomique par le moteur ; plusieurs instructions de liens seuls, enveloppées d'une transaction ; une fermeture qui porte des créations, `atomic: false` et sa raison. Rouge réel par faute injectée (deux variables, jamais active sans les deux) |
| `d25cad18b` | **le client mince de la forme C** : `mcp --daemon <adresse>` parle à l'hôte du démon par un `impl Hote` (`DaemonClient`), sans rien posséder ; une session perdue se rouvre une fois ; `--demon` est refusé en nommant `--daemon`. `e2e_mcp_stdio` rejoué entier par l'hôte, et deux sessions MCP partagent un backend |
| `fa04ccd3c` | **marche 1 bis du lien** : `Catalog::link` refuse un bout absent en le nommant, quand le lien se pose maintenant ; les liens différés se comptent au drain comme avant. Le ticket du bout absent est fermé |
| `b1b673d2b` | la page du lien, §3 bis : **la cellule des bouts** — avec l'identité par cellule (variante a), un lien résout ses bouts dans la cellule de la session ; un lien entre cellules se déclare. À faire avec la migration de l'arbre principal, pas avant |
| `442791fde` … `d03fbfe31` | **la page de l'enveloppe d'un outil d'écriture**, avec l'arbre principal (`03-l-enveloppe-d-un-outil-d-ecriture.md`) : le recensement, ce que le paquet garantit, les formes A à D et leur coût |
| `cd919b053` | **forme A de l'enveloppe** : chaque outil d'écriture dit `atomic` et sa raison ; l'échec d'un put dit si la ligne existe (dans la cellule de l'appel) et ce qui est prêt d'elle ; `add_ref` et `create_ref_type` lisent enfin leur rapport (ticket fermé). Témoin `e2e_outils_disent_ce_qui_reste` (trois tests, faute injectée à l'insertion) |
| `e161365d5` | **le banc `pg_trgm` tranché** (voir plus bas) |

**`e2e_postgres`, première exécution réelle** (conteneur de Lucie, port 5433) : 19 passés
sur 20. Le rouge, `ou_vit_la_frontiere_entre_le_vrai_et_le_bruit`, est identique au
commit d'avant l'échappement : il n'est pas dû au travail de F. Mesuré dans le
conteneur, `word_similarity('sauterau baroqe', …)` vaut 0,5789, au-dessus du plancher
0,3 : l'attente du banc est juste. La cause est le pool `deadpool` : le plancher est
posé par un `SET` joué une seule fois, donc sur une seule session. Confié à F
(`SET LOCAL` dans la transaction de la requête, sur la connexion épinglée).

## La fin d'après-midi, sur master

| commit | ce que c'est |
|---|---|
| `aeddf002a`, `f7c83a4f4` | la recette MCP de la marche 3, jouée de zéro ; la page des fiches classées par sortie structurée (cinq réponses prises par Lucie : on tente, on mesure, on ajuste, chaque tentative datée) |
| `2b1818d68` | **marche 4 de la forme C** : `daemon.json` (adresse servie, pid, jeton, bâti, 0600), le jeton local sur chaque requête sauf `/sante` (403), le client mince par défaut de `mcp`, l'hôte lancé par le premier client (verrou de fichier, détaché, survit au client), relancé quand il tombe, `daemon_status` même sans hôte, un second hôte refusé, `--standalone` pour l'ancienne forme ; six témoins ; côté hôte relu par l'arbre principal (`DaemonServer`/`Stopper`, jeton à temps constant) |
| `bfec631cc` | **review_anchored en une passe** : 40 fiches, 1 drain et 94 instructions au lieu de 40 drains et 1 318 ; une transition d'état ne change plus le hash du contenu (plus de redécoupage ni de réembarquement) ; le rapport dit `atomic` |
| `e9387d545` | **la session d'un client mort se ferme** : le pid et l'instant de démarrage du client, relevés toutes les 10 s et à chaque état |
| `e60f2342e` | **marche 5, première part** : le client vers un hôte désigné — `--daemon-json`, `--daemon-json-command` (le jeton relu sur un refus), `--daemon` pour le tunnel ; éprouvé contre l'hôte de luciepc ; README §7, page C |
| tickets | ouverts : extension vector relative au gabarit, écritures du cycle de vie absentes d'une réaction, hôte qui hérite de l'environnement du premier client, montre à 0,7 cœur, journal de l'hôte sans heure ; fermés : add_ref/create_ref_type, client mort, pg_trgm (par F) |

**L'hôte de Lucie** a été redémarré trois fois, toujours à une heure annoncée, par
`kill -TERM` et au repos (16 h 55, 17 h 10, 17 h 50). Il tourne sur `e60f2342e` (binaire
`2af4554702ac`), lié à la lib d'A8 et A9 (inodes vérifiés dans `/proc/<pid>/maps`), avec
les trois services d'embarquement. Sa copie du gabarit `code` a reçu la montre et sa
réaction (sauvegardes `*.avant-montre-11-oct-17h02`) : une édition à la main est vue par
`usages` en 5 à 6 s, un fichier créé en 4,7 s, et retirés en 4 à 6 s.

## Le soir, sur master

| commit | ce que c'est |
|---|---|
| `30ce3aadc`, `519ae1d9f` | **le banc des fiches classées** et ses deux tentatives datées (`templates/experiments/…-fiches-classees-1` et `-2`) ; puis la suite gardée par la feature `daemon` — **master était rouge par moi** sous deux jeux, mon contrôle comptait des verts au lieu de chercher un rouge |
| `25c8a5d5e` | **le bâti dit son moteur** : `build_id()` porte `CALL build_id()` (relu par l'arbre principal) |
| `e9d690915` | **la cellule des bouts d'un lien** : `from_cell` / `to_cell`, refus nommé avec la cellule, `add_ref` dans la cellule de l'appel (rouge prouvé sur l'ancien code) |
| `edfe2aa9d` | **les bancs à services écartés en le disant** dans `run_e2e.sh` (le seul rouge de la batterie de la recherche) |
| `1266adef0` | **le client mince revient seul** : démarrage jamais bloqué plus de 4 s, reprise toutes les 10 s, `notifications/tools/list_changed` ; une coupure en plein appel se dit et ne se rejoue pas (retours du cœur C++) |

**La tentative 2 des fiches** : Qwen3-1.7B local, 25 / 39 fiches justes, 8 / 8 abstentions,
3 faux ancrages sûrs d'eux (0,96 à 1,0 sur l'étiquette choisie : un seuil n'y peut rien),
62 ms la décision ; Gemini tronqué par le plafond de 512 jetons de `Decide` (le proto pose
`max_tokens`, la tentative 3 l'attend).

**La tentative 3 des fiches** (12 oct., 4 h 47, `templates/experiments/2026-10-12-fiches-classees-3/`) :
Gemini 3.5 Flash sur `Decide` à `max_tokens` 2 048 : 32 / 39 fiches justes, 8 / 8
abstentions, **1 faux ancrage**, 4 manqués (des fiches de règle refusées sous leur domaine),
5,35 s la décision ; aucune réponse tronquée. Gemini se trompe vers l'abstention, le Decider
vers le rangement. La tentative 4 : une décision par fiche (« lequel, ou aucun »).

**Le binaire de Lucie, réparé à 21 h** : la lib de l'arbre principal a changé d'ABI avec B2 ;
le binaire installé ne démarrait plus (`undefined symbol …SystemConfig…`), donc aucune
nouvelle session de Lucie n'aurait eu ses serveurs MCP. Rebâti contre B2, prouvé à l'écart
(`--describe`, un hôte d'essai sur 7990, un client de boutique), installé par renommage
sans toucher à son hôte, qui tourne sur l'ancienne image. Son redémarrage attend les
défenses de l'arbre principal contre la boucle de `run_index` (le gel de luciepc), au plus
tard à 0 h 15.

## Ce qui reste

- **Pour Lucie : la séquence B4, au matin.** À 2 h 56 (12 oct.), l'arrêt de l'hôte a été
  refusé par les permissions de ma session ; rien n'a été exécuté, l'hôte tourne et répond
  (pid 2061518, `3e95f0302023`). Depuis, le cœur a corrigé le balayage filtré des chaînes
  (`e6f626df4`, cause du P1 et de la boucle de luciepc) puis le balayage sous petit masque
  (`41d282843`, `usages` sous 100 ms attendu), puis l'insertion de vecteurs (`3cca0ae1e` :
  le point de reprise après les SET de vecteurs n'est plus quadratique, ce qui touche la
  réindexation du matin), enfin la jointure par clé (`2b11b55ec` : `usages` 352 → 43 ms sur
  l'index entier, mesuré par F) ; l'arbre principal a publié la lib au magasin à 10 h 15
  (`librag3db.so` 105a2f09…, libvector 7b8a30da…). Le binaire à installer est bâti sur
  `1309ab5a4` (avec les lots de la nuit : le saut en deux temps de F, la
  montre qui laisse passer une lecture) et prouvé, **pas installé** :

  - `~/.cache/rag3weaver-build/binaire-matin-22e156eb9a57`, md5
    `22e156eb9a57d59c3115de48905f0932` ;
  - RUNPATH vers `~/.cache/rag3db-libs/rag3db-build-2b11b55ec1746973592d5c6e7eb493b281212348-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`
    (plus de `LD_LIBRARY_PATH` : un rebâti de la lib ne le touche plus) ;
  - preuves sans `LD_LIBRARY_PATH` : `readelf -d` montre le RUNPATH, `ldd` résout la lib
    du magasin, `rag3db-check-lib` dit « même bâti », `--describe` des deux manifestes
    (memory 9 outils, code-rag3db 11, et 11 encore sur une copie qui porte
    `queries_via`), un hôte d'essai sur 7990 et un client de boutique (`initialize`,
    8 outils, `list_categories`, `daemon_status` au même bâti), arrêt propre.
    `649a84722b3e`, `d2b83e5e5080`, `a327c0da16e3` et `46f910c3fd19` sont dépassés.

  Lucie arrête son hôte — pid lu dans `daemon.json`, jamais par nom :

  ```sh
  P=$(jq .pid ~/.local/share/rag3weaver/daemon.json); readlink /proc/$P/exe && kill -TERM $P
  ```

  (`readlink` doit nommer `~/.local/bin/rag3weaver-backend` ; sinon ne rien tuer.) Ou elle
  autorise l'arrêt dans ma session. J'enchaîne : installation par renommage (l'installé
  gardé en `.avant-b4`) ; le bloc `workspace.queries_via` (les tests par requêtes) posé
  dans `~/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json` par le
  script de codeparsers (`~/.cache/codeparsers-notes/b2d/scripts/poser-queries-via.py`,
  gabarit figé à `38b1c9f51`, md5 f4994a5f… ; sauvegarde `rag3db.json.avant-queries-via`,
  le bloc seul — 24 mots-clés —, diff écrit ici après coup) ; signal à codeparsers
  (code-rag3db mis de côté et réindexé) ; puis redémarrage, 5 minutes de surveillance
  (`/etat` porte l'état des index) et la remesure de la montre (`watches.code`, pour le
  proto). Au rouge, `3e95f0302023` revient.

  **À trancher par Lucie, sans que j'y touche** : `backend.json`, à côté de `rag3db.json`,
  n'a pas de `db_path` et vise donc une petite base ancienne,
  `~/.local/share/rag3weaver/code-rag3db.rag3db` (3,4 Mo, dernière écriture le 11 oct.
  à 16 h 55) ; la base réelle est `code-rag3db-depot.rag3db`, que vise `rag3db.json`.
- **La tentative 4 des fiches** : une décision par fiche, l'instruction sur les fiches de règle.
- **La marche 5, seconde part** : l'hôte déclaré au manifeste, le backend nommé.
- **La forme B de l'enveloppe** : attend la §6.

## Tickets

Fermés aujourd'hui par cette session : le nœud qui ne relierait pas deux entités
gérées ; l'outil d'ancrage absent ; le huitième refus ; un lien vers un bout absent
qui rend `Ok` ; `add_ref` / `create_ref_type` qui jettent leur rapport.
Ouverts par elle, toujours ouverts : le canal d'échecs d'une réaction sans lecteur ;
un `put` sans le champ de cycle de vie ; une réaction qui écrit dans la cellule
basculée ; le banc `pg_trgm` (cause trouvée, à F).

## Ce que la session a appris

- **Une prémisse se mesure avant d'être promise.** J'ai écrit « six lignes » pour
  l'ancre ; l'orchestration a dit oui sur ma parole ; un essai de cinq minutes a
  montré que le nœud refusait. Le bon ordre est l'essai, puis la promesse.
- **Une garde sans témoin se prouve avant de s'enlever, et la preuve suffit à la
  trancher.** Le témoin champ à champ a rendu la décision évidente pour tous les
  relecteurs.
- **Les relectures ont attrapé trois vrais défauts** que j'aurais livrés : drainer
  « ses seuls liens » (un lien perdu sans erreur), la paire donnée deux fois
  (perdue sans un mot), et l'affirmation fausse sur les transactions. C'est le prix
  d'une page avant le code, et il est bas.
- **Un témoin écrit avant le code attrape le code.** La forme A disait « the record
  exists » pour une ligne absente : `get` rend `Ok(None)`, j'avais testé `is_ok()`.
  Le témoin l'a vu à sa première passe.
- **Un rouge s'attribue avant de s'expliquer.** Rejouer le banc au commit d'avant
  l'échappement a mis F hors de cause en une passe. Ensuite, une requête SQL
  directe a tranché ce que le banc ne séparait pas, sans une heure de carte.
- **Un critère de repos se lit hors de ce qui tourne toujours.** Le redémarrage de 17 h 50
  s'est arrêté de lui-même : la montre prend 0,7 cœur en continu, et le CPU du processus
  entier ne dit plus rien du repos. Les tops des autres fils, si.
- **Un script dérivé garde ses effets de bord.** Le redémarrage « sans réinstallation »
  gardait la ligne de sauvegarde : elle a écrasé la sauvegarde du binaire précédent. Un
  script qu'on dérive se relit en entier, pas à la ligne qu'on retire.
- **Une passe longue ne gagne pas la course contre master** : trois pushes perdus, puis
  une fenêtre, puis `poste push` (le verrou de G) ; une avance faite de docs seules se
  rebase et se pousse sans rejeu.
- **Vert est aucun rouge.** Compter « 4 verts » a laissé passer un rouge quand la liste est
  passée à cinq ; et `$?` après un tube est celui de `tail`. Le contrôle juge maintenant
  l'absence de rouge et le vrai code de sortie — il a ensuite refusé, à raison, un push où
  l'index des tickets avait perdu une ligne.
- **Un tick n'est pas un pour cent.** 69 tops de `/proc/<pid>/stat` en 10 s font 7 % d'un
  cœur, pas 69 % : le proto l'a relevé, mon ticket surestimait d'un facteur dix.
- **Un binaire installé dépend d'une lib qu'on rebâtit sous lui.** Un changement d'ABI casse
  les clients au prochain démarrage, pas l'hôte qui tourne : après tout rebâti de la lib,
  vérifier que le binaire installé démarre encore.
- **« Au repos » = le journal soldé, pas le CPU nul** (minuit) : un rattrapage de vecteurs
  qui attend le service d'embarquement ne prend aucun top ; mon redémarrage de minuit l'a
  interrompu, l'index `Scope_Chunk` s'est reconstruit. Puis, à 0 h 26, le remède à ce qui
  avait l'air de la boucle (25 min pour 2 096 morceaux, alors que tout le dépôt se vectorise
  en 12) l'a coupé de nouveau. Écrit dans la page C, §5. L'hôte de Lucie tourne depuis 0 h 26
  sur `3e95f0302023` (eabc53e86 : les défenses de l'arbre principal et le client qui revient
  seul) et reconstruit.
