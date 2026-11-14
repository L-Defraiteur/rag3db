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
