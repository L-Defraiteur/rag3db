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
Banc, 14 h 52 : **l'écrivain d'ED (A) est VERT** (d087e02c7 contre e95a1b8c6 ; garde « les deux côtés ont écrit leurs 30 cas » passée ;
parité propre 30/30 au bit près avant et après l'index, usages verts ; base migrée : 7 écarts sans réponse changée, ex aequo
du parent). Repasser le lot sur une base d'avant = 10 × le parent (~45 min au dépôt) : chiffre pour la bascule. ED repose
(1) sur master, place après lucivy. Embarquement peut jouer son vrai jeu sur card2 (créneau avec le banc). N puis F 50 k.
ED, 14 h 53 : le repassage d'une base d'avant par l'écrivain coûte 10 × le parent (2 656 s contre 254 s ; grain du fichier contre
réécriture sur place) : première synchronisation de chaque base de Lucie après l'atterrissage, puis à chaque montée de
REGLES_DE_L_ANALYSE. Ticket ouvert. Tranché **(a)** : l'écrivain atterrit tel quel ; l'état dit le rattrapage long quand il
joue ; nouveautés avec le chiffre ; le grain du paquet = lot suivant de l'écrivain, parité rejouée par le banc.
Arbre, 14 h 55 : la fenêtre de la bascule (1e1a7edae) : par base maintain → instantané → 1b → repassage → vérification ; dépôt ~1 h 30,
code-lourd ~45 min, code-principal ~35 min (extrapolés) ; la question des créneaux et de la semaine de garde transmise à Lucie.
2b′ : rouges 4/4 (5f64d1aeb), le vert en cours.
ED, 14 h 55 : **l'écrivain (1) retenu** — défaut vu dans les relevés du banc : après repassage, DEFINED_IN va aux File fermés (73 335)
et presque pas aux neufs (542) ; un scope inchangé ne mène plus à son fichier visible ; la parité ne le voit pas (search_code
et usages n'y passent pas). Témoin 13 rouge d'abord (chaque Scope visible a un DEFINED_IN vers un File visible), correctif,
rejeu. La parité du banc s'élargit : outline, fichier d'un scope, parent_path, compte des arêtes par étiquette vers le visible.
Extrapolation : 0,35 s par fichier, vecteurs non recalculés ; code-lourd 45 à 50 min.
Banc, 14 h 57 : **ne confirme pas** le défaut DEFINED_IN : après repassage, 73 340 arêtes vers des File ouverts (autant qu'au
parent), 537 vers des fermés ; les 3 814 Scope sans File visible sont des deux côtés (la base : ticket des scopes sans
DEFINED_IN chez codeparsers). La lecture d'ED inversait sans doute le total de P0 et le reste. ED départage avec les sorties ;
seule question : les 537 vers des fermés. Le juge étendu reste pour la prochaine parité.
ED, 14 h 58 : lecture retirée (une soustraction de deux totaux, pas un relevé par fermeture) ; l'écrivain n'a pas ce défaut. Les 537
vers des File fermés : attendus légitimes (Scope fermés avec leur File d'avant, leur histoire ; le parent efface, d'où son 0) ;
chiffre demandé au banc : Scope fermé → File fermé = 537, Scope ouvert → File fermé = 0. L'écrivain repart en file tel quel
dès son rejeu, avec l'état du rattrapage ; le témoin 13 gardé comme filet.
Cœur, 14 h 59 : SIGSEGV rouge (db7eed64c : SIGSEGV, et le défaut de la colonne ajoutée lu sur 100 lignes au lieu de 3 100) puis vert
(ee598e074 : AddColumnRefusedMidway nœuds et relations, AddColumnRollback, AlterWaitsForWriters, transaction_test 387). Les ×10
de DdlEveryMinute tournent (~65 min) ; le pool plein n'est pas réglé par ce lot.
15 h 01 : 96 repose c733b81dd sur ccfa7efea maintenant (rejeu sans push) ; le lot `usage` (cx-memoire-usage 1c85fa06e : Decide pose
usage, archive_rapport.rhai fait usage_total ; signe d'absence = prompt_tokens == 0 ; le vert en cours sur six suites puis la
lib) en pile dessus, même « à toi » s'il est vert avant. Archi repose le relai sur ccfa7efea. G pose 5b8f8994f au portable
après 15 h 05, puis le ticket des hashes courts.
Banc, 15 h 02 : F à 50 k finie (244 décisions) : 14 788 793 caractères de contexte + 722 399 de sections envoyés, soit un plafond de
**3,88 M de jetons** (à 4 caractères, sans cache, instruction et candidates en plus) — chiffre pour Lucie, à côté du réel de sa
console ; temps par décision sous réserve (la passe a tourné à côté de l'index des parités sur 7879). Jugement P1/P2 de F 50
contre N en cours, par Gemini seul. card2 donnée à embarquement. Scopes sans DEFINED_IN : 3 931 avant les versions, 3 814 après,
parent comme écrivain — l'ingestion (constructeurs de JsonMutWrapper : seul le dernier a son lien), ticket codeparsers complété.
Recherche, 15 h 02 : second compte (fermé 20 s) : Scope 32, File 4 — **les mêmes _node_id** ; entre les deux, 18 Scope effacés et 156 nés
sans un orphelin de plus : **le correctif tient en service**. Ticket fermé (8b8e14ef0). Les 36 restants (nés sous fac67a208) :
purge par /backend/maintain base ouverte, à recherche ; copies effacées, instantanés gardés jusqu'au soir. Recherche passe à 2b-C.
Hotfixs, 15 h 03 : troisième rouge de shell test corrigé (fonction static morte, -Werror=unused-function, d65af3792) ; compile tout
src/ sous les options exactes de la CI avec témoin avant de dire « prêt ». La purge des 36 : par la route base ouverte chez
recherche (la route refuse si la table a bougé), pas de fenêtre ni de compact.
Recherche, 15 h 03 : la purge des 36 attend le relevé à blanc de lucivy — le jeton de maintain ne couvre pas l'intervalle entre un graphe
lu à 15 h et le relevé ; une place réemployée ferait une ligne sans document, la panne inverse. Le relevé à blanc lit le graphe
sous le même jeton puis maintain purge : sans trou, base ouverte. Recherche sur 2b-C.
Banc, 15 h 05 : F 50 k contre N, provisoire (58645ea33) : N = 78 fiches dont 9 doublons (0,115, casse), 6,2 s/décision ; F 50 k = 30 fiches,
1 doublon (0,033), P1 0,81 (63/78 sujets, au seuil), 7,9 s/décision, 3,88 M de jetons au plafond. Le contexte ôte les doublons
mais double les « rien » (72 contre 38) : à départager en relisant ces tours (aide ou endort). Suite : relecture du juge et des
« rien », F 25 % et S 50, 300 k au lot usage.
Embarquement, 15 h 06 : jeu sur card2 (15 h 03, hôte isolé 00828f25) : repli dit, 2 284 morceaux par 7879, index 34,1 s (904 s le 14),
rien sur card2 — mais la carte avait 15 190 Mio libres et la marge a été montée à 4096 pour forcer le repli : le jeu prouve le
mécanisme, pas la carte pleine (dit dans la page ; le vrai cas en ticket mesure). Lot prêt (6 commits, base 09bb33e64).
G, 15 h 06 : la pose du portable attend : trois clients sur 7970, dont `rag3weaver-tty --tache essai-tty` (pid 814707, 3 h 22, un essai de
tty) ; G lui demande. Lot des hashes courts prêt (garde-hash-entier 1e5902a83, 2 rouges vus sur la garde de master).
tty, 15 h 07 : le pid 814707 n'est pas un essai de tty, c'est **le terminal de Lucie** (fish du terminal intégré de VSCodium, 11 h 44) — on
ne le coupe pas ; il reprend son flux seul après une coupure de l'hôte (Last-Event-ID). G pose au portable sans attendre.
Banc, 15 h 08 : les 72 « rien » de F 50 k relus contre N (73278ea4c) : 34 **aide** (N l'a rangé sous un sujet que F couvre ailleurs — mais
l'apport du tour est perdu : la section n'est pas rangée), 22 **endort** (sujets que F ne couvre nulle part, neuf sujets entiers
perdus), 16 d'accord. Juge relu à la main : 0,90 sur 30, indulgent. Pour 96, variante de consigne : « rien » seulement si la
section n'apporte rien, sinon rejoindre le sujet montré ; « le contexte montre ce qui existe, pas ce qui est rangé de cette
section ». F 25 % et S 50 tournent depuis 15 h 06.
ED, 15 h 09 : le témoin 13 trouve un vrai défaut, **dans l'histoire** : au rang d'avant, un scope inchangé ne mène plus à son File d'avant —
l'oubli des arêtes défaisait le DEFINED_IN du scope partagé vers la ligne File fermée (« Scope ouvert → File fermé : 0 » là où
l'histoire en attend ~73 000). Correctif : l'oubli ne défait pas une arête dont l'autre bout est une ligne fermée (CONSUMES
aussi). L'état du rattrapage (témoin 14) dans le même envoi. Témoins, passe épinglée, tête ; le banc rejoue le relevé par paire
de fermeture sur une copie neuve.
G, 15 h 10 : l'hôte du portable relancé (pid 788162 → 2474552), binaire **5b8f8994f** (md5 f7133291) sur 373b5477b, /sante répond : les deux
hôtes portent le même binaire sur la même lib.
Archi, 15 h 10 : la forme (B) touche encore cinq témoins d'avant (le motif du relevé de tty par construction) : ils déclarent
`expects: "none"`, une ligne chacun ; et la trace de (B) (fichier écrit, commande, plan clos) devient durable (fichier d'état à côté
du curseur) pour ne pas relancer à tort après un redémarrage de l'hôte — dans le lot. Tranché oui aux deux.
G, 15 h 11 : pose au portable faite (5b8f8994f, f7133291 ; 26a091474 en .avant), mais **un accroc** : le réveil de l'hôte a refusé à 15 h 09 m 10 s,
le tunnel 7980 → 7879 de luciepc étant injoignable — **le 7879 est tombé et a redémarré (NRestarts 21 depuis 11 h 39)**, l'hôte
coupé 55 s au lieu de 15. Deux tickets chez embarquement : pourquoi le 7879 meurt (journal muet) ; un hôte ne doit jamais
refuser de démarrer parce qu'un service d'embarquement distant est injoignable (dire, servir le lexical, retenter).
Codeparsers, 15 h 12 : `new` vert (new-rust 87126e4 ; 3 témoins rouges d'abord, 330 passés) ; banc sur rag3weaver/src (288 fichiers) :
références 263 402 → 271 732, relations vers un scope `new` 542 → 2 894 (×5,3), scopes appelant un `new` 403 → 1 596 ; 20/20
CONSUMES de l'échantillon au bon type. Lot rag3db (pointeur, tickets, nouveauté) à monter ; place après lucivy. Ordre : CI →
format → archi → 96 (+usage) → lucivy → codeparsers → ED (1) → embarquement → arbre. Puis fmt/format.h (sonde rouge : 45 scopes).
Embarquement, 15 h 13 : (1) le 7879 meurt sur « failed to reserve N bytes of device memory » (2,4 à 4,1 Go, cubecl-wgpu), puis sort
avec un code 0 que systemd relance ; douze tailles refusées reviennent dans le même ordre, cycle après cycle (la passe F du
banc, 13 h 53 → 15 h 08) ; ticket 565715be0 : refuser la demande trop grosse (503 nommé) au lieu de mourir, recouper les lots.
(2) Le réveil refusé n'est pas l'hôte (`embedder_or_absent` ouvre en plein texte seul et le dit) : c'est la garde de la chaîne de
pose de G, qui a attendu le tunnel — à G : la garde dit, elle ne retient pas le réveil. Pas de ticket chez embarquement.
15 h 13 : correction — la garde du réveil de G (`bascule-reveil.sh`) est **voulue** : elle évite qu'un hôte se rouvre en plein texte seul
et pose 0 vecteur sans bruit (ticket du 25 octobre) ; elle a attendu le 7879 et réveillé à son retour. Rien à changer chez G
ni chez l'hôte ; les 55 s sont le temps de redémarrage du 7879 (ticket 565715be0).
15 h 14 : codeparsers — lot `new` monté (codeparsers/new-rust f1172dac5, un commit : pointeur 5885e13 → 87126e4, Cargo.lock inchangé,
ticket fermé + ticket basse, nouveauté ; check_tickets --lot VERT) ; rejeu check_features + e2e_code au mot ; passe à fmt/format.h.
G — garde du réveil inchangée (modification annulée, worktree propre) ; sur le juge known_red ; le lot des hashes courts
(garde-hash-entier 1e5902a83) prend place après embarquement, avant arbre.
Embarquement et banc, 15 h 15 : **la cause du 7879** : un seul texte trop long — le gabarit memory embarque la Source entière (un tour,
jusqu'à 43 137 caractères) ; 4 102 146 304 octets = 16 têtes × 4 octets × 8 006², la matrice d'attention d'un texte de 8 006 jetons.
Le budget de caractères ne coupe jamais un texte. Décision : **F 25 % coupée**, S non lancée (les hôtes de Lucie passent devant la
mesure : le tunnel du portable tombe avec le 7879) ; lot chez 96, après `usage` et avant la variante : découper la Source longue
à l'écriture (`source_chunk_chars`, chevauchement, rappel par le meilleur morceau) ; plafond de jetons côté service chez
embarquement (ticket a4b9de029). Reprise du banc sur ce lot.
96, 15 h 19 : correction — la Source n'est pas embarquée (bm25 seul) et les textes écrits sont déjà découpés (1 500/200) ; la vraie
cause est **`embed_query`** (search.rs:968) : la recherche des candidates d'archive_decide embarque la section entière comme
requête, en un seul texte sans plafond. Deux lots : 96, `query_chars = 4000` (titre + début de section, trace de la longueur) ;
embarquement, le plafond défensif `query_max_chars` dans embed_query (dérivé de context_tokens, coupe dite), car toute requête
longue d'un outil tue le 7879 pareil. Le 503 côté service à part.
96, 15 h 19 : la passe d'`usage` a un rouge d'environnement (put_fiche : « 7980 ne répond pas », 15 h 1x = la chute du 7879 au début de
F 25 %, ticket 531fd4639) ; rejoué seul à la fin de la passe, dit au journal. Le faux embarqueur qui note les tailles : chez
embarquement, dans son lot du plafond d'embed_query.
Embarquement, 15 h 20 : conception du plafond `query_max_chars` relue et acceptée : défaut fixe 6 000 caractères (~2 000 jetons, ~260 Mo
d'attention), `context_tokens × 3` en plafond haut (un déclaré au-dessus est refusé au chargement ; le contexte dit ce que le
modèle accepte, pas ce que la carte tient) ; une fonction avant les trois chemins (dense, dual, creux) et dans les deux nœuds,
coupe sur un mot, avertissement dit, le plein texte garde la requête entière ; champ de `models.embed` ; six témoins rouges.
tty, 15 h 24 : tty-14 (/model, cx-tty-14, 14 commits ; hôte : Tache.modele, GET /ide/teammate/models, POST /ide/teammate/model, refus nommé ;
terminal : liste, choix, Tab ; relu par ED ; lib 2 426 verts sur c2fa35406, avant les ajouts d'ED) prêt, placé après codeparsers,
avant ED (1) ; lib entière au rejeu sur la tête reposée. Ordre : CI → format → archi → 96 → lucivy → codeparsers → tty-14 → ED (1)
→ embarquement → G → arbre.
Hotfixs, 15 h 28 : shell test bâtit sous le GCC 13 de la CI ; il reste trois cas périmés par le renommage kuzu → rag3db (largeur du
mode box), corrigés (2f6143617) ; « prêt » glisse à ~16 h. Archi et 96 passent devant : « à toi » à archi pour le relai dès sa
tête verte ; 96 suit en pile (c733b81dd → usage → query_chars, ce qui est vert). Puis CI + format, lucivy, codeparsers, tty-14,
ED (1), embarquement, G, arbre.
15 h 30 : archi — relai reposé (pont-navigateur-relai-2 76b7d757b, témoins verts), rejeu court derrière sa double passe de l'équipier
(place lourde jusqu'à ~15 h 40), vert attendu ~15 h 55, push au vert sans second mot. 96 — c733b81dd reposé (lot-96-compose-pile
a3bb9db72) : lib 2472/0, archiviste 20/20… ; compose_restart 1/2 et cited_by 0/3 = 7879 tombé (environnement), rejoués seuls ;
`usage` peut-être vert à 15 h 45 ; `query_chars` au second tour. ED — l'écrivain (1) bff6c7409 (ecrivain-2a-prime-2-pile-2) : passe
épinglée entière VERTE (e2e_code 36, lectures 27, demon_hote 20, postgres 39, lib 2485, navigateur 45), avec le correctif
d'histoire (témoin 13), l'état du repassage (témoin 14) et la nouveauté chiffrée ; le banc rejuge depuis 15 h 11 (~2 h) ; le grain
du paquet (témoin 15) sur une branche à part, lot suivant.
Embarquement, 15 h 33 : le plafond `query_max_chars` vert (plafond-requete, 4 commits sur le repli ; six témoins rouges d'abord ; lib
2 360 ; défaut 6 000, refus au chargement d'un nul ou > context_tokens × 3, état {value, source}). La pile repli + plafond
avance juste après 96 (elle protège les hôtes et le banc) ; rejeu e2e_code + e2e_lectures sur la tête reposée. Ordre : archi →
96 → embarquement → lucivy → codeparsers → tty-14 → ED (1) → G → arbre ; CI + format de hotfixs à son « prêt ».

## 15 h 38 — Lucie répond : autorisation explicite, oui à la bascule, oui aux mots, « choisis une politique »

Lucie, 15 h 38 : « je t'autorise explicitement » (les runs CI), « oui pour basculer », « oui pour le reste », « mes bases on s'en fout
c'est là pour ça », « choisis une politique relevante ». Appliqué :
- **CI** : le run 43610263500 annulé (completed/cancelled) ; un run neuf de master (43724738336) tient le groupe à son tour ; les
  quatre du 11 novembre « completed » à l'API, « queued » à la liste. Au « poussé » du lot CI de hotfixs : annuler tout run plus
  ancien en file pour que le sien joue.
- **Mots des modèles** : `access` + `provider`, alias deux semaines (fin le 29 novembre), chez ED après la page 2a′-3.
- **Bascule** : ce soir dès la file vidée (l'écrivain atterri), les trois bases en parallèle après maintain, répétition sur copie
  aujourd'hui ; **retour arrière 72 h** (instantané + ancien index), puis retrait base par base à mon mot sans redemander ;
  journal de bascule par base, état « basculée le … ».
- tty-15 (les deux pistes tmux, cacfd9223) en pile derrière tty-14.
Archi, 15 h 43 : le relai du pont navigateur sur master à **5ff5bdfb2** (2 commits, 35863e1d9 test → 5ff5bdfb2 feat, gmail, 0 attribution ;
rejeu court vert 15 h 41 ; un premier rejeu tué par son propre timeout pendant l'attente de la place — la borne couvre l'attente
désormais). « À toi » à 96 pour sa pile (c733b81dd, + usage si vert) sur 5ff5bdfb2. Puis embarquement, lucivy, codeparsers,
tty-14 (+ tty-15), ED (1), G, arbre ; CI + format de hotfixs à son « prêt ». Arbre : répétition de la bascule vers 17 h à luciepc
sur copies ; 2b′ a trouvé un vrai défaut (NULL typé STRING dans merge_generation cassait la génération 0), corrigé avec témoin.
Lucie, 15 h 46 : premier essai du terminal neuf (tty-11/12/13) sur essai-tty : « qu'est-ce qu'on sait sur les orphelins du plein
texte ? » — 12 appels d'outil (les bons : ticket lucivy, catalog.rs), puis `step_done` « Synthèse documentaire détaillée… » sans
un mot dans la boîte, « fait sans plan ». Le défaut de l'équipier muet vu par Lucie elle-même ; relevé passé à archi pour son
lot (B), qui passe en tête de sa file après le relai.
ED, 15 h 48 : le grain du paquet vert en lot à part (ecrivain-grain-du-paquet d57b28c80, sur l'écrivain bff6c7409) : témoin 15 rouge
d'abord (862 puis 1 622 instructions pour 40 et 80 fichiers, ~19 par fichier), vert ensuite (au plus un quart de plus pour deux
fois plus de fichiers) ; passe épinglée verte. En file derrière l'écrivain, au vert du banc et après sa mesure du temps de
repassage (cible : au plus deux fois le parent). Arbre : le produit n'appelait pas migrer_vers_les_generations — une CLI
`migration generations --base` en petit lot sur la pile d'ED. La transaction par paquet derrière la bascule.
Arbre, 15 h 49 : la fenêtre de la bascule change — (1) il manquait la montée aux générations (`migrer_vers_les_generations` n'était
appelée que par des tests) : lot `migration generations --base` (arbre-principal/migration-generations df983fd49, sur la pile
d'ED), refus nommé si la base est tenue ou pas à versions, rouge en cours ; (2) le repassage se fait base ouverte (un « non
indexé » d'un instant sur les File d'un paquet) : la fenêtre fermée = maintain, instantané, 1b, montée ; (3) chiffres réels :
code-lourd 7 278 fichiers / ~42 min de repassage, code-principal 6 314 / ~37 min, dépôt ~45 min. Répétition vers 17 h 30 sur les
instantanés de cette nuit (aucune base vivante fermée), script repetition-bascule.sh à luciepc.
Lucie, 15 h 54 : « il n'a toujours pas d'outils pour éditer du code comme un agent standard ; j'aimerais retrouver l'expérience
d'un agent de code standard tout en gardant toutes ces fonctionnalités, là ça parle un peu trop de gabarits ». Fait : l'équipier
d'une tâche n'a que les outils que le manifeste du backend déclare, et celui du dépôt de Lucie (rag3db.json) n'avait ni
edit_file ni run_command (le gabarit code les a). Direction donnée à archi (copie ED, tty) : socle d'agent de code standard
dans toute tâche (read, edit, write, run_command derrière la porte, grep, glob), le backend n'ajoute que ses outils, prompt
d'agent de code, déclaratif en bas ; page dans l'heure, lot après (B). Pour aujourd'hui : edit_file et run_command déclarés
dans son manifeste et dans la copie de la tâche essai-tty (sauvegardes .avant-edit-file-*), `read_only` → `task_writes`
(son arbre reste intouchable, la tâche écrit dans son worktree), `commands: standard` (liste sûre, le reste refusé en le
disant) ; backends fermés et rouverts par l'hôte (session s-8b7c3d669ca1e3fd).
Archi, 15 h 55 : page du socle (f0e29b3dc) ; découpe : ED = le jeu de primitives déclaré une fois (read_file 200 lignes + empreinte,
replace_text, replace_lines, insert_lines, write, glob, list, grep ; diff, réindexation, run) ; archi = le pas (socle dans toute
tâche quel que soit le manifeste, homonyme du backend qui gagne, déclaratif en bas, invite d'agent de code, porte des écritures
sur le modèle de run_command, Landlock) ; un témoin commun sur un manifeste sans outil d'écriture. Ordre : primitives d'ED
**devant la mesure 2a′-3** (Lucie l'a demandé), branchement d'archi après (B). Porte des commandes de Lucie passée en
`approval` (o/n dans tty), backends rouverts 15 h 55.
Lucie, 15 h 56 : second essai — le report est bien rendu, mais signé « réponse apportée au superviseur », suivi de « étape faite :
fait : … » et « fait sans plan » : « faut toujours une réponse à l'user ». Pour le socle (archi) : l'invite dit à qui l'équipier
parle (dans une tâche du terminal ou du chat, le superviseur est l'utilisateur), une réponse par tour, le résumé de step_done
reste une ligne d'état ; pour tty : replier étape faite / fini en une ligne discrète. La porte des commandes est déjà dans le
terminal (o/n) ; edit_file écrit dans le worktree de la tâche sans porte (voulu) ; tty-16 = o/n sur une proposition.
Lucie, 15 h 57 : « les étapes faites, je les vois défiler de manière stylée à côté du truc comme quoi ça travaille, plutôt que
l'historique dans mon terminal ; pour un utilisateur comme moi j'ai pas besoin des logs de chaque étape ». Pour tty : les
étapes défilent dans la zone vivante, remplacées à mesure ; l'historique garde la demande et la réponse (et les `!`) ; le
détail à la demande par Ctrl-O et /journal ; réglage show_steps.
Codeparsers, 16 h 01 : fmt/format.h corrigé (macro-format-h 8a68915) — cause : `FMT_BEGIN_NAMESPACE namespace detail {…}` lu par
tree-sitter-cpp comme une function_definition sans function_declarator, prise pour un AnonymousFunction parent de tout ;
correctif : une telle définition n'est plus un scope, son contenu prend le namespace de sa tête pour parent ; témoin sur le
vrai fichier (MIT gardée) ; format.h : scopes 455 → 481, AnonymousFunction 4 → 0, dessous 45 → 0. Limite (grammaire) : class
format_error sous FMT_CLASS_API sans scope, en ticket basse. Un seul lot rag3db avec `new` (rebase sous un nom neuf).
Lucie, 16 h 02 : la vision dite et gardée (`docs/orchestration/visions/15-11-2026-l-agent-est-un-agent-de-code-qui-gere-ses-worktrees.md`) :
un agent de code d'abord, qui gère ses worktrees, dont les recherches sont en phase avec le worktree, qui sait quand l'index
est prêt, qui parle à l'utilisateur. tty, 16 h 02 : la forme des étapes faite en premier morceau de tty-16 (cx-tty-16 27d76c1a0 :
l'historique garde demande et réponse, le reste défile en sourdine sur la ligne du haut de la zone vivante ; --etapes /
/etapes pour tout voir ; crate 99/99) ; un binaire d'essai pour Lucie (`rag3weaver-tty-essai`, bâti sur cx-tty-16, avec /model).
Lucie, 16 h 06 : essai sur le binaire tty-16 — l'accord d'une commande a marché, mais « les accords demandés, je dois pas les voir
défiler comme ça, ça doit être vivant et pas rester dans les lignes de mon terminal » : une demande d'accord est un état de la
zone vivante (commande, o / n), qui disparaît une fois répondue ; l'historique n'en garde qu'une ligne de résultat au plus.
Pour tty, dans tty-16 ; `git status` sur la liste sûre.
96, 16 h 08 : le composeur sur master à **39ce7c1ae** (1 commit, compose_restart + context d'archive, 11 fichiers, gmail, 0 attribution ;
rouge d'environnement du 7879 rejoué vert seul). `usage` au second tour (deux rouges à lire : banc_etage_qui_perd, banc_fiches_classees
rc=1). « À toi » à embarquement (repli + plafond) sur 39ce7c1ae. Codeparsers : lot complet (f1172dac5 new + e8289254e fmt, tête
e8289254e), après lucivy.
Lucie, 16 h 08 : « rien qu'un `ls` me demande permission, et je dois attendre qu'il ait rendu son tour pour accepter » : deux défauts
du produit pour le socle — l'accord dans le tour (l'appel attend, borné), liste sûre large. Mon passage de sa porte en `auto`
a été refusé par ma garde : elle reste en `approval` ; à Lucie si elle le veut.
Lucie, 16 h 10 : « auto par défaut, presque jamais me demander si c'est dans ce que j'ai dit, un modèle décide, pas trop strict »
(ajout à la page de vision). Pour le socle : auto par défaut avec Landlock, un décideur modèle (demande + plan + action →
passe / demander), le doute vers « passe », la sentinelle pour privilèges, hors-bac, réseau ; métrique = demandes inutiles.
Archi : `auto` passe par Landlock et ne peut s'en passer (sûr à poser) ; en auto, ce qui ne touche ni écriture ni réseau passe.
Codeparsers, 16 h 12 : cause des scopes sans DEFINED_IN — l'uuid d'un scope = blake3(fichier:nom:genre:hash(parent+signature)) ; les
constructeurs de JsonMutWrapper ont la même signature reconstruite (« void JsonMutWrapper() », paramètres perdus, void inventé)
donc le même uuid ; rag3weaver (code.rs:1494) garde la dernière clé par uuid, toutes les arêtes des homonymes se rabattent sur
elle. Correctifs (a) rang d'apparition dans l'uuid à partir du second homonyme, (b) la signature d'un constructeur C++ garde
ses paramètres. Accordé ; la version de l'analyse monte ; compte avant/après sur rag3weaver et la copie de code-lourd.
Embarquement, 16 h 12 : pile reposée (pile-embarquement-r1 e69c84e63, 14 commits), rejeu depuis 16 h 09 sous timeout 7200 ; push au
vert. Hotfixs **prêt** (b940cefb8, 11 commits ; shell test vert) : à lui au « poussé » d'embarquement, puis la fenêtre de formatage.
tty-16 : trois commits (l'accord vivant 2504c33e4 : l'animation recouvrait « attend ton accord ») ; binaire d'essai reposé 16 h 09.
Lucie, 16 h 12 : « insupportable, j'ai dû faire 15 fois oui » — l'équipier croit edit_file inactif et écrit par printf | tee, cp,
git diff --no-index, python3 -c, chaque commande à la porte. Archi lit le run (edit_file dans les defs ? pourquoi ces commandes
demandent ? correctif minimal aujourd'hui).
Archi, 16 h 14 : les faits du run de Lucie — edit_file **était** dans les outils et le modèle l'a appelé ; l'outil a refusé (« l'arbre
principal est en lecture seule ; l'agent écrit dans une tâche (POST /ide/taches…) »), ReadOnly(source, task_writes) : le backend
de la tâche a ouvert sans sa surcharge d'écriture. **Cause probable, de ma main** : mes close/open par /backend/open sur le
manifeste de la tâche (15 h 53, 15 h 55) ont rouvert un backend nu, sans la surcharge (read_only = !ecrit, root). Le modèle a
répété le refus et s'est rabattu sur printf | tee, cp, python3 -c (qui demandent : python3 hors liste sûre, tee écrit). Second
défaut : sous Landlock, le worktree de la tâche vit dans le .git de Lucie que le bac ne laisse pas lire (git status rend 128),
d'où le `git -c core.excludesFile` inventé. À faire : rouvrir le backend de la tâche par le bon geste ; le bac lit le .git
commun ; le refus de ReadOnly dans une tâche ne conseille plus POST /ide/taches.
Archi : le lot (B) de l'équipier **vert** (une-demande-attend-une-ligne a1a74f661 ; 35 témoins, lib 2470, hooks 45, e2e_equipier
sous Vertex 2/2) ; place après codeparsers, avant tty-14. Codeparsers : (a) seul dans le lot DEFINED_IN ; (b) les paramètres par
pointeur/référence perdus pour toute fonction C++, lot à part avec banc, ticket.
Archi, 16 h 15 : correction — mon fermer-rouvrir n'y est pour rien (/backend/open passe par open_slot, la surcharge tient : root =
essai-tty/arbre). La cause : `workspace.ecrit` d'une tâche est calculé **une fois à sa création** (taches.rs:526, sur le manifeste
parent du moment) et figé au registre ; essai-tty est née le 14 à 18 h 34 sans task_writes, donc ecrit = false, donc
read_only. Fait : une tâche neuve **atelier** (POST /ide/taches, branche agent/atelier, backend 2fb80390ed75, ecrit = true) pour
Lucie. Pour le socle : (d) ecrit recalculé à chaque ouverture ; (b) le bac lit le .git commun ; (c) le message de ReadOnly.
G, 16 h 20 : défaut de plafond-menage — `cargo clean -p` refuse un target sans CACHEDIR.TAG (les nôtres naissent de la garde), et le
ménage échouait en silence : les targets grossissaient (33 Go chez archi). Correctif plafond-cachedir 78789cd89 (la garde pose
l'étiquette, un clean qui échoue le dit ; 22 677 → 4 209 Mio sur un vrai target), rouge d'abord. Place : juste après
embarquement, avant le lot CI de hotfixs. Ordre : embarquement → G (cachedir) → CI → format → lucivy → codeparsers → archi (B) →
tty-14/15/16 → ED (1) → G (hashes, known_red) → arbre.
Cœur, 16 h 20 : les ×10 de DdlEveryMinute sur ee598e074 : **0 SIGSEGV sur 10** (4 sur 12 avant), 2 tours jusqu'aux comptes justes, 8
arrêtés par le pool plein (« Cannot find property ») — les Note attendent le ticket du pool. **ALERTE** : tour 7, après un
CHECKPOINT réussi qui suit des DDL refusés par le pool, les 20 écrivains reçoivent « Schema changed, retry: » sans fin (75 s
jusqu'à l'arrêt) : un état du catalogue pris pour « changé » à jamais, possiblement le lot du verrou de schéma (101ff50a6, sur
master). Sonde hors dépôt (P1–P4) en cours, verdict dans 20 min ; si c'est le lot, correctif devant la fenêtre de formatage.
Les hôtes de Lucie sont sur 373b5477b : non exposés.
96, 16 h 24 : `usage` (1c85fa06e) vert au compte juste (lib 2465/0 ; archiviste 21/21 rejoué seul après le rouge d'environnement
du 7879 ; compose, memoire_modele, idempotent_registration verts ; banc_fiches_classees NON JOUÉ, banc à services) ;
banc_etage_qui_perd rouge sur master aussi : 8a931f78d (11 nov.) refuse un changement de search_index sur une entité peuplée
sans migration déclarée, le banc réenregistre Scope sur la base indexée — ticket chez 96, rien à relâcher. query_chars :
rouge d'abord poussé (c0e9d8f1b). Arbre : répétition lancée 16 h 20 (binaire figé b2635b1b0, montée 6/6), en file derrière 4 lourds.
Embarquement, 16 h 30 : la pile repli + plafond sur master à **73aa05511** (15 commits, gmail, 0 attribution ; passe sur f8b553928 toute
verte : embarqueur à la demande, lectures 27, code 36, features 8/8, lib 2 495, navigateur 45 ; un premier rouge de check_features
sur une famille de ticket « mesure » hors vocabulaire, corrigée et passe rejouée). « À toi » à G pour plafond-cachedir. 96 : query_chars
rouge vu (aveu : le rouge a envoyé 40 000 caractères au vrai 7879, qui a tenu ; règle écrite), correctif cfa6d22fb en vert.
G, 16 h 33 : plafond-cachedir sur master à **2e11d5345** (2 commits, gmail, 0 attribution ; garde_target.sh n'a pas de copie au cache :
run_e2e.sh le source depuis l'arbre de chaque passe, les passes sur une base ≥ 2e11d5345 ont le ménage corrigé). « À toi » à
hotfixs pour le lot CI (b940cefb8). Cœur : la sonde ne tranche pas (les quatre cas courts passent) ; contrôle de schéma
instrumenté sur stderr, DdlEveryMinute jusqu'au premier tour bloqué (~1 h).
Embarquement, 16 h 34 : la sortie muette du 7879 — une panique de calcul (réservation refusée) tue un fil sans réponse, le serveur de
la carte reste empoisonné, les quatre fils meurent tour à tour, `servir` rend Ok et main sort par SUCCESS. Lot du 503 nommé :
catch_unwind → 503 nommé ; après une panique, arrêt dit avec le code 70 (après avoir répondu aux requêtes en vol) ; plafond par
texte en caractères (`max_chars_per_text`, défaut 12 000 ≈ 4 000 jetons, 413 nommé), (b) en jetons en ticket. Tranché (a).
Hotfixs, 16 h 35 : le lot CI sur master à **fa15e5fa3** (13 commits dont 11 du lot, gmail, 0 attribution ; PR 2 fermée). Ménage des runs,
avec l'autorisation de Lucie : 43724738336 annulé, les quatre du 11 novembre « completed » à l'API (zombies de la liste) ;
**le run 43730682329 (a6d49b99b) est en cours — le premier run de master qui joue depuis le 14 à 15 h 27**. « À toi » à hotfixs
pour la fenêtre de formatage (cherry-pick -n 644d1d8c5, commit gmail, bâti + un test par cible).
Recherche, 16 h 38 : sa part de 2b-C verte (vues-2b-c-2 97b8c8d7d, sur 2b′ 77cd6b049 ; `view = {layer, rank?}` sur tout outil qui lit,
composé dans l'instantané de l'appel ; passe entière verte sous MOTEUR_ANCIEN dit). En pile derrière 2b′, en fin de file ; le
témoin 0 au bit près du banc sur sa tête conditionne le « à toi ».
Codeparsers, 16 h 49 : (a) vert (defined-in-homonymes d024118) : sur rag3weaver/src, **scopes sans DEFINED_IN 65 → 0**, 16 222 scopes
inchangés ; l'arbre entier (code-lourd, third_party compris) en passe confinée par lots de 300. REGLES_DE_L_ANALYSE n'est pas
encore sur master (elle arrive avec l'écrivain d'ED) : le pointeur DEFINED_IN fait un troisième lot après ED (1), avec la montée.
96, 16 h 49 : pile du second tour prête (cbe5ba25c : usage, query_chars, variante context_title, description de trace), après tty-16.
Hotfixs, 16 h 55 : **la fenêtre de formatage est close** — un commit pur sous gmail, 9ea2e2408, 25 fichiers (+191/−155), prouvé format pur
deux fois ; bâti Release vert et une suite par cible touchée, toutes vertes. Le run de master 43730682329 (a6d49b99b) a rendu
l'attendu : sanity, python lint, shell test verts, format rouge ; le run de 9ea2e2408 doit être le premier tout vert, et le
ticket de la CI (ouvert depuis le 25 octobre) se fermera dessus. « À toi » à lucivy pour le relevé à blanc.
Banc, 16 h 57 : **le témoin 0 de 2b′/2b-C est vert au bit près** (97b8c8d7d contre 77cd6b049) : sans `view`, mêmes réponses que le
parent ; avec `view: {layer: backend}`, mêmes réponses et `view = {backend, 0, 1}` rendu. La condition du « à toi » d'arbre et de
recherche est remplie ; en fin de file.
ED, 17 h 02 : **les primitives du socle sont vertes** (socle-agent-de-code 7d43f06cf, sur e0b3725ee, indépendante de l'écrivain ; passe
épinglée verte) : write et glob neufs (rouges « unexposed tool »), read_file 200 lignes, replace_text (old unique), replace_lines
et insert_lines (empreinte exigée), grep, list ; diff unifié et réindexation à chaque écriture. Réserves : le run par écriture
à vérifier ; l'empreinte tenue par le chat, non affichée. Place : après codeparsers, avant archi (B). Ordre : lucivy →
codeparsers → ED (socle) → archi (B) → archi (branchement) → tty-14/15/16 → 96 → ED (écrivain) → G → arbre + recherche.
Lucivy, 17 h 02 : le relevé à blanc sur master à **a8b3b25bb** (gmail, 0 attribution ; rejeu vert sous timeout). « À toi » à codeparsers.
CI de master : chaque push annule le run en attente d'avant (un seul en attente par groupe) ; avec un push de docs toutes les
quelques minutes, un run de 20 min ne joue que dans un creux — lot pour hotfixs : `paths-ignore` des docs sur le déclencheur
de master (un push de docs seules ne déclenche pas le CI Workflow).
Hotfixs, 17 h 04 : lot ci-docs-sans-run prêt (94e3a484f) : paths-ignore des dossiers de docs et des .md de la racine sur le push de
master (pas '**/*.md' : des .md de templates/ sont compilés dans le binaire par include_str!) ; le workflow rag3weaver reçoit
des exclusions dans ses paths ; ticket qui se ferme au premier run de code qui joue jusqu'au bout malgré des pushes de docs.
Place après ED (socle).
Embarquement, 17 h 05 : le 503 nommé du service vert, rouge d'abord (service-503 : e9874c1f8 rouges, 20487cfb7 correctif, 8a3af2b66 docs) :
une panique de calcul répond 503 nommé, le poison noté sous le verrou de la passe (une vraie course trouvée par le témoin
réécrit), les requêtes en vol toutes répondues, puis arrêt dit avec le code 70 (jamais Ok) ; plafond par texte 12 000 caractères
(413 nommé) sur les quatre routes ; l'activation dit la cause lue au journal. Ticket du 7879 fermé ; deux ouverts (plafond en
jetons ; cause d'arrêt d'un service posé à la main). Lib 2 373. Place : après hotfixs (ci-docs), avant archi (branchement) ;
le binaire du service à poser au 7879 et 7878 par G après l'atterrissage.
Banc, 17 h 13 : **l'écrivain bff6c7409 n'a pas le vert du juge étendu** — réponses 30/30 au bit près, mais le graphe : DEFINED_IN en double
sur 72 798 couples ouverts (146 138 contre 73 340 au parent), CONSUMES −4 084, CONSUMED_BY −3 857 (miroir rompu), MENTIONS −983.
Le cas visé : réponses égales, graphe différent. Retenu ; témoin 16 (comptes par étiquette vers le visible = parent) rouge
d'abord ; le grain (d57b28c80) rejugé sur la même base. Repassage : parent 243 s, bff6c7409 2 476 s. La répétition d'arbre mesure
la part fermée seule en attendant.
Archi, 17 h 13 : **le côté « pas » du socle est vert** (socle-le-pas d2e10d6e1, sur (B) ; 6 rouges vus deux fois ; témoins 65/0, lib 2 476,
hooks et navigateur 45, e2e_equipier Vertex 2/0) : (d) ecrit relu sur le parent à chaque ouverture ; (c) le refus dans une tâche
dit la vraie cause, sans POST /ide/taches ; (b) le bac git lit info/ du dossier commun et pose core.excludesFile=/dev/null (la
vraie cause du 128 : ~/.config/git/ignore illisible sous le bac) ; l'invite d'un backend de code commence par celle d'agent de
code ; l'équipier répond à l'utilisateur une fois par tour ; run_command dit git status/diff/log nus ; le déclaratif était déjà
en bas. En pile sur (B), même « à toi ».
Codeparsers, 17 h 19 : `new` + fmt/format.h sur master à **6c8546763** (pointeur codeparsers 5f7c837 ; e2e_code 55/55, check_features 8/8 ;
gmail, 0 attribution ; le banc de l'arbre entier relancé sur un worktree à lui). « À toi » à ED pour les primitives du socle
(8ef56dd5e) sur 50ed5a677. ED : les deux défauts de l'écrivain lus (la recopie des arêtes entrantes copie vers le File neuf ce que
l'ingestion repose déjà ; une cible née au rang en attente a un uuid à couche et rang, l'uuid nu ne vise rien) ; témoins rouges,
correctif, banc de nouveau ; pas ce soir.
Recherche et lucivy, 17 h 20 : le poids du plein texte du dépôt — **un seul fichier fait 57,6 % du texte**, le bundle de l'IDE
(templates/ide/app/app.js, une ligne de 133 Ko : 1 581 documents, 98 Mo de doublons, chaque scope recevant la ligne entière) ;
l'index ferait 0,76 Go au lieu de 1,39 sans lui. Le manifeste de Lucie n'avait pas `index_exclude`. Fait : `index_exclude
{minified_line_max_bytes: 5000, code_file_max_bytes: 1048576}` posé sur les trois manifestes (dépôt, code-lourd, code-principal ;
sauvegardes), backend du dépôt rouvert ; réindexation et compact dans la fenêtre d'arbre. Ticket codeparsers : le contenu d'un
scope doit être sa portion de ligne.
Codeparsers, 17 h 24 : le second chiffre (arbre entier) résiste — trois passes confinées coupées par un SIGTERM 2-3 min après le début,
sans cause trouvée ; décision : le banc le mesure sur sa copie de code-lourd avec sa sonde (3 814 avant) après la pose du lot ;
le SIGTERM en ticket (piste : l'enveloppe mémoire du confineur ou de poste). Recherche : témoin « exclusion après coup »
(réindexer sous la règle retire un fichier déjà indexé) en cours avant la fenêtre d'arbre.
Codeparsers et G, 17 h 26 : le ticket des trois SIGTERM (16 h 53, 16 h 59, 17 h 23) rédigé ; piste : OOMPolicy=stop sur la portée
`systemd-run --scope -p MemoryMax` de poste lourd, ni confirmée ni infirmée (journal systemd muet). Lot court à G : le confineur
écrit memory.peak et memory.events de sa portée quand il reçoit un signal, et dit « enveloppe mémoire crevée » au lieu de
« signal reçu : 15 ».
Recherche, 17 h 30 : l'exclusion après coup prouvée (771eb216b : app.js indexé sans règle, réindexé sous index_exclude → 0 scope et
autant de documents que de lignes) ; une garde, pas un correctif, en fin de file avec 2b-C. La fenêtre d'arbre peut compter sur
le retrait.
G, 17 h 31 : la cause des trois SIGTERM **confirmée** sur une vraie portée (systemd-run --scope -p MemoryMax, DefaultOOMPolicy=stop :
le noyau tue l'enfant par SIGKILL, systemd arrête la portée par TERM, journal muet). Lot confineur-enveloppe 541d282e5 : au
signal, le confineur lit memory.peak et memory.events et dit « enveloppe mémoire crevée (pic, plafond, oom_kill) » ; 3 rouges
vus ; ticket de codeparsers fermé dans le lot. En pile derrière les hashes et known_red.
ED, 17 h 32 : **le socle d'agent de code (les primitives) sur master à 7b2f019e0** (6 commits, 20cda3bc5 → 7b2f019e0 : write et glob
neufs, le run d'une écriture vérifié, nouveauté ; gmail, 0 attribution ; rejeu vert sur e4a14c555). « À toi » à archi pour (B) +
le pas en pile sur 7b2f019e0. L'écrivain : témoins 16 et 17 rouges d'abord, deux correctifs posés (la recopie hors du lot ; la
résolution des symboles bornée aux définisseurs visibles — une ligne fermée gardait son DEFINES et rendait le nom ambigu),
rejeu puis banc. Codeparsers : (b) vert (parametres-cpp ff82d22 sur d024118) : paramètres 6 998 → 16 934, références −11,7 %,
37 % des signatures C++ changent ; réserve à lever (aucune vraie référence perdue) ; les deux pointeurs montent ensemble.

## 17 h 33 — la CI de master est verte

Hotfixs : le run 43733500502 (sur 6e94ff627, le premier run fini qui contient le format) passe sanity checks, python lint, code
format et shell test, les self-hosted sautés : **le premier « CI Workflow » tout vert sur master depuis le 18 octobre**. Le
ticket de la CI, ouvert le 25 octobre, est fermé (9db8474e9). Reste le lot ci-docs-sans-run (un push de docs annule le run en
attente), après archi.
Codeparsers, 17 h 33 : réserve de (b) levée — sur src/ du moteur, 22 213 références perdues dont 22 177 noms de paramètres du même scope
(voulu) et 36 accès à un membre via une variable locale ; aucune vraie référence ne part ; 696 types de paramètres gagnés.
Arbre, 17 h 33 : répétition relancée à 17 h 32 (la première avait rougi à 0 s sur des chemins relatifs du manifeste copié, non vu
par son attente ; corrigé, attente qui guette tout rouge) : par base, banc d'avant 8-9 s, purge (103 et 102 orphelins) + compact 8 s,
1b en cours.
Codeparsers, 17 h 35 : lot homonymes + paramètres C++ monté (d1c7961c7 : pointeur 5f7c837 → ff82d22, deux tickets fermés, nouveauté « toutes
les bases repassent une fois ») ; tranché (i) : pas de montée de REGLES_DE_L_ANALYSE sans règle changée (le pointeur entre déjà
dans la version). Après l'écrivain d'ED, passe complète au rejeu. Ensuite la famille de la résolution d'arêtes.
G, 17 h 36 : la pose du soir en préparation (copie d'essai de l'hôte sur b82af880a, lib 373b5477b ; l'essai du service d'embarquement dès
service-503 sur master). Ce que la chaîne attend : le hash de master à l'atterrissage des quatre lots (~19 h ; bâti 15 min, verdict
des trois suites 40 min : pose vers 20 h) ; une fenêtre sans passe qui lit les services (7878/7879 : 10 à 30 s par service, l'hôte
~1 min par poste), « va » du banc et d'arbre avant ; lib inchangée sauf verdict du cœur. Trou pour `/model` : le manifeste du
dépôt de Lucie n'a pas de `models.llm` (models: {sparse} seul) — ED donne le bloc, je le pose, G vérifie par check-manifests.
Codeparsers, 17 h 36 : l'ordre de la résolution d'arêtes, par ce qu'une réponse fausse coûte à Lucie (Rust et C++ du moteur) : 1 marque
nom (un appel de méthode standard relié à un homonyme d'une autre crate ; 1 949 receveurs « nom » sur 3 026 visent un nom std ;
règle : un type lu mais hors de l'index s'abstient et le dit), 2 receveurs sans type, 3 définition liée à l'en-tête C++, 4 membre
platform JS ; 5 import ES externe (réponse incomplète, +6,5 à +9,2 % de scopes) à porter à Lucie. Accepté ; 1 commencé en rouge.
ED et c7, 17 h 39 : `models.llm` posé dans le manifeste du dépôt de Lucie et la copie de sa tâche atelier (gemini-3.8-flash par Vertex,
compatible/vertex, location global, restart_budget_tokens 100 000, défaut ; sans context_tokens, non vérifié pour la 3.8 ;
VERTEX_MODEL absent de l'hôte), backends rouverts : `/model` aura sa liste à la pose du soir. Défaut du lot models.llm trouvé :
budget_de_reprise reconnaît le local par le protocole « llama_server » que connect_llm refuse — ticket (le local par l'adresse de
boucle). Archi : le branchement greffe dans toute tâche read, list, grep, glob, replace_text, **replace_lines**, insert_lines,
write (si droit d'écrire) et run_command (si porte ouverte) ; l'invite dit « un texte unique, ou les lignes a à b après les avoir
lues » (Lucie : « aussi par offset ») ; deux témoins sur trois verts, le bout en bout rejoué derrière la pile.
Lucie, 17 h 41 : « il ne sait pas lancer de sous-agent ni leur parler en temps réel » (test de son harnais : « tu sais lancer un
sous-agent qui rajoute des lignes à ton md ? » → l'équipier dit, à raison, qu'il n'a aucune primitive de délégation). Pour archi,
après le branchement : un outil `delegate {nom, demande, expects}` (sous-tâche parent_task, worktree et branche forkés, un
équipier lancé, dialogue en temps réel par la boîte : reports et questions du sous-agent chez le parent, `reply`, `stop`, `wait`,
fusion ou proposition à la fin) ; dans tty, les sous-agents en zone vivante. Page avant le code.
Lucie, 17 h 44 : sur le binaire d'essai, tâche atelier : « et j'ai plus la petite animation de chargement :/ » — tty garde l'animation
du thème à côté de la dernière étape en sourdine. Embarquement : deux commits de plus sur service-503 (le binaire ne lisait pas
RAG3WEAVER_EMBED_MAX_CHARS ; rouge d'abord, corrigé, refus nommé d'une valeur invalide).
Arbre, 17 h 45 : **la part fermée de la répétition est verte** sur les deux copies (luciepc, binaire b2635b1b0, lib 373b5477b, en
parallèle) : code-lourd — purge + compact 8 s, 1b 700 s (82 855 occurrences → 76 920 versions, 123 957 morceaux, dette 0), montée
1 s : ~12 min ; code-principal — 8 s, 532 s (65 914 → 61 224), 1 s : ~9 min ; mémoire 4,6 et 8,7 Gio. Le dépôt : 861 s de 1b le 14.
La fenêtre fermée de ce soir : ~15 min par base, en parallèle. Le repassage a rougi en 1 s (l'outil index exige confirm=true
au-delà de 5 min de vecteurs estimés, ~10 annoncées — à vérifier contre ED : les vecteurs ne se recalculent pas) ; repris avec confirm.
Lucie, 17 h 45 : « pourquoi ça m'écrit "attend", c'est horrible, on doit jamais voir "attend" comme ça : que des animations de
chargement ou bien des trucs vivants ». Règle pour tty : jamais un mot d'état nu sur la ligne d'état ; animation du thème au
travail, invite vivante au repos ; un équipier fini sans réponse = une phrase à Lucie, pas un mot machine.
ED, 17 h 46 : l'estimation de l'outil index compte la source entière fois le rythme (estimate_of), sans regarder ce qui est embarqué —
défaut d'avant son lot, visible à toute réindexation, ticket à ED (lire la dette réelle) ; le repassage réembarque peu : scopes
identiques sans vecteur neuf, ~7 600 petits morceaux de File (File_CHUNKED_FROM 7 608 → 15 218), quelques dizaines de secondes —
un coût de son lot, à faire tomber (un File garde son morceau quand seule son analyse change). confirm=true est juste.
Lucie, 17 h 48 : « "fait sans plan" veut rien dire pour l'user, il s'en cale » — aucun verdict de l'hôte à l'écran (fait sans plan,
fini, étape faite, comptes de pas) ; la réponse et une zone vivante qui respire ; les mots de l'hôte vont au journal et à Ctrl-O.
Pour tty, même morceau. Son essai de 17 h 4x : l'équipier a cherché, lu et répondu juste (teammate.rs, declare.rs, teammate_tools.rs,
le manifeste) — deux comptes rendus pour une question, à ramener à un par l'invite du socle.
Lucie, 17 h 49 : « les étapes "attend" : je veux voir l'animation et comprendre ce qu'il attend exactement (quota ou quoi que ce
soit, l'indication à côté de l'animation — voire que ça fasse partie de l'animation, des glitchs ou selon le thème) ». Pour tty :
l'animation + la cause nommée dans le style du thème ; pour archi et ED : chaque attente de la boucle dite dans la boîte
(`waiting {cause, since, retry_in?}` : modèle, quota et réessai, outil, index, accord), effacée quand ça repart.
Archi, 17 h 50 : **(B) + le côté pas du socle sur master à a50ea65c35** (6 commits, b4c7b88a8 → a50ea65c35 : une demande où l'équipier n'a
rien laissé est relancée une fois puis attend le superviseur au plus teammate.reply_wait_s, échec nommé ; le droit d'écrire relu,
le bac lit le .git commun, le refus dit sa cause, l'invite parle à l'utilisateur ; gmail, 0 attribution). « À toi » à hotfixs
pour ci-docs-sans-run. ED : l'écrivain corrigé vert à la passe entière (c5ec47cc4 : recopie hors du lot, résolution bornée aux
lignes visibles — la vraie cause des CONSUMES perdus —, pas de recopie d'une arête déjà posée) ; le banc rejuge sur une P0 neuve.
Lucie, 17 h 50 : « les réponses du modèle, on les a bien en streaming et pas d'un seul coup ? » — non : la boucle de l'équipier rend
son texte par l'appel d'outil `report` (arguments entiers), et ni teammate.rs ni le crate tty ne traitent de deltas. Lot pour archi
(le texte libre du modèle streamé vers la boîte par `partial`) et tty (affichage mot à mot), après le branchement. Archi : le
branchement vert (68/0, bout en bout compris ; le pas de l'équipier a maintenant l'empreinte de lecture, un replace_lines sans
lecture écrivait), reposé sur master, passe complète ; place après hotfixs et embarquement.
Hotfixs, 17 h 51 : ci-docs-sans-run sur master à **6c59d5c78** (un commit, gmail, 0 attribution ; le bandeau de poste push dit CI Workflow
et rag3weaver verts). La preuve du filtre : ce push de docs-ci ne doit lancer aucun run. « À toi » à embarquement (service-503).
Ordre restant : embarquement → archi (branchement) → tty-14/15/16 → 96 → ED (écrivain, au vert du banc) + arbre (migration) → G
(hashes, known_red, confineur) → arbre 2b′ + recherche 2b-C + exclusion → codeparsers (pointeurs).
17 h 52 : **le filtre des docs est prouvé** — mon push de docs 3b4ba3cb7 n'a lancé aucun run ; les runs de 6c59d5c78 jouent, celui de
7b2f019e0 (rag3weaver) n'a pas été annulé. Archi : le texte libre du modèle **est** streamé (Event::Token sur `tokens`, run du pas,
tty abonné) ; seul l'argument de `report` arrive entier → validé : la réponse de l'équipier en texte libre, `report` pour la
boîte seulement ; phrase d'invite + témoin dans le branchement ; tty affiche mot à mot. Arbre : la montée reposée sur l'écrivain
corrigé (migration-generations-2 4b28027c8) ; répétition complète relancée sur c5ec47cc4 avec des copies neuves.
G, 17 h 52 : copies d'essai de la pose du soir faites — l'hôte (b82af880a, 373b5477b) vert aux deux postes sur 7991 (bâti final incrémental
~5 min) ; le binaire d'embarquement (daemon, burn-embedder, burn-ocr) bâti vert, mais pas d'essai à côté possible (burn en CPU
panique sur un tampon de 384 Mo ; card2 interdite aux essais) : la preuve se fera en place, 7878 puis 7879, vecteur témoin au bit
près, retour automatique à l'ancien si rouge ; coupure 10 à 30 s par service ; le défaut 12 000 de max_chars vaudra. Un ticket
d'archi (gabarits d'un autre arbre au target quand build.rs n'est pas rejoué), remède chez G plus tard.
Lucie, 17 h 56 : « essaie chacun de tes outils » — l'équipier a testé lecture, recherche, graphe, déclarations, run_command, et
trouvé deux restrictions, toutes deux connues (git status refusé sur ~/.config/git/ignore sous le bac : corrigé par le pas du
socle sur master, pas encore posé ; echo soumis à l'accord : la porte en approbation). Défaut d'affichage : la réponse streamée,
puis sa copie par l'hôte (« le pas 2 s'est terminé sans step_done ; sa réponse : … »), puis un second compte rendu : trois fois
le même bilan ; et « fini : plus rien à faire » encore sur la ligne d'état. Pour tty et archi.
G, 17 h 57 : quatrième lot prêt, ticket-gabarits-autre-arbre 4085128d4 (la garde lit les sorties des build scripts et retire une sortie
qui vise un autre worktree ; 8 cas neufs, test_garde_target 35 OK, 34 ms sur un target de 32 Go ; ferme le ticket d'archi). Sa file :
hashes entiers → known-red-cas-longs → confineur-enveloppe → gabarits-autre-arbre, scripts seuls, après la pose du soir.
Lucivy, 17 h 58 : lot suivant — la maintenance périodique par l'hôte (`fts.maintenance {at: "03:30", max_purge_ratio: 0,05, compact,
enabled}`, relevé à blanc puis maintain sur chaque base ouverte hors de toute passe, refus nommé au-delà du ratio ou sous écriture
continue, résultat à l'état d'index et au journal ; rouge d'abord sur e2e_demon_hote avec une horloge injectable). Tranché 0,05.
Lucie, 17 h 59 : « je veux qu'on puisse donner des noms aux agents, et ce premier-là, je voudrais qu'il s'appelle **Algareth** »
(un hommage à une période à elle, où elle avait les mêmes ambitions d'agents de code sans encore connaître le sujet). Pour le
socle : `agent.name` déclaré (backend, surchargeable par tâche), l'invite qui dit son nom à l'équipier, la boîte et le journal au
nom, la fiche d'identité `agent:<clé>` qui le porte, tty qui l'affiche et `/nom`. Dès que la clé existe, posée dans son manifeste.
Archi, 18 h 01 : la forme du nom — l'invite « You are <name> », présentation une fois au premier pas ; dans la boîte le rôle reste
`from: teammate` (toutes les gardes le lisent), le nom s'ajoute en `name` sur chaque ligne de l'équipier et au journal du run ;
(1) la clé au manifeste chez ED, (4) le libellé de la fiche agent:<clé> chez 96, (5) l'affichage chez tty. tty : binaire d'essai
reposé 18 h 00 — un compte rendu égal au texte streamé ne se réaffiche pas, le préfixe « le pas N s'est terminé sans step_done »
jamais ; le binaire d'essai n'a plus « fini : » (l'écran de Lucie venait d'un binaire d'avant 17 h 54).
ED, 18 h 03 : le nom de l'agent, part (1), pris tout de suite (nom-de-l-agent sur bf187d9e6 ; `agent: {name}` au manifeste, défaut
« équipier », refus nommé si vide ou multiligne ; la tâche surcharge par le registre ; PreparedBackend::nom_de_l_agent()) ; la
création nommée d'une tâche (POST /ide/taches {agent_name}) dans le même lot. Place après 96, avant l'écrivain. `agent` ne se pose
dans le manifeste de Lucie qu'après l'atterrissage (deny_unknown_fields avant).
Lucie, 18 h 04 : « et une persona, un personnage assez bref à incarner, comme un ton de réponse ? » — `agent.persona` à côté de
`agent.name` (texte court, plafond réglé, défaut vide, surcharge par tâche) ; dans l'invite après le nom, après les règles du socle,
pour colorer le ton sans défaire une règle. Même lot, chez ED et archi.
Embarquement, 18 h 06 : **le 503 nommé du service sur master à f85977d09e** (5 commits, gmail, 0 attribution ; passe sur 2dfcdad5f :
embarqueur à la demande, lectures 27, code 36, features 8/8, lib 2 516, navigateur 45). « À toi » à archi pour le branchement dès sa
passe verte ; G bâtit le binaire du service sur f85977d09e. Restent avant le hash de l'hôte : tty-14/15/16, 96, ED (nom + persona).
Lucie, 18 h 06 : la persona « peut contenir des motivations, "tu as goût aux abstractions" ; utile ou technique sur les bords, "tu as goût
aux beaux paradigmes et abstractions" » — un personnage avec ses motivations et ses goûts techniques, présenté ainsi dans l'invite
après les règles du socle, qui peut peser sur les choix de conception sans primer sur la demande ni une règle.
Archi, 18 h 06 : le branchement n'est pas vert — un cas d'e2e_equipier sous Gemini (l'équipier rend le README dans un report au lieu
d'écrire README.md), qui passait sur (B) + pas : la liste d'outils plus longue ou la phrase de l'invite ; rejoué deux fois pour
départager ; « write your answer as text » et son témoin encore à poser ; repose sur f85977d09e ensuite. Les autres passent devant :
« à toi » à tty pour tty-14, puis 15 et 16 à la suite, sans second mot.
tty, 18 h 19 : **tty-14 (/model) sur master à e9e47391a** (14 commits, ece931354 → e9e47391a : GET /ide/teammate/models, POST
/ide/teammate/model, le registre écrit du chat, un choix qui ne désigne plus rien se voit ; gmail, 0 attribution). tty-15 puis
tty-16 suivent sans second mot. G : le binaire du service bâti sur f85977d09e (sha a70f1e46…), non posé.
Lucivy, 18 h 21 : la maintenance nocturne verte (59dfa40b4 : `fts.maintenance` au manifeste, un fil demon-entretien à la minute, par
backend ouvert et dû ; relevé à blanc, purge si ≤ 0,05 sinon refus dit, compaction ; note aux métas et à l'état d'index ; rouges
sur bouchon, e2e_demon_hote 26/26). Place après ED (nom + persona), avant le hash de la pose : première maintenance cette nuit.
Ordre restant : tty-15 → tty-16 → 96 → ED (nom) → lucivy → **hash** → archi (branchement, à son vert) → ED (écrivain) + arbre →
G ×4 → arbre 2b′ + recherche → codeparsers.
tty, 18 h 24 : correction sur tty-14 — son rejeu du crate partageait un target entre worktrees (target-cx-tty-6) : cargo a rejoué un
binaire de test bâti depuis l'arbre de tty-15 (un test de tty-15 au journal de tty-14, aucune ligne Compiling). Tiennent : la lib
entière et check_features dans leur target ; master rejoué dans un target à lui : crate 97 + 19 vert. Le script de rejeu a
maintenant un target par worktree et rougit si le crate n'a pas été compilé depuis l'arbre du lot ; tty-15 rejoué ainsi.
tty, 18 h 26 : **tty-15 sur master à 677d361ad** (7 commits : sous un tmux qui permet les touches étendues en CSI u, le terminal demande
le mode 1 ; l'indication garde son début ; les nouveautés retrouvent leurs lignes vides ; gmail, 0 attribution). tty-16 suit.
Arbre, 18 h 30 : la répétition sur c5ec47cc4 verte au banc, mais le repassage n'a pas tout joué (le pilote lisait `busy` comme une fin :
code-lourd arrêté après 1 paquet sur 4) et code-principal a échoué sur un index arrière faux de LOCKS (les instantanés de la nuit
sont d'avant la réparation de hotfixs). Rejouée depuis 18 h 28 : copies neuves réparées avant 1b, attente sur la dernière ligne du
journal. Part fermée confirmée : 731 s et 569 s. Garde pour la vraie fenêtre : check_rel_directions avant 1b sur chaque base.
Repassage complet ~35 min à code-lourd.
Embarquement, 18 h 31 : conception du repli par activation nommée — recette comparée au chargement moins le placement (périphérique,
variables de carte ; aucun arg), recette complète attestée à l'activation du repli ; tranché : la carte du repli se mesure aussi
(required_mib propres, défaut ceux de la principale), les deux pleines = principale quand même et l'état le dit ; le plafond du
doublement de la tenue est un réglage (défaut 16). Six témoins rouges.
Hotfixs, 18 h 32 : le ticket « un push de docs annule le run de code » fermé (4ad76536e) : le run de f85977d09 est allé au bout, vert
sur les quatre jobs, dix commits de docs derrière sans lancer un run ; GitHub juge le filtre sur tout le push. **La CI de master est
verte et rend un verdict à chaque push de code.** Lot suivant pour hotfixs : une base qui s'ouvre sans dire qu'une extension n'a pas
pu être chargée (perte d'index, atteignable), après le verdict du cœur.
ED, 18 h 34 : le nom et la persona de l'agent verts (nom-de-l-agent 911ef78ee ; passe épinglée verte ; deux témoins) ; réserve dite : leurs
rouges n'étaient que de compilation — demandé : les faire rougir sur une méthode faussée avant le push. Embarquement : l'adresse du
repli dans le bloc d'activation (`address`), jamais card0 dans un exemple.
Hotfixs, 18 h 35 : l'extension non chargée — les .extensions des bases vivantes visent déjà 373b5477b, mais à chaque bascule de lib la
première ouverture refuse l'extension de la lib d'avant (extension_error de SHOW_INDEXES, jamais lu par rag3weaver), l'index est
détaché et rebâti en entier en silence, attribué à tort à un arrêt brutal, rien au reçu ni à l'état. Correctif en Rust (lire
extension_error, avertissement nommé au reçu et à l'état, vraie cause du rebâti), témoin rouge d'abord ; après les lots du soir.
tty, 18 h 40 : **tty-16 sur master à 96a05586b** (13 commits : l'historique ne garde que la demande et la réponse, les étapes en sourdine,
l'accord vivant avec boutons, o/n sur une proposition, jamais un mot d'état nu ni un verdict de l'hôte, animation dès le départ et de
repos, la réponse affichée une fois ; gmail, 0 attribution). « À toi » à 96 pour la pile du second tour sur 96a05586b.
Codeparsers, 18 h 48 : marque « nom » — rouge sur trois cas, deux règles : (a) un chemin sur un type hors appel lit son type et s'abstient
(2 326 arêtes fausses ôtées sur src/ du moteur, « nom » 5 092 → 2 705) ; (b) une locale sans type lisible ne se relie pas entre
fichiers — tel quel, elle perdrait ~950 liens justes sur rag3weaver (méthodes du projet sur une locale) ; tranché **(b) étroit** :
seules les méthodes de la bibliothèque standard du langage (liste en données par langue) s'abstiennent, abstention dite ; le reste
au ticket 2 (lire le type de la locale). ED : les témoins du nom vus rouges sur une méthode faussée, puis verts. 96 : pile du second
tour reposée (447bb495b), rejeu sous 90 min.
96, 18 h 54 : segment vert partout (llm_nodes 17, foreach 16, archiviste 25 dont 3 sujets → 3 L1 §a-b et le non-JSON refusé) sauf le
témoin de M4 (« LlmNode reste hors de toute politique ») : LlmNode mis sous call_models pour charger archive_segment.mmd. Tranché
**voie 2** : LlmNode admis sous call_models seulement avec `response_schema` déclaré (borné par son schéma, comme Decide par ses
candidates) ; validate_tool_policy lit la configuration du nœud. Aveu de 96 : les deux e2e de segment commités sans avoir été vus
rouges ; rejoués rouges sur 1224f80a6 avant tout push.
Archi, 18 h 55 : le rouge du branchement est un effet du lot (2 sur 4 sous Gemini contre 3/3 avant : le modèle rend le README dans un
report et dit « rédigé ») ; correctif dans l'invite (« une demande de fichier s'écrit dans le fichier, la réponse dit ce qui est
écrit ») vu rouge ; 4 tirages Gemini puis repose sur f85977d09, vert vers 19 h 30. Décision : **la pose du soir attend le branchement
jusqu'à 19 h 35** (c'est lui qui met le socle dans la tâche de Lucie) ; sinon elle part sans lui.
Banc, 18 h 55 : le verdict de c5ec47cc4 vers 20 h 50 — le parent ne compilait pas (la coupe de la sonde emportait la fonction de la vue
2b′ ajoutée après), une heure perdue ; corrigé (la coupe s'arrête à la fonction suivante), relancé 18 h 55 sur copies et P0 neuves.
L'écrivain et la montée passent après la pose du soir.
Cœur, 19 h 02 : **verdict de l'alerte** — aucun tour bloqué depuis le tour 7 (17 tours sous la sonde, aucune tête non validée vue) : 1
tour bloqué sur 27 sur ee598e074 ; rare, cause non lue ; ticket ouvert avec l'établi et la sonde (f32b0687f) ; pas de garde sans la
cause. La lib peut basculer. Le lot du SIGSEGV (ee598e074, vert : transaction_test 387, ×10 sans SIGSEGV) se repose sur master
(le format est atterri) ; « à toi » après le hash de la pose.
Archi, 19 h 03 : le correctif de l'invite tient (Gemini 4/4, le report dit ce qui est écrit) ; repose sur master sous socle-branchement-3,
témoins, rejeu court, push au vert vers 19 h 20. 96 et archi poussent dans la même fenêtre, sérialisés par poste push.
Cœur, 19 h 05 : colonne-ajoutee-se-relit reposée (colonne-ajoutee-se-relit-2, 5 commits « = » + un commit de format pur 68769674f après
le contrôle à blanc de clang-format-18) ; rejeu complet après la sonde (~19 h 25, 45 min) ; « à toi » après le hash. Procédure :
le contrôle à blanc par l'image docker rag3db-clang-format-18.
Archi, 19 h 11 : **le branchement du socle sur master à b5de98d3b** (5 commits : le socle greffé dans toute tâche, l'empreinte du pas, l'invite
d'agent de code, une demande de fichier s'écrit dans le fichier et la réponse est du texte ; gmail, 0 attribution ; Gemini 4/4). La
tâche de Lucie aura write, glob, replace_lines, insert_lines, replace_text dès la pose. « À toi » à ED (nom + persona) ; 96 pousse
en parallèle ; lucivy au « poussé » d'ED ; puis le hash.
Embarquement, 19 h 12 : **master rouge sur la lib entière**, déterministe à luciepc — `modele_de_tache::tests::a_task_lists_its_declared_models…`
(modele_de_tache.rs:42, la tâche ne se rouvre pas sur son manifeste changé), 6/6 rouge sur master 6c6e421cdf ; les commits sont de
tty-14 (4399be2c8, 39d81457c, 3d829b2b9). tty corrige devant tout ; ED et 96 ne poussent pas un rouge. Embarquement : repli par
activation nommée vert (646556c26, 47 témoins), en attente.
tty, 19 h 14 : le témoin était vert dans son rejeu de lib à e9e47391a (journal, ligne 1138) ; il rougit depuis : le code arrivé entre est
celui d'embarquement (f85977d09e). Piste de c7 : la garde `query_max_chars` au chargement avec un `models.embed` sans
`context_tokens` (plafond haut 0 → 6 000 refusé). tty rend le refus exact ; embarquement vérifie sa garde ; ED, 96, G prévenus :
rien ne pousse avant le correctif.
19 h 15 : tty écarte la piste f85977d09e (déjà dans la base de son rejeu vert ; entre vert et rouge, rien de l'hôte n'a changé) ; 96 a le
témoin **vert** dans sa lib entière (18 h 45) avec RAG3WEAVER_EMBED_SERVICE posé : le rouge « seul » est sans doute joué sans service
— un témoin ne dépend d'aucun service, et une tâche doit se rouvrir sans service d'embarquement (plein texte seul, dit). Le refus
exact du diagnostic de tty tranche ; correctif en lot court devant tout.
Arbre, 19 h 32 : la répétition réparée a fini sa part fermée sur les deux copies (code-lourd : réparation 1 719 s VERT, purge 9 s, **1b 683 s,
montée 2 s** ; code-principal : 1 402 s, 9 s, **623 s, 1 s**) ; repassage en cours (paquet 1 sur 4, plus lent que prévu sous deux
autres lourds : 35 à 45 min par base). Pour la vraie fenêtre : instantané + ~10 s + 1b 10 à 12 min + montée 2 s, par base, en parallèle ;
le contrôle des deux sens en garde avant.
Archi, 19 h 33 : la conception de delegate sur master (695a9c19f, page 10) : delegate/reply/stop/wait, sous-tâche parent_task sur
`agent/<parent>--<nom>` forkée du parent, dialogue recopié chez le parent en from: subagent, merge par défaut ou proposition,
bornes teammate.delegation {max_depth 2, max_children 3, wait_s 1800}, enfants en zone vivante. Deux points à vérifier avant le
premier rouge (une tâche sur le backend d'une tâche ; parent_task exposé par la liste).
Codeparsers, 19 h 36 : la marque « nom » montée (codeparsers/marque-nom 7d2ef159a, en pile sur homonymes + paramètres) : (a) tel quel, (b) étroit
par des listes std générées (rustc 1.98.1 : 1 266 noms ; clang 23 sur libstdc++ : 193 ; Python 3.14) ; rag3weaver/src « nom »
10 702 → 10 114 (646 ôtées : 373 chemins par un type, 273 méthodes std ; prix : ~20 liens justes homonymes d'une std, au ticket 2) ;
src/ du moteur 5 092 → 2 705, toutes fausses. Touche code.rs : montée de la constante quand la garde la demandera. Ticket 2 ensuite.
19 h 41 : **décision** — master n'est pas rouge sous la passe épinglée (le témoin du modèle d'une tâche y est vert chez tty, 96 et ED,
avec RAG3WEAVER_EMBED_SERVICE posé) ; il rougit en jeu isolé sans service, sans doute sous burn-embedder (tty : vert au portable sans
burn-embedder, avec ou sans service). Les pushes repartent : ED (nom + persona, vert sur 5f0472603, repose sur b5de98d3b), 96 (pile du
second tour), lucivy, puis le hash. tty rend le témoin indépendant de l'environnement en lot à part, et le produit si une réouverture
échoue sans service sous burn-embedder. Hotfixs : extension-non-chargee-dite rouge puis vert, passe en file.
ED, 19 h 42 : **le nom et la persona de l'agent sur master à 76f1d44ed** (7 commits, 9813019fc → 76f1d44ed : `agent: {name, persona}` au
manifeste, défaut « équipier », surcharge par tâche, création nommée d'une tâche ; témoins vus rouges sur un faux non commité ;
gmail, 0 attribution). `agent: {name: "Algareth"}` se pose dans le manifeste de Lucie **après la pose de l'hôte** (le binaire en
place refuse un champ inconnu). tty : diagnostic vert à luciepc aussi sans service sous le jeu courant ; reste la piste des
features (burn-embedder), en cours au portable. Restent 96 et lucivy, puis le hash.
Embarquement, 19 h 51 : **le rouge départagé : la feature `openai-llm`** — les six rouges étaient joués sans elle (rag3db-native,code,daemon) ;
le témoin déclare deux modèles compatible/openai, l'hôte les refuse sans la feature (« déclarer local ou compatible », trompeur),
open rend ok: false. tty garde le témoin par la feature (lot court) ; décision produit chez ED : l'ouverture passe, le reçu dit le
modèle indisponible et la feature absente, le tour de chat est refusé avec la cause ; ticket 095cb0ac0f. Un aveu de méthode :
tty a cherché 30 min un rouge que personne ne reproduisait, faute d'avoir eu dès le début la commande exacte et le jeu de features
du rouge — un rouge se rapporte avec sa commande, son jeu et son environnement.
Lucivy, 19 h 55 : **la maintenance nocturne sur master à f4131ed400** (gmail, 0 attribution ; rejeu vert) : avec un hôte ≥ f4131ed400, chaque
base ouverte a sa maintenance à 03 h 30, l'état la dit demain. Il ne manque que la pile de 96 pour le hash (attendue jusqu'à 20 h 15).

## 19 h 56 — le hash de la pose du soir : 23dc8efd47 (code f4131ed400)

96 pas avant 20 h 45 (repose sur le socle d'archi puis rejeu) : la pose part sans sa pile (le plafond d'embarquement sur master
protège déjà le 7879 ; query_chars ne concerne que l'archiviste). G bâtit l'hôte des deux postes sur 23dc8efd47, verdict des trois
suites, puis pose hôte + services dans une même fenêtre (annonce, « va » du banc et d'arbre, preuve du service en place). Dedans :
le socle (primitives + branchement), (B), nom + persona, /model, tty-15/16, plafond de requête, relevé à blanc, maintenance nocturne,
service 503 ; lib 373b5477b. « À toi » au cœur pour le SIGSEGV au vert de son rejeu. Après la pose : Algareth dans le manifeste de
Lucie ; la pose suivante avec 96 ; tty-17, ED (ouverture sans feature), repli-activation, G ×4, arbre + recherche, codeparsers.
ED, 19 h 58 : la mesure de 2a′-3 (page 01 du 15) : 0 réapparition sur 50 commits (2 423 clés, 702 changements) ; (3a) ×1,9 à ×2,5 sur outline,
usages, impact, racines et ×9,9 sur le compte ; (3a′) ×1,3 à ×2,1 (le moteur n'élague pas l'annexe vide). **Tranché : (2) le doublon
au bord haut reste le défaut** ; la variante § 5 (uuid par le contenu sans annexe) en ticket à prouver. Arbre code 2a′-3 sur (2).
G, 20 h 02 : l'hôte bâti sur 23dc8efd47 (md5 f8ce107c), vert en essai aux deux postes ; verdict des trois suites à luciepc depuis 20 h 01.
L'ordre de la pose d'après les « va » : l'hôte aux deux postes vers 20 h 40 (ne coupe pas les services) ; 7879 au « va » du banc vers
20 h 50 (sa parité lit 7878 et 7879) ; 7878 au mot d'arbre vers 21 h (sa répétition embarque par 7878) et au « va » du banc. Arbre ne
lance la vraie bascule qu'après le « fini » de G. 96 : rejeu vert à 20 h 01, pile reposée (b2a031f2e), rejeu court en cours, pour la
pose suivante.
Arbre, 20 h 03 : code-principal répété en entier (réparation 1 402 s ; purge 9 s ; 1b 623 s ; montée 1 s ; **repassage complet 2 956 s**,
4 paquets puis ~20 000 morceaux neufs à embarquer), vert au banc de recherche, mais un défaut de graphe : **1 947 CONSUMED_BY visibles
sans leur CONSUMES** (0 avant ; le CONSUMED_BY entrant recopié sur le scope renaissant même quand la résolution neuve ne consomme
plus la cible) ; impact et appelants faux sur ~2 %. **Décision : la vraie bascule attend** le correctif d'ED (témoin 18) et
l'atterrissage de l'écrivain ; pas ce soir avec un défaut connu (un second repassage coûterait 45 min par base). code-lourd au
paquet 2/4, finit vers 21 h ; 7878 ne se coupe qu'à son mot.
Embarquement, 20 h 03 : la mesure « carte réellement pleine » prête (nuit.sh : garde carte < 13 548 Mio lue + va-du-banc + 7879 qui sert,
bâti de l'hôte sur master, marge 2048, fragment de résultat) ; card2 est réellement pleine ce soir (11 844 Mio libres à 20 h 03, la
parité et la répétition tournent). Mot de c7 : après le « fini » de la pose de G (~21 h 05), au « va » du banc, pas pendant la pose.
Ticket (b) : compter les jetons avant la troncature du tokenizer.
ED, 20 h 08 : le miroir corrigé (5ed5d1fc9 ; témoin 18 rouge d'abord : `recopier_les_aretes_entrantes` traitait CONSUMED_BY comme une
entrante, alors que c'est le miroir d'un CONSUMES sortant que l'ingestion repose ; la boucle le saute). Passe épinglée puis le banc
rejuge. Banc : sa sonde de structure contrôle désormais les miroirs directement (0 et 0 attendus) ; « va » à G vers 20 h 50, va-du-banc
à embarquement après le « fini » de G vers 21 h 05.

## 20 h 15 — l'hôte 23dc8efd47 posé aux deux postes ; Algareth

G : verdict vert (e2e_code 36, lectures 27, demon_hote 24), hôte posé à luciepc 20 h 13 m 03 s (réveil vert) et au portable 20 h 13 m 07 s →
25 s (18 s de coupure, cinq bases rouvertes) ; binaire f8ce107c, lib 373b5477b ; l'atelier expose 23 outils dont le socle. Les services
7879 puis 7878 aux « va ». **`agent: {name: "Algareth"}` écrit** dans le manifeste du dépôt de Lucie et la copie de sa tâche atelier
(sauvegardes .avant-algareth-*), backends rouverts par l'hôte (session neuve, l'ancienne fermée par la relance). 96 : segment vert au
portable (voie 2 : LlmNode admis dans un outil seulement avec response_schema sous call_models ; un rouge de charge Text file busy en
ticket), en pile derrière le second tour pour la pose d'après.
G, 20 h 15 : check-manifests vert après l'écriture (6 manifestes, 0 refusé ; `agent.name` Algareth, un seul models.llm). Archi : le nom de
l'agent, parts (2) et (3), vert (le-nom-de-l-agent f089a74e9 : « Your name is Algareth » dans l'invite, présentation une fois, la
persona pour le ton ; `name` sur chaque ligne teammate ; un rouge vu : « je m'appelle équipier » quand aucun nom n'est choisi, corrigé) ;
« à toi » dessus. L'hôte posé ce soir a le nom au manifeste mais pas l'invite : Algareth se présentera à la pose suivante.
tty, 20 h 21 : tty-17 sur master à a8907ac57d (le témoin du modèle d'une tâche gardé par openai-llm ; gmail, 0 attribution ; lib 2 495 sous le
jeu le plus large). CI Workflow rouge sur 76f1d44ed : shell test du moteur, test_prev_history_multiline, un pexpect TIMEOUT (1/245),
instabilité probable — hotfixs relance et ticket.
Arbre, 20 h 22 : la répétition finie sur les deux copies (page 02, 70012da2b) : code-lourd 1b 683 s, montée 2 s, **repassage 3 661 s** (7 278
fichiers, vecteurs compris), banc vert ; code-principal 623 s, 1 s, **2 956 s**, banc vert ; le miroir faux aux deux (2 241 et 1 947
CONSUMED_BY sans CONSUMES), correctif d'ED en 5ed5d1fc9. Vraie fenêtre : ~15 min fermée par base (contrôle des deux sens en garde),
repassage 50 à 60 min base ouverte avec confirm=true. Dernière répétition sur copies neuves au vert d'ED (juge : CONSUMES = CONSUMED_BY).
G a le « va » pour 7878 depuis 20 h 20.
Banc, 20 h 24 : le juge étendu sur c5ec47cc4 (dépôt migré) : DEFINED_IN réparé ; **CONSUMES −4 080, CONSUMED_BY −3 917, MENTIONS −984
restent**, 163 CONSUMED_BY sans miroir (la sonde du miroir voit l'écart : pas de trou du juge) ; réponses non jugées (le repassage a
dépassé l'heure du relevé ; plafond relevé à deux heures). À ED : la cause des CONSUMES encore perdus avant de rejuger 5ed5d1fc9.
Parité finie ; G a le « va » du banc pour 7879 depuis 20 h 23.
20 h 24 : la pile de 96 a perdu deux courses (le socle d'archi, puis le témoin de tty-17 : « master a apporté du code ») ; reposée sur
658a613e0 (8c34a3b07), rejeu relancé 20 h 24, vert vers 20 h 45. **Le code de master est tenu jusqu'à son « poussé »** : archi (nom), cœur
(SIGSEGV), ED (llm-sans-feature, écrivain), hotfixs (extension dite) attendent. G coupe les services après le « va » d'ED (sa passe
les lit) et le rejeu de 96 (qui lit 7879), vers 21 h.
20 h 32 : ED a donné son « va » à G à 20 h 27 (passe finie 20 h 25) ; sa passe épinglée sur 5ed5d1fc9 est verte, la tête n'est pas encore
au banc : les CONSUMES perdus (−4 080, dont 2 634 entre deux scopes non réécrits, cibles à noms communs) se lisent par une sonde
bout à bout après le « fini » de G. Archi : un témoin de temps rougi sous charge (usages pendant un écrivain) → ticket chez recherche,
tranché en rapport (≤ 4 × sans écrivain, garde 500 ms) dans son lot de gardes (ba6963b45), après le nom d'archi. tty : lot /nom et
l'affichage de name. Chaîne post-96 : archi (nom) → cœur → ED (llm) → hotfixs → recherche → G ×4 → arbre 2b′ + recherche 2b-C →
codeparsers (3 lots) → embarquement (repli-activation) ; l'écrivain au vert du banc ; segment de 96 au troisième tour.
96, 20 h 34 : **la pile du second tour sur master à 0653a6aeff** (8 commits, b0482fa2d → 0653a6aef : usage par section et total, absence =
prompt_tokens == 0, query_chars 4 000, context_title et l'instruction de la variante, la trace ; gmail, 0 attribution). « À toi » à archi
(nom), puis le cœur, ED, hotfixs, recherche, G ×4 à la suite. G pose les services (tous les « va » donnés). Le banc reprend F 25 % sur
0653a6aeff après la mesure d'embarquement.
20 h 36 : G coupe et repose les services à luciepc à 20 h 36 (7879 puis 7878, f85977d09e, preuve en place, retour automatique). Banc : la
réserve sur la Source levée (bm25 seul, textes découpés, trois plafonds en place) ; reprise après les deux « fini ». Hotfixs : le rouge
de la CI sur 76f1d44ed est un test instable du shell (ticket b2580a56b), le run suivant vert. Cœur : la garde arrêterait son push sur
le Rust arrivé → **la voie des suites C++ transférables de G** doit le couvrir sans rejeu (à confirmer par G) ; ED, hotfixs et recherche
passent devant lui, il pousse à 21 h 15. tty : tty-18 (/nom par POST /ide/teammate/name, convenu avec ED ; `[Algareth · …]` sur les
lignes qui portent name) en branche, attend la route d'ED et le name d'archi.

## 20 h 37 — la pose du soir est complète

G : les services f85977d09e posés en place à luciepc (7879 bge-m3 à 20 h 36 m 00 s, 14 s de coupure, vecteur témoin écart 0,0, 413 nommé
sur 13 000 caractères ; 7878 granite à 20 h 36 m 15 s, 5 s, mêmes preuves ; NRestarts 0 ; les tunnels du portable rendent le neuf) ;
l'hôte 23dc8efd47 aux deux postes depuis 20 h 13 ; Algareth au manifeste, check-manifests vert. La mesure d'embarquement sur card2
part au va-du-banc ; le banc reprend F 25 % sur 0653a6aeff ensuite.
Embarquement, 20 h 38 : la mesure de la carte pleine n'est pas partie — la garde a lu 20 332 Mio libres (la carte n'était pleine que par la
parité et la répétition, arrêtées pour elle). Tranché : la mesure part pendant la dernière répétition d'arbre (qui charge card2 par
7878), avec un va-du-banc frais ; le banc note la minute de 7879 dans ses temps.
ED, 20 h 40 : les CONSUMES perdus (−4 080) ne sont **pas** l'écrivain : le lot refait la résolution sous la règle actuelle, le parent garde des
arêtes d'un index plus ancien (1 547 sur un nom ambigu — la règle s'abstient ; 2 533 sur un nom unique que la règle actuelle refuse
aussi à un index neuf : témoin 19, C → C++ `main` → `rag3db_connection_init`). Décision : le banc juge contre un **index neuf** du même
arbre ; le C → C++ refusé en ticket chez codeparsers. Les MENTIONS −985 : défaut probable de l'écrivain (les mentions restent sur la ligne
fermée quand le scope renaît), témoin 20 en cours ; 5ed5d1fc9 n'est pas donné au banc avant.
Cœur, 20 h 43 : rejeu du SIGSEGV vert (témoin 2/2, cibles 16, transaction_test 387, e2e_test 1 867 et DropNodeTableReclaim seul) sauf le juge
known_red, rouge dans le sens vert : SameVectorForEveryRow…ZeroRowsFirstCopy passé une fois dans le banc complet, rouge 10/10 seul
(1 222/1 402). Tranché (a) : VERT au journal avec ce rouge dit, la ligne known_red inchangée, push ; ticket chez G : un known_red qui
verdit une fois (ordre, graine) ne rougit pas un lot — « passé cette fois (intermittent) », retrait de la ligne après N verts seuls.
Recherche : lot de gardes vert (ba6963b45), après hotfixs.
Cœur, 20 h 44 : **le SIGSEGV de la colonne ajoutée sur master à 2c11a7995f** (6 commits, d59a56e9e → 2c11a7995 ; gmail, 0 attribution), poussé
par la voie des suites C++ transférables de G (base aa9a7b72a, rien de code du moteur entre les deux) : la voie marche. Suivant chez le
cœur : le pool qui sature sous vingt écrivains. ED : le témoin 20 vert (une ligne renée garde ses MENTIONS) ; les MENTIONS absentes
sont, comme les CONSUMES, des arêtes que le parent garde de l'ancien contenu ; tête 794b14232 proposée au banc (index neuf en référence).
Cœur, 20 h 45 : tickets posés (1a8d226f3) — le SIGSEGV fermé ; le refus sans fin à 1 tour sur 30 sous la sonde ; le ticket du juge
intermittent ouvert (en double avec celui de G dans sa branche : un seul, celui de master, déplacé chez G dans son lot). Le cœur prend
le pool qui sature. Archi : son script de repose et push du nom attendait la place du portable (tenue depuis 20 h 29) ; luciepc a deux
places libres, il y passe.

- 20:51, cœur, pool saturé, premier résultat (sondes hors dépôt sur 2c11a7995f) : un SET sur un nœud déjà sur disque garde du pool jusqu'au point de reprise (8,5 Ko une chaîne, 25 Ko un FLOAT[64] de 256 octets, 42 Ko les trois colonnes du scénario) ; une insertion coûte 300 octets par nœud, 59 par relation ; même coût à 1 ou 20 connexions, 2 ou 8 fils ; le CHECKPOINT rend tout. À 20 écrivains × 42 Ko, 2 Gio pleins en ~50 000 écritures quand le journal n'a grossi que de quelques Mio : le seuil de 16 Mio du point de reprise automatique arrive après le pool plein. Écart non expliqué : sa première sonde mesurait ~0 pour les mêmes SET, à comprendre avant tout correctif. Voies : (a) une mise à jour garde un tampon à sa taille et non à la page (cause probable), (b) le point de reprise aussi sur pression du pool (filet). Il cherche où ces tampons se prennent ; ticket à jour.

- 20:57, codeparsers, le C qui appelle une fonction C++ (ticket d'ED) : la cause est l'abstention « invisible » de la visibilité par les #include (25 oct.). Dans la vraie base l'en-tête est là, mais sa déclaration est derrière la macro d'export (`RAG3DB_C_API rag3db_state rag3db_connection_init(…)`, rag3db.h:360) et tree-sitter ne la relève pas comme prototype : limite n° 3 du ticket de la définition liée à l'en-tête ; toute l'API C est dans ce cas, d'où les ~2 533 CONSUMES. La recette à deux fichiers (sans l'en-tête) n'était pas le bon témoin. Correctif dans codeparsers : relever un prototype derrière une macro d'export, comme `class RAG3DB_API Foo` ; témoin rouge main.c + rag3db.h + connection.cpp, `appelants(rag3db_connection_init) == [main]`. Ordre accordé : juste après son lot en cours (receveurs sans type), avant la définition liée à l'en-tête ; il réécrit le ticket d'ED avec la cause. Comme toute règle d'analyse qui change les arêtes, le lot monte REGLES_DE_L_ANALYSE.

- 20:58, codeparsers, montée de REGLES_DE_L_ANALYSE : la constante n'est pas encore sur master, elle arrive avec l'écrivain d'ED. Homonymes+paramètres ne touche que le pointeur (rien à monter, le commit du pointeur entre dans la version par build.rs). Marque nom, receveurs sans type (en cours, code.rs) et prototype derrière la macro changent les arêtes : une seule montée, dans le dernier lot de la pile (le prototype), une fois la constante sur master ; s'ils atterrissent avant l'écrivain, rien à monter, la première valeur de la constante repasse tout. Receveurs sans type : rouge vu pour les cinq témoins ; sur rag3weaver/src « type » 7 116 → 9 072, « nom » 10 114 → 9 046, 182 pertes presque toutes des méthodes std reliées à tort (collect 123, join 16, keys 9), deux vraies pertes corrigées (retour générique, chemin masqué par une locale) ; sur le moteur « type » +131, un seul couple perdu ; verts finaux en cours.

- 21:04, Lucie, vision point 7 : pas de notification à chaque petit changement que l'agent cause lui-même ; à la recherche, estimer le reste à indexer ; sous 6 s (réglable) attendre en synchrone et répondre complet, au-dessus répondre « index pas prêt » tout de suite et notifier une fois quand il l'est. Ajouté à la page de vision ; transmis à archi (socle, outil de recherche du pas) et à arbre/recherche (estimation sur le journal d'index par worktree).

- 21:05, Lucie, suite du point 7 : « traduire par un bench d'avance à quoi correspondent 6 secondes vs combien de lots à embarquer ». Un banc d'étalonnage joué à l'avance par poste et par service d'embarquement donne le débit (lots/s, caractères/s) ; l'hôte convertit le reste à indexer en secondes et se corrige au débit observé. Transmis au banc (le banc d'étalonnage) et à embarquement (le débit par service, local, distant, carte, CPU).

- 21:06, arbre, l'estimation du reste : il repart du journal d'index. L'estimation existante se trompe 2 à 2,5 fois (répétition de ce soir : « ~27 min » annoncées, 61 min réelles pour code-lourd ; « ~19 min », 49 min pour code-principal), parce que le rythme du premier paquet ne compte ni les paquets lourds ni la phase des vecteurs ; les lignes « N fichiers sur M » et « vecteurs X sur Y, reste Z s » (à la minute près) tiennent. Pour une estimation par vue sans constante : horodater chaque ligne du journal (aujourd'hui sans heure), un débit observé par phase (plein texte, relations, vecteurs) lissé sur les derniers paquets, le reste de la vue en fichiers (pointe moins génération publiée, 2a′-3) ; le seuil de 6 s se juge sur reste × débit de la phase. Page à part, après 2b′ et la migration ; ses chiffres vont au ticket de l'estimation.

- 21:06, archi, le nom à luciepc : premier essai rouge sur e2e_demon_hote par la faute du script (passe lancée sans POSTE_MEM_MAX=32G, run_e2e.sh a refusé : « aucun plafond effectif <= 32 Gio »), témoins équipier et chat verts 60/0 ; relancé (archi-nom-luciepc-2) plafond posé, target bâti, « poussé » vers 21 h 20 sans second mot. Le point 7 ira en page 12 (autre sujet que delegate), après le nom ; sa part est le pas, l'estimation reste à l'arbre et à recherche. Les chiffres de l'arbre et le chemin du relevé horodaté sont versés au ticket de l'estimation.

- 21:07, banc, le banc d'étalonnage : conception dans une page du banc, soumise avant toute mesure, créneau vers 23 h après le jugement de 794b14232. Ce que ses passes mesurent déjà : le temps total d'un index de code par côté (même P0 : parent 243 s, bff6c7409 2 476 s, d57b28c80 193 s), sans phases ni lots ; les lignes [montre] des journaux d'index (« 56 morceaux embarqués hors du verrou : 16 474 ms ») d'où un débit en morceaux/s par passe et par service se tire après coup ; rien en caractères/s par service, ni CPU ni distant. Son départ : un jeu de lots fixe (1 500 caractères, la taille du découpeur), par service et par poste, lots/s et caractères/s, la phase mesurée sur un index réel à côté.

- 21:07, embarquement, le débit existe déjà au catalogue : `Rate` (chars_per_second), noté par `note_measured_embedding_rate` à la fin d'un index de 512 morceaux et plus, relu par `known_embedding_rate` pour l'estimation et le « reste environ N min » ; par catalogue, pas par service ni par poste : le point d'accroche est une table par service (adresse, modèle, poste). La provenance par passe (livrée aujourd'hui) dit quel service ou quelle carte a servi chaque morceau : avec la durée, un débit par service sans banc à part. Sa mesure de ce soir donnera la valeur du 7879 sur carte pleine. Décidé : le banc d'étalonnage écrit au même endroit que l'index (une seule source pour l'hôte) ; banc dessine le jeu de lots et la mesure, embarquement la table par service ; une page chacun, à soumettre, pas de code avant.

- 21:11, **le nom est sur master** : 3e8587f0e7 (archi, depuis luciepc) : une tâche nommée dit son nom et sa persona dans l'invite, ses lignes portent `name` ; sans nom choisi l'invite ne dit pas le défaut « équipier » (rouge Gemini : « Bonjour, je m'appelle équipier »). Quatre commits gmail, zéro attribution. **« À toi » à ED** pour llm-sans-feature sur 3e8587f0e7 ; hotfixs prévenu, suivant.

- 21:12, archi, page 12 posée (a2b64a5c1) : l'attente d'index du pas, trois témoins rouges listés ; contrat demandé à l'arbre et à recherche : estimation lisible par l'hôte (reste, débit observé) et attente bornée. Il pose le pas dès que le contrat existe ; sa file revient à delegate (page 10). Lien ajouté depuis la vision.

- 21:13, hotfixs, correction : la passe d'extension-non-chargée-dite n'avait pas tourné (chemin complet donné au script qui prend un nom sous ~/git_workspaces, arrêt au premier pas) ; relancée à luciepc 21 h 1x, poste lourd timeout 7200, target-hotfixs-etude ; seul le témoin est vu rouge puis vert à la main, aucun verdict de passe. Ordre ajusté : après ED, recherche (gardes) passe devant hotfixs si sa passe n'est pas verte à ce moment ; hotfixs repose sur la tête du moment, rejeu court.

- 21:35, cœur, pool saturé : l'écart est expliqué et (a) est vert. L'écart : le COPY de la première sonde échouait (NULL sur la clé primaire), table vide, les SET ne modifiaient rien ; les mesures suivantes sont les bonnes. La cause (UpdateInfo::update) : chaque transaction qui met à jour une ligne déjà écrite crée une version du vecteur de 2 048 lignes (VectorUpdateInfo) dont le tampon naissait à pleine capacité (DEFAULT_VECTOR_CAPACITY) pour une seule ligne, gardée jusqu'au point de reprise ; les petites allocations passent par malloc à leur taille exacte, que le pool réserve. Branche mise-a-jour-a-sa-taille : témoin UpdateMemory (abc5117fd) rouge (25 136 octets de pool par mise à jour d'un FLOAT[64], 8 528 pour une chaîne) ; correctif ec7a0bf30 (2 fichiers) : la version naît à 4 lignes et double jusqu'à 2 048 ; vert sur 3 cas dont un vecteur entier relu après réouverture, transaction_test 390. En route depuis 21 h 35 : sonde avant/après, known_red, e2e_test, DdlEveryMinute ×10 (pic du pool, refus, comptes des Note) ; fin vers 23 h 15. (b) se juge sur ces ×10. Voie C++ transférable candidate (src/, test/ seuls).

- 21:41, codeparsers, receveurs sans type poussé en branche (codeparsers/receveurs-chaines, tête f1bae1e34, en pile sur la marque nom 7d2ef159a ; codeparsers 65b1a93) : une locale liée à une chaîne de méthodes du projet sur une racine de type connu lit son type (`DeferredType::OfType`, pelage (type, méthode) → retour écrit, `Self` vaut le type ; la table std apprend map_err, ok_or, ok) ; lecture au mieux, sinon la règle d'avant. Deux gardes, rouge vu : un retour qui nomme un paramètre de gabarit ne type rien (les fonctions Rust relèvent leurs generic_parameters) ; un chemin de module ne prend pas le type d'une locale homonyme. Témoins : cinq rouges puis verts, e2e_usages 28/28, e2e_code 55/55, suite codeparsers ; deux témoins anciens corrigés (`self.prepared()` se lit désormais). Sonde (corpus figé, avant = marque nom) : rag3weaver/src « type » 7 116 → 9 070, « nom » 10 114 → 9 048 ; 864 couples passent de nom à type, 1 071 naissent, 178 partent (std reliées à tort : collect 123, join 16, keys 9) ; moteur « type » 10 214 → 10 345, 1 couple part. Restent catalog.drain() et outil.bind au ticket 2. Nouveauté : « toutes les bases repassent une fois ». check_tickets VERT, Cargo.lock inchangé. Il enchaîne sur le prototype derrière la macro, rouge d'abord. Sa pile : homonymes+paramètres → marque nom → receveurs → prototype, une passe complète à mon « à toi » après embarquement.

- 21:43, **ED poussé** llm-sans-feature : master c224658dc5 (repose llm-sans-feature-2 a068d01d6 sur 3e8587f0e7 ; cinq commits gmail, zéro attribution ; rejeu court vert : check_features 8/8, lib 2526/0/72, navigateur 45, témoin e2e_demon_hote, lib 373b5477b sous MOTEUR_ANCIEN ; garde bf8915004 ; trois contrôles). Un modèle de langage dont le fournisseur n'est pas compilé n'empêche plus l'ouverture, l'avertissement nomme la feature ; le ticket d'embarquement est fermé (son premier constat était faux : une fixture sans la feature). ED passe à la route de renommage pour tty-18. **« À toi » à recherche** (gardes) sur c224658dc5, la passe de hotfixs n'étant pas sortie ; hotfixs suit à son « poussé ».

- 21:45, hotfixs, la passe de 21 h 12 à luciepc est morte en silence (attente en file puis disparue, aucun processus, journal de lancement et dossier de journaux vides, rien dans journalctl utilisateur ni noyau entre 21 h 11 et 21 h 50) ; relancée 21 h 44 (pid 897922, poste lourd timeout 7200, CIBLE=target-hotfixs-etude, passe_epinglee.sh rag3db-extension-dite, passe entière), un moniteur toutes les cinq minutes. Piste de c7 : la session ssh qui l'avait lancée s'est fermée (SIGHUP sur la file) ; lancer en unité systemd --user comme archi ; si elle meurt encore, ticket d'outillage.

- 21:51, hotfixs, la passe de 21 h 44 morte une seconde fois de la même façon, bien que sous `setsid nohup` ; relancée 21 h 49 en unité (`systemd-run --user --unit hotfixs-extension-dite … ./poste lourd timeout 7200 ./passe_epinglee.sh rag3db-extension-dite …`, pid 998474, en file). Ticket d'outillage pour G avec lui : docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-15-une-passe-en-file-sous-poste-lourd-disparait-sans-trace-a-luciepc.md (86943675f). Piste de c7 : setsid nohup ne tient pas contre `KillUserProcesses=yes` de logind sans linger (toute la portée de session meurt à la fermeture du ssh ; une unité --user vit dans le gestionnaire utilisateur) ; à vérifier : Linger de l'utilisateur, logind.conf, journal des portées ; `loginctl enable-linger` est à l'utilisateur.

- 21:52, hotfixs : logind n'explique pas les deux morts (Linger=yes ; KillUserProcesses non réglé, défaut « no » sous systemd 262 ; journal sans scope arrêté, ni processus tué, ni OOM ; rien côté --user à 21 h 12 ni 21 h 49). La piste de c7 était fausse, écrite au ticket 86943675f ; cause inconnue, règle maintenue : à luciepc une passe part en unité systemd-run --user. L'unité hotfixs-extension-dite est active à 21 h 52, en attente de poste lourd.

- 21:54, G, pile prête sur 1c1e741aa4, noms neufs poussés : garde-hash-entier-2 (ba87f1c238), known-red-cas-longs-2 (88998af1ef), confineur-enveloppe-2 (395fc0aeda), ticket-gabarits-autre-arbre-3 (add185deff) ; fichiers disjoints, sans conflit ; sur la tête : test_push_garde 42, test_compare_known_red 2, test_confiner_validation 24, test_garde_target 35, check_tickets VERT ; scripts seuls. Rejeux courts (SUITES=-) lancés au « poussé » de hotfixs, repose --onto puis un rejeu par lot, puis « à toi ». Le ticket de la passe disparue après ces quatre lots.

- 21:55, **recherche poussé** les gardes : master 6e493fd5db (mode simple, garde bf8915004 ; rien de code entre c224658dc5 et 1c1e741aa4 ; sept commits gmail, zéro attribution ; passe verte sous 373b5477b MOTEUR_ANCIEN : témoin, e2e_code_sync 20, e2e_lectures 27, check_features 8/8, lib 2 526, navigateur 45). Une exclusion posée sur un index déjà fait retire le minifié du plein texte ; le témoin de temps de usages juge deux bornes et sa garde de 500 ms est dite par ce qu'elle sépare ; ticket du témoin de temps fermé (doublon ouvert/fermé retiré en route, fc33a83c3). Ordre : la passe de hotfixs attend encore poste lourd à luciepc ; **G repose et rejoue dès maintenant** sur 6e493fd5db et passe devant hotfixs s'il est prêt avant son vert.

- 22:00, hotfixs, le ticket 86943675f est un faux constat : passe_epinglee.sh fait `J=$HOME/.cache/rag3weaver-build/$2`, le chemin absolu donné s'est emboîté sous ~/.cache/rag3weaver-build/home/lucied/… et il lisait le dossier vide d'à côté ; les passes ont tourné et rendu leur verdict là-bas. La dernière (21 h 55) : ROUGE témoin code 2, « aucun plafond effectif <= 32 Gio » : confiner_validation.py refuse le MemoryMax=40G par défaut de poste lourd (les autres passent POSTE_MEM_MAX=32G), le témoin n'a pas joué. Relancée 21 h 58 (hotfixs-extension-dite-2, POSTE_MEM_MAX=32G, dossier relatif), en file 4/4. Décidé : fermer le faux constat avec la cause ; deux tickets d'outillage à part pour G : passe_epinglee.sh accepte un chemin absolu et l'emboîte en silence ; le défaut 40G de poste lourd contre le seuil 32G de confiner_validation (deux rouges de script le même soir, archi et hotfixs).

- 22:02, G prêt sur 6e493fd5db, rejeux courts verts l'un sur l'autre (21 h 57 – 22 h 02) : garde-hash-entier-3 (4a12c1e0b7, test_push_garde), known-red-cas-longs-3 (510bb01720, test_compare_known_red), confineur-enveloppe-3 (31b741f76a, test_confiner_validation 24 + un vrai lancer par poste lourd VERT), ticket-gabarits-autre-arbre-4 (37887a7fb6, test_garde_target 35 + CHECK_FEATURES 8/8 sous 373b5477b MOTEUR_ANCIEN) ; un worktree et des journaux par lot. **« À toi » à G** pour les quatre, l'un après l'autre.

- 22:05, hotfixs, passe sur 053cdad28 rouge d'outillage (22 h 02) : run_e2e.sh refuse la lib 373b5477b plus vieille que le dernier commit C++ (9ea2e2408, formatage pur) sans MOTEUR_ANCIEN. Lot reposé sur 6e493fd5db (extension-non-chargee-dite-2 → 2b22584f4, trois conflits à côté de avertissement_du_modele résolus par union, les deux chaînés dans avertissements()), relancé 22 h 04 (hotfixs-extension-dite-3, POSTE_MEM_MAX=32G, MOTEUR_ANCIEN=1 par la ligne 107 de la procédure). Témoin : test unitaire de la lib (backend.rs), sa ligne ok dans le journal de la lib entière, rouge vu avant ; l'e2e voisin à l'étape « témoin » du script est un remplissage dit tel quel ; ticket d'outillage pour G : run_e2e.sh ne joue pas un test de lib comme témoin. Reposera sur la tête de G, rejeu court.

- 22:06, **G poussé** les quatre, l'un après l'autre sous poste push : garde-hash-entier d68dbe3cb6 (22 h 03, mode simple sur 6e493fd5db), known-red-cas-longs 0274c53f14, confineur-enveloppe 042030cc9f, ticket-gabarits-autre-arbre e692fa3a58 (22 h 05) ; piles tenues (range-diff `=`), trois contrôles après chacun ; 18 commits gmail, zéro attribution (vérifié). check_tickets VERT, 212 ouverts, 368 fermés ; SIGTERM et gabarits fermés, le juge known_red ouvert chez G. **Garde neuve au cache des deux postes : push_garde.sh sha256 ad0be1f36ff12ea5** (hash entier ; l'ancienne en .avant-garde-hash-entier ; témoin 0 rouge contre chaque copie ; vérifié au portable). Suite : hotfixs repose sur e692fa3a58 (passe en file à luciepc) ; arbre prépare 2b′ et recherche 2b-C sur la même tête ; le premier vert a « à toi », l'autre repose.

- 22:07, ED, tty-18 côté hôte prêt, passe épinglée verte (agent-renommer, a6710eccf, base c224658dc, quatre commits gmail) : POST /ide/teammate/name {backend, task, agent_name} → {ok, result: {agent_name}} ; nom validé comme à la création, gardé au registre, changé en place dans le backend ouvert (`PreparedBackend::renommer_l_agent`, nom vivant sous verrou), le tour suivant le dit sans réouverture ; "" rend celui du backend ; refus « agent_name : … ». Rouge d'abord, deux témoins (bouchon « pas encore » ; « route inconnue : POST /ide/teammate/name »), verts avec les 14 tests des tâches ; passe : e2e_demon_hote 24, check_features 8/8, lib 2528/0/72, navigateur 45. **« À toi » à ED** sur e692fa3a58 (rejeu court, recherche et G passés depuis sa base) ; tty-18 repose dessus à son « poussé » ; hotfixs et arbre reposent sur ED ensuite.

- 22:07, hotfixs, pas de vert : la passe sur 2b22584f4 (22 h 05) rouge avant le témoin, la garde du target refuse target-hotfixs-etude (« target non vide sans provenance », target d'étude du 8 nov. bâti hors garde, 1,4 Go, vidé). Reposé sur e692fa3a58 : extension-non-chargee-dite-3, tête dee35f6a0, quatre fichiers. Relancée 22 h 07 (hotfixs-extension-dite-4, POSTE_MEM_MAX=32G, MOTEUR_ANCIEN=1), passe entière sur la tête finale. La lib 373b5477b est désormais plus vieille qu'un vrai C++ de master (2c11a7995f, le SIGSEGV du cœur), plus seulement que le formatage ; MOTEUR_ANCIEN le dit. Il reposera encore sur ED (route) : rejeu court.

- 22:08, arbre : passe sur e692fa3a58 lancée 22 h 07, arrêtée 22 h 09 par son groupe avant le premier bâti ; branche reposée arbre-principal/vues-2b-prime-2 (da4d2f7fa, range-diff =), repartira --onto la tête d'ED. Passe prête à luciepc : MOTEUR_ANCIEN=1, timeout 14400, e2e_lectures, e2e_code, e2e_postgres (code.rs, IR, dialecte), check_features, lib, navigateur. La répétition de code-lourd tient une place jusque vers 23 h ; code-principal a fini.

- 22:10, G : le chemin absolu de passe_epinglee.sh est fait (passe-chemin-absolu, ec0654ce5 : refus nommé code 2, aucun dossier créé, « journaux : <chemin> » au premier pas ; deux cas rouges d'abord, 23 vert). Le 40G de poste lourd : (a) défaut à 32G, une source, risque d'oom sur un bâti de lib ou un banc qui dépasse sans le savoir ; (b) les entrées de validation (run_e2e.sh, temoins_navigateur.sh, passe_epinglee.sh) vérifient le plafond au premier pas, avant la file, et refusent en le nommant. Décidé : (b) tout de suite ; (a) seulement après le pic relevé d'un bâti de la lib à -j8 et d'un banc, avec le cœur demain ; prochaine pile de G : chemin absolu, (b), témoin de lib de run_e2e.sh.

- 22:19, **ED poussé** la route de renommage : master a178fb9231 (quatre commits gmail, zéro attribution, vérifié) : POST /ide/teammate/name renomme l'agent d'une tâche ouverte sans la rouvrir, nouveauté écrite. **« À toi » à tty** pour tty-18 (/nom) sur a178fb9231, rejeu court ; l'arbre repose 2b′ sur a178fb9231 et lance sa passe (reposera d'un cran si tty-18 atterrit avant lui) ; hotfixs repose à son vert.

- 22:21, arbre : 2b′ reposé sur a178fb9231, tête 75f827250 (arbre-principal/vues-2b-prime-3, range-diff =) ; passe épinglée lancée 22 h 20 à luciepc, place tout de suite (MOTEUR_ANCIEN=1, timeout 14400, e2e_lectures, e2e_code, e2e_postgres, check_features, lib, navigateur ; journal ~/.cache/rag3weaver-build/arbre-vues-2bp/resume.log). « Prêt » au VERT ; repose d'un cran si tty-18 atterrit avant.

- 22:26, **tty poussé** tty-18 : master 4802a147a5 (cx-tty-18-2 sur a178fb923, remis sur 6373fb42e par la garde ; quatre commits gmail, zéro attribution ; rejeu court dans le target du worktree, compilé depuis lui : crate 109 + 20, check_features 8/8 ; une résolution dans nouveautes.md). /nom nomme l'agent de la tâche par la route d'ED, ses lignes portent le nom ; binaire de Lucie rebâti depuis 4802a147a et posé 22 h 25. **Pose suivante demandée à G** aux deux postes sur 4802a147a5 (les hôtes sont sur 23dc8efd47, sans la route ni le nom) : 96 second tour et composeur, nom et persona, ouverture sans la feature, gardes, route de renommage, les quatre lots de G ; sans attendre hotfixs ni l'arbre.

- 22:27, G : pose sur 4802a147a5 lancée (bâti à luciepc, essai 7991 aux deux postes, verdict des trois suites sous MOTEUR_ANCIEN, lib 373b5477b ; ~15 min ; coupure annoncée une minute avant ; services inchangés). tty corrige : « /nom y marche » était faux, joué contre aucun hôte ; sur le 7970 de Lucie la route répond 404 « route inconnue », que le terminal traduit en « cet hôte ne sait pas encore renommer l'agent » ; essai réel sur une tâche d'essai après la pose.

- 22:29, embarquement : la mesure « carte réellement pleine » n'a pas eu de fenêtre ce soir ; card2 lue toutes les 15 s de 20 h 40 à 22 h 28, jamais sous 13 548 Mio libres (16 966 à 21 h 41, 16 901 à 22 h 06 après les vecteurs de code-principal, 16 773 à 22 h 28 après code-lourd) : les vecteurs de l'arbre passent par le 7878 (granite), qui ne remplit pas la carte ; ce qui la remplissait à 20 h 03 (11 844 Mio libres) était la parité du banc et une étape plus ancienne. Rien lancé, rien touché au 7879 ; guetteur arrêté. Prochaine occasion : une passe qui charge card2 en mémoire (modèle résident en plus, gros travail GPU), au « va » de son propriétaire ; ticket de mesure ouvert avec le relevé. Le va-du-banc peut s'effacer.

- 22:29, arbre, **dernière répétition de la bascule VERTE sur les deux bases** (binaire a57557040 : écrivain 794b14232 + `migration generations --base`, branche arbre-principal/migration-generations-3) : code-lourd 1b 649 s, montée 1 s, repassage vecteurs compris 3 632 s ; code-principal 575 s, 0 s, 2 657 s ; miroirs CONSUMES/CONSUMED_BY visibles 0/0 des deux côtés ; banc d'avant retrouvé VERT après. Dans la page de la fenêtre. Il ne reste à la bascule que l'atterrissage de l'écrivain avec la montée en pile, puis une pose qui les porte.

- 22:30, hotfixs : passe entière VERTE sur dee35f6a0 (luciepc 22 h 06 → 22 h 3x, lib 373b5477b, MOTEUR_ANCIEN dit ; e2e_lectures 27, e2e_code 36, check_features 8/8, lib 2527/0/72, navigateur 45 ; vrai témoin `backend::tests::une_extension_notee_qui_ne_se_charge_pas_se_dit_a_l_ouverture ... ok`, rouge vu sur master avant). Reposé sur 0294f31d6d : extension-non-chargee-dite-4, tête f4570d27f (le rebase apporte la route d'ED et le tty) ; rejeu court en file (`cargo test --lib -- backend::`, jeu le plus large, ligne ok du témoin exigée). « À toi » à sa couleur.

- 22:33, **hotfixs poussé** extension-non-chargée-dite : master 7ece05d8a2 (e00261d89, 5c33ffa82, 7ece05d8a sur f044f6b87 ; trois commits gmail, zéro attribution, vérifié). Rejeu court VERT sur f4570d27f (`cargo test --lib --features rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm -- backend::` : 150/0/1, ligne ok du témoin, lib 373b5477b, MOTEUR_ANCIEN dit) ; garde ad0be1f36ff12ea5, « rien de code » entre 0294f31d6d et f044f6b87 ; trois contrôles. Une extension notée qui ne se charge pas se dit à l'ouverture et à l'état. La place est à l'arbre (2b′, passe en cours), qui repose d'un cran à son vert avec un rejeu court (backend.rs apporté).

## 16 novembre

- 00:03, reprise après un reset de Lucie : toutes les sessions en pause depuis ~22 h 37. Master inchangé à 7ece05d8a2 (hotfixs). **Hôtes posés aux deux postes sur 4802a147a5** (md5 8d9e62c3, portable pid 2214198 ; lib 373b5477b) : 96 second tour et composeur, nom et persona d'Algareth, ouverture sans la feature, gardes de recherche, route de renommage, les quatre lots de G. Services 7878/7879 actifs à luciepc ; aucune unité de passe n'y tourne (la passe de 2b′ de l'arbre à vérifier). Toutes les sessions recontactées pour l'état de leur travail.

- 00:04, 96, segment prêt (cx-memoire-segment-pile, tête b0fcd340c, 8 commits d307beeaa → b0fcd340c sur cbe5ba25c, l'ancienne tête du second tour) : LlmNode sous response_schema (JSON sur le port `json`, réponse non conforme refusée en nommant response_schema ; `value` Map prend le texte d'un RhaiNode) ; ForEachNode `when` et `flatten` (ordre gardé, when_missing et dropped au rapport) ; archive découpe en sujets « <titre> §a-b » au-delà de `segment_over` (0 par défaut, éteint) ; voie 2 : LlmNode admis dans un outil seulement avec response_schema et sous call_models, validate_tool_policy lit les nœuds, sous-graphes compris. Rouges vus avant le code (LlmNode 3, when/flatten 3, deux e2e, politique) ; vert au portable (politique 1, llm_nodes 17, foreach 16, archiviste 25, compose 2, memoire_modele 4 ; lib 2472/1, le 1 = codex_threads « Text file busy » sous charge, vert seul, ticket 733975bc9). À son « à toi » : rebase --onto des 8 commits, rejeu court à luciepc, garde en mode simple ; après l'écrivain d'ED et le grain du paquet.

- 00:05, après la reprise : lucivy, aucun lot, 4802a147a5 porte bien la maintenance nocturne et le relevé à blanc ; lecture de fts_maintenance après 03 h 30, fermeture du ticket du dictionnaire des effacés au vert ; mot donné pour effacer ses copies (poids-familles 4,7 Go + Lucivy_Scope 1,4 Go ; mesure-fts 4,3 Go ×2) et ses worktrees par la garde de retrait, le target de session gardé. Embarquement : repli-activation 1f1895eb64 (9 commits) prête, aucun processus ; rag3db-repli-nuit gardé jusqu'à la mesure ; « à toi » après codeparsers. ED : écrivain reposé sur 7ece05d8a2 (ecrivain-2a-prime-2-pile-3, 2db4fd78e, 41 commits gmail, un conflit nouveautes.md), passe complète à luciepc ; rien d'autre n'a bougé ; le grain (8cddace0b) se repose sur l'écrivain atterri et se rejoue. Ordre : 2b′ (à vérifier), 2b-C, puis l'écrivain à son VERT et au verdict du banc, migration en pile, puis codeparsers, embarquement, grain, segment de 96.

- 00:06, banc, verdict sur 794b14232 contre l'index neuf (fini pendant la pause) : graphe égal à deux arêtes près (MENTIONS +1, USES_LIBRARY −1), CONSUMES et CONSUMED_BY égaux, miroir 0/0, aucun doublon, histoire de DEFINED_IN gardée, usages verts ; **mais 23 cas sur 30 ne rendent pas le classement de l'index neuf** (de vrais rangs). Piste non vérifiée : 136 181 morceaux de ScopeContent au plein texte contre 114 854, l'histoire pèse sur BM25. Décidé : pas de push de l'écrivain avant la cause prouvée par ED sur les exemples (requête, candidats des deux côtés, origine des morceaux en plus) et la réponse « prévu par 2a′ ou non, et ce qu'une vue voit à la requête » ; si non prévu : ticket + mesure P1/rappel vue courante contre index neuf par le banc, puis décision. La mesure longue n'avait pas tourné (script recopié sans droit d'exécution, refusé par poste lourd dès 20 h 49, journal non relu) ; relancée 00 h 04 : F 25 %, variante, S 50, 300 k, sous le veilleur du 7879.

- 00:07, ED, lecture du verdict : le filtre de visibilité ôte les versions fermées des résultats mais pas des statistiques du corpus (IDF, longueur moyenne) ; le rang dépend de l'histoire gardée (PortDef 4 → 3, basic_json au rang 8 dans « json »). Trois issues : (a) accepter et juger contre un repassage ; (b) retirer du plein texte les morceaux des versions fermées (une lecture épinglée perd la recherche lexicale) ; (c) statistiques BM25 au rang visible, dans lucivy. ED recommandait (b) pour la bascule, (c) en ticket. **Décision c7** : pas (b), une vue épinglée est un worktree en retard, le cas de la vision, et un filtre ôte ; (c) est la voie, lucivy (libre) en fait l'estimation et le dessin tout de suite, page soumise avant le code, ticket chez ED ; le banc prouve la cause (effacer les fermées du plein texte sur sa copie, rejouer les 30) et mesure P1/rappel sur les 30 ; **l'écrivain atterrit** à sa passe verte (les bases d'aujourd'hui n'ont pas d'histoire) avec la migration en pile ; **la bascule des bases de Lucie attend** (c) posé, ou une mesure sans bonne réponse perdue, auquel cas elle part avec (a) en attendant (c). Lucivy : ménage fait par la garde (worktrees et copies des deux postes, archives 20261116-0005xx ; garde ROUGE au premier passage à luciepc pour les traces de run_e2e.sh, recette du ticket appliquée, VERT) ; target-lucivy-session (26 Go) gardé.

- 00:09, ED : ticket e47736a79 (l'histoire gardée au plein texte déplace le classement BM25 de la vue courante). Lecture du code : le dessin 2a′ le prévoyait (plein texte commun, visibilité par le masque passé à lucivy en `allowed_ids`) ; le masque est un pré-filtre, `search_filtered` (lucivy 4.3.7) compte N et les fréquences dans la vue ; **ce qui fuit est la longueur moyenne du champ, gardée exprès sur le corpus** : la normalisation de longueur bouge avec l'histoire. À vérifier sur la copie (au ticket) : `tout_visible` faux à la réouverture ; une version fermée ôtée par le masque et non par l'Admission. Le couple MENTIONS/USES_LIBRARY est un scope inchangé qui garde MENTIONS gzip d'une résolution ancienne (famille du parent). Lucivy resserre son dessin à la longueur moyenne sur les admis. **« À toi dès ton VERT, sans second mot » à ED** pour l'écrivain (2db4fd78e ; e2e déjà verts : code 36, lectures 27, demon_hote 24, postgres 39).

- 00:09, ED retire son lot : **défaut trouvé** : à l'ouverture d'une base à générations, `Visibilite::tout_visible` est posé à true sans condition (catalog.rs, fin de l'activation des générations) ; seuls l'écrivain dans son processus et un témoin le remettent à false. Une base avec histoire rouverte par un autre processus croit tout visible : lecteurs sans filtre (read_view.rs:393, :547) et recherche sans filtre partent sans masque et voient les lignes fermées. C'est le cas du juge du banc (copie rouverte après l'index) : recherche lexicale sans allowed_ids sur 136 181 morceaux, N, fréquences, longueur moyenne et candidats portaient l'histoire ; le graphe était juste parce que le juge borne par _jusqu_a. Correctif : tout_visible lu sur la base à l'ouverture (requête bornée par table à générations) ; témoin rouge d'abord ; passe ; le banc rejoue les 30 sur la tête corrigée ; lucivy attend ce rejeu, (c) ne garderait que la longueur moyenne. **« À toi » retiré.**
