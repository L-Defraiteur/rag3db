# La file en pile : rejouer ensemble, pousser un par un

14 novembre 2026, 16 h 06. Lucie : « rien de commençable en parallèle, même si invalidable de
suite ? si on définit des contrats ça peut se faire non ? ». Oui : c'est le pipeline dépendant
(Zuul) ou le train de fusion (bors). Ce qui coûtait n'était pas le push mais le rejeu : chaque lot
qui atterrissait invalidait le rejeu de tous ceux qui attendaient, et ils rejouaient l'un après
l'autre, dix fois de suite, même sans rouge.

## Le contrat

1. **La file est publiée, fixe, avec le hash de chaque lot** (ci-dessous). Un lot en file ne change
   plus son contenu. S'il doit changer (un rouge, une relecture), il le dit à c7 et au lot derrière
   lui **dans la minute**, sort de la file et revient derrière.
2. **Chaque session rejoue sur la pile de ceux devant elle**, pas sur master : sa branche rebasée
   sur la tête du lot d'avant, elle-même rebasée sur celle d'avant, jusqu'à master. La branche de
   pile se pousse sous un nouveau nom (`<branche>-pile`) et son hash se dit au lot derrière et à c7 :
   c'est la base du suivant. Tout le monde rejoue **en même temps**, dans la limite des lourds (au
   plus quatre à luciepc, un au portable) : les quatre premiers de la file bâtissent d'abord, les
   autres préparent leur worktree.
3. **Le rejeu court sur la pile vaut rejeu du lot** : ce que le lot touche, plus la lib entière et
   check_features pour du Rust, comme aujourd'hui ; `e2e_lectures` si le lot touche l'arrêt ou le bus,
   `e2e_postgres` si code.rs, l'IR ou un gabarit.
4. **Au « à toi », rien ne se rejoue si tout a atterri comme prévu.** Le dernier pas, **sans rebase à
   la main** (appris aux trois premiers pushs : rebaser avant déplace HEAD et la garde ne retrouve
   plus l'intervalle testé) : le worktree reste sur la tête testée, les commits de docs ajoutés
   (un ticket) se posent par-dessus, et `PILE_AVANT=<tête du lot d'avant> poste push … push_garde.sh`
   fait lui-même `rebase --onto master <tête d'avant>` après avoir vérifié que la pile d'avant est
   sur master avec le contenu testé (§ « Le dernier pas en mode pile »). Le push part sous `poste
   push`, dans `if grep -qx VERT`, trois contrôles lus. Si la pile a changé — un lot devant a changé
   ou a été retiré —, « ARRÊT : la pile a changé », et c'est un rejeu court sur la nouvelle pile.
5. **Le « à toi » reste le seul mot de push**, un à la fois ; il ne porte plus le rejeu. Les docs
   seules passent hors file comme avant.
6. **Un rouge sur la pile se lit d'abord contre son propre lot** : avant d'accuser un lot d'avant,
   la session rejoue le témoin sur sa branche seule rebasée sur master. Si le rouge vient d'un lot
   d'avant, elle le lui dit et à c7, avec le témoin : c'est ce lot qui sort.
7. **Le C++ du moteur** (`src/`, `test/`) n'est pas lié par la lib épinglée : un lot C++ seul
   rebase sans rejeu Rust et rejoue ses tests sur la tête rebasée ; il entre dans la pile comme les
   autres, sans en changer la base Rust.

## La file au 14 novembre, 16 h 06 (master aa953c8f6)

| rang | session | lot | branche | tête testée |
|---|---|---|---|---|
| 1 | lucivy | montée 4.3.7 | lucivy-4-3-7-4 | cd6d02907, la base de la pile — **atterri** à 16 h 16, master c99b2f05e, contenu identique |
| 2 | tty | tty-6 (tmux/mosh mesurés) | cx-tty-6-2 → cx-tty-6-pile | 7ac2c13d9 sur cd6d02907 — **atterri** à 16 h 25, master ce4a3140a, range-diff 5/5 en `=`, premier push en mode pile |
| 3 | recherche | surveillant, puis plafond d'instantané | surveillant-2, plafond-d-instantane → recherche-pile | 2c3bc3eb7 sur 7ac2c13d9 — **atterri** à 16 h 27, master a48b535d1, dix commits en un push, pile tenue |
| 4 | G | cas 6c du poste (trap avant la CI) | tenant-signal → tenant-signal-pile | c93f73f06 sur 2c3bc3eb7 — **atterri** à 16 h 30, master 89f46c87d, pile tenue ; cas 6c clos |
| 5 | embarquement | reprise après plantage | reprise-apres-plantage → reprise-apres-plantage-pile | 512b5e420 sur c93f73f06 — **atterri** à 16 h 39, master 67dabc121, 6/6 en `=` plus le `>` du ticket ; le `!` attendu sur nouveautes.md n'a pas eu lieu, la pile d'avant ayant atterri avec les mêmes lignes de tête |
| 6 | 96 | miroir, nom exact, about | lot-96-pile (miroir, nomex, about) | 84f949d2c sur 512b5e420 — **atterri** à 16 h 40, master c350dc9c3, trois lots en un push, 3/3 en `=` |
| 7 | hotfixs | témoin du dictionnaire (C++ test seul) | chaine-au-point-de-reprise-2 → chaine-au-point-de-reprise-pile | cabcc8640 sur 84f949d2c — **atterri** à 16 h 44, master 42c84cb77, pile tenue |
| 8 | ED | 2a′-1 (générations de l'ancre) | generations-de-l-ancre-pile | e00fd562d sur cabcc8640 — **atterri** à 16 h 46, master 78c8997de, 20/20 en `=` |
| 9 | archi | jauge du contexte (+ quatre mineurs s'ils sont verts) | jauge-ce-qui-pese-pile-2 | 67d759962 sur e00fd562d — **atterri** à 16 h 47, master cdbd346aa, pile tenue |
| 10 | cœur | correctif (a) perte de Note | annulation-garde-les-autres-pile-3 27cfe33a8 | **atterri** à 18 h 20, master 373b5477b, pile tenue, juge vert 18 h 17 ; **la lib d'après se bâtit à 373b5477b** (G) |
| 11 | recherche | la CI : `fetch_update` déprécié par la chaîne stable de la CI (read_pool.rs:261), pas par le poste 1.98.1 | ci-try-update-pile 34271fc37 | **atterri** à 18 h 17, master a646bf215, pile tenue ; la CI se rejoue dessus |
| 12 | tty | tty-7, le journal de séance et le repli sous mosh | cx-tty-7-pile-2 cce8970af sur 27cfe33a8 | **atterri** à 18 h 29, master 0de3884b9, 8/8 en `=` |
| 13 | G | la garde apprend la pile (six corrections depuis cd103b619, ticket du quota fermé) | garde-pile-2 **40f6b63a5** sur a4936c7a3 ; relecture d'ensemble par recherche ; recopie au cache à l'atterrissage | cd103b619 sur c93f73f06 (garde 2f1914405 : --onto, push refusé remis, 27 témoins ; relue par recherche à 16 h 2x ; **posée au cache des deux postes à 16 h 22**, sha256 c57a3fb1 → a58a649f, l'ancienne en push_garde.sh.avant-garde-pile) |
| 14 | codeparsers | l'import pointé (lot 1) | import-python-pointe-pile 9b4d82175 sur 40f6b63a5 | **vert** (lib 2390/0, e2e_code 54/54, e2e_postgres 36/36, MOTEUR_ANCIEN dit) ; attend garde-pile-2 |
| 15 | recherche | exclusions 1 et 5 dans le manifeste | à venir | à son vert |
| 16 → 13 bis | archi | les quatre mineurs de l'atelier | atelier-quatre-mineurs-pile **c14a389c4** sur cce8970af | rejeu court sur delais-c |
| 17 | embarquement | activations nommées au manifeste, le groupe bge-m3/relecteur/OCR à la demande | activations-nommees 7074130e7 + docs 73262341b (vert, luciepc) | vrai jeu par hôte isolé 7971 / fournisseur 7891 après la mesure d'ED |
| 18 | arbre principal | 1b, la migration par lots (représentant par version) | arbre-principal/migration-1b c26f906b7 (vert sur la copie réelle) | jeu fixe de requêtes puis passe complète |
| 19 | G | `RAG3WEAVER_DB_THREADS` → `SystemConfig::max_num_threads` (le pool du moteur suit le quota ; avant, `hardware_concurrency` l'ignorait) | fils-du-moteur 0a8142493 | **rouge lu (« 24 ») puis vert lu** ; dernier de la chaîne, base = 1b |
| 20 → 13 ter | archi | aucun client HTTP sans délai (reponse_s 120, sonde_s 5 dans daemon.json, borne jusqu'aux en-têtes) | clients-avec-delai-pile **a4936c7a3** sur c14a389c4 (vert, e2e_lectures 27/0, e2e_code 36/0) | rejeu court après les mineurs ; garde-pile-2 est posée dessus |
| 21 | hotfixs | les arêtes en double : le témoin de la clé primaire sous point de reprise (rouge e93710f36 attendu, vert 0c41f9fe6) | liens-sous-point-de-reprise ae7e6ae70 | en file au portable |

Chaque session dit à c7 et au rang suivant la tête de sa branche de pile dès qu'elle est poussée.

## La garde apprend la pile (16 h 2x, vu par recherche)

`push_garde.sh` ne savait pas la pile : après `rebase --onto origin/master <tête d'avant>`, HEAD ne
descend plus du commit testé, et la base du rejeu ferait voir le code des lots d'avant. G écrit un
mode `PILE_AVANT=<tête du lot d'avant>` dans un lot à part, `garde-pile` (rang 11 : sa tête de pile était
déjà publiée et prise pour base par embarquement) : (a) `git range-diff
<avant>..<testé> origin/master..HEAD` à `=` pour tout commit qui touche du code ; un `!` toléré si la
différence entre les deux versions ne touche que des `.md` (le conflit de `docs/nouveautes.md` résolu au
rebase), un `>` toléré s'il ne touche que des `.md` ; (b) `<avant>` ancêtre du testé ; (c) `code_apporte(<avant>, origin/master)` vide, sinon
« la pile a changé » et rejeu. Rouge d'abord dans `test_push_garde.sh`, relecture par recherche,
recopie au cache des deux postes au mot de c7, avant le dernier pas de tty, le premier de la pile.
La copie du cache fait autorité et n'attend pas master : entre la recopie et le rang 11, `--derive`
dit « garde en retard sur le dépôt » sur les pushs de la pile, c'est attendu et ce n'est pas un arrêt.

## Premier atterrissage (16 h 16) et une leçon de la garde

lucivy a poussé c99b2f05e : ses trois commits de cd6d02907 rebasés par la garde sur abe3ff1e7,
contenu identique, « rien de code » entre la base testée et master. La pile tient : tty rejoue sur
cd6d02907. Deux aléas au push : un premier essai refusé « fetch first » (master avait bougé entre la
relecture et le push) a laissé le worktree rebasé, et le second essai s'est arrêté sur « HEAD ne
descend pas du commit testé » ; remis sur la tête testée par `checkout --detach`, le troisième est
passé. **À faire dans garde-pile** : un push refusé remet le worktree sur la tête testée.

## Le dernier pas en mode pile (dès que la copie du cache a garde-pile)

L'appelant pose la variable, **sans avoir rebasé** : `PILE_AVANT=<tête du lot d'avant> poste push <session>
-- push_garde.sh <worktree> <journaux>`, le worktree sur la tête testée. La garde remplace alors « HEAD descend du testé » et « code depuis la base »
par `verifier_pile` : (b) la tête d'avant est un ancêtre du commit testé ; (c) `code_apporte(avant,
master)` vide, sinon « ARRÊT : la pile a changé » ; (a) `git range-diff avant..testé master..HEAD` à
`=`, `!` toléré si les deux versions du commit ont le même diff hors `.md`, `>` toléré s'il ne touche
que des `.md`, le reste refusé. Sans `PILE_AVANT`, rien ne change. Les têtes d'avant : tty cd6d02907 ·
recherche 7ac2c13d9 · G 2c3bc3eb7 · embarquement c93f73f06 · 96 512b5e420 · hotfixs 84f949d2c · ED
cabcc8640 · archi e00fd562d · cœur 67d759962. Un push refusé (« fetch first ») remet le worktree sur
la tête testée : à ajouter dans garde-pile (aléa de lucivy).

## Un lot change dans la pile (16 h 3x) : le rang se garde quand c'est un commit ajouté

La passe entière d'ED était rouge à un témoin d'avant le lot (`une_base_neuve_recoit_l_estampille_des_versions`,
qui figeait la forme Versions ; 2a′-1 pose Generations, c'est l'objet du lot). Le contrat dit « sort et revient
derrière » ; ici sortir coûtait plus : archi et le cœur sont posés sur ses 19 commits, les retirer de sous eux
changeait leur code autant qu'un témoin ajouté. **Règle précisée** : un changement qui est un **commit ajouté**
par-dessus la pile (pas une réécriture) garde son rang ; ceux derrière se reposent sur la nouvelle tête et
rejouent leur court (un fichier de tests/ est du code pour la garde) ; un C++ seul derrière ne rejoue rien.
Une réécriture des commits, elle, sort de la pile.

La relecture de recherche sur garde-pile : (c) juge les deux côtés, le `!` compare les diffs hors .md (les `@@`
gardés, côté prudent), sans PILE_AVANT rien ne change ; **un défaut** : le chemin du push faisait `git rebase
$neuf` sur une HEAD de pile au lieu de `git rebase --onto $neuf $PILE_AVANT` — un lot d'avant atterri après un
conflit aurait fait s'arrêter le push à tort. Cas 8 sur le chemin complet, rouge d'abord, puis la recopie.

## Premier push en mode pile (16 h 25, tty)

La garde du cache (a58a649f) a imprimé : « garde en retard sur le dépôt » (attendu), « lib de toute la
passe : 0c41f9fe6 », « mode pile : tête d'avant cd6d02907, commit testé 7ac2c13d9 », « pile tenue :
les commits du lot sont ceux du rejeu, et la pile d'avant est sur master », les trois contrôles, POUSSE.
Deux écarts de forme pour garde-pile : `$J/base` lu avant de connaître le mode (« cat : Aucun
fichier », sans effet) ; la liste des journaux où la garde cherche la lib ne couvre pas une passe
courte (tty a copié `e2e_tty.log` en `temoins.log`) — à élargir à `*.log`, une seule lib, zéro journal
reste un arrêt. Et la lecture de `nproc` sous un lourd (G) : le quota du cgroup rend 5, pas 24 ;
`nproc --all` dans `fils_par` et le cas 8 du témoin, dans garde-pile, avec un ticket.

## Trois pushs de pile en cinq minutes (16 h 25 → 16 h 30)

tty (ce4a3140a), recherche (a48b535d1, dix commits en un push), G (89f46c87d) : trois « pile tenue »,
zéro rejeu. Deux défauts de forme de plus pour garde-pile, vus par G : `--derive` fait `realpath "$0"`
après le `cd` et imprime une empreinte vide quand `$0` est relatif ; et le `cat base` déjà dit.

## Cinq atterris (16 h 39) ; ce que la garde devrait dire de plus

embarquement : « mode pile : tête d'avant c93f73f06, commit testé 512b5e420 / pile tenue ». Le `!`
attendu sur nouveautes.md n'a pas eu lieu : la pile d'avant a atterri avec les mêmes lignes de tête
que celles sur lesquelles le conflit avait été résolu, le commit est `=`. Pour garde-pile : la phrase
« garde en retard sur le dépôt » dit l'inverse quand c'est la copie du cache qui est en avance (dire
les deux empreintes et le sens) ; `verifier_pile` doit imprimer le range-diff jugé, pas seulement
« pile tenue ».

## La CI rouge (16 h 45) : la chaîne de la CI flotte, celle du poste non

`poste push` signale « CI rouge depuis 15 h 30 UTC » à chaque push de la pile : depuis a48b535d1
(recherche), le jeu `sans-feature` de check_features rougit sur la CI sous `-D warnings` —
`Atomic::<u64>::fetch_update` déprécié (renommé `try_update`) à `src/read_pool.rs:261`. Le poste est
en rustc 1.98.1 et ne le voit pas ; la CI prend `dtolnay/rust-toolchain@stable`, plus récent. Ce
n'est ni un lot cassé ni le poste : c'est deux chaînes. Recherche corrige en une ligne au rang 11 ;
G ouvre le ticket du fond (épingler la chaîne de la CI à celle du poste par `rust-toolchain.toml`,
ou laisser flotter et monter le poste), décision à Lucie.

## La mesure d'ED tient le poste (17 h 3x) : la pile attend, au mot de Lucie

La mesure de 2a′-2 a dit son premier chiffre : **un search_code sans filtre coûte environ 6,8 s** sur
une base à versions de la taille du dépôt (79 662 scopes), dont 0,8 s de masque des versions ; la
migration vers les générations, 20 ms. Lucie : « faut se concentrer sur optim ça, ça fait partie du
lot : si on apporte la feature faut faire le max pour qu'elle devienne viable » et « filtré ça prend
combien ? faudrait voir filtré vs non filtré ». La mesure unique relancée (sonde ca428691b, durées
par nœud lues sur NodeRun, trois formes sans filtre / par chemin / par langage sur la référence R —
la même base avant 1b — et sur A ; B et C sans filtre ; 10 requêtes × 5) tient `poste mesure` environ
1 h 10 dès son départ : les rejeux de la pile (tty-7, les quatre mineurs, codeparsers) attendent vers
19 h. Les pushs sous `poste push` (recherche pour la CI, le cœur) ne sont pas des lourds et passent.

## Ce que l'après-midi a appris (17 h 4x)

- **1b est vert sur la copie réelle** (arbre principal, 861 s, 119 887 vecteurs repris sans un appel au
  modèle, RSS 18 Gio, deux sens égaux sur 21 tables, couverture sans trou, comptes identiques). Le
  tampon plein n'était pas le moteur mais la jointure du transfert ; la version la plus partagée est
  `namespace rag3db` (295 occurrences). Pose sur les vraies bases : une bascule à part.
- **Hotfixs** : 0 orpheline sur la base migrée, 0 morceau sans parent ; un défaut antérieur à 1b,
  1 291 morceaux `Scope_Chunk` à 2–7 arêtes vers le même Scope (2 205 de trop), ticket ; la cause à
  hotfixs, le sort de la table à arbre principal (reste-t-elle après 1b ?).
- **G** : l'hôte n'a aucun quota ; les 168 `demon-#` sont **cinq pools du moteur à 32 fils** (une base
  ouverte = `max_num_threads` 0 = tous les cœurs) ; page 03 d'optimiseur, drop-in proposé à Lucie
  (le poids protège l'écran plus que le quota) ; le lot Rust `RAG3WEAVER_DB_THREADS` à G.
- **Embarquement** : activations nommées (forme B) et le groupe à la demande verts.
- **Codeparsers** : l'import pointé vert ; les règles d'import Python en données (`data/imports/
  python.toml`, 297 noms de la stdlib 3.14.9, défaut compilé, dossier remplaçable par langue).
- **Banc** : le juge de parité de `search_code` pour 2a′-2 (jeu commun avec ED, `_uuid`, ordre aux
  10 premiers entre R et la base migrée, au bit près entre la base migrée et chaque optimisation).

## La mesure a parlé (18 h 0x) ; la chaîne de la pile reprend

Lexical ×8 sur une base à versions (874 → 6 928 ms par search_code sans filtre ; par chemin ×8,7, par
langage ×6 : le filtre réduit sans contourner) ; le drapeau ajouterait 1,2 s ; la piste est le
`allowed_ids` de 72 652 uuid passé à lucivy à chaque requête — la **vue lucivy nommée** par (couche,
rang) est le lot. Le poste est libre : la chaîne reprend, chacun dit sa tête au suivant — recherche
(CI, PILE_AVANT=96f06eb87) → cœur (27cfe33a8, PILE_AVANT=34271fc37, à son juge) → tty-7 (reposé sur
27cfe33a8) → mineurs → clients (archi) → garde-pile-2 (G) → import pointé (codeparsers) → exclusions
(recherche) → activations (embarquement) → 1b (arbre principal) → fils du moteur (G).

## L'ordre réel de la chaîne (18 h 1x)

recherche (CI) → cœur (27cfe33a8, juge) → tty-7 (cce8970af) → mineurs (c14a389c4) → clients (a4936c7a3)
→ garde-pile-2 (40f6b63a5) → import pointé (codeparsers, à poser) → exclusions (recherche, à son vert)
→ activations (embarquement, à poser) → 1b (arbre principal, à poser) → fils du moteur (G, à poser).
La mesure d'ED n'a pas été arrêtée par quelqu'un : l'oom-killer de sa portée (`POSTE_MEM_MAX=32G`) a
tué la sonde à 33 Gio de RSS après ~250 search_code — un second défaut à départager (RSS par appel).

## Douze atterris (18 h 2x) ; la lib d'après

recherche (la ligne pour la CI, a646bf215) puis le cœur (373b5477b, le correctif (a) de la perte de
Note avec ses trois témoins, juge vert à 18 h 17) : douze lots atterris par la pile, zéro rejeu au
dernier pas. G bâtit la lib d'après à 373b5477b et bascule par la procédure sans attendre Lucie, après la
passe de 96 (mémoire fermée 18 h 20 → ~18 h 50). La chaîne : tty-7 (à toi à son vert) → mineurs →
clients → garde-pile-2 → import pointé → exclusions → activations → 1b → fils du moteur ; devant 1b, le
lot d'arbre principal « sans filtre ni vue, pas de masque » (le ×8 de la recherche à versions, 5,0 s de
`_uuid IN [72 652]` → offsets côté moteur, ticket moteur ; lucivy mesurée hors de cause, 6 à 10 ms).

## Treize atterris (18 h 5x) ; la lib d'après bâtie ; deux défauts de la recherche à versions

tty-7 atterri (0de3884b9). La lib d'après **373b5477b** est bâtie à luciepc (ABI : aucun symbole exporté
ajouté ni retiré ; md5 librag3db 4fa330b5, libvector ce6b2db4), verdict e2e_code/e2e_lectures en cours,
bascule annoncée ensuite. En file : archi (mineurs, un SIGSEGV à la sortie rejoué vert seul, SEGV_ACCERR
comme le core du dictionnaire ; puis les clients) → garde-pile-2 (vert, cas 8 joué dans le lourd) →
import pointé (vert) → exclusions → activations (vrai jeu parti 18 h 46 sur 7971/7891) → sans-filtre
(arbre principal, rouge posé, code en jeu) → 1b → fils du moteur → chaîne Rust 1.98.1 (G, d950d8790,
dernier). **Deux défauts de la recherche à versions**, qui tiennent 1b loin des vraies bases : le ×8 (5,0 s
de `_uuid IN [72 652]` → offsets côté moteur, ticket moteur ; le court-circuit sans filtre d'arbre
principal) et **une rétention de ~0,5 Gio par search_code** (deux sondes tuées à 32 Gio par l'oom-killer
de leur portée ; ED départage moteur seul contre recherche entière ; le cœur entre si c'est le moteur).

## Dix-sept atterris (19 h 1x)

mineurs (d4f0b5a74) · clients (69b2f5474) · garde-pile-2 (8565a06bf, la garde de master posée au cache des
deux postes, f9521fb3, « garde à jour ») · import pointé (b45056943). La chaîne : tty-8 (696cbf5a3, rejeu)
→ exclusions (bb0cea68d, passe) → activations (a7d39e4e6, vrai jeu vert, passe courte) → sans-filtre
(b0ecb3bb3 : 406 s → 12 s sur 24 requêtes, 24/24 identiques) → 1b (0ed318ad7, passe complète en file) →
fils du moteur (G) → chaîne Rust 1.98.1 (G, d950d8790) → pointeur des règles d'import (codeparsers,
regles-imports-2 d7d9ce7a2, rang 22).

## Dix-huit atterris (19 h 2x)

tty-8 (b3f58c30e, 15/15 en `=`, la CI de master verte). La chaîne : exclusions (recherche, bb0cea68d,
passe) → activations (embarquement, a7d39e4e6, passe courte) → sans-filtre (arbre principal, b0ecb3bb3)
→ 1b (0ed318ad7, passe complète en file) → fils du moteur (G) → chaîne Rust 1.98.1 (G) → pointeur des
règles d'import (codeparsers, a1972c0f7 dans codeparsers : toutes les langues en données, 311/0). En
attente d'une place à luciepc : le verdict de la lib 373b5477b, la parité au bit près de banc, la
passe de 1b, la mesure du dense d'ED.

## Un rouge sur master pendant la pile (19 h 3x → )

`e2e_demon_hote::une_base_a_rouvrir_ne_lache_que_son_backend` est rouge sur master depuis un lot du soir
(vert à 2a1f52c21, rouge dès c350dc9c3) : après un point de reprise échoué, la réouverture de A dit
« already open for writing in this process — 1 référence à l'instance précédente tenue après 10 s ». Ce
n'est pas 2a′-1 (manifeste carnet, `generations = None`), ni le plafond ni le Surveillant de recherche
(lecture, et le message n'est pas celui d'un instantané rompu). Premier suspect commun : la régénération
des fichiers composés à l'arrêt du miroir (96, 27f5c21a1), qui relit la base sur le catalogue du backend
que l'hôte lâche ; contre-essai avec/sans par 96, bissection par archi (2a1f52c21..42c84cb77, cinq pas).
Règle tenue : **aucun lot ne pousse avec un rouge qu'on ne comprend pas** ; les lots dont les passes ne
jouent pas cette suite continuent, le coupable corrige en tête de pile.

**19 h 5x — le rouge date d'avant la pile** : archi a rejoué sa borne, le test est rouge dès 2a1f52c21
(15 h 12), et le dernier vert archivé est du 13 novembre à 8 h 01 sous la lib 29a7bc52a ; 96 et recherche
sont hors de cause. Deux jeux croisés tranchent lib contre Rust (2a1f52c21 sous 29a7bc52a ; 4f6f80ef4 sous
0c41f9fe6). Leçon : le verdict d'une bascule ne jouait pas e2e_demon_hote ; il y entre. La pile reprend ses
pushs : le rouge n'est à aucun lot de ce soir.

**20 h 0x — la pile se repose d'un cran** : les exclusions (bb0cea68d) cassent `cargo check
--no-default-features` (`exclusion.rs:45` sans `cfg(feature = "code")`), vu au rang 22 par codeparsers,
qui ne corrige pas un code qui n'est pas le sien ; recherche ajoute un commit, et la chaîne au-dessus
(activations → sans-filtre → 1b → fils du moteur → chaîne Rust → pointeur des règles) se repose et rejoue
son court. Un commit ajouté garde son rang ; ceux derrière paient un court.

**20 h 0x — l'ordre après la repose** : exclusions (recherche, + le commit du `cfg`) → **tty-9** (cx-tty-9
63021448d, cinq défauts de l'essai, s'insère) → activations (embarquement) → sans-filtre → 1b (arbre
principal) → fils du moteur (G) → chaîne Rust 1.98.1 (G) → pointeur des règles d'import (codeparsers).
Chacun dit sa tête au suivant ; rejeux courts à la repose. Le rouge de l'hôte est **du Rust du 13**
(vert à 4f6f80ef4 sous 0c41f9fe6, rouge à 2a1f52c21 sous 29a7bc52a) ; bissection d'archi entre ces
bornes jouées ; la lib 373b5477b a e2e_code 36/36 et e2e_lectures 27/27, e2e_demon_hote en cours.

## Deux rouges de base d'affilée au rang 22 (20 h 1x) : une règle de plus

Le dernier maillon (codeparsers) a vu deux fois le jeu sans-feature de check_features rougir sur du code
d'un lot sous lui : `exclusion.rs:45` sans `cfg(feature = "code")` (recherche, corrigé par un commit
ajouté, f63d1bf4e), puis `catalog.rs:2802` deux méthodes jamais utilisées sans feature (1b d'arbre
principal, 23bf41b2c, commit ajouté en cours). Chaque fois, la chaîne au-dessus se repose et rejoue son
court. **Règle (suggestion de codeparsers) : chaque maillon joue check_features — les huit jeux — avant
de donner sa tête au suivant.** Le rejeu court d'un lot de la pile, c'est au moins check_features.

**Le rouge de l'hôte, bissecté** : 1e9bf5089e / 2f6c34643 (arbre principal, raccord physique du 13) —
le registre des écrivains attend que `Weak<Database>` et `Weak<ReopenState>` tombent ; or le crochet de
test `FailedCheckpointHook` tient un `Arc<ReopenState>` pour toute la durée du test, donc la
réouverture attend 10 s puis refuse. Aucun chemin produit ; correctif = `Weak` + `upgrade` dans le
crochet, en lot à part en bout de chaîne (base = la tête de codeparsers), rouge = le test puis un témoin
court. La règle du 13 (attendre les deux références) reste.

**20 h 3x — la chaîne reposée, deux rangs de plus** : exclusions (recherche, f63d1bf4e, passe) → tty-9
(805af3bb6, vert) → activations (embarquement, 767d214fd, passe) → sans-filtre → 1b corrigée (arbre
principal, 5fc11e0f1, passe) → fils du moteur (G, be641a3c5, check_features 8/8 avant la tête) → pointeur
des règles d'import (codeparsers, 5551741db, passe) → **rang 23 : la fermeture qui tient** (archi, le
ticket du rouge de l'hôte avec, Pour : arbre principal) → **rang 24 : le crochet de test en `Weak`**
(arbre principal : le rouge de l'hôte se ferme) → chaîne Rust 1.98.1 (G, dernier). La bissection d'archi
confirme : 9977acbdc vert, 1e9bf5089e rouge, seul. La bascule de la lib 373b5477b reste suspendue
jusqu'au verdict commun de G et archi (jeux croisés + jeu instrumenté sous les deux libs).

**20 h 35 — dix-neuf atterris** : les exclusions de recherche (1bb929755, quatre commits, 4/4 en `=`, le `cfg`
avec : `workspace.index_exclude`, minifié par contenu et fichiers de plus de 1 Mio, défauts dans les
gabarits). tty-9 a le « à toi » ; puis activations, sans-filtre, 1b, fils du moteur, pointeur des règles,
la fermeture qui tient, le crochet en `Weak`, la réécriture du `IN` (recherche, rang 25), la chaîne Rust.

**20 h 5x** : activations prêt (8105b7f02 reposé à la main après un conflit de nouveautes.md ; la garde
apprend `PILE_TESTE`, garde-pile-3 posée au cache d9a6c083) → sans-filtre → 1b → fils du moteur (vert) →
pointeur des règles → fermeture qui tient (7c0cb6f6c) → crochet en `Weak` (arbre principal) → réécriture
du `IN` (recherche) → gestes de 96 → (B) l'`IN` haché (cœur, ab26b045d, C++) → garde-pile-3 (52f674ad7) →
chaîne Rust 1.98.1. La bascule de la lib 373b5477b part après la recopie ; les passes épinglent la lib.

**20 h 5x — vingt atterris** (activations 441f28e08, par `PILE_TESTE` après un conflit de nouveautes.md).
L'ordre du bout de chaîne : sans-filtre → 1b (arbre principal, passe) → fils du moteur (G, vert) →
pointeur des règles (codeparsers, passe) → fermeture qui tient (archi, 7c0cb6f6c, rejeu) → **gestes de
96** (rang 24, sur 7c0cb6f6c) → **crochet en `Weak`** (arbre principal, 25) → **réécriture du `IN`**
(recherche, 26) → (B) l'`IN` haché (cœur, 27, C++) → garde-pile-3 (G, 28) → chaîne Rust 1.98.1 (G). La
bascule de la lib 373b5477b est partie à 20 h 52 (hôtes arrêtés, réveil vers 21 h 10 et 21 h 25).

**21 h 0x — le bout de chaîne** : … fermeture qui tient (archi, rejeu) → gestes de 96 (eeefcc493, check_features
8/8, rejeu) → crochet en `Weak` (arbre principal, vert sur master, à poser sur 96) → réécriture du `IN`
(recherche, à poser) → (B) l'`IN` haché (cœur, C++) → garde-pile-3 (G) → **dense sur `Tenue<C>`**
(embarquement, 4905e56e3, vert sous 373b5477b, rang 29) → chaîne Rust 1.98.1 (G, dernière). Le rang 22
(codeparsers) est vert hors un rouge d'environnement nommé (le crash physique, vert seul 3/3).
Les mesures : un index plein texte neuf du dépôt pèse 771 Mo (4,59 Go ce matin) par le correctif des
octets d'ED ; les exclusions en ôtent 17 de plus ; le dictionnaire garde 94 Mo de textes morts après purge
(lucivy : la fusion lâchera les ids sans document vivant, puis une compaction filtrée du dictionnaire).

**21 h 2x — rang 30 : le masque en prédicat** (ED, vue-publiee 959d73513, sous 373b5477b) : le lexical et le
dense passent chacun par une requête courte (une version est admise si une de ses occurrences est visible)
au lieu d'une liste de 72 652 uuid — sans filtre 7 032 → 2 832 ms (le court-circuit d'arbre principal
reste meilleur et garde le sans-filtre), par chemin 2 154 → 1 299, par langage 1 371 → 1 020 (×1,3 à ×1,74
de la base d'avant) ; plus de projection de 2,9 Mo ; mémoire plate à 20 Gio au filtré (MALLOC_ARENA_MAX=2 à
mesurer), B (le drapeau tombé) à l'OOM : deux tickets. Après le dense d'embarquement (29), avant la chaîne
Rust. Le fils du moteur de G est vert sur sa pile sous 373b5477b ; luciepc contrôle encore ses deux bases.

**21 h 3x — arbre principal a le « à toi »** : ses deux lots (sans-filtre e1857b85b + 1b c9a86e006, 11
commits) en **un seul push**, puisque la passe (verte à 21 h 09, lue à 21 h 35 : une passe détachée se
relit à l'heure) a testé la tête de 1b ; PILE_AVANT=8105b7f02. Puis G (fils, sur c9a86e006), codeparsers
(sans rejeu), archi, 96, le crochet, le `IN`, (B), garde-pile-3, le dense, le prédicat, la chaîne Rust.
La bascule : le portable réveillé sous 373b5477b ; **un écart sur code-lourd** à luciepc (24 relations,
les deux index d'une table de relations en désaccord, antérieur à la lib neuve) ; décision c7 : la
bascule continue, la base se répare dans la fenêtre d'hotfixs (reconstruction des trois tables sur copie).

**21 h 3x — vingt-deux atterris** : le court-circuit et 1b (194a4e572, 11/11, un push). G a le « à toi »
pour les fils du moteur après le réveil de luciepc (code-principal diverge aussi, décision (b) tenue).

**21 h 3x — vingt-quatre atterris** : les fils du moteur (77b9e5f18, sous 373b5477b) et le pointeur des règles
d'import (7215d18ad0, sept langues en données). **La lib 373b5477b est courante aux deux postes depuis
21 h 33**, hôtes réveillés, bases rouvertes sans refus ; les deux écarts de luciepc (code-lourd,
code-principal) en un ticket pour hotfixs, reconstruction des six tables sur copie. Archi a le « à toi »
(fermeture qui tient) ; puis 96, le crochet, le `IN`, (B), garde-pile-3, le dense, le prédicat, la chaîne
Rust.

**21 h 4x — vingt-six atterris** : la fermeture qui tient (1cbf1cf3a) et confirmer_fiche (10b7a0edb). Le
crochet en `Weak` (arbre principal, d1578cd56) pousse ; puis la réécriture du `IN` (recherche) → (B) et le
pliage des listes (cœur, deux pushs, rang 27) → garde-pile-3 (G) → dense sur `Tenue<C>` (embarquement) →
le prédicat (ED, après la parité) → la chaîne Rust 1.98.1 (G, dernière).

**21 h 4x — vingt-sept atterris** : le crochet en `Weak` (a76f36a08d, 3/3) **ferme le rouge de l'hôte** qui
dormait depuis le 13 (e2e_demon_hote 14/14 sous 373b5477b). Recherche pousse la réécriture du `IN`
(e0aaa2d1f) à son vert ; puis le cœur (deux pushs), G (garde-pile-3), embarquement (dense sur `Tenue<C>`,
vrai jeu vert), ED (prédicat, après la parité), G (chaîne Rust 1.98.1).

**21 h 5x — vingt-huit atterris** : la réécriture du `IN` (recherche, 9fd946ec3, 2 commits : rouge
843927e68 puis le filtre par la clé primaire ; 30 000 uuids 1,10 s → 0,28 s ; range-diff tout en « = » sur
PILE_AVANT=d1578cd56 ; rejeu court vert sous 373b5477b, check_features 8/8). Le cœur a le « à toi » pour
ses deux pushs du rang 27, posés sur e0aaa2d1f : (B) l'`IN` haché (in-liste-hachee-pile a56af398d,
3 commits, PILE_AVANT=e0aaa2d1f) puis le pliage des listes littérales (projection-rend-sa-memoire-pile
9d90edbb6, 5 commits, PILE_AVANT=a56af398d) ; ni `src/` ni `test/` n'ont bougé entre ses bases et
e0aaa2d1f, les suites C++ se rejouent quand même sur les têtes reposées (DropNodeTableReclaim seul admis,
sous ticket). ED : la parité du prédicat (959d73513 contre 3e3a07f9d, 30/30 au bit près, sans filtre comme
sous filtre ; le commit rouge intermédiaire ne compile pas seul sous `deny(warnings)`, la tête l'appelle) —
le lot est prêt au rang 30. Puis G (garde-pile-3), embarquement (dense), ED (prédicat), G (chaîne Rust).

**22 h 0x — un lot s'insère au rang 27 ter** : les orphelins de recherche (orphelins-pile e947089dd :
`DeleteRecordNode` lit en base l'offset des lignes que le cache de session ignore, `select_offsets_by_uuids`
avec sa forme Postgres ; témoin rouge 6 documents pour 5 Scope), posés d'abord sur master 9fd946ec3, à
reposer sur le pliage du cœur 9d90edbb6 ; Rust seul sous deux lots C++ : rejeu court sous 373b5477b avec
`RAG3WEAVER_MOTEUR_ANCIEN=1` dit au journal, check_features avant la tête. G repose garde-pile-3 dessus.
Ordre : cœur (B) → cœur pliage → recherche orphelins → G garde-pile-3 → embarquement dense → ED prédicat →
G chaîne Rust.

**22 h 0x — banc** : la parité du `IN` (e0aaa2d1f contre 08aa781db) verte au bit près 30/30 ; un appel
sans chauffe, parent → lot : chemin 2 272 → 1 176 ms, langage 1 508 → 910 ms, sans filtre 623 → 624 ms.
Les trois optimisations du soir (court-circuit, prédicat, `IN`) sont vertes au bit près. Laya synthétique à
question fixe : 0,09 en zéro-coup, 0,58 spécialisé (créer 0,91, rejoindre 0,24 — le hasard parmi cinq) ;
le titre seul apprend « créer » ; le vrai jeu attend la trace de 96.

**22 h 0x — codeparsers au rang 28 bis** : la provenance par langue (codeparsers cf90e1c : tables
`declared_dependencies_by_language` et `project_packages_by_language`, noms normalisés par les variantes
de la langue, csharp.toml tiré de dotnet-api-docs a010100, 417 espaces de noms ; rag3db
codeparsers/provenance-par-langue d1ff77ac7 : manifestes rangés par langue, go.mod lu, code.rs ne passe que
les tables par langue, Cargo.lock inchangé ; témoin `l_analyse_compare_aux_manifestes_de_la_langue`).
Découverte : un import Go ne produit aucune bibliothèque (le parseur Go passe par l'extraction JavaScript),
ticket dans le lot ; lot suivant validé : les imports Go par tree-sitter. Base = la tête de garde-pile-3 ;
e2e_postgres au rejeu (code.rs), `RAG3WEAVER_MOTEUR_ANCIEN=1` dit. Ordre : cœur ×2 → recherche orphelins →
G garde-pile-3 → codeparsers → embarquement dense → ED prédicat → G chaîne Rust.

**22 h 0x — archi au rang 27 quater** : le fil `index-suivi` joint à l'hôte (index-suivi-joint-2 b1d99ae6f,
passe entière verte sur master 9c71f3b80 sous 373b5477b, target propre 15 Go après un plafond à 33 :
check_features, lib 2401/0, e2e_hooks 2+2, navigateur 45/45, e2e_demon_hote 14/14, e2e_lectures 27/27),
reposé sur la tête des orphelins de recherche ; rejeu court (check_features, e2e_demon_hote, e2e_lectures,
témoin du script) sous `RAG3WEAVER_MOTEUR_ANCIEN=1`. Ordre : cœur ×2 → recherche orphelins → archi
index-suivi → G garde-pile-3 → codeparsers → embarquement dense → ED prédicat → G chaîne Rust.

**22 h 1x — codeparsers** : les imports Go par tree-sitter rouges puis verts (branche imports-go sur cf90e1c :
chaque `import_spec` donne une ImportReference Namespace, alias sinon dernier élément, `/vN` renvoie à
l'élément d'avant, `gopkg.in/yaml.v3` → yaml, `_` SideEffect, `.` ne lie rien ; 315 passés). Voie (a) :
le pointeur du rang 28 bis monte sur cette tête, le témoin Go de manifestes.rs revient, le ticket se ferme
dans le lot.
Prêt à 22 h 1x : deux commits (d1ff77ac7, 50601f55e), pointeur a1972c0 → cf90e1c → 6ba8478 ; témoin
`go_l_analyse_garde_la_casse_du_module_requis` rouge sous cf90e1c, vert sous 6ba8478 ; manifestes:: 8/8 ;
deux tickets fermés, check_tickets VERT 508 ; Cargo.lock inchangé. Attend la tête de garde-pile-3.

**22 h 3x — le cœur** : ses suites de pile attendaient derrière son témoin des deux sens (plus de 20 min par
tour, 20 000 relations versées par recherche de clé), arrêté par ses pids à 22 h 29 ; son premier tour sur
f41c0ba5b, avant le correctif, était déjà rouge (la relation validée de B perdue dans les deux sens). Les
suites partent sous `poste lourd` : planner_tests sur a56af398d, puis projected_graph_memory_test,
planner_tests et e2e_test `--gtest_filter='*'` sur 9d90edbb6 ; deux pushs vers 23 h. Leçon : une mesure ou
un témoin long ne se met pas devant un push de pile dans le même arbre — un arbre de pile ne porte que le
lot qui pousse.

**22 h 3x — G, second push du rang 28** : `check-rel-ancre` (tools/check_rel_directions : bouts par tranches
de 500 offsets ancrées sur l'étiquette, quatre comptes par table — lues au sens avant, au sens arrière,
union, perdues d'un sens — ; rouge d'abord sur une base jetable où une relation n'est lisible que par
l'ancrage, EXPLAIN dans le témoin ; ticket 2329a532f). Outillage de procédure, mais code pour la garde (c) :
dans la pile, posé sur garde-pile-3, même tour. Codeparsers se pose sur sa tête.

**22 h 3x — 96 au rang 28 ter** : la trace d'archive (lot-96-trace : `ForEachNode` reçoit `report_results`,
`archive` reçoit `trace` qui rend par section les candidates, l'étiquette brute, le sujet, la voie ; rien en
base, le rapport va dans reponse.jsonl du lanceur ; e2e_archiviste 15/15, foreach_node 38/38, rouges sur
6ba1427e4), derrière codeparsers. Le rejeu des deux corpus sur copie pour banc part sans attendre.
Ordre : cœur ×2 → recherche orphelins → archi index-suivi → G garde-pile-3 + check-rel-ancre → codeparsers →
96 trace → embarquement dense → ED prédicat → G chaîne Rust.
22 h 4x : les replace de go.mod rejoignent le 28 bis (un replace local rend `project`, un replace distant
garde la dépendance sous son nom et ajoute la cible ; manifestes:: 10/10) : trois commits.

**22 h 4x — arbre principal au rang 30 bis** : les sources suivies (arbre-principal/suivies 2e41a4a11 :
déclaration, résolution, racine, parente, flux, id stable, migration des clés, cache, branchement à la place
de `couches.rs`, test par ligne du tableau ; 14 tests, check_features 8/8), passe complète en file à luciepc.
Base = la tête du prédicat d'ED (rang 30) ; devant la chaîne Rust de G, toujours dernière.

**22 h 5x — les orphelins sortent de la file** : reposés en 832a5896d sur 9d90edbb6, rejeu court vert
(e2e_code, e2e_code_sync, e2e_postgres, check_features 8/8) mais le témoin de la lib toujours rouge
(6 documents pour 5 Scope : lire l'offset en base au moment de `DeleteRecordNode` ne suffit pas) ; recherche
trace à luciepc, la parité de banc sur 832a5896d sert de témoin indépendant. Archi se pose directement sur
le pliage 9d90edbb6 ; recherche revient à son vert, sur la tête du dernier lot posé alors. Ordre : cœur ×2 →
archi index-suivi → G garde-pile-3 + check-rel-ancre → codeparsers → 96 trace → embarquement dense → ED
prédicat → arbre principal suivies → G chaîne Rust.
22 h 5x : le 28 bis fait cinq commits (nouveautés avec « une dépendance déclarée pour une autre langue ne
compte plus », README de la provenance, ticket souhait du `[lib] name`, puis requirements : dossier
requirements/, `nom[extras] @ url`, `-e …#egg=nom`, une URL sans nom n'en invente pas ; manifestes:: 11/11).
Le lot 1 (renommages Python en données + champ `imports`) entrera à part, à son vert.

**22 h 5x — archi a donné sa tête** : index-suivi-joint-2-pile 58b7a8eab sur 9d90edbb6, rejeu court en
cours sous MOTEUR_ANCIEN=1 ; G repose garde-pile-3 dessus. À son atterrissage, un commit de docs retire dans
la minute la copie ouverte du ticket qu'il ferme (le `git mv` du lot n'a rien trouvé sur 9fd946ec3, plus
ancien que le ticket 7aac92bfb). G : le relevé ancré trouve 11 relations perdues d'un sens dans LOCKS de
code-principal là où l'ancien outil disait 0 ; le témoin de check-rel-ancre joue sur une copie reflink
jetable de la base réelle, ROUGE « non joué » sans elle, hors de tout jeu sans base. Codeparsers : 28 bis
de3d450c1 (cinq commits) ; lot 1 sur la table de pipreqs (Apache-2.0, attribution) + champ `imports`
(`replace`, `distributions`) du manifeste.
Banc : la parité des orphelins (832a5896d contre 9d90edbb6) verte au bit près 30/30, mais le témoin des
comptes ne juge pas sur la forme à versions (84 211 / 84 211 avant et après l'effacement d'arbre.rs, parent
et lot identiques). Deux faits à lire par les colonnes de version, pas les comptes : les Scope d'un fichier
effacé sont-ils fermés (`_jusqu_a`) ou ouverts ; la ligne File est-elle fermée ou effacée (une histoire
perdue sous versions). Banc vérifie ; ticket pour ED selon l'issue.
Archi, 22 h 5x : rejeu court vert (check_features 8/8, témoin du script, e2e_demon_hote 14/14, e2e_lectures
27/27, une seule lib) ; attend le « à toi » au « poussé » du cœur, PILE_AVANT=9d90edbb6.
G, 22 h 5x : les deux lots du rang 28 prêts — garde-pile-3-pile 79d15a5fb sur 58b7a8eab (test_push_garde
33/0, check_features 8/8 après un rouge de la garde du target repris par RAG3WEAVER_TARGET_REPRENDRE=1) ;
check-rel-ancre-pile 0f9a149ed dessus (témoin : 4 ROUGE sur l'ancien outil, 5 vert sur le neuf ; LOCKS de
code-principal : avant 436, arrière 425, union 436, perdues 11 ; hors de tout jeu automatique, vérifié).
Codeparsers a sa base : 0f9a149ed.

**22 h 5x — l'`IN` haché est sur master** (c8e6e7f18, dd43a2be3, e5e626803) ; le pliage suit. Codeparsers :
28 bis reposé sur 0f9a149ed, provenance-par-langue-pile 498799f34 (cinq commits, pointeur a1972c0 →
6ba8478, Cargo.lock inchangé, check_tickets VERT 493), rejeu court en file sous le lourd ; 96 se posera sur
498799f34. Lot 1 : la partie codeparsers verte (renommages-python 85fe233, 318 passés).

**22 h 5x — vingt-neuf atterris, le cœur a poussé ses deux** : l'`IN` haché (e5e626803, 3/3 en « =») et le
pliage des listes littérales (1d5580d74, 5/5 en « = ») ; C++ seul, pas de rejeu Rust ; planner_tests (rapport
IN/UNWIND 0,64), projected_graph_memory_test 10, e2e_test 1 867 verts, DropNodeTableReclaim seul rouge sous
ticket. Réserve au ticket d'ED : les cas LongQueryPeak ne départagent que joués seuls, chacun dans son
processus. ABI inchangée : la lib d'après portera les deux. Archi a le « à toi » (PILE_AVANT=9d90edbb6) ;
puis G ×2, codeparsers, 96, embarquement, ED, arbre principal, G chaîne Rust.

**23 h 0x — trente atterris** : le fil `index-suivi` joint à l'hôte (archi, 0a5cd8168 : témoin 9befdc3b0,
correctif ef12ba4ea, ticket fermé ; 3/3 en « = », « pile tenue ») ; la copie ouverte du ticket retirée dans
la minute (863b81b46), check_tickets VERT 511. G a le « à toi » pour ses deux (garde-pile-3 79d15a5fb,
PILE_AVANT=58b7a8eab ; check-rel-ancre 0f9a149ed, PILE_AVANT=79d15a5fb) ; puis codeparsers (498799f34),
96, embarquement, ED, arbre principal, G chaîne Rust.

**23 h 0x — trente-deux atterris** : garde-pile-3 (533e5cbb3, PILE_TESTE pour une tête reposée à la main) et
l'outil des deux sens ancré (a34fe130a2 : tranches de 500 offsets par étiquette, quatre comptes par table,
plans sous CHECK_REL_PLANS=1). Codeparsers a le « à toi » à son vert (PILE_AVANT=0f9a149ed) ; puis 96
(sur 498799f34), embarquement, ED, arbre principal, G chaîne Rust. G recopie la garde au cache des deux postes.
G, 23 h 05 : la garde du cache (d9a6c083afc8aed6, posée à 20 h 43) est celle de master à a34fe130a, rien à
recopier, `--derive` dit « garde à jour » ; docs 9da00dfd8 (ticket de l'outil fermé, les 11 perdues au
ticket 9426bc2a8, ticket souhait du crochet d'injection pour le cœur, page 04 : le portable sain).

**23 h 0x — trente-trois atterris** : la provenance par langue (codeparsers, a12f08bdf2, cinq « = », pointeur
6ba8478 ; rejeu check_features 8/8, manifestes:: 11/11, e2e_code 55/55, e2e_postgres 36/36). 96 a le « à toi »
à son vert (lot-96-trace-pile 77fccf493 sur 498799f34, check_features 8/8) ; embarquement a sa base 77fccf493.
**CI rag3weaver rouge à 22:00 UTC sur 0a5cd8168** : check_tickets « 1 refus », la minute où le ticket d'archi
vivait en deux copies (fermée par 863b81b46) ; les runs d'après annulés par les pushs successifs ; celui de
a12f08bdf2 en cours dira le vert. Leçon : un lot qui ferme un ticket par `git mv` se repose sur une base
qui porte la copie ouverte, ou retire la copie dans le même push ; une minute à deux copies rougit la CI.
Embarquement, 23 h 1x : dense-sur-tenue-pile ac8644784 sur 77fccf493 (`=` sur le test et le remaniement, `!`
sur le commit de docs seules : la page des règles avait déjà le paragraphe du vrai jeu sur master) ; passe
courte à luciepc sous MOTEUR_ANCIEN=1. ED a sa base (ac8644784). Le dual du creux (dual-du-creux, 26da6dafb
rouge, 7d3cb5935 vert : activation 23, chargement, e2e du groupe 2/2) prend le rang 30 ter après arbre
principal. Ordre : 96 → embarquement → ED → arbre principal → embarquement (dual) → G chaîne Rust.
Codeparsers, 23 h 1x : le lot 1 vert (renommages-python-2 b45530b2a : pointeur 6ba8478 → 85fe233 avec
1 094 distributions de pipreqs + 13 du témoin croisé ; `workspace.imports` par langue, `replace`,
`distributions`, refus nommé d'une langue inconnue ; 13/13) prend le rang 30 quater, base = la tête du dual.
**La fin de la pile est fixée** : … → ED (30) → arbre principal (30 bis) → dual du creux (30 ter) →
renommages Python (30 quater) → G chaîne Rust 1.98.1 ; tout lot neuf passe derrière. Un faux rouge chez
codeparsers : cargo a repris l'artefact du sous-module bâti dans un autre worktree (dates plus vieilles) ;
sans check_features, RAG3WEAVER_TARGET_REPRENDRE n'agit pas — question posée à G pour la garde du target.
Banc, 23 h 1x (page 02 § 4, eef0a6e90) : pas de défaut de version derrière les comptes des orphelins — forme
Versions sans générations, la fin d'instantané efface les lignes que la source n'a plus ; seul écart : le
document de plein texte de File (7 608 pour 7 607), ce que vise le lot. Rejeu sous générations au vert de
recherche ; Symbol à compter par sa relation au File.
G, 23 h 1x : la garde du target ne note que l'arbre, pas le commit des sous-modules ; cargo reprend les
artefacts d'un sous-module changé dont le checkout porte des dates plus vieilles ; cargo_magasin.sh seul ne
passe pas par la garde. Correction validée (commit des sous-modules dans `.arbre-du-target`, effacement
d'elle-même des empreintes, cargo_magasin.sh appelle la garde, rouge d'abord par un sous-module ramené en
arrière) : second push du tour de la chaîne Rust. Codeparsers : lot 1 à deux commits (57e2c1847), page 03
avec la reprise ; commence les manifestes .NET à part.
Recherche, 23 h 1x : la vraie cause des orphelins — le drain ne pose aucun FlushNode après DeleteRecordNode ;
`delete_by_node_id` n'est vu qu'au commit de la poignée lucivy, une fin d'instantané qui ne fait que
retirer laisse le document vivant puis le perd à la fermeture. Correctif : un FlushNode `deletes_flush` sur
les tables effacées + l'offset d'une ligne inconnue du cache lu par select_node_ids ; deux témoins (avec et
sans redémarrage) rejoués à luciepc. Rang : derrière la chaîne Rust de G.

**23 h 1x — ED a poussé sa pile** : vue-publiee-pile 3a6bf34cd sur ac8644784 (tête réelle 895d5a5c9, un
docs de plus que 2bdb8a6e4). Deux conflits dans `occurrence_mask_options` contre le court-circuit d'arbre
principal, résolus : sans filtre ni vue ni orpheline le court-circuit (Admission::Toutes) ; sinon le
prédicat (Admission::Parmi) ; la liste en repli compté. **Sens changé** : sans filtre avec des orphelines,
le prédicat sert au lieu de la liste (sa jointure les exclut). range-diff 25 `=`, 2 `!` de code, 1 `<`
absorbé. Passe courte sous le lourd depuis 23 h 11. Règle : deux « ! » de code et un sens changé → la parité
de banc se rejoue sur la tête de pile (3a6bf34cd contre ac8644784, + un cas orphelines sans filtre) ; le
« à toi » attend la passe et la parité.
ED, 23 h 1x : tête de pile 23101838b (un commit de docs, page 05 § 9, au-dessus de 3a6bf34cd testé) ;
PILE_TESTE=3a6bf34cd au push ; arbre principal se posera sur 23101838b.
Banc, 23 h 1x : la parité de la pile d'ED en file à luciepc (3a6bf34cd contre ac8644784, 30 cas ; puis une
copie à versions orphelines fabriquées — occurrences des versions des trois premiers rangs effacées,
compte des versions sans origine relevé —, RAG3WEAVER_PARITE_ORPHELINES, 64bf68daf0) ; puis les orphelins
de recherche (ff4ced00a contre 9d90edbb6).
G, 23 h 1x : garde-sous-module dba057156 prêt (le commit de chaque sous-module noté dans
`<target>/.sous-modules-du-target`, empreintes des paquets du sous-module changé effacées d'office,
`garde_target.sh --reprendre <target> <arbre>` ; rouge 3 ÉCHEC sur la garde de master, 21 OK ; check_features
8/8) : second push du tour de la chaîne Rust. Correction : cargo_magasin.sh appelait déjà la garde (par
cargo_garde.py), le contournement de codeparsers était un `cargo test` direct (ticket 18ad5cef3). À la pose,
la première passe rebâtit codeparsers une fois par target : voulu.

**23 h 2x — la passe des suivies : rouge d'environnement à e2e_estimate** (arbre principal, 2e41a4a11) : vert
sur check_features, lib entière, e2e_lectures, e2e_code, e2e_demon_hote, e2e_etat_non_declare ;
`ce_depot_s_estime_avant_de_s_indexer` panique dans l'aide des tests sous RAG3WEAVER_EMBED_SERVICE : aucun
service ne sert granite-107m à luciepc (7880 arrêté depuis le 12). Rejeu seul sans la variable, dit au
journal, + les quatre suites restantes ; verdict 23 h 45. Ticket à embarquement : l'aide se replie sur le
chargement sur place pour un modèle sans service, au lieu de paniquer (pas de SAUTÉ).
Embarquement, 23 h 2x : passe courte verte sur ac8644784 (activation/service 0 rouge, les deux e2e, lib
2302/0, check_features 8/8, navigateur 45/0) ; dernier pas prêt, attend le « poussé » de 96. Le ticket de
l'aide des tests : le repli charge sur place sur le périphérique permis (CPU si rien n'est dit), jamais sur
la carte que la variable épargne (card0 de Lucie) ; interdit → rouge nommé, jamais sauté.
Archi, 23 h 2x : l'équipier rend le contenu (equipier-rend-le-contenu 47b1955de : report ou la réponse
porte le texte lui-même, le texte d'un pas fini par step_done va dans la boîte sauf si un report le porte,
sans heuristique ; lib 2403/0, e2e_hooks 2+2, navigateur 45/45, cas Gemini 1/1 — vert aussi une fois sans
correctif, le rouge sûr est le faux modèle ; ticket de tty fermé). Place : derrière la chaîne Rust et les
orphelins. Banc : dix tirages Gemini avec et sans, pour un taux.

**23 h 2x — la passe d'ED : rouge d'environnement à e2e_postgres** (7/36 : « referenced schema was
concurrently dropped », `_dataflow_node_state` absente) — une autre passe jouait e2e_postgres sur le même
schéma `rag3weaver_test` ; le lot ne touche pas au dialecte postgres. Vert partout ailleurs (lib 2 413/0,
e2e_code 36, e2e_code_sync 19, e2e_scope_versions 8, e2e_lectures 27). Rejeu seul + e2e_masque_sans_filtre
(demande d'arbre). Ticket à G : un schéma postgres par passe, pas un verrou.
G, 23 h 2x : le schéma `rag3weaver` est fixé par le produit (dialect.rs), `rag3weaver_test` est la base ;
correctif validé : **une base par passe** créée par run_e2e.sh (`rag3weaver_test_<worktree>_<pid>` sur 5433,
RAG3WEAVER_PG exporté, effacée au trap, un RAG3WEAVER_PG explicite respecté), ménage des bases orphelines au
départ ; rouge d'abord par deux e2e_postgres en parallèle. Lot derrière les deux pushs de G.
Codeparsers, 23 h 2x : .NET vert (manifestes-dotnet 3474cea : plus long préfixe déclaré pour C# ; rag3db :
.csproj, packages.config, Directory.Packages.props sous `csharp`, 13/13) ; défaut trouvé : un `using` C# ne
produit aucune bibliothèque (extraction JavaScript, comme Go). Décision : voies 1 + 3 (une bibliothèque par
using sans symbole deviné ; les alias et `using static` lient leur nom), dans le même lot que .NET, derrière
le 30 quater et les lots de G.
23 h 27 : **la CI rag3weaver est verte sur a12f08bdf2** (la tête de codeparsers) : le rouge de 23 h (deux
copies du ticket d'archi pendant une minute) est fermé ; les runs intermédiaires annulés par les pushs.

**23 h 3x — la parité de la pile d'ED est verte** (3a6bf34cd contre ac8644784, 30/30 au bit près, parent =
ligne de base migree.json ; le parent lit la forme Versions, le lot les générations, mêmes réponses) ; le cas
des orphelines finit (~3 min), puis banc coupe sa file pour laisser une place à 96 et relance les orphelins
de recherche au « poussé ». ED (page 06 § 10, fac67a208) sur le Symbol sans relation : pas de fermeture à la
publication — un Symbol est visible à un rang s'il a une relation visible à ce rang, l'effacement physique
revient à la rétention. 96 : vert partout sauf quatre suites courtes en attente d'une place (quatre lourds
tenus à luciepc : p2c.sh, chaine-23h.sh, wt-suivies-reste.sh, run_e2e e2e_code du verdict de G).
Codeparsers, 23 h 3x : .NET complet (e6c06581e sur 57e2c1847 : .csproj/packages.config/Directory.Packages.props,
pointeur 85fe233 → 3474cea → 4194456 ; chaque `using_directive` porté par le plus petit scope, alias lié,
`using static` en voie 1 ; témoin de bout en bout Newtonsoft.Json.Linq dependency, System.Text.Json builtin,
Mon.Outil.Interne project ; 14/14, check_tickets VERT 515). File derrière G : chaîne Rust + garde-sous-module →
orphelins de recherche → équipier d'archi → .NET → (base par passe de G).
ED, 23 h 3x : passe courte verte sur 23101838b (lib 2 413/0, e2e_code 36, e2e_code_sync 19,
e2e_scope_versions 8, e2e_lectures 27, e2e_postgres rouge à la passe par le schéma partagé puis 36/36 rejoué
seul, e2e_masque_sans_filtre 2/2 avec les orphelines) ; journaux ed-pile-vue-journaux ; attend le « poussé »
d'embarquement. Arbre principal a sa base : 23101838b.

**23 h 34 — trente-quatre atterris** : la trace d'archive (96, d234d4d18, « = », pile tenue ; rejeu complet
vert dont e2e_postgres 36/36). Banc : la pile d'ED aussi verte sur la copie à orphelines (30/30). Embarquement
a le « à toi » (PILE_AVANT=77fccf493) ; puis ED (PILE_AVANT=ac8644784), arbre principal, dual, renommages
Python, G ×2, orphelins, équipier, .NET, base par passe. Hotfixs, le cœur et banc relancent leurs lourds.

**23 h 37 — trente-cinq atterris** : le dense porté sur `Tenue<C>` (embarquement, e14b32ad1 : test 6da82374d,
remaniement 7846abddb en « = », docs « ! » tolérée). Le premier dernier pas avait buté au rebase sur sa page
des règles (la conception du dual poussée hors file plus tôt) : repose à la main sur dense-sur-tenue-pile-2,
PILE_TESTE=ac8644784, « pile tenue ». **Leçon** : pendant qu'un lot est en pile, les docs de sa page restent
dans le lot ou attendent son atterrissage. ED a le « à toi » (PILE_AVANT=ac8644784) ; puis arbre principal,
dual, renommages Python, G ×2, orphelins, équipier, .NET, base par passe.

**23 h 4x — trente-six atterris** : le masque des versions en prédicat, la vue publiée par (couche, rang)
(ED, e89cdb8e6 : 28 commits, tout « = », tête testée 23101838b, git diff du code vide). Arbre principal suit
à son vert (PILE_AVANT=23101838b) ; puis le dual (embarquement), les renommages Python (codeparsers), la
chaîne Rust + garde-sous-module (G), les orphelins (recherche), l'équipier (archi), .NET (codeparsers), la base
par passe (G), la troncature d'un sens (cœur).
Embarquement, 23 h 4x : dual-du-creux-pile 870bb4a5e sur arbre-principal/suivies-pile 8d9c15310 (« = » test
et code, un commit de docs dans le lot : ticket du dual fermé, page des règles, README) ; passe entière à
luciepc. Codeparsers se pose dessus.
Codeparsers, 23 h 4x : renommages-python-3 4a10a5939 sur 870bb4a5e (deux commits, lock inchangé,
check_tickets VERT), passe complète en file. G a sa base (4a10a5939) pour la chaîne Rust puis
garde-sous-module ; recherche se posera sur la tête du second.

**23 h 43 — trente-sept atterris** : les sources suivies (arbre principal, 844d01247, 19/19 en « = », tête
testée 8d9c15310 ; rejeu court check_features 8/8, daemon_host::taches 11/11, suivies:: 14/14, e2e_lectures
27/27, e2e_demon_hote 14/14 ; passe complète sur 2e41a4a11 avec e2e_estimate 5/5 rejoué sans service).
Embarquement a le « à toi » à son vert (dual, PILE_AVANT=8d9c15310) ; puis codeparsers (PILE_AVANT=870bb4a5e),
G ×2 (sur 4a10a5939), recherche, archi, .NET, base par passe, cœur.
Codeparsers, 23 h 4x : controle-du-worktree.sh refuse « 275 commits de retard (plafond 200) » sur une base de
pile non atterrie — il compte en hashes, les lots atterris en portent d'autres ; par `--cherry-pick
--right-only` il en reste 133. RETARD_MAX=300 pour cette passe, dit au journal ; ticket pour G : compter avec
--cherry-pick en mode pile.
G, 23 h 4x : chaine-rust-pile b1990e6e2 sur 4a10a5939 (rustc 1.98.1 par rust-toolchain.toml) et
garde-sous-module-pile 8a2b85f81 dessus, « = » ; rejeux au portable dans un target neuf (1.98.1 rebâtit tout,
40 à 60 min), MOTEUR_ANCIEN dit ; la tête du second va à recherche au vert. Le verdict de fac67a208 pour
l'hôte de luciepc tourne.
G, 23 h 4x : controle-du-worktree.sh corrigé (retard compté par contenu, `git log --cherry-pick --right-only
--no-merges HEAD...origin/master | wc -l` ; témoin sur dépôt jouet, ancien 4 deux ROUGE, neuf 3 deux vert ;
0cc9bbf67) ; recopie au cache des deux postes au mot de c7. Le script n'était dans aucun dépôt : demandé la
liste des scripts du cache sans dépôt, à faire entrer dans tools/poste/ un par un, en pile.

**23 h 5x — G, trois lots verts et prêts, un seul tour** : chaine-rust-pile b1990e6e2 (check_features 8/8 sous
rustc 1.98.1 ; PILE_AVANT=4a10a5939) → garde-sous-module-pile 8a2b85f81 (test_garde_target 21 OK, 8/8 ;
PILE_AVANT=b1990e6e2) → passe-epinglee 3f159b542 (passe_epinglee.sh entre dans tools/poste/ depuis la copie
de luciepc, sans 7880 ni 7981 ; témoin 16 vert, 3 ROUGE sur la copie du portable, 2 sur celle de luciepc ;
PILE_AVANT=8a2b85f81 ; push-d-un-lot.md le cite par son chemin). La base de recherche (orphelins) est
3f159b542. Ordre restant : dual (embarquement) → renommages (codeparsers) → G ×3 → orphelins → équipier →
.NET → base par passe → cœur → `cited` par défaut (arbre).
G, 23 h 5x : tête du troisième lot 0416455b5 (un .md par-dessus 3f159b542 : push-d-un-lot.md cite
tools/poste/passe_epinglee.sh) ; base donnée à recherche : 0416455b5.
Codeparsers, 23 h 59 : le 30 quater vert (4a10a5939 : check_features 8/8, lib 2 433/0, e2e_code 55/55,
e2e_postgres 36/36, MOTEUR_ANCIEN et RETARD_MAX=300 dits) ; dernier pas prêt, attend le « poussé » du dual.
Alias de résolution côté codeparsers verts (7c2c5aa, 322) ; côté rag3weaver : paths de tsconfig.json et
`workspace.imports.<langue>.local_prefixes`.
Banc, 00 h 0x : les orphelins de recherche (ff4ced00a contre 9d90edbb6) verts — l'écart File tombe à 0 au lot
après le retrait (1 au parent), les 203 d'avant aussi, 30 cas au bit près (page 02 § 4, 87f764243).
Recherche se posera sur 0416455b5 (G) à son vert.
Arbre principal, 00 h 0x : le défaut `cited` d'une source `refs:` fait (refs-cited dac27a8e2 : rouge
122f94a35 sur cinq sources, vert c8d06859c, suivies:: 14/14, check_features 8/8 ; nouveautés des suivies
ajoutées, page 02 d'orchestration touchée — c7 n'y écrit plus avant l'atterrissage). Place : derrière la base
par passe de G ; la troncature d'un sens du cœur après. Fin de file : dual → renommages → G ×3 → orphelins →
équipier → .NET → base par passe → `cited` → cœur.
Codeparsers, 00 h 0x : les alias de résolution verts (alias-resolution 9857eac19 sur .NET e6c06581e : pointeur
→ 7c2c5aa, `#` préfixe local JS, tsconfig*/jsconfig lus avec commentaires, `@app/*` → préfixe,
`workspace.imports.<langue>.local_prefixes` ; 16/16). Trois lots en file pour lui : 30 quater (prêt), puis
.NET + alias dans un tour. Pour archi : parent_name existe ; manquent le nom qualifié, impl → struct (Rust,
en premier), définition ↔ déclaration (C++), receiver (Go).
Recherche, 00 h 0x : orphelins-pile-3 0cf758890 sur 0416455b5 ; rejeu court à luciepc (rustc 1.98.1 rebâtit
une fois) ; tête à archi au check_features vert. Deux faux rouges ticketés sur ff4ced00a (postgres partagé ;
le témoin navigateur du chat d'une tâche sous charge, vert 3/3 seul) ; un « descendants laissés après la
commande » du confineur sur un e2e_code 36/36.

**00 h 08 — trente-huit atterris** : le dual d'un creux à la demande (embarquement, 92b7a09fa : test
922339c68, code 3bee9e5e3 en « = », docs « ! » tolérée ; passe entière verte ; target-activation à 34 Go
effacé plutôt que le plafond relevé, e2e du groupe rejoué 2/2 ; repose à la main sur dual-du-creux-pile-2 après
un conflit sur sa page des règles, PILE_TESTE=870bb4a5e). Codeparsers a le « à toi » (PILE_AVANT=870bb4a5e) ;
puis G ×3 (PILE_AVANT=4a10a5939, b1990e6e2, 8a2b85f81), recherche (0cf758890), archi, codeparsers ×2, G base
par passe, arbre `cited`, cœur, embarquement repli de l'aide.

**00 h 10 — trente-neuf atterris** : les renommages Python et `workspace.imports` (codeparsers, e519cfd69,
2/2 en « = », pointeur 85fe233). G a le « à toi » pour ses trois (chaîne Rust 1.98.1, garde des sous-modules,
passe_epinglee.sh dans tools/poste/) ; puis recherche (0cf758890 sur 0416455b5), archi (équipier),
codeparsers (.NET + alias), G (base par passe), arbre (`cited`), cœur (troncature), embarquement (repli).
Codeparsers : le récepteur Go générique vert (appartenance-des-methodes c6216ea, 324 passés) ; puis C++, puis
Rust (A).

**00 h 13 — quarante-deux atterris** : les trois de G — la chaîne Rust 1.98.1 (b09ce1760), la garde des
sous-modules (ac8ef511f : un premier push refusé par des docs de lucivy entre fetch et push, la garde a remis
le worktree, le même appel est reparti), passe_epinglee.sh dans tools/poste/ (727d6336e + 2146956e6 : rebase de
la garde buté sur push-d-un-lot.md, reposé à la main, **premier emploi de PILE_TESTE**). passe_epinglee.sh
recopié au cache des deux postes (5ac96c073b0df69f, 16 vert). Recherche a le « à toi » (PILE_AVANT=0416455b5)
à son vert ; puis archi (équipier, non reproduit par le banc : le prochain essai de Lucie en tty est le juge),
codeparsers ×2, G base par passe, arbre `cited`, cœur, embarquement repli.
Archi, 00 h 1x : equipier-rend-le-contenu-pile 55679c2d6 sur 0cf758890, rejeu court en cours ; le retrait de la
copie ouverte du ticket de tty prêt pour la minute d'après. Codeparsers se pose dessus.
Codeparsers, 00 h 2x : .NET (542ab4656) et alias (270b6284d) reposés sur 55679c2d6 ; un seul dernier pas sur
270b6284d (la passe testée est celle de la tête), PILE_AVANT=55679c2d6 ; passe complète en cours ; le
contrôle du worktree recopié compte 153 de retard réel, sans RETARD_MAX. G (base par passe) se posera sur
270b6284d. Appartenance : parent_path et le nom court d'un bloc impl verts côté codeparsers (5885e13, 327).

**00 h 23 — quarante-trois atterris** : les orphelins du plein texte (recherche, 262be6fc4, 4/4 en « = » :
`deletes_flush` après DeleteRecordNode, l'offset d'une ligne inconnue du cache lu en base ; rejeu complet vert,
lib 2 435). Archi a le « à toi » à son vert (PILE_AVANT=0cf758890) ; puis codeparsers (PILE_AVANT=55679c2d6),
G base par passe (sur 270b6284d), arbre `cited`, cœur, embarquement. La sonde d'ED par recherche sous `poste
mesure` attend la pile vide.
Le cœur, 00 h 3x : deux-sens-sous-annulation 6fddb1f5c (crochet rel-commit-between-directions + deux témoins ;
rouge 2/2 sur f41c0ba5b, vert 3/3 sur master ; transaction_test entier en cours), derrière `cited`, devant le
repli d'embarquement. Fin de file : archi → codeparsers ×2 → G base par passe → arbre `cited` → cœur →
embarquement repli → G igpu:0 dans passe_epinglee → lucivy maintenance (au tag).
Codeparsers, 00 h 3x : .NET + alias verts sur 270b6284d (check_features 8/8, lib 2 441/0, e2e_code 55/55,
e2e_postgres 36/36) ; dernier pas prêt au « poussé » d'archi. Appartenance côté rag3weaver écrite selon (b) ;
le genre `method` d'un autre fichier ne se réécrit pas hors de l'écrivain d'ED (ticket pour ED et archi).

**00 h 34 — quarante-quatre atterris** : l'équipier rend le contenu demandé (archi, b1a21a176, 4/4 en « = » ;
e2e_equipier d'abord sautée sans les identifiants Vertex, rejouée avec — un saut n'est pas vert ; copie du
ticket de tty retirée dans la minute, check_tickets VERT 523). Codeparsers a le « à toi » (.NET + alias,
PILE_AVANT=55679c2d6) ; puis G (base par passe), arbre (`cited`), cœur (deux sens), embarquement (repli),
G (igpu:0), lucivy (maintenance, au tag).

**00 h 35 — quarante-cinq atterris** : les manifestes .NET, les `using` C# et les alias de résolution
(codeparsers, 75492be97a, 3/3 en « = », pointeur 7c2c5aa). G a le « à toi » à son vert (base par passe,
PILE_AVANT=270b6284d) ; puis arbre (`cited`), cœur (deux sens), embarquement (repli), G (igpu:0), lucivy
(maintenance, au tag), codeparsers (appartenance : parent_path, extends_type, 62/62).
G, 00 h 39 : pg-base-par-passe-pile 7950c4789 sur 270b6284d, check_features 8/8 ; preuve rouge (deux
e2e_postgres en parallèle sur master) puis verte (sur la tête de pile) en cours à luciepc. Arbre principal a sa
base (7950c4789).
Arbre principal, 00 h 4x : refs-cited-pile-2 25d888148 sur 7950c4789 ; sa page 02 retirée du lot (la base de
pile ne porte pas le commit qui l'a créée : « supprimé dans HEAD » au rebase), sa correction poussée seule
(5f3bc7dc6) ; le lot ne touche plus que nouveautes.md et la page 02 d'orchestration (c7 n'y écrit pas).
Règle complétée : les docs d'une page restent dans le lot, sauf quand la base de pile n'a pas la page — à dire
avant de pousser. Une branche refs-cited-pile poussée pendant un rebase en conflit reste en place sans servir.
Le cœur, 00 h 4x : deux-sens-sous-annulation-pile c133ff976 sur 25d888148 (7 commits) ; sa file réordonnée au
mot de c7 : le rejeu de la pile d'abord, le transaction_test sur l'ancienne tête ne sert plus, la base à un
seul sens pour G après le push. Embarquement se pose sur c133ff976.
Embarquement, 00 h 4x : repli-de-l-aide-pile 03e89ca18 sur c133ff976 (« = » témoins et code, « ! » docs : le
ticket d'arbre n'est pas dans la base de pile) ; rejeu court à luciepc ; la copie ouverte du ticket à retirer
dans la minute du push (troisième fois ce soir : un lot qui ferme un ticket posé après la base de pile).
G, 00 h 4x : passe-ep-igpu 275ea1fe6 sur 03e89ca18 (igpu:0 posé dans la seule branche luciepc de
passe_epinglee.sh, ôté au portable même si l'appelant le pose ; 20 vert, 2 ROUGE sur la base). Push au
« poussé » d'embarquement, PILE_AVANT=03e89ca18, puis recopie au cache. Fin de file : G base par passe (en
preuve) → arbre `cited` → cœur deux sens → embarquement repli → G igpu → lucivy maintenance (au tag) →
codeparsers appartenance → G garde des tickets déplacés.
Arbre principal, 00 h 4x : refs-cited-pile-2 25d888148 vert (check_features 8/8, taches 11/11, suivies 14/14) ;
push au « poussé » de G, PILE_AVANT=7950c4789. Archi : le ticket de 96 (un backend fermé se rouvrait au
premier appel) fermé par la fermeture qui tient (6ac4750fb) ; prochain lot : une écriture refusée sous
fermeture se range en file bornée et se rejoue à l'ouverture.
G, 00 h 5x : garde-tickets 06bffb23c sur 275ea1fe6 (au dernier pas, les fichiers créés sous
docs/tickets/closed/ dont un homonyme exact vit encore sous docs/*/tickets/ sont retirés par un commit de docs
dans le même push, dit au journal ; rouge sur la garde d'avant, 34 vert). PILE_AVANT=275ea1fe6, derrière igpu.
Lucivy (maintenance) et codeparsers (appartenance) se poseront sur 06bffb23c.
Codeparsers, 00 h 5x : l'appartenance verte (94a580b59 sur master : check_features 8/8, lib 2 443/0, e2e_code
55/55, e2e_postgres 36/36) ; passe devant lucivy (tag non publié), repose sur 06bffb23c ; lucivy se posera sur
sa tête. Fin de file : G base par passe → arbre → cœur → embarquement → G igpu → G garde-tickets →
codeparsers appartenance → lucivy maintenance (au tag).
G, 00 h 55 : la preuve rouge de la base par passe ne valait pas (target-G/pile vide à luciepc, les deux
run_e2e.sh partis sur un target neuf : le second refusé « target non vide sans provenance », le premier a
bâti seul) ; corrigée (bâti du binaire e2e avant les deux passes). Décision : push au vert de la paire verte —
le rouge réel est aux tickets d'ED (7/36) et de recherche (9/36) avec leurs traces —, la paire rouge rejouée
ensuite pour la page.
Codeparsers, 00 h 5x : appartenance-des-methodes-3 8ef45a3af sur 06bffb23c (pointeur 7c2c5aa → 5885e13),
rejeu court en file ; lucivy se posera sur 8ef45a3af.

**00 h 57 — quarante-six atterris** : la base postgres par passe (G, 552299bec, « = » : chaque run_e2e.sh crée
`rag3weaver_test_<poste>_<pid>` sur 5433, l'efface au trap, ménage des orphelines ; paire verte 39/39 ×2 en
parallèle, aucune base restante ; le rouge d'abord est celui d'ED 7/36 et de recherche 9/36, cités au résumé).
Arbre principal a le « à toi » (`cited`, PILE_AVANT=7950c4789) ; puis cœur (PILE_AVANT=25d888148),
embarquement (c133ff976), G igpu (03e89ca18), G garde-tickets (275ea1fe6), codeparsers (06bffb23c), lucivy
(8ef45a3af).

**00 h 58 — quarante-sept atterris** : le défaut `cited` d'une source `refs:` (arbre principal, 4eb274033, 3/3
en « = », nouveautés des suivies comprises). Le cœur a le « à toi » à son vert (deux sens sous annulation,
PILE_AVANT=25d888148) ; puis embarquement (c133ff976), G igpu (03e89ca18), G garde-tickets (275ea1fe6),
codeparsers (06bffb23c), lucivy (8ef45a3af).
Archi, 01 h 1x : la file des écritures sous fermeture verte (file-des-ecritures 7bdeb8541 : écriture = appel
que reads_only ne prouve pas en lecture ; refus rangé avec rang, raison, heures ; fermeture et file sur disque ;
rejeu FIFO à /backend/open, âge 24 h, borne 1 000, /etat write_queue ; e2e_demon_hote 17/0, e2e_lectures 27/0,
lib 2 441/0). Place : derrière lucivy, base = sa tête.

**01 h 14 — quarante-huit atterris** : les deux sens sous annulation (le cœur, e8472e053, 7/7 en « = » : le
point de faute rel-commit-between-directions avec un délai, les deux témoins RefusedCommitRelDirections ;
transaction_test 378 vert, C++ seul). Embarquement a le « à toi » (repli de l'aide, PILE_AVANT=c133ff976) ;
puis G igpu (03e89ca18), G garde-tickets (275ea1fe6), codeparsers (06bffb23c), lucivy (8ef45a3af), archi file
des écritures (sur lucivy), ED écrivain (sur archi).
Lucivy, 01 h 1x : lucivy-maintenance-hote-2 c3b2e4dba sur 8ef45a3af (fts_maintenance 2/2, check_features 8/8,
e2e_demon_hote 19/19 ; 4.3.8 dans Cargo.toml et le lock) ; archi a sa base (c3b2e4dba).

**01 h 15 — quarante-neuf atterris** : le repli de l'aide des tests (embarquement, 8cba8638e, 4/4 en « = »,
docs comprises ; copie ouverte du ticket retirée 27 s après en f315a1563, check_tickets VERT 528). G a le
« à toi » pour igpu (PILE_AVANT=03e89ca18) puis garde-tickets (PILE_AVANT=275ea1fe6) ; puis codeparsers
(06bffb23c), lucivy (8ef45a3af), archi (c3b2e4dba), ED (sur archi). L'hôte 7970 reposé à 1 h 15 ;
code-rag3db-depot fermé par hotfixs de 1 h 18 à ~1 h 30 (purge).

**01 h 18 — cinquante et un atterris** : igpu:0 dans passe_epinglee.sh (G, cf3fdd6a6) et la garde qui retire
la copie ouverte d'un ticket fermé (G, 2fb7fdcec) ; les deux scripts recopiés au cache des deux postes
(passe_epinglee.sh be664721bbceaff7, push_garde.sh cb7331214b34d9c3, témoins 20 et 34 verts). Codeparsers a le
« à toi » (appartenance, PILE_AVANT=06bffb23c) ; puis lucivy (8ef45a3af), archi (c3b2e4dba), ED (sur archi),
recherche BEGIN (sur ED).

**01 h 19 — cinquante-deux atterris** : l'appartenance d'une méthode hors du bloc de son type (codeparsers,
f257b531b, « = » : parent_path, extends_type, le genre method au rendez-vous). Lucivy a le « à toi »
(maintenance par l'hôte avec lucivy 4.3.8, PILE_AVANT=8ef45a3af) ; puis archi (c3b2e4dba), ED (sur archi),
recherche BEGIN (sur ED). Rappel écrit : le « à toi » ne se relaie pas (G l'a relayé à codeparsers ; rien de
cassé, un seul mot d'une seule bouche).
96, 01 h 2x : les ForEachNode imbriqués verts (lot-96-foreach-imbrique : champs_manquants saute la clé graph
d'un GraphNode/ForEachNode, le ForEachNode rend une sortie list ; deux rouges du ticket mot pour mot ; 40/40,
e2e_archiviste 15/15 ; ticket clos). Place : derrière le BEGIN de recherche. Le plafond du Decide (4 096
déclaré dans le graphe) rejoint le lot s'il est vert avant la base.
Archi, 01 h 21 : file-des-ecritures-2 c6464e06a sur c3b2e4dba (« = », la conception reprise avec le lot), rejeu
court vert (e2e_demon_hote 20/0, e2e_lectures 27/0). ED a sa base (c6464e06a).

**01 h 22 — cinquante-trois atterris** : la maintenance du plein texte par l'hôte avec lucivy 4.3.8 (lucivy,
e95a1b8c6, 3/3 en « = » ; résumé écrit par la passe elle-même). Archi a le « à toi » (file des écritures,
PILE_AVANT=c3b2e4dba) ; puis ED (c6464e06a), recherche BEGIN (sur ED), 96 foreach (sur recherche).

**01 h 2x — archi refusé au range-diff** : la conception (ec69e7be7 = 8c3f766b1) reprise dans le lot parce que
la base de pile n'avait pas le document, mais déjà sur master : le rebase la laisse tomber, « < », la garde
refuse (worktree remis, master intact). Reprise : file-des-ecritures-3 = test + fix sur e95a1b8c6 (la tête de
lucivy atterrie), PILE_AVANT=e95a1b8c6 ; ED repart de la nouvelle tête. Ticket pour G : tolérer un « < » dont
`git cherry` dit le patch déjà sur master. **Hotfixs, 01 h 22** : code-rag3db-depot purgé et rouvert (53 s
fermé) — 10 671 Scope et 144 File orphelins, .fts 91 852 → 81 181 = les Scope du graphe, Lucivy_Scope
4,93 → 1,35 Go ; le défaut produit encore 970 orphelins par douze heures (le correctif de recherche ferme la
source) ; un faux rouge du banc corrigé (la clé ne lit plus que les lignes 📍) ; instantané
avant-fenetre-code-rag3db-depot-20261115-0121. Fin de sa fenêtre.
Codeparsers, 01 h 3x : go.work + C/C++ verts (go-work-2 1fe0a9582 : use notés et replace suivis ; conanfile
.txt/.py ; find_package et FetchContent_Declare ; 65/65 ; deux tickets vcpkg) ; place derrière le foreach de
96, sans e2e_postgres (code.rs intact). Fin de file : archi (sur e95a1b8c6) → ED → recherche BEGIN → 96 foreach
→ codeparsers go.work. Ensuite pour codeparsers : setup.py/setup.cfg, Pipfile, PEP 735, bundledDependencies.
G, 01 h 2x : garde-deja-sur-master b3a540346 (un « < » que `git cherry` dit déjà sur master est toléré et dit,
sinon arrêt nommé ; 2 ROUGE sur la garde d'avant, 36 vert) : derrière go.work de codeparsers. Le cœur fabrique
le miroir (relation au sens avant, absente du sens arrière) pour le témoin de check_rel.
96, 01 h 2x : le plafond du Decide (4 096 déclaré dans archive_decide.mmd, rouge d'abord, e2e_archiviste 16/16)
rejoint le lot foreach (trois commits, deux tickets clos) ; attend la tête de recherche.

**01 h 28 — cinquante-quatre atterris** : la file des écritures sous une fermeture qui tient (archi,
95c643a20, 2/2 en « = » ; reste du ticket de 96 fermé). Recherche (BEGIN, 9/9 rouge puis vert) passe devant
l'écrivain d'ED dont les suites tournent : repose sur 1f39da047, « à toi » à son vert (PILE_AVANT=1f39da047) ;
puis ED (sur recherche), 96 foreach (sur ED), codeparsers go.work (sur 96), G garde-deja-sur-master (sur
codeparsers).
Recherche, 01 h 3x : begin-erreur-d-origine-pile 4595fcdb5e sur 1f39da047, rejeu court en cours. 96 se pose
dessus et passe devant ED ; ED sur 96 ; codeparsers sur ED ; G sur codeparsers.
Codeparsers, 01 h 3x : les manifestes Python manquants verts (python-manifestes sur go-work-2 : setup.cfg,
setup.py littéral, Pipfile, PEP 735 dependency-groups, bundledDependencies ; 20/20) ; les deux lots passent
ensemble derrière ED (une passe sur la tête, un dernier pas), sans e2e_postgres dit au journal.
96, 01 h 3x : la pile de recherche (4595fcdb5e) est **rouge au jeu sans feature** (« unused import:
AtomicBool », read_pool.rs:34, -D warnings) ; le lot de 96 ne touche pas read_pool.rs ; 96 n'a pas donné sa
tête. Recherche corrige par un commit ajouté et rejoue check_features 8 jeux avant de redonner sa tête
(troisième fois ce soir qu'un maillon donne sa tête sans check_features : la règle 6 tient).
Recherche, 01 h 4x : tête corrigée 766e5bff2 (AtomicBool sous cfg(test), commit ajouté) ; passe entière dessus.
96 : lot-96-foreach-imbrique-pile-2 1bc2b8488 sur 766e5bff2, check_features 8/8, tête donnée à ED ; la pile
sur la tête rouge abandonnée (reste en place sans servir).

**01 h 5x — l'écrivain d'ED passe dernier** : ses quatre suites vertes sur e58d13c91 (36, 27, 14, 36) après sept
défauts de son lot trouvés et corrigés (le miroir CONSUMED_BY, scopes_deleted, l'empreinte par la clé nue, deux
aides d'e2e_code au rang lu, une constante et un champ sans feature, l'arrêt entre paquets de la
synchronisation — 89 s d'attente d'une fermeture) ; tête 3094b695c en passe courte, banc juge depuis 1 h 22 ;
tête de pile vers 2 h 30. Codeparsers (go.work + Python) passe devant, sur 96 (1bc2b8488) ; G sur codeparsers ;
ED sur G. Ordre final : recherche → 96 → codeparsers → G → ED.

**01 h 54 — cinquante-cinq atterris** : le BEGIN de read_pool (recherche, 68b1454d8, 4/4 en « = » : Phase::Rompu
porte la cause d'origine et a_rouvrir, une base à rouvrir ne recommence pas ; AtomicBool sous cfg(test) ;
passe entière verte, lib 2 447). 96 a le « à toi » à son vert (PILE_AVANT=766e5bff2) ; puis codeparsers
(1bc2b8488), G (sur codeparsers), ED (sur G).
Codeparsers, 01 h 5x : python-manifestes-2 ffe6efee8 sur 1bc2b8488 (deux commits : go.work/conan/CMake, puis
setup/Pipfile/PEP 735/bundled) ; passe en file ; tête à G au check_features vert.

**02 h 01 — cinquante-huit atterris** : les ForEachNode imbriqués, leur ticket et le plafond déclaré du
Decide (96, 0130904a5, 3/3 en « = »). Codeparsers a le « à toi » à son vert (PILE_AVANT=1bc2b8488) ; puis G
(sur ffe6efee8), ED (sur G). 96 : scinder et rejoindre, rouge d'abord.

**02 h 02 — soixante atterris** : go.work + conan + CMake, puis setup.cfg/setup.py/Pipfile/PEP 735/
bundledDependencies (codeparsers, fc206cba6, 2/2 en « = », lib 2 453/0, sans e2e_postgres dit). G a le « à toi »
(garde-deja-sur-master sur ffe6efee8) ; ED dernier (sur G). 96 : scinder et rejoindre — un apport ne se déplace
pas, il se recopie (son identité porte le nom de la fiche) ; Ref issue-de vers l'apport d'origine.
G, 02 h 04 : garde-deja-sur-master-pile 24d513d98 sur ffe6efee8 (36 vert) ; « à toi », PILE_AVANT=ffe6efee8 ;
ED se pose sur 24d513d98.
ED, 02 h 1x : un huitième défaut de l'écrivain trouvé par la lib large (owed_vectors_are_caught_up_on_open
75 s contre 20 s au parent : la synchronisation par générations réécrivait toute la source inchangée en un
paquet) ; correctif dd729317a (un fichier à empreinte égale est sauté, sauf passe en masse interrompue
signalée par relations_pending). Passe épinglée entière jusqu'à ~2 h 35, --onto sur 24d513d98, tête vers 3 h.

**02 h 07 — soixante et un atterris** : la garde tolère un « < » que `git cherry` dit déjà sur master (G,
bdeac0b86, « = », check_features 8/8 joué) ; garde du cache bc7d367eb3252d4d aux deux postes. **Il ne reste
qu'ED** (l'écrivain de 2a′-2, sur 24d513d98, tête vers 3 h, banc au jugement) : rien derrière lui. La file
du soir aura porté trente-quatre pushs depuis 21 h 58 sans un rejeu du dernier pas.
96, 02 h 1x : le lot A vert (lifecycle recopy, `default_filter` du SearchSourceNode, recall_apports must_not
recopied ; 137/137, e2e_archiviste 17/17) : passe devant ED, sur 24d513d98 ; « à toi » à son vert ; ED sur sa
tête. B (scinder, rejoindre) sur A ensuite.

**02 h 3x — soixante-deux atterris** : le filtre par défaut et l'état recopied (96, f553c2b2a). Il ne reste
qu'ED (sur fd33c5f06), tête vers 3 h.
ED, 02 h 50 : ecrivain-2a-prime-2-pile 26641b686 sur fd33c5f06 (« = » sur 17 commits de code ; le plan de la
page sorti du lot, en § 12 sur master) ; rejeu court vert (e2e_code 36, e2e_lectures 27, check_features 8/8,
lib 2 463/0, navigateur 45) ; e2e_demon_hote 14/14 et e2e_postgres 36/36 sur les têtes d'avant la repose ;
huit défauts corrigés. Attend le verdict de banc ; « à toi » au vert de banc, PILE_AVANT=fd33c5f06.
Banc, 02 h 51 : le verdict de l'écrivain (parent 1105a2590 contre 7ddedb026, même code que 26641b686) vers
3 h 45 — deux passes en parallèle à luciepc : A le parent (parité + usages), B la tête (retrait et remise
d'arbre.rs avec la visibilité des Symbol au rang, parité, usages, aretes_recopiees). Recherche a le « va »
pour sa sonde sous mesure.

**03 h 1x — l'écrivain d'ED sort de la file, rouge à la parité après publication** (banc, 7ddedb026 contre
1105a2590b) : avant l'index 30/30 au bit près, usages 5/5, aretes_recopiees 2 844 à la génération 1, retrait
et remise publiés ; **après l'index et la publication, 15 cas sur 30 rendent une autre liste dans leurs dix
premiers**. Piste : le plein texte voit encore les versions fermées. Cause à établir sur les deux copies
gardées ; témoin rouge puis correctif demain. **La file de la nuit est vide : soixante-deux atterris.**
ED, 03 h 1x : deux causes candidates — (A) le saut des fichiers inchangés (dd729317a) fige l'analyse d'avant
(la base migrée indexée par un code plus vieux : le parent réanalyse tout, le lot non ; un changement de
parseur ne s'appliquerait jamais à un fichier inchangé) ; (B) les documents des lignes fermées encore dans
lucivy. Départage par trois relevés de banc demain ; correctif (A) : la version de l'analyse entre dans
l'empreinte indexée.
Banc, 03 h 4x : la cause est (A), le saut des fichiers inchangés (aucun résultat du lot n'est une ligne
fermée) ; la parité était mal posée (base migrée indexée par un code plus vieux ; le parent laisse 14 201
ScopeContent orphelins, ticket chez ED). Demain : le correctif (version de l'analyse dans l'empreinte), puis
deux parités (propre et actuelle sur copie neuve, aux orphelins du parent près).

**03 h 5x — un lot de plus avant le matin** : scinder_fiche et rejoindre_fiches (96, lot-96-scinder, tout
déclaratif : graphes et Rhai du gabarit memory ; e2e_archiviste 19/19 ; genre `derived_from` ; deux défauts
contournés au ticket) ; la file étant vide, base = master 811f287103, « à toi » à son vert.

**04 h 0x — soixante-trois atterris** : scinder_fiche et rejoindre_fiches (96, 1f5d007b8, tout déclaratif). La
file est vide.

**04 h 3x — un lot d'outillage avant le matin** : publier-sans-courant (G, 047eca57f : la publication ne pose
plus `courant` sans `--poser-courant`), base = master, « à toi » à son vert.

**04 h 26 — soixante-quatre atterris** : publier-sans-courant (G, 7fd2300f9 : rag3db_libs.py ne pose `courant`
que sous `--poser-courant`, run_e2e.sh joue la lib qu'il bâtit par son identifiant ; § 1 de la bascule à jour ;
8 OK / 2 FAIL d'avant). rag3db_libs.py du cache de luciepc était une vieille copie du dépôt, recopiée
(e2e73d283ad9441f) ; il ne reste que lib_seule.sh sans dépôt. Le binaire du matin 26a091474 sur 373b5477b
(md5 56e3fd3f) est prêt, verdict 36/27/20. **La file est vide.**

**06 h 5x — un lot du cœur avant le matin** : le planificateur (A) de l'IN (in-cle-planifiee 0ebcd9e2e : un IN
sur la clé primaire, liste littérale ou paramètre, devient PRIMARY_KEY_SCAN sans FILTER ; ordre de la liste
rendu tel quel ; 10 clés sur 200 000 lignes 10,4 → 0,16 ms ; planner_tests 12, e2e_test 1 867 verts) ; base =
master, « à toi ». ABI : membres de classes internes, à la prochaine bascule.

**06 h 5x — soixante-cinq atterris** : le planificateur (A) de l'IN (le cœur, 7897f2453, mode simple sur master,
C++ seul). La lib suivante portera l'IN haché, le pliage, le point de faute des deux sens et l'IN planifié.

**11 h 51 — deux lots en file ce matin** : le cœur Rust du pont navigateur (archi, pont-navigateur-coeur
62d7ae4fc : cadrage Native Messaging, protocole révision 5, baux claim/mark/release/handoff, gardes fermées
par défaut ; 12 témoins sur une fausse extension ; lib 2 465/0, hooks 45/0) — « à toi » en mode simple sur
master ; puis le lot ALTER du cœur (passe finale, C++ seul) après son « poussé ». Deux rouges d'environnement
d'archi en chemin : MOTEUR_ANCIEN oublié ; target à 36 Go ramené à 5 Go par cargo clean -p (G prévenu).

**11 h 58 — soixante-sept atterris** : le cœur Rust du pont navigateur (archi, 36ee817a7) et le ménage du
target au plafond (G, 2bd2c136b : run_e2e.sh fait d'abord `cargo clean -p` des paquets locaux, reposé sur
master après le Rust d'archi, 25 OK). Embarquement a le « à toi » à son vert (A3, le repli cpu, mode simple) ;
le cœur rejoue son ALTER sur 36ee817a7 et pousse ensuite.

**12 h 0x — soixante-huit atterris** : tty-10, la saisie multiligne (tty, d187aec6a, mode simple ; crate 66/66 et
14/14, check_features 8/8). A3 d'embarquement est sorti (course du fil de calcul sous llvmpipe, ticket). Reste :
le cœur (ALTER, rejeu sur 36ee817a7, ~12 h 30), puis tty-11 et tty-12 à leur vert.

**12 h 3x — la file est tenue pour le cœur** : son lot C++ (l'ALTER qui attend) se repose sur master et rejoue
40 min ; personne n'atterrit d'ici son « poussé ». La voie de la garde qui transférerait des suites C++ quand
seul du Rust a atterri est au ticket de G.
G, 12 h 4x : garde-cpp-transferable 660586fab prêt (hors pile : si le lot ne touche que src/, test/,
extension/vector/, tools/rust_api/ et que `git diff base..master` est vide sous ces chemins, la garde accepte
en le disant ; en pile, (b) reste un arrêt ; rouge sur la garde de master, 39 vert). Push après le cœur.
tty, 12 h 4x : tty-11 prêt (cx-tty-11 67323f97e, 12 commits, crate tty seul : appels numérotés, aperçus
colorés, .md en prose, écritures en retraits/ajouts, « extrait, 4 Ko sur 16 Ko », Ctrl-O visionneuse en écran
alterné avec l'écran principal identique avant/après ; crate 80/80, 14/14 ; un commit rouge intermédiaire ne
compile pas seul, dit dans la page). Attend le « poussé » du cœur. Archi : l'équipier silencieux, après le relai.
ED, 12 h 5x : deux lots verts — (2) models.llm en liste + restart_budget_tokens (2b3340520, 7 témoins rouges
d'abord, passe épinglée verte) ; (1) l'écrivain avec le correctif (A) (f3e2a6f54, passe épinglée verte, attend
les deux parités de banc). **Ordre de la file** : cœur (ALTER) → tty-11 → ED (2) → G (garde-cpp-transferable)
→ archi (relai) → 96 (compaction) → ED (1) au vert de banc.
Archi, 13 h 0x : le relai du pont navigateur vert (pont-navigateur-relai 5c1035bcf : relai.rs Chrome ↔ hôte par
socket Unix seulement, adresses réseau refusées ; ecoute.rs un Pont par browserId, dossier 0700, socket 0600,
pas encore démarrée par l'hôte ; hote_natif.rs compose le manifeste d'hôte natif sans l'écrire dans le profil
de Chrome — geste de Lucie ; 21/0, lib 2 474/0, hooks 45/0). En file après ED (2) et G.
96, 13 h 0x : la compaction — compose_restart (2/2) et l'archiviste avec `context` (e2e_archiviste 20/20)
verts sur f67c8b535 ; e2e_compaction_archiviste rouge 4/4 par conception (attend le code de l'hôte). Décision :
pas de known_red pour une attente — 96 pousse le vert, le témoin de compaction part dans le lot d'archi comme
son rouge d'abord. Place de 96 : après archi (relai).
96, 13 h 08 : scindé — cx-memoire-compose-restart c733b81dd (compose_restart, `context`, leurs témoins, le gabarit ;
arbre de f67c8b535 moins le témoin de compaction, 0 ligne d'écart ailleurs) ; le témoin seul sur
cx-memoire-compaction-temoin-archi 3d30f16ee (rouge 4/4, pour archi, posé sur c733b81dd).
96, 13 h 09 : le pas `segment` — choix (1) : LlmNode prend `response_schema` et rend `json` sur un port de plus
(générique), plutôt qu'un `items_as` de plus sur Decide. Gardes : un modèle qui ne rend pas le JSON demandé → le
nœud le dit, port vide ; `segment_over` 0 = éteint, dit au manifeste. Lot à part après c733b81dd.
96, 13 h 12 : segment a besoin de rejoindre deux branches en ordre ; décision : `when='<champ>'` et `flatten=true`
sur ForEachNode (les éléments non concernés passent à leur place, une liste rendue est épissée), rouge d'abord ;
pas de fan-in JSON sans ordre défini. Gardes : champ absent = passe tel quel (trace), résultat non-liste = à sa place,
liste vide = élément retiré (trace).
96, 13 h 17 : le banc ne peut pas lire le cache Vertex — Decide jette l'Usage, LlmCall n'est émis que par la
boucle d'agent. Décision : lot `usage` **avant** segment (bloquant pour le banc et la borne de dépense) : Decide pose
usage {prompt_tokens, cached_prompt_tokens, completion_tokens, ms} sur l'élément, rendu par section sous trace et
total par passe ; champs absents quand le modèle n'en rend pas. Derrière c733b81dd.
banc, 13 h 17 : l'essai de l'archiviste sur Vertex (Gemini 3.8 Flash, 4 à 12 s par décision) vert ; N (244 tours,
sans contexte) tourne à luciepc depuis 13 h 16. Décision : F à 50 k ensuite (plafond ~6 M de jetons, compté par
caractères envoyés, pas estimé), 300 k seulement au cache lu par le lot `usage`.

## 13 h 18 — le cœur a poussé, la file repart

Cœur, 13 h 18 : alter-attend-les-ecrivains-3 sur master à **101ff50a6** (18 commits, cf62f6dbd → 101ff50a6 ; testé 8d3a78fe1,
docs seules au-dessus ; transaction_test 385 verts, known_red vert, e2e_test 1 867 verts et DropNodeTableReclaim seul,
ticket 392e63cd7). Lot C++ seul : la lib reste 373b5477b. Note d'ABI pour la prochaine bascule : Transaction gagne un
mutex, un ensemble de tables et un booléen ; LockResource gagne TABLE ; Catalog/CatalogSet des méthodes ;
Transaction::SCHEMA_CHANGED_RETRY. Le refus est au milieu du message (après « Transaction manager exception: ») : contains.
« À toi » à tty pour tty-11 (67323f97e), mode simple sur 101ff50a6. Ensuite ED (2), G, archi (relai), 96, ED (1).
tty, 13 h 31 : tty-11 sur master à **abfe6f92e** (13 commits, 412a940a0 → abfe6f92e, gmail, 0 attribution ; le crate à 79 tests).
« À toi » à ED pour models.llm (2b3340520), mode simple sur abfe6f92e. Ensuite G, archi (relai), 96, ED (1).
Ticket du vocabulaire des modèles (provider/protocol → access/provider) chez ED, 5291bba8e ; le mot et l'échéance des
alias posés à Lucie.
tty, 13 h 39 : tty-12 (les palettes, cx-tty-12 948e28111, 3 commits ; les cinq points d'archi ; crate 82/82) prêt, placé
après G et avant archi : ED (2) → G → tty-12 → archi (relai) → 96 → ED (1).

## 13 h 46 — Lucie tranche les priorités : les défauts du cœur, et terminer les vues par couche

Lucie, 13 h 46 : « les défauts du cœur restent prio, en parallèle de terminer les vues par couche ». Appliqué :
- **cœur et hotfixs** : par gravité — perte (Note sous pool plein, COMMIT sous index vectoriel, durabilité sur faute),
  plantage (quatre SIGSEGV), réponse fausse (index de relations, clé primaire au point de reprise, HNSW), blocages,
  lenteur ; la bissection du CREATE 237 s continue (ce n est pas une régression du lot ALTER : master d avant le lot donne 237 s ; le commit du 19 octobre fait 64 s aujourd hui à charge 6, une part d environnement et un facteur 4 réel ; seuil 150 s, sept pas). 55 tickets moteur ouverts au relevé de 13 h 46 (5 pertes,
  5 plantages, 9 réponses fausses, 6 blocages, ~30 lenteur/estimation/mémoire).
- **vues par couche** (état d'arbre : aucune recherche ne compose encore la pile ; une seule couche, `backend`) :
  banc joue les deux parités de l'écrivain d'ED devant F 50 k ; ED : écrivain (1), puis mesure (3a)/(3a′) contre (2),
  puis page 2a′-3 ; arbre : 2b′ (chargeur de la pile, point de composition) tout de suite sur `backend` seule, puis
  2b-C (paramètre `view` des outils, vue courante par le contexte) avec recherche ; la bascule des bases de Lucie
  se prépare en parallèle (copies, Gio, durée de validation à lui demander). Le vocabulaire des modèles et segment
  passent derrière.
- lucivy : 7 tickets ouverts, dont « une recherche filtrée ne classe pas comme sa vue » sans doute clos par 4.3.8 —
  à fermer ; hotfixs : le ticket « CI rouge depuis le 18 octobre » périmé, la CI est verte — à fermer.
ED, 13 h 48 : models.llm en liste sur master à **f9e894f36** (5 commits, b060f6203 → f9e894f36, gmail, 0 attribution ; ticket fait
2716a1c41). Note d'ED : la garde hors pile lit la base testée dans <journaux>/base, que passe_epinglee.sh n'écrit pas
(posée à la main) — pour G. « À toi » à G pour garde-cpp-transferable (660586fab) sur 2716a1c41. Ensuite tty-12, archi (relai), 96, ED (1).
lucivy, 13 h 48 : ticket « classe comme sa vue » fermé (e52a3bc92) ; sur les six autres, deux comptent pour Lucie (le poids du
plein texte, cent fois ses sources — exclusions chez recherche ; le dictionnaire des effacés, corrigé en 4.3.8, à fermer après
maintain par l'hôte), un pour les sessions (traces de run_e2e.sh → hotfixs), trois souhaits.
Arbre, 13 h 48 : question à Lucie — combien de temps garder le retour arrière après la bascule de ses bases (instantané +
ancien index, ~5 Go par base dont 4 de plein texte) ; proposition une semaine puis retrait base par base à son mot.
Arbre, 13 h 49 : page 2b′ posée (f8bbb0924) : vue nommée (couche, rang), chargeur qui lit la chaîne dans `_generation`,
composition en prédicat aux trois points, témoin 0 = une pile d'un étage identique au bit près (jugé par le banc),
deux étages à la main rouge d'abord, parente écrite à la publication ; `tracked.views {max_depth: 16, load_timeout_ms: 2000}` ;
rien d'autre que `backend` avant 2a′-3 ; la découpe de 2b-C pour recherche à la fin de la page. Va pour les rouges.
Banc, 13 h 49 : les parités de l'écrivain sont à refaire en partie — la sonde Cypher appelait rang_publie, absent du
parent : le parent n'a compilé dans aucune passe, la P0 copiée était la base intacte (leçon : une parité prouve d'abord
que les deux côtés ont bâti et joué). N quasi bloquée par le quota Vertex (11 tours sur 244 en 30 min). Décision (b) :
les parités tout de suite sous poste lourd (~1 h 20, verdict vers 15 h 10), N à côté sur son quota.
hotfixs, 13 h 52 : la CI de master — le vert vu par tty est le workflow `rag3weaver` ; le « CI Workflow » ne rend rien
depuis le 14 à 15 h 27 : le run 43610263500 attend pour toujours des machines `rag3db-self-hosted-testing` absentes et
tient le groupe de concurrence (cancel-in-progress faux), chaque run suivant est annulé à zéro job. Avant de se bloquer :
sanity checks vert (cassert clos), code format rouge, shell test rouge (Build). Mon `gh run cancel` des cinq runs en file
a été **refusé** par ma garde (vérifié : toujours queued) — à Lucie. Lot pour hotfixs : `if: vars.RAG3DB_SELF_HOSTED`
sur les jobs self-hosted, puis lire les deux rouges.
hotfixs, 13 h 52 : page des tickets moteur que Lucie peut rencontrer (1e3a4bb28) : 3 à fermer (corrigés dans 373b5477b :
COMMIT sous index vectoriel, clé primaire sous point de reprise ; les Note restent ouvertes faute des ×10, fe9337a7a) ;
atteignables : SIGSEGV optimisticRead, lecture hors colonne du CREATE concurrent, dictionnaire, colonne ajoutée ;
durabilité sur coupure, extension non chargée ; les deux index de relations (à dégrader), rang déplacé, plein texte après
arrêt brutal ; **et surtout les orphelins du plein texte qui reviennent (970 en 12 h)** — demandé à recherche si son
correctif est dans le binaire des hôtes.
Banc, 13 h 53 : pas de quota Vertex — le pilote lisait « 429 » dans le texte rangé des tours (un identifiant, une phrase
sur les quotas) et a attendu 31 min pour rien ; corrigé (il ne lit plus que l'erreur de l'appel). N relancée 13 h 51 ;
parités depuis 13 h 49, verdict vers 15 h 10.
Recherche, 13 h 53 : le correctif des orphelins (262be6fc4, 00 h 22) est dans 26a091474 (portable depuis 11 h 35) mais pas dans
fac67a208 (luciepc depuis 0 h 25, portable de 1 h 15 à 11 h 35) : les 970 du dépôt sont nés sous un binaire sans le
correctif. Hotfixs rebâtit l'hôte de luciepc depuis la tête de master et le repose ; recherche relève l'écart documents −
lignes au dépôt maintenant et dans une heure.
Hotfixs, 13 h 53 : lot CI prêt (ci-self-hosted-hors-de-master 809eb7ac9) : `if: vars.RAG3DB_SELF_HOSTED == 'true'` sur les
18 jobs self-hosted (benchmark sauté avec eux) ; une PR pour le jouer dans son propre groupe ; place après G, avant tty-12.
Recherche, 13 h 54 : les 970 orphelins sont nés entre 13 h 40 le 14 et 1 h 21 le 15, sous un hôte sans le correctif ; après la
purge (1 h 22, Scope 81 181/81 181, File 8 041/8 041), le portable a joué fac67a208 (sans correctif) jusqu'à 11 h 35. Pour prouver
que le correctif tient en service : deux comptes sous 26a091474 à une heure d'écart par `POST /backend/fts-live-ids`
(l'hôte du portable porte déjà les routes de lucivy, e95a1b8c6 est son ancêtre), sans fermer le backend.
Recherche, 13 h 56 : premier relevé par la route au dépôt : Lucivy_Scope 81 706 vivants, Lucivy_File 8 130 (zéro de 1 h 22 : 81 181 et
8 041) — sans les lignes du graphe, pas d'écart. L'hôte n'a pas de route Cypher ; la lecture du graphe que suppose
`fts-live-ids` est à l'appelant, non écrit. Décision : aujourd'hui la copie par la fermeture qui tient, deux fois ;
et un petit lot chez lucivy : `/backend/maintain` en `dry_run` qui lit le graphe lui-même et rend l'écart.
Hotfixs, 13 h 56 : PR L-Defraiteur/rag3db#1 ouverte (gmail, sans mention), run 43716213065 en cours dans son groupe ; la pose
du binaire de luciepc attend la place de G (sa chaîne preparer/poser-binaire, sa cible).
CI, 13 h 57 : le run de la PR 1 (43716213065) rend un verdict dans son groupe : 18 self-hosted sautés, sanity checks vert,
code format rouge (connu), shell test en cours, python lint annulé (à lire). Recherche ferme code-rag3db au portable à
14 h 00 et vers 15 h 00 pour ses deux instantanés ; relayé à toutes les sessions.
14 h 00 : codeparsers (sans lot depuis son rapport) prend `new` mot vide en Rust (usages des constructeurs), puis fmt/format.h
(macro lue comme un corps), puis les scopes sans DEFINED_IN ; embarquement prend la conception du repli d'embarquement
vers un service déclaré quand la carte est pleine (mesuré, dit, réversible ; réglages au manifeste). Lucivy : le relevé à
blanc (`/backend/maintain` dry_run, lit le graphe lui-même, reprend si le jeton bouge) prêt sur lucivy-releve-a-blanc
7a684087d7, en file après ED (1). Le run de la PR 1 complet : sanity vert, code format et shell test rouges (hotfixs lit).
Recherche, 14 h 01 : premier compte des orphelins au dépôt (fermé 21 s, instantané recherche-compte-orphelins-1-20261115-1400,
copie reflink, méthode de hotfixs) : Scope 81 674 lignes / 81 706 vivants = **32 orphelins** ; File 8 126 / 8 130 = **4** ;
aucune ligne sans document. Tous nés depuis le zéro de 1 h 22 ; les _node_id gardés pour comparer les listes au second
compte de 15 h 00. Si 32 et 4 tiennent, le correctif tient en service.
Embarquement, 14 h 01 : conception du repli posée (44f80a9d2) : `fallback` gagne {service} ou {activation} ; carte pleine
mesurée (VRAM libre avant activation contre mémoire requise + marge ; débit sous seuil pendant une fenêtre) ; dit, et
retour à la carte quand elle a la place ; sans déclaration, pas de repli. Tranché : modèle + dimension obligatoires,
recette comparée si le service la dit, sinon « repli sans recette vérifiée » et provenance de chaque lot au journal ;
clés du manifeste en anglais ; le vrai jeu sur card2 après le verdict du banc.
Hotfixs, 14 h 02 : les deux rouges du run de la PR 1 étaient d'une ligne chacun, corrigés dans le lot (892363395) : shell test =
GCC -Werror=missing-field-initializers sur ListLambdaBindData (quantifierCounts, ajouté par 9edf6f3b4) ; code format
mourait avant le format, « cargo-fmt is not installed for the toolchain 1.98.1 » (rustfmt ajouté avant cargo fmt) — le
verdict du format C++ n'avait jamais joué. Python lint annulé à la seconde : macos-14 non accordé (dépôt privé) → sur
ubuntu-24.04, même lot. Le binaire de luciepc : G le pose, hotfixs vérifie les ancêtres.
tty, 14 h 07 : tty-13 (« ! » et « / », cx-tty-13 28b2424e9, 16 commits, crate 93/93 ; gardes prouvées par le harnais : une ligne
« !touch » de l'équipier ne lance rien, le mot de passe d'un faux sudo n'est ni dans la consigne ni dans le journal) prêt,
placé juste derrière tty-12, en pile sur sa tête. Ordre : G → CI (hotfixs) → tty-12 → tty-13 → archi → 96 → ED (1) → lucivy → arbre.
Embarquement, 14 h 09 : la recette porte le périphérique, donc incomparable à un repli ; tranché : ce lot livre `fallback:
{service}` seul (modèle + dimension vérifiés, « repli sans recette vérifiée » dit), `{activation}` en lot suivant avec
ticket ; sept témoins dont la provenance des vecteurs au journal (carte / service, en morceaux).
Codeparsers, 14 h 12 : cause de « new mot vide en Rust » : RUST_STOP_WORDS est une copie de la liste JavaScript ; `new` est
dans six listes, juste en C++, C#, Go (mot-clé ou intégrée), à retirer de Rust, Python, C. Va, rien d'autre dans le lot ;
ticket basse pour la liste Rust copiée ; compte des références avant/après sur rag3weaver.
Lucivy, 14 h 19 : relevé à blanc vert (7d7788e57e ; rouge d'abord vu sur d8d80452c2 ; fts_maintenance 4/4, check_features 8/8,
e2e_demon_hote 23/23 ; `retries` rendu, refus nommé « base en écriture continue » au-delà de cinq lectures). Avancé devant
ED (1) : G → CI → tty-12 → tty-13 → archi → 96 → lucivy → ED (1) → arbre.
Cœur, 14 h 24 : bissection finie — le CREATE_VECTOR_INDEX lent sous écrivains vient de **f42d90d13** (25 oct., le CREATE en deux
transactions) : 36 s avant, 165 s après, six pas à charge 6. Piste (non mesurée) : dès T_A validée, le holder NOTED est visible
et chaque écrivain prend le verrou INDEX en exclusif pendant tout le bâti de T_B, alors qu'aucun n'écrit dans l'index. Mesure
d'où attend l'écrivain le plus long avant le correctif (un holder sans écritures ne fait pas prendre le verrou). Ticket
f96509f31. Après le SIGSEGV (témoin en cours depuis 14 h 3x).
Embarquement, 14 h 25 : repli carte pleine vert (repli-carte-pleine : b25c373af sept rouges, f26dc306d correctif ; mesure de la VRAM,
repli par `fallback {service}`, retour dit, « pas de repli déclaré » dit, provenance au journal, `embedding.fallback` dans l'état).
Garde ajoutée : `fallback_hold_secs` (défaut 4 × window_secs) contre l'alternance ; le doublement dans le ticket `{activation}`.
Place après ED (1), avant arbre.
14 h 27 : G en attente d'une place, pas bloqué (reposé 4f48b2d94, test_push_garde 39 vert ; check_features relancé à luciepc,
push vers 14 h 35) ; il pose ensuite le binaire de luciepc. Au portable, `wt-livrable-passe.sh` (archi, cargo test --lib)
tient la seule place lourde depuis 13 h 07 à charge 1,7 — un test qui attend, probablement ; quatre passes en file derrière
(96, arbre, codeparsers, un gcc-tous). Archi prié de regarder tout de suite.
Archi, 14 h 28 : place rendue à 14 h 27 (pid tué, aucun reste). Cause : son lot — le défaut « une demande sans expects attend une
ligne » fait attendre sans borne les témoins d'avant de l'équipier (3 FAILED, d'autres suspendus). Le lot n'est pas vert ;
archi relit chaque témoin (expects "none" ou une ligne) et borne la prochaine passe ; le produit lui-même doit borner l'attente
d'une ligne (délai réglé, échec nommé). Le relai reste à sa place. Procédure : toute passe lourde part sous `timeout` dans `poste`.
Archi, 14 h 29 : la garde de l'équipier a une portée large (une vingtaine de témoins d'avant postent du travail sans report) ;
tranché **(B)** : la relance ne joue que si l'équipier n'a rien laissé du tout (ni ligne, ni fichier, ni plan clos), puis l'attente
du superviseur bornée par le délai réglé, échec nommé.
Hotfixs, 14 h 29 : run de la PR sur 83fccc0b6 : lint, format, sanity verts, shell test rouge (un second agrégat incomplet,
DatabaseHeader::writerBuildID ; GCC syntaxe seule sur tout src/ pour tous les corriger, à luciepc). Le vert du format est
trompeur : sur une branche hors master, le job **pousse un commit « CI Bot »** (644d1d8c5, 25 fichiers = la dette de format
réelle). Tranché : aucun commit d'un bot dans le dépôt — l'étape disparaît du lot, « Fail if any change » joue partout ; le
format pur des 25 fichiers en un commit gmail juste après le lot CI (fenêtre), rien de C++ ne pousse d'ici là.
G, 14 h 33 : garde-cpp-transferable sur master à **5b8f8994f** (2 commits : les suites C++ transférables 72f5e1ffe, passe_epinglee.sh
écrit la base testée 5b8f8994f ; gmail, 0 attribution). G pose maintenant le binaire de luciepc depuis la tête de master.
« À toi » à tty pour tty-12 (900ce167f) puis tty-13 en pile sans second mot ; le lot CI de hotfixs (pas prêt : l'étape du bot
à retirer, les agrégats) passe après eux, puis la fenêtre de formatage. Embarquement : repli complet (215e4b989 tenue,
9df6c6cd9 docs ; lib entière 2 353 verts), base testée 09bb33e64, en file après ED (1).
Cœur, 14 h 33 : cause du SIGSEGV de la colonne ajoutée : NodeTable::addColumn pose la colonne puis la remplit groupe par groupe
et n'inscrit son annulation qu'à la fin ; un refus à mi-chemin (le pool plein, dans les quatre tours morts et eux seuls) laisse
la colonne au stockage quand le catalogue l'annule ; l'ADD suivant se range un cran trop loin, sa relecture tombe dans le
morceau d'à côté (memcpy de readCompressedValuesToVector). Branche colonne-ajoutee-se-relit : point de faute e444ee491,
témoin AddColumnRefusedMidway db7eed64c, correctif tout-ou-rien à chaque niveau 6d557d826 ; passe à luciepc en cours ;
puis DdlEveryMinute ×10 pour fermer aussi les Note perdues.
G, 14 h 33 : « poussé » reçu ; écart dit sans effet (base écrite ee3b5c901 au lieu de 42a916f08, lue sur origin/master partagé
après le rebase ; aucun code entre les deux ; passe_epinglee.sh écrit désormais la base par merge-base au --onto). Garde à jour
aux deux postes : push_garde.sh **bf891500**, passe_epinglee.sh **73333766** (témoins 39 et 21 verts, anciennes en .avant-cpp-transferable).
Binaire de luciepc depuis 5b8f8994f en essai, pose vers 14 h 55.
G et hotfixs, 14 h 40 : l'hôte de luciepc porte **5b8f8994f** sur 373b5477b depuis 14 h 40 m 14 s (ancêtres 262be6fc4, 26a091474 vérifiés ;
md5 f7133291 ; lié à rag3db-build-373b5477b ; /sante répond). code-lourd et code-principal ne fabriquent plus d'orphelins par
ce chemin. Les deux postes : portable 26a091474, luciepc 5b8f8994f, lib 373b5477b partout.
tty, 14 h 45 : tty-12 sur master à **87795452d** (4 commits, 471e2314f → 87795452d, gmail, 0 attribution ; la CI de rag3weaver joue
le --check des palettes). tty-13 suit en pile (PILE_AVANT=87795452d). Ensuite : CI (hotfixs, au « prêt »), fenêtre de
formatage, archi (relai), 96, lucivy, ED (1), embarquement, arbre.
tty, 14 h 45 : tty-13 en pile avec PILE_AVANT=900ce167f (la tête de tty-12 **avant** repose) : la garde a dit ARRÊT, « la pile a
changé » — master avait du code (backend.rs, chat.rs, model_source.rs) que cette tête n'avait pas. Juste : la pile se rebâtit
sur la tête atterrie. tty-13 reposé seul sur 87795452d (cx-tty-13-2), rejeu court, mode simple.
tty, 14 h 48 : tty-13 sur master à **ccfa7efea** (17 commits, e7363428b → ccfa7efea, gmail, 0 attribution). La file : CI (hotfixs,
au « prêt »), fenêtre de formatage, archi (relai), 96, lucivy, ED (1), embarquement, arbre.
