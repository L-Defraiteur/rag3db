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

- 00:10, ED confirme : il ne pousse pas 2db4fd78e même vert ; l'écrivain en service fait naître les premières lignes invisibles, et toute base rouverte verrait son histoire dans les lecteurs sans filtre (recherche, comptes, listes) ; témoin rouge et correctif sur ecrivain-2a-prime-2-pile-3, passe, une heure. tty : /nom joué pour de vrai sur le 7970 (4802a147a5), tâche d'essai à lui (essai-nom-tty, rejetée ensuite), les tâches de Lucie intactes : `/nom Ombre` renomme, la ligne porte « [Ombre · compte rendu] Je m'appelle Ombre… » ; `/nom -` reprend « Algareth » ; la route nue répond {ok, result: {agent_name}}. Trois défauts : (1) `/nom` seul après un renommage dit « l'agent n'a pas encore dit son nom » (la vue ne retient que le name des lignes) ; (2) la demande s'écrit deux fois (« › texte » puis « [toi · demande] ») ; (3) chez archi : une consigne après la fin du pas n'a eu que step_done, ni compte rendu ni relance, (B) doit valoir pour une consigne. (1) et (2) : tty-19 en branche ; (3) : ticket archi ; « Om bre » accepté par valider_le_nom_de_l_agent : à trancher chez ED. **Lucie peut essayer /nom.**

- 00:13, lucivy : page 01-bm25-au-rang-visible (7601927690) : la cause principale chez ED (tout_visible), à lucivy la longueur moyenne (mise à l'échelle N_vue/N_corpus depuis 5ede7b9 « a property of the text, not of the filter ») ; dessin : somme des longueurs des admis dans la boucle du masque, corrigée par segment, `with_subset(N, tokens par champ)`, pour toutes les recherches filtrées ; prix ≈ celui du masque (1,2 à 1,6 ms sur 72 652 ids), 4.3.9 ; témoin rouge A = V, B = V ∪ H, search_filtered(B, V) = search(A). Mot donné pour le lot ; le tag 4.3.9 à mon mot après le rejeu du banc sur la tête corrigée d'ED. tty-19 prêt (cx-tty-19 = 7e7a48deb : nom rendu par la route retenu par la vue, demande écrite une fois ; rouge vu, crate 111 + 20, harnais VERT) : **« à toi » à tty**. Lucie relance son terminal : `~/.cache/rag3weaver-build/rag3weaver-tty --depot rag3db --tache atelier`.

- 00:13, ED : tête corrigée de l'écrivain **5081d2281** (ecrivain-2a-prime-2-pile-3, base 7ece05d8a2). Témoin 21 `une_base_rouverte_sait_qu_elle_a_une_histoire` : rouge sur 6b3579115 (« la base rouverte a une ligne fermée : elle n'est pas tout visible »), vert sur 5081d2281 (lecteur sans filtre porte _jusqu_a, raccourci « tout visible » gardé pour une base sans histoire) ; suite de l'écrivain 19/19. Correctif (catalog.rs, pose des générations) : par table à générations, `MATCH (s) WHERE NOT (visible au rang en attente) RETURN s._uuid LIMIT 1` ; Postgres non touché. Passe complète en cours ; celle de 2db4fd78e arrêtée. Ticket BM25 : cause première posée (8ec70adb6). Le banc rejoue les 30 sur 5081d2281 au vert de la passe.

- 00:18, **Lucie, nouveau but** : « un agent que tu puisses orchestrer comme les autres, tout en ayant son terminal affiché, et sachant communiquer avec les sessions Claude traditionnelles, et vice versa, même genre d'affichage que quand toi tu reçois ou émets » ; « quitte à ce qu'il fasse au début des recherches pour vous, et quelques tâches bien établies que vous reviewez après ; puis une fois validé/débuggé on met argon dessus et il sert pour de vrai à coder avec vous ; qu'il ait chacun de vos worktrees sous la main en index aussi si possible ». Page de vision `visions/16-11-2026-algareth-pair-des-sessions-claude.md` : Algareth pair du bus des sessions (cc-socks), terminal affiché, tâches relues d'abord, worktrees des sessions comme vues ; archi relève le bus par une socket témoin puis dessine, tty affiche, arbre/recherche les vues. Première tâche réelle pour Algareth : le nom d'agent qui accepte un blanc (backend.rs:995), témoin rouge puis refus, « prêt » et c7 relit ; ED ne la prend pas. tty-19 poussé : master 3ad8950c4f (deux commits gmail, zéro attribution) ; binaire de Lucie posé 00 h 15 (md5 d49aa1fe), elle relance quand elle veut. Banc : rejeu des 30 sur 5081d2281 par copies reflink rouvertes (releve-tete.sh), après sa seconde preuve d'élagage (la première élaguait les morceaux, le plein texte des versions est sur ScopeContent).

- 00:20, banc : **la preuve par élagage est négative**, ce n'est pas l'histoire au plein texte seule (tout lu par 794b14232, lexical seul) : l'élagage de la copie du lot retire 14 935 versions fermées et leurs morceaux, plein texte rebâti depuis les lignes (1,2 Go → 721 Mo), l'élagué a exactement les comptes de l'index neuf (70 018 ScopeContent, 114 854 morceaux, 77 154 Scope) ; pourtant 24 cas rouges sur 30 contre l'index neuf (23 avant), 28 contre le lot tel quel ; et « read_text » filtré src/include/ ne rend rien à l'élagué (plein texte rebâti ou filtre). Sous réserve : à contenus égaux le classement dépend encore de l'histoire des lignes (supprimés gardés dans les statistiques, ordre des identifiants) ou de la lecture (tout_visible). La relecture par 5081d2281 sur les mêmes copies départagera ; sorties à luciepc banc-parite/sorties/preuve-bm25/. Lucivy : ce n'est donc pas la longueur moyenne ; son lot reste juste mais n'est pas la cause, pas de tag pour ça ; il compare lucivy seule sur les deux plein texte (neuf, élagué), résultat par résultat par empreinte du contenu, N et tokens par champ : même classement par contenu → l'écart est au-dessus (offsets ↔ lignes après rebâti, filtre de scope, Admission) ; différent → la statistique en cause.

- 00:21, banc corrige : la preuve par élagage est **invalide**, l'élagué est faussé par l'élagage lui-même. Lexical seul, rappel à 10 (avec / sans le seul cas vide « read_text » src/include/) : neuf contre lot 23/30 rouges, 0,900 (23/29, 0,899) ; neuf contre élagué 24/30, 0,668 (23/29, 0,672) ; lot contre élagué 28/30, 0,585 (27/29, 0,589). L'élagué s'éloigne de l'index neuf plus que le lot, à comptes exacts : **le plein texte rebâti depuis les lignes n'est pas celui que l'écriture tient au fil de l'eau** (offsets ↔ lignes) ; une copie rebâtie ne sert pas de référence ; rien n'est tiré sur la cause. Le départage reste la relecture par 5081d2281 sur copies non rebâties, au VERT d'ED. Un défaut à part en sort : « dossier jeté, rebâti depuis les lignes » rend un plein texte qui classe autrement (rappel 0,90 → 0,67) : ticket demandé à ED avec lucivy.

- 00:24, lucivy, lucivy seule sur copies reflink de ScopeContent (15 requêtes : les 10 du banc en contains + parse ; top 10 par empreinte de contenu ; test_compare_two_indexes, branche bm25-longueur-de-la-vue 055ace9). (1) Neuf contre élagué : N 70 018, tokens identiques, 12/15 SAME, 3 DIFF à écart de score nul (ex æquo rangés autrement, max 1,6e-7) : **lucivy classe le rebâti comme le neuf**, le 0,67 du banc vient d'au-dessus (attribution aux morceaux, Admission, offsets ↔ lignes) ou de l'ordre des ex æquo que rag3weaver ne départage pas. (2) Neuf contre lot avec histoire (visible approché, 70 660) : N 84 953, content 31 618 718 tokens contre 9 397 734, l'histoire est trois fois plus longue que le visible ; **main (moyenne du corpus) : 11/15 DIFF, écart de score jusqu'à 69 %**, premier résultat changé pour deux requêtes, PortDef perd son 2ᵉ ; **sa branche (moyenne des admis) : 4/15 DIFF, écart ≤ 2,6 %**, trois premiers identiques partout. Correction de lucivy : la longueur moyenne **est** la grande cause du banc sur le lot. Reste : l'ensemble visible approché (642 documents d'histoire au contenu identique) et le codage des longueurs ; rejeu avec le vrai fichier d'offsets d'ED. Décidé : le lot de lucivy est la correction ; 4.3.9 au vert et au rejeu du banc ; pointeur dans rag3weaver derrière l'écrivain ; ex æquo non départagés par rag3weaver : ticket (ordre stable).

- 00:25, première tâche réelle d'Algareth (le nom avec un blanc) : il n'a pas trouvé `valider_le_nom_de_l_agent` et l'a dit honnêtement ; cause : le worktree de la tâche atelier (branche agent/atelier) était à 88cbad8611, **350 commits derrière master**, la fonction n'y existait pas ; il n'a pas dit la position de son arbre. c7 l'a avancé en avance rapide sur master (aucun commit propre) ; Lucie redonne la demande. Ticket chez archi : à l'ouverture, une branche agent/* sans commit propre avance sur master et le dit ; sinon l'agent dit sa position quand une demande cite du code (vision du 15 nov., point 1). ED : **5081d2281 VERT** (e2e_code 36, lectures 27, demon_hote 24, postgres 39, check_features 8/8, lib 2548/0/72, navigateur 45) mais second défaut lu : `allowed_ids_in` (generic_search_nodes.rs, BM25 et Sparse) ne passe aucun allowed_ids sans condition de l'utilisateur, même sur une base à histoire ; le top-k se prend fermées comprises et la visibilité ôte après (`admitted_parent_uuids_for`) : une perte possible ; le dense est juste (clause SQL). Correctif dans le même lot : sans condition et si tout n'est pas visible, `Must(vec![])` ; témoin rouge (fermée mieux classée, k=1, l'ouverte rendue) ; passe ; rejeu du banc. Pas de contrordre.

- 00:28, deuxième demande de Lucie à Algareth (arbre à jour) : il localise la fonction (backend.rs:995–1003) et le test existant (vers 9044), puis rend la main avec « ce qui reste à exécuter au prochain pas » (cinq points), sans rien écrire ni jouer. Défaut prioritaire du socle, chez archi, avant le relevé du bus : le pas se prend pour la tâche ; il doit se réenchaîner jusqu'à la demande faite ou un accord manquant ; le compte rendu vient à la fin, les étapes dans la zone vivante ; un « reste à faire » final déclenche une relance bornée par l'hôte. En attendant Lucie lui dit « continue jusqu'au bout sans t'arrêter ».

- 00:31, Lucie : « joue mon propre rôle jusqu'à ce qu'on sache vraiment s'en servir comme autre session (nouveau but court/moyen terme) ; fais-le sans moi ; ça devrait être ça vos tests, pas unitaires, empiriques : contrôler chaque outil de recherche s'il a bien marché ». c7 ouvre la tâche `c7-essai` (Algareth, backend 12389e156742, arbre sur master) et la page `docs/orchestration/16-11-2026/01-essais-empiriques-algareth.md` (protocole : attendu avant, outils relevés au journal, verdict par demande, ticket par rouge). D1 lancée (où est définie valider_le_nom_de_l_agent).

- 00:34, ED retire le second défaut : une sonde dans le nœud BM25 (branche sonde-masque-lexical) montre que la visibilité arrive au nœud **en condition de filtre explicite** (`Must[Scope._couche = backend, _depuis <= rang, _jusqu_a nul ou > rang…]`), `allowed_ids_in` la résout, lucivy reçoit l'ensemble avant le top-k (152 offsets au témoin) ; pas de perte sur search_code. Le témoin reste en garde (trois fermées mieux classées, k=1, l'ouverte rendue). Les 23 cas : ni tout_visible ni perte, la seule longueur moyenne (lucivy). 5081d2281 reste vrai pour les lecteurs sans filtre hors de ce chemin. Tête = 5081d2281 + tests, passe rejouée dessus. **« À toi dès ton VERT, sans second mot » à ED** ; le banc rejoue les 30 sur master après, en référence pour lucivy ; la bascule attend lucivy 4.3.9 et son pointeur.

- 00:37, **Lucie, règle** : « stop au code en français, ou aux /commandes en français, etc. ; on va pas tout changer maintenant, mais les prochains on évite : que de l'anglais » (elle ne l'avait pas dit « pour ne pas nous freiner »). Portée : tout ce qu'une machine lit ou que l'utilisateur tape comme commande : identifiants, noms de tests, /commandes du terminal, options, clés de manifeste et de JSON, noms d'outils et de nœuds, kind/from des lignes de boîte, unités et scripts. Le français reste pour la prose (docs, commits, commentaires, messages à l'utilisateur). Pas de lot de renommage de l'existant ; un alias anglais quand on touche la chose. Dit aux treize sessions.

- 00:42, Lucie, précision : « les outils et descriptions, quand c'est pas pour des tests mais pour un produit, on met toujours en anglais (et gabarits etc.) » : la prose que lit le modèle ou qu'expose le produit (descriptions d'outils, invites, textes des gabarits builtin et des prompts, erreurs rendues par l'API) est en anglais ; docs, commits, commentaires, tickets et l'affichage à l'utilisatrice restent en français ; les textes de tests peuvent rester en français. Dit à ED, archi, 96, tty. D1 verdict écrit ; D2–D6 déléguées à un agent de fond (trouver la trace des appels d'outils, accorder les lectures, verdicts).

- 00:46, G, **posé** 4802a147a5 aux deux postes (md5 8d9e62c3, check-lib f4956edf, lib 373b5477b) : verdict 22 h 31–35 (e2e_code 36, lectures 27, demon_hote 24, MOTEUR_ANCIEN) ; luciepc 22 h 37 min 04 réveil VERT, code-lourd et code-principal rouverts, de nouveau au repos par la socket ; portable 22 h 37 min 07 → 15 (8 s de coupure), cinq bases rouvertes ; relu 0 h 45 : pid 2214198, l'atelier s'ouvre avec 23 outils ; anciens binaires en .avant-4802a147. Trois lots d'outillage, rien poussé : chemin absolu (passe-chemin-absolu, ec0654ce5, témoin 23 vert, rejeu court à faire) ; **(b) le refus avant la file est dans poste lui-même** (poste-plafond-validation, f39f565cd : une commande qui nomme run_e2e.sh, temoins_navigateur.sh, passe_epinglee.sh ou confiner_validation.py sous un plafond > 32 Gio est refusée code 2 « lancer sous POSTE_MEM_MAX=32G », seuil = MAX_MEMOIRE du confineur, test_poste 35) ; témoin de lib `run_e2e.sh --lib <filtre>` (3 rouges, correctif en cours). Renommage en anglais des noms neufs avant la pile. Hotfixs : rien de mort ; test shell instable reproduit à froid (1/100 TIMEOUT multiline-33-50 ; montage à part sous ~/.cache/rag3db-locks/shell-instable), cause cherchée dans la lecture des séquences d'échappement ; défaut du cœur à choisir avec le cœur.

- 00:47, réponses aux relances (personne n'était bloqué, messages croisés). **Cœur** : passes finies 23 h 48, vertes ; pool par écriture 42 095 → 471 octets (vecteur 25 149 → 373, titre 8 526 → 102, instruction à 20 connexions 42 101 → 477) ; known_red vert, e2e_test 1 867 ; DdlEveryMinute ×10 CONCURRENCE_LONG : 0 refus du pool, 0 SIGSEGV, 0 « Cannot find property », 10/10 comptes justes (Note 40 427 → 66 616), pic 2 000-2 052 → 331-445 Mio ; (b) pas demandé ; un rouge sur 10 : create-rel-table-holds-writers-at-most-its-duration (837 ms pour 720), intermittent d'avant (5 journaux sur 50), ticket. Décidé : la ligne known_red de DdlEveryMinute part, le cas va dans probabilistic.txt avec le ticket cité ; tickets pool et Note fermés dans le lot. **« À toi » au cœur**, voie C++ transférable. **Codeparsers** : prototype vert en branche ; la cause réelle : une source .c ne relevait aucun #include (fermeture réduite à elle-même) et la garde extern "C" de rag3db.h, fermée dans un autre #ifdef, plus un #if defined(__APPLE__) dans une struct, faisaient perdre à tree-sitter les 167 prototypes (→ 0) ; deux correctifs (includes du C comme le C++ ; garde masquée pour l'arbre, octets et lignes inchangés) ; témoin e2e rouge sans l'un ou l'autre (13 « invisible »), vert avec les deux ; codeparsers entière, e2e_usages 29, e2e_code 55 ; sonde sur 1 725 fichiers : CONSUMES « nom » 4 572 → 4 618, 27 couples naissent (main.c → 16, test/c_api → 11), 0 partent ; noms neufs en anglais ; pile ea493327d + commit des noms. **Archi** : cause de « s'arrête après un pas » = la boucle (un pas sans step_done attend le superviseur, rien ne relance) et l'invite (« a step is one piece of work… remaining »), pas le budget ; lot court rouge d'abord : invite « au bout dans le pas », relance bornée par MAX_STALLED, copie de l'hôte en champ `origin: "host_copy"` ; passe juste après l'écrivain d'ED ; puis relevé du bus, (B) pour une consigne, position du worktree. **Recherche** : 2b-C reposé sur 75f827250 (ea64665c67), rejeu court vert, commit des noms anglais puis rejeu, attend la passe de 2b′. **G** : rien de bloqué.

- 00:50, **CI rag3weaver rouge sur master** depuis 7ece05d8a2 (run 43773815569, 21 h 33 UTC) et sur 3ad8950c4f ; « CI Workflow » vert. Job « check_features (cinq jeux) et tests unitaires », pas `cargo test --lib (rag3db-native,code,daemon,openai-llm)` : `daemon_host::atelier_fichiers::tests::a_workshop_write_leaves_a_run_with_its_diff` panique à atelier_fichiers.rs:1350 « le run se relit », 2479/1/72. Dernier vert e692fa3a58 ; entre : a178fb9231 (route de renommage, ED), 4802a147a5 (tty seul), 7ece05d8a2 (backend.rs, hotfixs). Les passes locales (jeu large) le voient vert : jeu étroit ou ordre. Hotfixs rejoue ce jeu en local et bissecte ; ticket. ED a demandé qui avait la CI rouge au bandeau de poste push. Ordre corrigé (ma faute, deux mots à la fois) : le cœur pousse d'abord (C++ seul), ED attend son « poussé » ; si la garde refuse le C++ apporté sous lib épinglée, repose sans rejeu et on l'écrit à la garde. Codeparsers prêt : pile d7dffe335 (noms anglais), place après archi et 2b′/2b-C. Arbre : six points sur les vues des worktrees (sélecteur par motif, générations par branche et couche de travail = 2a′-3, plafond max_work_layers, sucre de nom, fin d'une vue, fraîcheur) ; ordre accordé, page de conception avant le code ; 2b′ se pousse tel quel, renommage anglais dans un petit lot juste après avec recherche. Recherche : 2b-C reposé sur d5a62bc97 (f2a0d1beab, 4 commits dont les noms anglais), rejeu court en cours. ED : écrivain reposé sur 09bd9851fe (ecrivain-2a-prime-2-pile-4, 818903d61, 47 commits gmail), rejeu court à luciepc, push par la garde au VERT après le cœur.

- 00:50, cœur ne peut pas pousser tout de suite (dit plutôt que forcé) : sa tête porte aussi le déplacement de DdlEveryMinute vers probabilistic.txt (1d64f47ab, .txt et une ligne de CMake), juge en cours jusqu'à ~1 h 05 ; et depuis sa base 514841e8e master a changé test/transaction/concurrence/compare_known_red.cmake et test_compare_known_red.sh (juge de G), la voie transférable exige rien sous src/ ni test/ : repose et rejeu du juge + cibles (~20 min). **Ordre inversé : ED d'abord** (« à toi dès vert »), le cœur repose sur sa tête ensuite avec le déplacement, à toi dès son vert. ED a arrêté son script passe+push et relancé la passe seule (ed-ecrivain-4-passe, 818903d61). Codeparsers : tête 8eaf6edf5 (rapport du 15 ajouté, code inchangé). Arbre : page de conception de 2a′-3 posée (b39c50c0f, docs/arbre-principal/16-11-2026/01) : couches d'histoire par branche (objets git sans checkout), couches de travail par worktree (parente épinglée au HEAD, attend `tip`, HEAD détaché refusé), montre à un fil avec `tracked.writes.max_work_layers` 32 par défaut, sélecteur worktree:<motif>, fin d'une couche, fraîcheur par la page 12 ; sept témoins rouges ; code après 2b′/2b-C et la bascule.

- 00:51, lucivy : lot de la longueur moyenne vert, PR lucivy #39 (9980b11), CI en cours, fusion dans main au vert ; le codage visé (longueur lue au milieu du palier de fieldnorm, la borne basse sous-estimait de 6 %, corrigée par le total exact du segment) ; témoin rouge sur main (écart 3,98e-1), vert à 1 et 2 shards (rangs identiques, écart 3,97e-4 et 1,89e-3 sous 5e-3) ; lucivy-core 237/237, clippy propre, coût du filtre inchangé. Copies du banc : main 11/15 différents (69 %), branche 4/15 (≤ 2,65 %). Reste : le vrai ensemble visible par resolve_filter_to_ids (ED, ~30 min), rejeu des 15, puis « prêt ». Le tag 4.3.9 à mon mot.

- 00:53, 96 : segment vert, tête f5a59210a (10 commits dont les deux passes à l'anglais : tests/aides/nœuds/clés, puis prompt des sujets et descriptions neuves) ; lib entière 2473/0 au portable. Hotfixs, la CI rouge : **une course dans le journal des runs**, antérieure aux trois commits : journal_des_runs.rs `ecrire_le_fichier` fait `std::fs::write` (fichier créé vide puis rempli) ; runs_de() liste déjà le run, lire() échoue à décoder un zstd vide ou tronqué → « le run se relit » sur un runner lent ; même défaut pour tout lecteur du journal pendant une écriture (IDE, chat). Correctif accordé : écriture atomique (temporaire + rename), témoin rouge d'un lecteur en parallèle ; lot court juste après l'écrivain et le lot court d'archi, devant 2b′.

- 00:54, banc, F 25 % finie sur 0653a6aeff (244 décisions, 43,5 min, aucun refus du 7879) : 40 créer, 160 rejoindre, 43 rien, 1 sans étiquette (avant : 66, 106, 72) ; 31 fiches, 2 doublons (0,065) ; P1 contre N 0,77 (60/78), juste sous le seuil ; les 43 « rien » : le contexte aide 19, endort 6 (22 avant), accord 18 ; sujets encore perdus : banc des fiches classées, coordination des intégrations, diagnostic des crashs, indexation des worktrees, outils rag3weaver en session, vision produit. Usage réel 6 197 085 jetons d'invite (607 761 en cache, 10 %), 6 989 de sortie ; le plafond à 4 caractères/jeton disait 4,28 M ; le cache implicite sert peu (le préfixe bouge). Réserves : N jouée sur l'ancien binaire avec les pannes du 7879 ; binaire et chevauchement changés ensemble ; N rejouée sur 0653a6aeff en file après la variante, S 50 et 300 k. Détail page 02 du 15, § 9.

- 00:55, archi : « le pas va au bout » vert sur ses témoins (pas-va-au-bout, 12a7f6987 sur 7f9465fa9 ; rouge 7d3151673 avec 2 rouges vus : l'équipier attendait 90 s, l'invite parlait du pas comme d'une unité). L'invite : « work the request through in your step until the request is done… step_done only when the request is done, or when you need something… never stop to describe what you will do next: do it now » ; la boucle relance un pas fini sans step_done, borné par MAX_STALLED, frontière « continue until the request is done » ; copie de l'hôte : réponse seule dans text avec `origin: "host_copy"`, plus de préfixe ; un pas sans step_done ni réponse dit en anglais ; le témoin du 7 nov. relit l'origin. Équipier et chat 62/0 ; passe complète et e2e_equipier sous Vertex en file au portable (luciepc plein). Repose au « poussé » d'ED, rejeu court, « prêt ».

- 00:57, archi, relevé du bus, première partie (rien lu des sessions vivantes) : une socket témoin posée dans /run/user/1000/cc-socks (0600, journalisante) de 0 h 55 à 0 h 58 n'apparaît pas dans ListAgents et personne ne la sonde : **la découverte passe par un registre**, `~/.claude/sessions/<pid>.json`, un fichier par session (pid, sessionId, cwd, startedAt, procStart, version, peerProtocol 1, peerFeatures [notify_idle, reply_across_default_dirs, artifact_yield], kind, entrypoint, pidDomain, messagingSocketPath, name, nameSource, nameSince, status, updatedAt). Pour qu'Algareth soit un pair : une entrée dans ce registre (procStart vérifiable) + une socket au protocole 1. **Archi ne le fait pas sans le mot de Lucie** : c'est écrire dans l'état de Claude Code et se présenter comme une session à toutes les autres. Question posée à Lucie ; repli : les canaux Claude Code. Socket témoin retirée.

- 00:59, **Lucie : « oui au registre »** (entrée témoin sobre dans ~/.claude/sessions, retirée après le relevé de la trame), **non aux canaux** maintenant (il faudrait redémarrer toutes les sessions), mais à préparer pour plus tard. Archi pose l'entrée algareth-temoin, c7 lui envoie un message, relevé de la trame, puis conception du mode pair de l'hôte ; page des canaux pour une prochaine relance des sessions.

- 01:02, **ED poussé l'écrivain des générations** : master c33574eb9 (ecrivain-2a-prime-2-pile-4, 818903d61 ; 47 commits gmail, zéro attribution, vérifié ; rejeu court vert : e2e_code 36, check_features 8/8, lib 2549/0/72, navigateur 45 ; passe complète du même code verte sur 4f582f1a9 ; garde ad0be1f36, aucun code entre f503bbe2a et 86c856e47 ; trois contrôles). Suite d'ED : offsets visibles pour lucivy, grain du paquet reposé (noms anglais), tickets ex æquo et plein texte rebâti. Recherche : 2b-C VERT sur d5a62bc97 (f2a0d1beab, noms anglais ; témoin, e2e_lectures 27, e2e_code 36, check_features 8/8, lib 2 542, navigateur 45). Ordre : **cœur** (repose sur c33574eb9, juge + cibles, à toi dès vert) ; archi repose « pas au bout » sur c33574eb9 (rejeu court, « prêt ») ; hotfixs le journal des runs ; puis 2b′ + 2b-C, codeparsers, embarquement, migration (arbre repose sur c33574eb9), grain, segment, G, lucivy.

- 01:04, arbre, reposé sur c33574eb9 : migration (migration-generations-5, ebb80f7d9, range-diff =, aucun fichier commun, pas de rejeu ; passe sur e887f09b5 aux témoins navigateur, e2e_migration, check_features, lib 2 549 verts) ; 2b′ (vues-2b-prime-6, 2fe80df17) : conflit dans generations.rs (l'écrivain ajoute `rang_publie` à Visibilite), résolu (PartialEq sur les deux champs) ; raccord : le prédicat de l'écrivain (code_sync.rs:697) prend le masque vide de 2b′ ; retrait de `Catalog::poser_la_vue` (sans appelant ; avec l'écrivain, `rang` est le rang où il écrit, une vue posée sur son catalogue l'aurait déplacé ; recherche passe par `avec_la_pile`) ; rejeu depuis 1 h 03 à luciepc (e2e_lectures, e2e_code, e2e_postgres, check_features, lib, navigateur). Recherche repose 2b-C sur 2fe80df17.

- 01:05, arbre, un trou de sens : `view: {layer: backend}` sans rang lit la dernière génération publiée n (promesse 1 de 2b′), l'appel sans vue lit le rang en attente w = n+1 ; or rien du produit n'appelle `publier_la_generation` : n reste 0 (l'état de la bascule) et une recherche sous vue serait périmée sans le dire. Tranché : (1) tout de suite dans 2b-C, la réponse dit `view: {layer, rank, pending, pending_rows}` (recherche, petit commit, témoin rouge) ; (3) la publication de la couche backend à chaque fin d'index dans 2a′-3 (ticket chez l'arbre, lié à la page 12 : « index prêt » = génération publiée) ; pas de (2) (refus nommé).

- 01:07, arbre, la montée prête (ebb80f7d9 sur c33574eb9 ; passe verte sur e887f09b5 : e2e_migration 6/6, check_features 8/8, lib 2 549, navigateur 45 ; range-diff =, aucun fichier commun). **« À toi » à l'arbre** pour la migration, tout de suite, pendant le juge du cœur ; le cœur attend son « poussé » puis à toi dès son vert.

- 01:10, arbre : la montée n'est pas poussée, deux refus justes de la garde (master bougé pendant l'appel ; « la pile a changé » : du tty apporté depuis la tête d'ED sous le commit testé) ; rejeu court depuis 1 h 09 sur ef0d2d867 (migration-generations-6 sur a572f5b6e), ~20 min ; un faux pas (rebase parti de la mauvaise base, conflit sur la file-en-pile, arrêt sur arbre sale, annulé, rien poussé). Hotfixs : témoin du journal des runs ROUGE sur master (24 runs listés non relus, puis 11 ; a_listed_run_always_reads_while_it_is_written, resserré avant de compter) et VERT sur le correctif (`.<nom>.<pid>-<n>.tmp` puis rename, trois appelants) ; branche run-journal-atomic-write sur d335078fd ; job unitaire exact de la CI en cours au portable ; tickets 3c5e6bce2d (cause) et faec3f6250 (le témoin de run_e2e.sh compte l'attente du poste, pour G). Ordre : le premier vert de l'arbre et du cœur pousse ; l'autre repose.

- 01:14, **cœur poussé** le pool : master 2385b5f616 (UpdateMemory, version de mise à jour née à 4 lignes, DdlEveryMinute vers probabilistic.txt ; voie C++ transférable, base c9ac4e2c3 → 8e6ef256a, rien dans le moteur ; trois contrôles ; commits gmail, zéro attribution, vérifié) ; tickets pool et Note fermés (b242a17ec), rapport c2f9a5a31. Sa file : refus « Schema changed, retry » sans fin après CHECKPOINT (blocage, 1/30, sonde prête) ; CREATE REL TABLE qui retient un écrivain ; eu-stack du CREATE_VECTOR_INDEX 237 s ; dictionnaire sous ASan. Pour la bascule de lib ce matin : master porte ALTER, colonne ajoutée, pool. **G prêt** : seconde pile sur fd4ff56c4d, rejeux verts l'un sur l'autre (1 h 06–13) : passe-chemin-absolu-2 788e9d5f6d, poste-plafond-validation-2 76ebbba88f (VALIDATION_MEMORY_MAX, to_bytes), run-e2e-temoin-lib-2 26c7422b09 (`run_e2e.sh --lib <filtre>`, vraie passe 1/1, filtre vide rouge dit) ; ticket faec3f625 (le témoin de run_e2e.sh compte l'attente du poste). **Campagne D2–D6 écrite** (9178cb58a9) : D2–D4 VERT, D5 juste sans search_code, D6 ROUGE (0 ligne écrite en 20 min, 7 pas, 82 appels, 0 edit_file) ; défauts S1–S6, J1 chez archi ; P1, P2, G1 chez ED ; T1 chez tty ; les appels d'outils se lisent au journal des runs (`~/.local/share/rag3weaver/runs/<backend de la tâche>/agent-*.json.zst`, tours entiers).

- 01:16, G, plan de la bascule de lib sur master (docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md, d33d6b28d1) : lib nommée 52101b7fd6 (22 commits moteur depuis 373b5477b : ALTER/DROP qui attendent les écrivains, « Schema changed, retry », colonne ajoutée, pool, IN sur la clé) ; ABI : l'IN sur la clé change PrimaryKeyScanInfo et PrimaryKeyScanNodeTable hors API publique, le cœur confirme ABI_CHANGEE ; étapes : bâti 30 min, verdict trois suites sans MOTEUR_ANCIEN 10 min, hôtes bâtis et essayés 10 min, fenêtre (instantanés, contrôle des deux sens, courant aux deux postes, réveil ; 20 min au portable, ~40 à luciepc), services d'embarquement inchangés, ~2 h ; ensuite un lot court : passe_epinglee.sh épingle la lib neuve sans MOTEUR_ANCIEN. **Mot donné : bâtir maintenant et jouer le verdict** (sans toucher aux hôtes) ; la fenêtre attend le mot de Lucie sur l'heure et l'ABI du cœur. ED : ordre accordé (offsets de lucivy, G1, P2, P1 avec archi, grain, ex æquo, rebâti).

- 01:19, cœur : ABI vue par les binaires Rust inchangée (aucune classe changée atteinte depuis main/rag3db.h, 80 en-têtes au préprocesseur ; test_hooks.h gagne une valeur en fin d'enum ; réserve : l'extension vectorielle du même bâti que la lib) ; sonde du blocage « Schema changed » en cours (pool réduit à 384 Mio). tty : T1 prêt (cx-tty-20 = d1b422a32 sur f105f551a) : en --plain la relecture de la boîte repartait de sa première ligne et prenait la fin d'une demande d'avant pour la sienne ; correctif : lecture depuis la longueur avant la demande, flux filtré sur le manifeste de la tâche (le mode texte écrivait les trames de toutes les tâches) ; rester attaché est le défaut ; témoin plain_stays_attached.py rouge sur master puis vert ; crate 111 + 20 ; page docs/tty/16-11-2026/01. Place : juste après la montée de l'arbre, avant archi.

- 01:19, hotfixs : job unitaire exact de la CI VERT en local sur le correctif du journal (5b63ce87e : 2481/0, le témoin ok) ; passe entière à luciepc en file (hotfixs-run-journal) ; ticket 3c5e6bce2. Test shell instable : cause prouvée (ce81a4e4f) : les fixtures lançaient le shell sans -p sur ~/.rag3db/history.txt de la personne, et Ctrl-P ne monte d'une ligne que sous Windows ; le test ne passait que grâce à l'historique des tests d'avant (vide → 5/5 rouge) ; branche shell-test-isolated-history (875cf3d31 historique isolé → rouge 5/5 ; c5af421d4 flèche haute → 245/245, 200/200) ; reste : d'autres tests sans -p, la suite écrit dans l'historique de la personne (ticket). Décidé : une pile par la garde (journal, puis shell), un « à toi » après archi.

- 01:21, **arbre poussé la montée** : master 1bf1483d67 (`migration generations --base` : une base passée par 1b monte aux générations par son chemin ; le bilan dit la génération publiée et le rang ; rejeu court vert sur ef0d2d867 ; garde : base a572f5b6e, rien de code jusqu'à b028c2e5c ; quatre commits gmail, zéro attribution, vérifié). 2b′ se rejoue sur bf0a8f6b8 (commit vide « sans effet sur l'analyse » pour la garde neuve de l'écrivain), 2b-C dessus. Hotfixs : ticket d54772cd44 (la suite du shell écrit dans l'historique de la personne, ~130 ShellTest() sans -p). **« À toi » à tty** pour tty-20 ; puis archi, hotfixs (pile journal + shell), 2b′ + 2b-C, codeparsers, embarquement, grain, segment, G, lucivy.

- 01:22, arbre : 2b′ rouge à la lib sur bf0a8f6b8, vrai défaut d'intégration avec l'écrivain : 13 témoins de `code::tests_de_l_ecrivain` et le témoin lexical sans filtre, « Parameter parente_rang not found » ; 2b′ ajoute `$parente_couche` et `$parente_rang` à merge_generation et la publication de l'écrivain (`publier_la_generation`, catalog.rs) l'appelle sans eux, chemin neuf sans conflit au rebase. Correctif 0e0fc6655 (une génération de la couche backend est une racine, ('', -1)), les témoins de l'écrivain jugent ; passe rejouée. Leçon (déjà en règle) : après un rebase qui apporte des appelants à une requête qu'on change, relire tous les appelants, pas seulement les conflits.

- 01:23, ED corrige : « tout_visible ne changera probablement pas les 30 cas » était faux : `masque_en_predicat` (generic_search_nodes.rs) n'ajoute _couche, _depuis, _jusqu_a que si `!tout_visible` ; avant 5081d2281 une base rouverte avait tout_visible vrai, son masque ne gardait que « la version a au moins une occurrence », fermées comprises : le cas du juge du banc ; le témoin d'ED avait l'écrivain du même processus (tout_visible faux), d'où la conclusion trop large. Reste vrai : pas de perte par allowed_ids_in ; la longueur moyenne est une grande cause (lucivy). Change : sur les copies du banc N, fréquences et longueur moyenne portaient aussi les fermées, candidates ; le correctif sur master (c33574eb9) les retire ; le rejeu des 30 par le banc sur master dira la part de chacun (mesure, pas condition). Le premier fichier d'offsets pour lucivy (77 154) est faux (sans la condition de visibilité), refait.

- 01:24, G, **lib bâtie** à luciepc (1 h 17 → 1 h 23) : rag3db-build-52101b7fd6…, md5 librag3db c79f71bc, libvector 5f45f60d du même bâti ; symboles contre 373b5477b : 13 ajoutés, 0 retiré, aucune vtable changée ; ABI_INCHANGEE (cœur) ; publiée au magasin sans courant (courant 373b5477b aux deux postes). En cours : verdict des trois suites sous RAG3DB_LIB explicite sans MOTEUR_ANCIEN sur ed14435570, puis hôtes bâtis contre la lib neuve, essayés sur 7991, lib copiée au portable. La fenêtre attend l'heure de Lucie.

- 01:24, hotfixs : passe entière du journal des runs VERTE à luciepc sur 5b63ce87e (e2e_lectures 27, e2e_code 36, check_features 8/8, lib 2530/0 avec les deux lignes ok, navigateur 45) ; pile prête shell-test-isolated-history-2 → 4397a5736 (journal puis shell, sur d335078fd ; `--code-apporte` vide pour le shell, preuve 245/245 et 200/200). Ordre : hotfixs **avant archi** (pas prêt) ; au « poussé » de tty-20, repose d'un cran, rejeu court, à toi dès son vert.

- 01:25, **tty poussé** tty-20 : master 9f0cbe7ce4 (cx-tty-20-2 sur d54772cd4, remis sur 368f08e2a par la garde ; crate 111 + 20, check_features 8/8, plain_stays_attached VERT ; trois contrôles) ; binaires de Lucie et d'essai rebâtis depuis 9de69102c et posés 1 h 24 (md5 db755e5d), T1 rejoué VERT dessus. Hotfixs repose d'un cran, à toi dès son vert.

- 01:32, G, **la bascule est prête jusqu'à la fenêtre** : verdict sous RAG3DB_LIB=52101b7fd6 explicite sans MOTEUR_ANCIEN sur ed14435570 (1 h 23 → 1 h 28) : e2e_code 36, e2e_lectures 27, e2e_demon_hote 24, aucun saut, chaque passe a lié la lib neuve ; hôtes bâtis contre elle (md5 72023043, check-lib d2848945), essai 7991 VERT aux deux postes, lib chargée 52101b7fd ; check-manifests 2 et 6, 0 refusé ; lib copiée au portable (c79f71bc, 5f45f60d) ; binaire en .nouveau au portable et nuit-ed14435570/ à luciepc ; courant 373b5477b et hôtes 8d9e62c3 inchangés. La fenêtre (~40 min luciepc, 20 portable) attend l'heure de Lucie ; ensuite le lot court qui épingle la lib neuve dans passe_epinglee.sh.

- 01:32, **hotfixs poussé** sa pile : master 08643d9232 (le journal des runs s'écrit d'un coup, temporaire puis rename, un run listé se relit toujours ; le test shell a son propre historique et remonte par la flèche haute ; cinq commits gmail, zéro attribution, vérifié). La CI rag3weaver doit redevenir verte au prochain run. Suivants : archi (à son « prêt »), 2b′ + 2b-C (au VERT de l'arbre), codeparsers, embarquement, grain, segment, G, lucivy.

- 01:33, lucivy **prêt** : rejeu avec le vrai ensemble visible d'ED (70 018 offsets) sur 15 requêtes de search_code, lucivy seule : v4.3.8 (moyenne du corpus) 11/15 différents, écart jusqu'à 69 %, premier résultat changé pour deux parse ; main b9f328de8 (PR #39, moyenne des admis) 2/15, écart ≤ 4,5e-3 ; les deux restants (read_text rang 4, transaction rollback rang 5) sont les ex æquo rangés par l'adresse du document (ticket d'ED ad9bf16185). Commit de version 4.3.9 prêt (branche version-4.3.9, f7da3be : neuf Cargo.toml, sept package.json, pyproject, CHANGELOG, What's new), format d'index inchangé. Mot donné pour main, la CI et le pointeur rag3weaver en branche ; le tag v4.3.9 au mot de Lucie.

- 01:34, arbre, **2b′ prêt** : 0e0fc6655 (vues-2b-prime-6) sur l'écrivain + docs ; passe VERTE (e2e_lectures 27, e2e_code 36, e2e_postgres 39, check_features 8/8, lib 2 559 dont les 13 témoins de l'écrivain, navigateur 45 ; MOTEUR_ANCIEN dit ; journal arbre-vues-2bp-6, rouges d'avant gardés à côté). Master a reçu tty-20 et hotfixs depuis sa base : repose sur 08643d9232, rejeu court, **à toi dès son vert** (archi pas prêt) ; recherche repose 2b-C dessus et pousse ensuite.

- 01:42, hotfixs : lot des ~130 ShellTest() sans -p prêt (shell-tests-own-history sur 3e53b8608) : un test créait history.txt dans tools/shell/test et tout shell sans -p le préférait à ~/.rag3db (shell_runner.cpp:139), d'où un premier témoin qui passait sans prouver ; fixture de session qui compare les historiques partagés avant/après (97144956e), témoin étendu au dossier courant → ROUGE (2e88cb430), correctif : un TemporaryDirectory par ShellTest sans -p, posé après l'exécutable (b2ac66efd), 245/245, rien de laissé. Tests Python seuls : **« à toi » maintenant**, hors du code que compte la garde. Lucivy : 4.3.9 sur main (f7da3be), CI en cours, pointeur rag3weaver en branche par révision (lucivy-4-3-9, f5c2f3124d), rejeu à luciepc ; tag retenu jusqu'au mot de Lucie.

- 01:43, hotfixs n'a pas poussé shell-tests-own-history : la garde exige un journal qui nomme une lib du magasin (push_garde.sh l. 188-192, « zéro reste un ARRÊT ») et ce lot de tests seuls n'en a pas ; il refuse de faire nommer une lib à un journal (juste). Décidé : (a) le lot s'empile sur son prochain lot qui a une passe avec sa lib ; (b) ticket chez G : un mode « tests seuls » de la garde (diff borné aux dossiers de tests déclarés, journal de la suite qui nomme le binaire joué et sa provenance, pas de lib), pendant du mode docs.

- 01:46, **arbre poussé 2b′** : master 988651d39e (le chargeur lit la pile d'une vue dans _generation, la composition en prédicat, le chargeur sur connexion, le nom et le dépôt des couches ; 2b′ sur l'écrivain : prédicat de code_sync avec le masque vide, Catalog::poser_la_vue retiré ; la publication par l'écrivain passe la parente de 2b′, une génération de la couche backend est une racine ; six commits gmail, zéro attribution, vérifié). **« À toi » à recherche** pour 2b-C avec le champ `view` de la réponse. Hotfixs : ticket 4b4f5ae700 (la garde sans mode « tests seuls »).

- 01:46, hotfixs prend le SIGSEGV d'optimisticRead (ticket du 8 nov., laissé par le cœur) : mécanisme de l'étendue prouvé et corrigé sur master depuis le 8 nov. (9bae60786 : numPages ne baisse plus, returnOwnerlessPages borné, plage au-delà du fichier refusée, pin et optimisticRead lèvent une erreur) ; aucun core ni garde déclenchée depuis, aux deux postes ; 340 bases de plus de 50 Mo comparées, seules code-essais-c7 et ses copies du 8 nov. sont creuses. Reste inexpliqué : le premier événement (pages vers 83 millions dans une base de 455 000, 8 nov. 0 h 08 → 8 h 50, sous 17a094d12 ou dadbf0041). Décidé : gravité baissée à « métadonnée fausse dite par une erreur », ticket ouvert pour le premier événement, lecture bornée (un lot) des commits 17a094d12..dadbf0041 avec la base etat-ferme-17h14 ; sans cause, confort et défaut suivant.

- 01:50, lucivy : la CI de main sur f7da3be est rouge sur le témoin du dictionnaire de la 4.3.8 (maintain_gives_back_the_dictionary_of_deleted_documents : 64 048 octets contre 60 290, 6,2 % au-delà de la marge de 5 % fixée sur deux mesures de luciepc ; textes exactement égaux ; cause : la table d'ids des .termtexts compte une plage de 8 octets par suite d'ids gardés, l'ordre des quatre fils écrivains du runner décide du morcellement). Correctif PR #41 : borne de la structure (index neuf + 2 % FST + 8 octets par texte gardé), égalité des textes stricte ; le tag visera ce SHA au mot de Lucie. Pointeur rag3weaver vert (lucivy-4-3-9, f5c2f3124d par révision : fts_handle 13, fts_maintenance 5, check_features 8/8, e2e_code 38, lectures 29, demon_hote 26). Arbre : liste des noms du renommage envoyée à recherche ; lot après 2b-C (~30 min), passe à demander ; 2a′-3 après la bascule.

- 01:51, **recherche poussé 2b-C** : master 457892de98 (mode pile, PILE_AVANT=89776f292, six commits gmail, zéro attribution, vérifié ; passe verte : témoin, e2e_lectures 27, e2e_code 36, check_features 8/8, lib 2 564, navigateur 45). `view` sur tout outil qui lit, chargé et composé dans l'instantané de l'appel ; la réponse dit `view: {layer, rank, depth, pending, pending_rows}` ; identifiants neufs en anglais ; page docs/recherche/16-11-2026/01-la-vue-des-outils.md. **Les vues par couche sont sur master** (écrivain, migration, 2b′, 2b-C) ; reste 2a′-3 (générations par branche et par worktree) et la bascule des bases. **« À toi » à codeparsers** (base 457892de98, quatre lots, passe complète).

- 01:53, hotfixs : le premier événement du SIGSEGV d'optimisticRead n'est pas trouvé au bout du lot de lecture, ticket en confort (7e01d5898). La pièce : deux segments vivants au-delà de l'étendue (96 pages chacun = 128 vecteurs de 3 072 octets) et une entrée libre de 64 pages, 256 pages de 83 108 096 à la fin du fichier : des écritures banales posées en fin d'un fichier qui comptait déjà 83 108 096 pages, donc un allongement antérieur ; écartés : retrait des générations, budget en octets, spiller (son propre .tmp), troncature au rejeu, pages fantômes ; reste non lu : l'écriture partielle du COPY borné (977c2c866). **Défaut à part trouvé** : Spiller::clearFile() vide <base>.tmp mais laisse au FileHandle son compte de pages, états et groupes de cadres ; chaque déversement suivant écrit au-delà (fichier creux) et réserve des cadres de plus ; non reproduit. Décidé : le témoin du spiller d'abord (candidat au premier événement), puis la durabilité sur faute. Recherche : sa suite est le point 7 côté outils (la vue par le contexte et le renommage sont chez l'arbre). Rapport c7 : bloc de la nuit ajouté (49a9e2ff47).

- 01:55, codeparsers : pile rebasée (codeparsers/pile-resolution, 37e2bb77b sur 7e01d5898b, quinze commits dont le final : REGLES_DE_L_ANALYSE 1 → 2, ticket d'ED fermé avec la cause et la recette à trois fichiers, nouveauté « toutes les bases repassent une fois » ; pointeur 7702f03, Cargo.lock inchangé) ; passe complète depuis 1 h 54, garde en mode pile à son vert. Recherche : plan court du point 7 côté outils sur master (docs/recherche/16-11-2026/02-plan-index-pas-pret.md) : débit observé mesuré par ReindexFileNode, `Lag::estimate` (fichiers en attente, eta possiblement inconnu), `Lag::wait_caught_up` borné, `search.index_wait_s` 6 par défaut ; sous le seuil ou eta inconnu : attente au plus le seuil puis réponse complète ; au-dessus : `index: {ready: false, eta_s, pending_files}` et « index pas prêt, environ N s » ; la notification unique et le silence sur ses propres écritures restent à archi ; trois témoins rouges en cours ; l'estimation par vue de 2a′-3 se branchera par cellule et par couche.

- 01:57, arbre : lot du renommage en anglais écrit (arbre-principal/vues-en-anglais, a76dace1a sur 457892de9 : les noms de 2b′ et de la montée, les appels de recherche dans backend.rs, les témoins ; rien d'ancien ; textes de refus intacts ; second commit « sans effet sur l'analyse », la garde des règles lit la mention en minuscules seulement : ticket), passe à luciepc. Place : après le « poussé » de codeparsers, repose d'un cran, rejeu court si fichier commun.

- 01:59, ED **prêt**, une pile pour G1, P2, P1 (agent-tools-short-lots, d039c96d1 sur d33d6b28d, 8 commits gmail ; passe épinglée verte : e2e_code 36, lectures 27, demon_hote 24, check_features 8/8, lib 2552/0/72, navigateur 45) : G1 rev-parse, ls-files, describe en LECTURE_SEULE ; P2 grep_files refuse au-delà de `refuse_above` (1 000 par défaut au nœud, DEFAULT_GREP_REFUSAL, 0 = jamais) en nommant total, seuil, trois dossiers les plus lourds, path_prefix/extension, et renvoie au sens vers search_code ; P1 descriptions de search_code (« use first » pour une question en mots) et grep_files (« for an exact string only ») en anglais dans les gabarits code, snapshot, atelier ; chaque témoin rouge d'abord. Réserves : seule une D5/D6 rejouée dira si l'agent choisit search_code ; les tâches lisent une copie du manifeste de Lucie, à mettre à jour (c7, après la pose). Ordre après codeparsers : arbre (renommage), embarquement, ED, archi, G, 96, grain, pointeur lucivy. Arbre : ticket de la casse pour ED (dc8052174) ; un faux pas dit : une première version poussée sans le VERT de check_tickets, corrigée dans la minute.

- 02:02, hotfixs : témoin du spiller ROUGE sur master (EmptyBufferManagerTest.ClearingTheSpillFileStartsTheNextSpillAtPageZero : un déversement, clearFile, le même déversement → file_size 262144 au lieu de 131072, écrit au-delà du compte gardé, fichier creux) ; correctif bc894ccfc : FileHandle::restartAtPageZero() sous fhSharedMutex (numPages à 0, fichier tronqué, capacité et groupes de cadres gardés), appelé par Spiller::clearFile (resetToZeroPagesAndPageCapacity écarté : il raccourcit frameGroupIdxes sans rendre les groupes à la VMRegion) ; branche spill-file-restarts-at-zero sur 2b7202779 ; tests de storage puis passe par la voie C++ (tampons + transaction_test). Risque à part, antérieur : clearFile tronque <base>.tmp à la fin de chaque requête sans regarder les autres connexions (ticket). Le lien avec le premier événement reste à établir (le .tmp n'est pas le fichier de données). ED : pile à 9d44657d8 (+ la mention lue sans casse, rouge puis vert), ticket de l'arbre fermé à la repose.

- 02:07, **master rouge sur e2e_code_sync**, trouvé par la passe de codeparsers (tête e610eed8c : check_features vert après la correction de tests/sonde_racines.rs qui ne connaissait pas la variante OfType ; lib 2 564 vert ; e2e_code 45 passés, 11 échoués dans e2e_code_sync). Départage : la suite seule sur master e264210dc7, worktree neuf, lib 373b5477b, MOTEUR_ANCIEN=1 → 9/11, les mêmes onze nom pour nom que sur la pile (synchroniser_une_source_retire_les_fichiers_supprimes, le_plan_seul_ne_retire_rien, une_source_tronquee_est_refusee_sans_force rend Ok files_ingested 0, une_edition_apres_un_point_de_reprise, apres_un_point_de_reprise_les_deux_sens…) : synchronisation et point de reprise. **Aucune passe de la nuit ne jouait cette suite** (les « e2e_code 36 »). Suspects : l'écrivain (c33574eb9, code_sync.rs), 2b′ (988651d39e, prédicat code_sync.rs:697). Demandé : ED joue la suite sur c33574eb9 et 7ece05d8a2 ; l'arbre sur 988651d39e et 457892de98 ; codeparsers départage la lib (52101b7fd). Rien ne pousse avant l'explication ; correctif en tête de file. Ticket à venir chez G : les passes des lots qui touchent code.rs ou code_sync.rs jouent e2e_code_sync.

- 02:09, codeparsers : la lib n'y est pour rien (master e264210dc7 avec 52101b7fd : les mêmes 11 rouges, la synchro rend Ok avec files_ingested 0 ou 1 sur 4, scopes_written 0) ; c'est le Rust de master ; pile e610eed8c intacte ; wt-master-temoin gardé pour ED. Ticket pour G posé (7a2dd39a01 : les passes des lots ne jouent pas e2e_code_sync). Arbre : e2e_code_sync lancée seule dans deux worktrees neufs à luciepc (988651d39e, 457892de98), passe du renommage arrêtée ; sa relecture : code_sync.rs:697 même chaîne pour une pile d'un étage, la parente racine écrite NULL, publish_read_view appelé une fois de plus à l'ouverture, rien qui touche suppression, troncature ou point de reprise. Banc : relecture des 30 cas sur master lancée 2 h 04 (copies reflink rouvertes, verdict dans 30–40 min) ; mesure longue : F 25 % défaut 40/160/43, 31 fiches, P1 0,77, 2 doublons, 6 endormis ; F 25 % variante 63/162/19, 19 fiches, P1 0,69, 0 doublon (ramasse trop, le rappel baisse) ; S 50 42/149/53, 32 fiches, P1 0,77, 13 endormis (le chevauchement divise les endormis par deux à rappel égal) ; 300 k depuis 1 h 57, N rejouée ensuite.

- 02:11, banc : relecture des 30 cas sur master e264210dc7 (index neuf et lot rouverts par le même code, sans réindexer) : exactement les chiffres d'avant, 23 rouges sur 30, rappel à 10 de 0,900, 25 premiers résultats identiques, 8 listes ordonnées sur 30 : **la correction de tout_visible ne change rien aux 23** ; ce qui reste est chez lucivy (longueur moyenne, documents supprimés dans les statistiques, ordre des identifiants), ce que lucivy a mesuré (11/15 → 2/15 avec sa branche). **Défaut du juge dit par le banc** : il n'a jamais joué que le lexical, le relevé ouvrait le backend sans embarqueur (`prepared.open(…, None)`, « index en plein texte seul ») ; les relevés « tous signaux » et « lexical seul » étaient identiques au bit près ; toutes ses parités de search_code depuis le 14 ont jugé le lexical seul (les « meilleurs signaux tous lexicaux » du 14 venaient de là) ; les verts restent vrais pour le lexical, muets sur le dense. Correctif du relevé (connect_embedder comme les témoins, signal actif dit par cas), relecture de master tous signaux rejouée, devant N rejouée.

- 02:12, banc, précision : les index de ses parités étaient faits sans embarqueur (vecteurs « en dette ») ; l'index neuf n'a aucun vecteur, le lot ceux qu'il gardait : une relecture « tous signaux » de ces copies ne dirait rien du dense ; pour BM25 (lucivy) le lexical seul est la bonne mesure, référence : master 23 rouges, rappel 0,900, tout_visible sans effet ; pour l'écrivain les verts disent « même classement lexical, même graphe », le dense n'est pas jugé ; le relevé ouvre désormais l'embarqueur (banc-parite-search-code 4ce0a93b), RAG3WEAVER_PARITE_NO_EMBEDDER=1 garde le lexical exprès. Décidé : pas de parité dense par un index neuf ; une parité tous signaux sur les copies de la répétition de l'arbre (code-principal migré contre le courant, vecteurs déjà là, 30 requêtes), après N rejouée : la mesure qui manque à la bascule des bases. CI de master : le job « check_features (cinq jeux) et tests unitaires » tourne depuis 0 h 56 UTC sur le runner auto-hébergé (lent sous les lourds).

- 02:14, G : lot du ticket e2e_code_sync fait (required-suites, a3332d51d4, en tête de sa pile) : tools/poste/required_suites.txt (chemins → suite : e2e_code_sync pour code.rs, code_sync.rs, catalog.rs, catalog/*, generations.rs, marque_d_ecrivain.rs, tests/e2e_code_sync.rs), required_suites.sh pour une plage de commits ; passe_epinglee.sh ajoute ces suites même sous SUITES=-, résumé « suites exigées par le lot : … » puis « suites jouées : … » ; push_garde.sh exige la ligne « == <suite> » et un journal vert, sinon ARRÊT nommé. Rouge d'abord (test_passe_epinglee 2 cas, 25 vert ; test_push_garde 4 cas, 4 rouges sur la garde de master, 46 vert). Recopie de la garde au cache à mon mot, annoncée (un lot en vol qui touche ces chemins sans e2e_code_sync sera refusé jusqu'à son rejeu). Pile reposée dessus : passe-chemin-absolu-3 b2abc8bddd, poste-plafond-validation-3 e79f1fb2f3, run-e2e-temoin-lib-3 6bb672adc6 ; rejeux courts en cours. Banc : montage de la parité tous signaux accordé (copie migrée contre un instantané réparé pareil et réindexé par le code d'avant, embarqueur ouvert), après N rejouée.

- 02:15, lucivy : main prêt pour le tag v4.3.9 sur 0825bcd (#41 borne du dictionnaire par la structure et #40 outil de comparaison fusionnés ; CI de main verte sur le SHA ; contenu contre 4.3.8 : la longueur moyenne des admis et deux tests) ; au mot de Lucie : tag, release.yml, registres, CLAUDE.md, pointeur rag3weaver à « 4.3.9 » avec rejeu court. Banc : parité de la bascule prévue 3 h 45 (après 300 k, tour 106/244, fin ~3 h, 6,78 M jetons dont 24 % en cache ; puis N rejouée, ~35 min). Lucie n'a pas répondu depuis 1 h 1x : le tag et la fenêtre de lib attendent le matin.

- 02:16, **la cause du rouge de master est l'écrivain** (ED) : e2e_code_sync seul, même worktree, même lib et jeu : c33574eb9 ROUGE 9/11 (les onze, plus un_fichier_genere, une_exclusion_posee_apres_coup, une_edition_pendant_la_synchronisation, un_fichier_supprime_apres_son_passage, un_fichier_edite_deux_fois), 7ece05d8a2 VERT 20/20. Cause lue : une source déjà indexée passe sous les générations par `synchroniser_par_generations` (code_sync.rs), qui n'a pas la garde maxMissingRatio (catalog/synchronisation.rs:598), ni `files.removed`, ni le plan seul, ni probablement la reprise des éditions concurrentes et le point de reprise. Sa passe ne jouait jamais e2e_code_sync (dit par ED). Décidé : **pas de retour arrière** (migration, 2b′, 2b-C bâtis dessus ; hôtes sur 4802a147a5, rien en service) ; correctif court en tête de file, chaque garantie reprise avec son témoin, e2e_code_sync dans la passe de l'écrivain et la procédure ; l'arbre arrête ses relectures.

- 02:16, ED, les onze en trois familles, une heure et demie passe comprise : (A) le chemin des générations prend les garanties (maxMissingRatio et source vide avec force/allow_empty ; rapport files.removed, scopes.removed, applied ; le plan seul par l'écrivain : écrit les présents au rang en attente, ne retire rien, dit ce qui partirait) ; (B) les lecteurs bruts des tests (scopes_nommes, fichiers, etat, graphe) font des MATCH sans visibilité et voient l'histoire (dessin de 2a′) : bornés par Catalog::condition_visible aux deux bouts, un par un sans masquer un vrai écart ; (C) deux changements de sens, **accordés** : files_unchanged au rapport (le test attend ingested + unchanged == 4) ; le test des sessions d'instantané sur une base sans générations et un témoin neuf pour les générations (l'édition retire au rang en attente, le scope lisible au rang publié).

- 02:17, arbre : relectures arrêtées, places rendues ; 2b′ (988651d39e) ROUGE 9/20 comme master (posé sur l'écrivain), 2b-C arrêtée avant son témoin ; worktrees sync-2bp et sync-2bc au ménage ; vues-en-anglais attend le correctif. **G prêt** : pile de quatre sur d33d6b28d1, rejeux verts l'un sur l'autre (2 h 13–16) : required-suites a3332d51d4 (mode simple), passe-chemin-absolu-3 b2abc8bddd, poste-plafond-validation-3 e79f1fb2f3, run-e2e-temoin-lib-3 6bb672adc6 (test_passe_epinglee 29 cas, vraie passe --lib verte, filtre vide rouge dit). **« À toi » à G** maintenant (scripts seuls, avant le correctif Rust d'ED pour que la garde neuve le juge) ; recopie de la garde et annonce de la règle à son « poussé ».

- 02:17, hotfixs : témoin de clearFile ROUGE et silencieux (EmptyBufferManagerTest.AnotherQueryEndingDoesNotEraseAGroupStillSpilled : un groupe de 16 384 INT64 déversé, clearFile comme la fin d'une autre requête, relecture : aucune exception, 4 valeurs fausses) ; l'en-tête du spiller disait « Must only be used once all chunks have been loaded from disk », ClientContext l'appelle à chaque fin de requête sur un spiller commun. Correctif 3311437745 (clearfile-keeps-live-spills, empilé sur spill-file-restarts-at-zero) : compte des tampons encore sur disque (+1 au déversement, −1 à la relecture ou au destructeur), déversement et vidage sous un même verrou, vidage seulement à zéro, preventDestruction hors compte. Décidé : une pile des deux par la voie C++, passe tampons + transaction_test sur la tête, « prêt » au vert, indépendante du correctif Rust d'ED.

- 02:19, **G poussé** sa pile de quatre : master ccfded41c4 (les suites qu'exige le lot, e2e_code_sync jouée par la passe et redemandée par la garde ; passe_epinglee.sh refuse un chemin absolu et dit où elle écrit ; une validation au-dessus des 32 Gio refusée avant la file, noms en anglais VALIDATION_MEMORY_MAX/to_bytes ; run_e2e.sh --lib joue les tests de la lib comme suite, SUITE_DU_TEMOIN=lib ; quatre tickets fermés ; sept commits gmail, zéro attribution, vérifié). Reste : la recopie de la garde au cache des deux postes et l'annonce de la règle, à son « poussé ».

- 02:20, **la CI rag3weaver de master est verte** sur 457892de98 (job « check_features (cinq jeux) et tests unitaires » en succès ; les runs intermédiaires annulés par les pushes suivants) ; run suivant en cours sur ccfded41c4. Hotfixs ferme les tickets du journal des runs et du test shell. Relevé des sessions : toutes présentes ; archi toujours « waiting », muet depuis 1 h 05.

- 02:23, G, « poussé » les quatre (7ff6ba0ffe, a8f29c91a6, 627d8385f6, ccfded41c4 ; piles tenues, trois contrôles). **Garde recopiée au cache des deux postes** (2 h 20 et 2 h 22, .neuf puis mv, anciens en .avant-pile-4-du-16) : push_garde.sh sha256 **57bd32f59f8e96cb** (avant ad0be1f36ff12ea5), passe_epinglee.sh **ec1529fc2f3dcf07** (avant 7333376698388fe6), poste **79e3a8bbf457e3bc** (avant fc78c8fbd56c43b2), plus required_suites.sh/.txt pour les témoins (en vraie passe la table se lit dans l'arbre du lot). Témoins contre les copies : test_push_garde 46, test_passe_epinglee 29, test_poste 34/35 (le cas qui compare le seuil à MAX_MEMOIRE cherche le confineur à côté du poste jugé, absent du cache ; valeur copiée juste ; correctif du témoin au prochain lot). Annonce de la règle aux sessions par G. Hotfixs : tickets du journal des runs et du test shell fermés (3838ab37b, le vert confirme, le témoin prouve) ; pile du spiller en passe à luciepc.

- 02:25, G : mode « tests seuls » de la garde prêt (guard-tests-only, fbf579f5e2 sur f7d390a117 ; test_push_garde 49 vert ; sous TESTS_ONLY=1 un lot qui ne touche que les dossiers de tools/poste/tests_only_dirs.txt (tools/shell/test/ d'abord) et des .md passe sans lib, avec un journal « tested binary: <chemin> commit <40 hex> sha256 <64 hex> », sinon ARRÊT nommé ; le reste inchangé ; trois cas rouges d'abord). **« À toi » à G**, recopie et ligne de provenance à hotfixs ensuite. Ticket du témoin de poste (cherche le confineur à côté du poste jugé) ouvert, 589dcfde5b.

- 02:26, **G poussé** le mode tests seuls : master eeb1ba8db5 (53813ba549 feat(garde), ticket 4b4f5ae700 fermé ; deux commits gmail, zéro attribution, vérifié). Recopie de la garde et ligne de provenance à hotfixs ensuite ; shell-tests-own-history pourra passer seul sous TESTS_ONLY=1.

- 02:27, G, « poussé » guard-tests-only (2 h 26, eeb1ba8db5, mode simple, trois contrôles) ; **garde recopiée aux deux postes : push_garde.sh sha256 95a1d85779f0105a** (avant 57bd32f59f8e96cb, .avant-tests-only) ; test_push_garde 49 vert contre chaque copie ; ligne de provenance envoyée à hotfixs.

- 02:42, hotfixs prêt pour shell-tests-own-history-2 (8d404496d sur a899dd583, conftest.py seul ; passe verte à luciepc, 245 sous HOME temporaire, rien laissé ; binaire bâti du commit et déclaré avec sa provenance, la copie de rag3db-colonnes refusée car « -dirty ») : **« à toi » sous TESTS_ONLY=1**. Pile du spiller pas prête : transaction_test 10 rouges de montage (extension vector non bâtie, LOAD EXTENSION refusé) ; témoins et 11 binaires de test/storage verts (102) ; rejoué avec l'extension bâtie (hotfixs-txn-vector).

- 02:43, ED : **e2e_code_sync vert 21/21** sur generations-sync-guarantees (base aa0e499da), passe complète en cours sur 1d9e72600. (A) le chemin des générations : maxMissingRatio et source vide refusées sans force/allow_empty, files.removed, scopes.removed, applied, le plan seul par l'écrivain ne retire rien, un fichier indexé écarté par l'admission à la lecture quitte l'index ; **un douzième défaut** : reingest_files_retenu admettait sous la politique du généré de la source et non de l'appel (une règle levée ne laissait jamais entrer le généré), corrigé. (B) lecteurs bruts bornés par condition_visible, un par un, aucun écart restant. (C1) files_unchanged au rapport. (C2) le test des sessions sur une base SansVersions, témoin neuf under_generations_an_edit_retires_at_the_pending_rank_and_the_published_rank_keeps_the_scope vert. « Prêt » à son vert, tête de file ; hotfixs a « à toi » (tests seuls) en attendant.

- 02:43, recherche a vu le même rouge de master (94af134aaf, 11/20 ; la suite verte sur c224658dc5 le 15 à 21 h 45) et proposait une bissection : déjà cerné (l'écrivain), pas de doublon. Son lot « index pas prêt » (dcaaf9584) est vert sur ses témoins, e2e_lectures et e2e_code ; place après codeparsers, l'arbre, embarquement et la pile G1/P2/P1 d'ED, passe avec e2e_code_sync. Relevé des sessions : toutes présentes, archi « waiting » et muet.

- 02:44, **hotfixs poussé** shell-tests-own-history-2 : master 1b2be095c9 (quatre commits gmail, zéro attribution, vérifié ; ticket des ~130 ShellTest() fermé). Premier vrai passage du mode tests seuls de la garde : « mode tests seuls : le lot ne touche que tools/shell/test/ ; binaire joué : tested binary … commit 8d404496d4d0 sha256 2297d8da… », « rien de code » entre a899dd583 et 9a51e6a94. La pile du spiller rejoue transaction_test avec l'extension bâtie.

- 02:55, ED : passe complète VERTE sur 1d9e72600 (e2e_code_sync 21/21, e2e_code 36, lectures 27, demon_hote 24, postgres 39, check_features 8/8, lib 2564/0/72, navigateur 45) mais ARRÊT de la garde 95a1d85779f0105a : master a apporté depuis aa0e499da les scripts de la passe de G (run_e2e.sh, poste, confiner_validation.py) ; « suites exigées : pas de table dans l'arbre » (base antérieure à required_suites.txt). Repose generations-sync-guarantees-2, rejeu court avec e2e_code_sync par la table, push ; à toi dès vert tient.

- 03:05, CI de master : rag3weaver vert sur ccfded41c4, CI Workflow vert sur eeb1ba8db5, run en cours sur 1b2be095c9. Relevé des sessions : toutes présentes ; archi « waiting », muet depuis 1 h 05 malgré trois relances. Moniteur réarmé.

- 03:07, **ED poussé les garde-fous de la synchronisation sous les générations** : master fff425b3c (generations-sync-guarantees-2, 5d053baac ; quatre commits gmail, zéro attribution, vérifié ; rejeu court : e2e_code 36, e2e_code_sync 21/21, check_features 8/8, lib 2564/0/72, navigateur 45 ; garde 95a1d85779f0105a : « suite exigée jouée verte : e2e_code_sync (code_sync.rs) » ; passe complète du même code verte). **Master n'est plus rouge.** **« À toi » à codeparsers** (pile reposée d'un cran, passe complète, garde en mode pile) ; puis l'arbre (renommage), embarquement, ED (G1/P2/P1), recherche (index pas prêt), archi s'il revient, le grain, le segment, le pointeur lucivy.

- 03:09, hotfixs **prêt**, pile du déversement VERTE (tête 6cf5626fd sur 2b7202779 : spill b035716df/443bb3715/bc894ccfc, clearFile 14d8ebf1b/3311437745, deux .md ; passe à luciepc : 3 témoins, 11 binaires de test/storage 102 verts, transaction_test 390 verts + 2 sautés avec l'extension bâtie ; master sans fichier sous src/, test/, extension/vector, CMake depuis la base). Il demande la commande de la voie C++ : elle est dans push_garde.sh (cpp_transferable) ; la lib cherchée au journal est le BUILD_ID du moteur bâti. **« À toi » à hotfixs** ; si la garde s'arrête sur la lib, ticket pour G, rien fabriqué. Arbre, embarquement, ED (G1/P2/P1) reposent sur fff425b3c en attendant codeparsers.

- 03:09, codeparsers : pile reposée sur fff425b3c (codeparsers/pile-resolution-2, 7724d0cc6, quatorze commits, pointeur 7702f03, check_regles et check_tickets VERT) ; passe depuis 3 h 08 (témoin c_api_prototype_is_visible dans e2e_usages, e2e_code_sync exigée par le lot, e2e_lectures, e2e_code, e2e_usages, e2e_postgres, check_features, lib ; 373b5477b, MOTEUR_ANCIEN) ; garde en mode pile PILE_AVANT=fff425b3c au vert. Arbre : renommage reposé (vues-en-anglais-2, 6e0a467e6, range-diff =, aucun ancien nom restant), passe lancée à luciepc (arbre-anglais-2) ; le banc a l'arbre (284cc00c0b) et la commande de réparation pour 3 h 45.

- 03:11, **hotfixs poussé la pile du déversement** par la voie C++ : master 5dc3247c81 (le fichier de déversement repart de la page 0 ; clearFile ne vide plus sous un groupe encore déversé, le spiller compte ; deux tickets fermés ; sept commits gmail, zéro attribution, vérifié). Deux pertes du moteur de moins (une métadonnée fausse, une perte silencieuse de valeurs).

- 03:12, hotfixs, la garde par la voie C++ : « lib de toute la passe : rag3db-build-331143774542… » (le BUILD_ID de sa tête), « suites C++ transférables… master n'y a rien changé », « copies ouvertes retirées dans le même push » ; un premier essai arrêté proprement (un commit supprimait la copie ouverte d'un ticket que master avait modifiée ; pile refaite sous clearfile-keeps-live-spills-2). Deux recettes circulaient (cœur : la lib courante déclarée non liée ; c7 : le BUILD_ID du moteur bâti) : **une seule fixée dans push-d-un-lot.md** (5775380106) : la lib du journal est le BUILD_ID du moteur bâti depuis la tête du lot, rien ne se déclare à la main. Hotfixs passe à la durabilité sur faute avec le cœur.

- 03:17, embarquement : repli-activation reposé sur 59e9e0b12f (repli-activation-r1, 0697adbc7e, dix commits, check_tickets VERT) et rejeu court VERT (témoin e2e_embarqueur_a_la_demande, check_features 8/8, lib 2571/0 avec les sept témoins anglais, navigateur 45 ; e2e_code_sync non exigé). Tenu prêt : il passe après codeparsers et le renommage de l'arbre.

- 03:21, **codeparsers poussé** sa pile de la résolution d'arêtes : master 41144f8d22 (quatorze commits gmail, zéro attribution, vérifié ; pointeur codeparsers 7702f03 ; passe épinglée verte sur 7724d0cc6 : e2e_code_sync 21/21, lectures 27, code 36, usages 29, postgres 39, check_features 8/8, lib 2 564, navigateur 45 ; garde en mode pile, pile tenue). Atterri : homonymes + paramètres C++, la marque nom, les receveurs sans type (OfType), l'appel C vers l'API C par ses includes, les noms anglais, le rapport, **REGLES_DE_L_ANALYSE = 2** (toutes les bases repassent une fois), le ticket d'ED fermé, la sonde des racines. Mot donné : master du dépôt codeparsers avancé à 7702f03 en avance rapide ; ménage des worktrees. **« À toi dès son vert » à l'arbre** (renommage, repose d'un cran) ; embarquement ensuite.

- 03:22, arbre : renommage reposé sur codeparsers (vues-en-anglais-3, 17daca44d sur 41144f8d2), une seule passe lancée 3 h 21 à luciepc, push à son vert ; un faux pas de base de rebase annulé avant tout push ; sa règle : « --onto <neuf> HEAD~<n> », jamais une base de mémoire. Codeparsers : master du dépôt codeparsers avancé f413dc6 → 7702f03 (27 commits gmail, avance rapide) ; journaux archivés (archives/codeparsers-pile-resolution-16-nov, 67 Mo), quatre worktrees retirés, restent wt-pile et wt-master-temoin ; le banc mesure les vraies bases vers 4 h 15 (marque `resolution`, API C par src/c_api visibility = declaration). Sa suite : la définition liée à l'en-tête, le membre platform, le reste des receveurs.

- 03:26, ED : pile G1/P2/P1 reposée sur fff425b3c et prête (agent-tools-short-lots-2, f704af038, 11 commits gmail ; rejeu court vert : e2e_code 36, check_features 8/8, lib 2567/0/72, navigateur 45 ; la garde des règles sans casse dedans, ticket de l'arbre fermé) ; après le renommage de l'arbre et le repli d'embarquement. **Hotfixs, rouge grave** (durabilité sur faute) : sous une coupure qui emporte les entrées de répertoire non synchronisées, une base neuve disparaît entière avec ses validations « ok » (témoin PowerCut.CommitsOfANewDatabaseSurviveACutOfItsDirectoryEntries, 1688239b2 : 2 000 lignes validées, coupure, « Table T does not exist ») ; la doublure CutFileSystem gagne pendingDirectoryEntries() et syncDirectory ; correctif 4676ce513 borné : VirtualFileSystem::openFile synchronise le répertoire parent quand l'ouverture crée le fichier (défaut seul ; LocalFileSystem fsync du répertoire, EINVAL toléré ; Windows rien) ; en jeu PowerCut*, transaction_test, test/storage ; coût : un fsync de répertoire par cycle de point de reprise, à mesurer dans le lot ; la suppression non synchronisée en ligne suivante du ticket.

- 03:33, codeparsers mesure avant tout rouge (sonde tests/probe_unique_definer_misses.rs, arbre figé après la pile) : 3 871 mentions C/C++ à définisseur unique sans arête, la plupart des abstentions justes (2 182 valeurs d'enum, 272 méthodes std, 537 src → fichier de test, 18 même fichier) ; reste 862 (484 src → src, 335 test → test, souvent des champs de fixtures homonymes). Les vrais manques qui se répètent : **les méthodes héritées** (setOID défini dans CatalogEntry, type lu TableCatalogEntry ; setEntries, isUnfiltered, resetOutVectors, hasWherePredicate) : la branche du type lu exige un définisseur dont le parent est le type lu lui-même. Règle à venir : sans définisseur sur le type lu, chercher dans ses bases par INHERITS_FROM, marque « type » ; rouge d'abord sur un receveur d'une classe dérivée ; sonde avant/après ; au ticket de la définition liée à l'en-tête.

- 03:36, **arbre poussé** le renommage en anglais de 2b′ et de la montée : master a529ea144e (passe verte sur 17daca44d : e2e_lectures, e2e_code, e2e_postgres, e2e_migration, e2e_code_sync 21/21, check_features 8/8, lib 2 564, navigateur 45 ; garde : « suite exigée jouée verte : e2e_code_sync (catalog.rs) », base 41144f8d2 ; deux commits gmail, zéro attribution, vérifié). Sa suite : le refus typé du chargeur (StackError, petit lot, signatures à recherche d'abord), 2a′-3 après la bascule, le ménage de luciepc après la bascule. **« À toi » à embarquement** (repose d'un cran, backend.rs commun, rejeu court) ; ED (G1/P2/P1) ensuite, puis recherche (index pas prêt), le grain, le segment, le pointeur lucivy.

- 03:53, banc, **parité tous signaux de la bascule (3 h 51) : ROUGE, pas une perte franche**. Montage : A la copie migrée de la répétition, B l'instantané de 0 h 51 réparé et purgé pareil (Scope au plein texte 66 016 → 65 914), lus par master 35780193b3 sans migration, embarqueur ouvert, vector et bm25 exigés. Régime reponses : 28 rouges sur 30, rappel à 10 **0,867**, premier résultat identique 27/30, 117 rangs sur 300 changent. Trois causes mêlées : (1) le content_hash recalculé par le repassage (162 rangs, même résultat) ; (2) les scores où le dense joue diffèrent d'~1 % (vecteurs de A réembarqués par le 7878, ceux de B de la base vivante) ; (3) le BM25 avec l'histoire (lucivy, 11/15 → 2/15 sur sa branche). Structure : Scope 65 914 (B) / 65 814 ouverts (A) ; miroir 0 en A ; CONSUMES 101 753 (B, règles d'avant) / 96 812 (A). Réserve : les outils de la répétition sans scope ni language, mêmes graphes des deux côtés. Décidé : **la bascule attend** le pointeur lucivy 4.3.9, une parité refaite sur A et B, et le dense seul (signals vector) pour sa part ; pas de réembarquement de B ; copies gardées ; sorties banc-parite/sorties/migration-parity/.

- 03:55, cœur : le refus « Schema changed » sans fin n'est pas reproductible tel quel (12 tours de plus sur 2385b5f616, sonde filtrée, pool réduit : aucun blocage ; à 384 Mio plus aucun DDL refusé, à 200 Mio seul CREATE_VECTOR_INDEX l'est ; 1 tour bloqué sur 42, aucun depuis le correctif du pool) : en attente d'une occurrence, ticket c1f758845, sonde prête. **Le SIGSEGV du dictionnaire est reproduit** sur master sous 200 Mio (tour 8/12, core 3079782, même pile qu'au 14 nov. : DictionaryChunk::appendString sous StringChunkData::finalize dans un point de reprise demandé par un COMMIT) : remonté devant le (2), passe ASan sous 200 Mio, voie C++.

- 03:57, **embarquement poussé** le repli par activation nommée : master 6157600059 (repli-activation-r2, passe verte : témoin, check_features 8/8, lib 2571, navigateur 45 ; dix commits gmail, zéro attribution, vérifié ; un faux signal dit : son premier moniteur avait lu le résumé de la passe d'avant, rien poussé dessus). **« À toi » à ED** pour G1/P2/P1 (repose d'un cran). **Banc, dense seul** sur A et B (signals vector exigé) : rappel à 10 **0,793** (238/300), premier résultat identique 24/30, 139 rangs changent, et ce sont d'autres résultats, pas des permutations (« json » rang 2 : json.hpp en B, bindings/nodejs/index.d.ts en A). Lecture du banc : ce que le repassage embarque diffère de ce que la base vivante avait embarqué (rendu de la version, découpage, ou modèle : granite-278m par le 7878 contre celui de la base vivante). Demandé à l'arbre et à ED : modèle et colonne d'embarquement des deux bases, la table du texte embarqué pour une comparaison texte + vecteur sur deux ou trois scopes. Point de la bascule indépendant de lucivy : si le modèle diffère, la bascule change le modèle de la base, à dire à Lucie ; si le rendu diffère, défaut de l'écrivain.

- 03:59, banc : **A et B portent le même modèle** (embedding__granite_278m FLOAT[768] ; B sur Scope_Chunk, forme d'avant les versions ; A sur ScopeContent_Chunk, la colonne restée sur Scope_Chunk). Ce qui diffère est le texte où le dense cherche : B les morceaux de chaque occurrence de scope, A les morceaux des versions de contenu dédoublonnées, rendues par le gabarit de version (nom, signature, contenu, docstring, le render de 1a). Lecture : l'effet de la forme à versions (1b), pas des générations ni de l'écrivain. Demandé : la comparaison de texte sur deux ou trois scopes, et le jugement de A et de B **contre l'oracle des bonnes réponses** des 30 cas (rang 1 et 10), pas l'un contre l'autre : c'est ce chiffre qui décide si le changement de rendu est une amélioration ou une perte. Pour Lucie : la bascule change le classement dense par le texte embarqué, pas par le modèle.

- 04:00, ED, le rendu embarqué : l'écrivain ne change pas le rendu (une version ScopeContent embarque content et docstring en morceaux de 30 lignes ou 1 500 caractères, comme avant) ; **c'est le texte de content qui a changé par dessein** : depuis 500b1ede4 (14 nov., le correctif des 844 Mo de lucivy) il est borné aux octets du nœud et non aux lignes ; un scope qui commence ou finit au milieu d'une ligne a un autre texte, et le repassage l'a appliqué partout. Si le banc le confirme sur ses scopes, ce n'est pas un défaut à régler : c'est l'index juste contre un index d'avant le correctif, que la bascule fera de toute façon ; à dire à Lucie : ses recherches denses rendront d'autres résultats. Le jugement contre l'oracle dira si c'est mieux. Pile G1/P2/P1 en rejeu court sur 6157600059.

- 04:00, arbre : la cause de l'écart dense est le découpage, pas le modèle (granite_278m 768 des deux côtés, vecteur identique à texte identique, vérifié) : sur 107 708 empreintes de morceaux de A, 9 679 (9 %) ne sont pas dans B ; B garde les morceaux de la date où chaque fichier a été indexé (découpeur plus ancien, des trous, un scope de json.hpp sans son morceau 1), A est découpé par le découpeur d'aujourd'hui (default_scope_chunking), avec content borné aux octets du nœud (ED). **La bascule change ~9 % des morceaux et leurs vecteurs**, à dire à Lucie. Décidé : (1) le banc juge A et B contre l'oracle (rang 1 et 10) ; (2) B′, un index neuf du même arbre dans une base vide par le 7878 (~1 h), A contre B′ = la vraie parité de la migration ; défaut à nommer seulement si A ≠ B′.

- 04:06, banc : **B′ parti à 4 h 04** à luciepc (7878 libre, card2 à 0 %) : index neuf de 284cc00c0b dans une base vide, code et manifeste de master 49a8804580 plus les gabarits de A, embarquement par le 7878 seul ; puis A relu par le même code, tous signaux, 30 cas en régime reponses ; coût mur au journal. **Le jugement contre l'oracle tourne depuis 4 h 01** : résultats de A et de B mis en commun, Gemini Flash note 0/1/2 à l'aveugle, P@1 et pertinents à 10 par côté, dense seul puis tous signaux, avec les bonnes réponses trouvées par un seul côté. Comparaison de texte faite : PortDef, port_defs, json_pointer ont le même contenu et les mêmes morceaux des deux côtés ; l'écart vient bien du découpage. La mesure codeparsers des vraies bases relancée 4 h 05 (course perdue de 23 s contre un correctif).

- 04:09, **ED poussé la pile G1/P2/P1** : master c09632d0f (agent-tools-short-lots-3, 640d66232 ; douze commits gmail, zéro attribution, vérifié ; rejeu court vert : e2e_code, check_features 8/8, lib 2574/0/72, navigateur 45). Atterri : rev-parse, ls-files, describe en lecture seule ; grep_files refuse au-delà de 1 000 lignes en nommant où restreindre ; descriptions de search_code et grep_files en anglais dans les gabarits code, snapshot, atelier ; la garde des règles sans casse. Reste pour c7 : les manifestes de Lucie (gabarit code-rag3db, copies des tâches) après la pose des hôtes, puis rejouer D5/D6. File : recherche (index pas prêt), le grain, le segment, le pointeur lucivy ; archi s'il revient.

- 04:14, banc, **A et B contre l'oracle** (Gemini Flash à l'aveugle, température 0, note 0/1/2 sur 25 lignes lues dans l'arbre 284cc00c0b ; bonne réponse = 2) : **tous signaux : rang 1, A 4/30 et B 4/30 ; à 10, A 29 et B 31 sur 38 (rappel 0,763 contre 0,816) ; dense seul : rang 1, A 8/30 et B 3/30 ; à 10, A 33 et B 27 sur 40 (0,825 contre 0,675)** : le découpage d'aujourd'hui améliore nettement le dense. La perte à 10 (2 sur 38), scope par scope, deux cas filtrés sur src/include/ : rag3db_database (c_api/rag3db.h:146) et getValue (value.h:214) absents de A ; ailleurs gains et pertes s'annulent. Limites : une seule passe de notes, bruit du juge non mesuré ; P@1 bas des deux côtés (requêtes d'un mot, juge strict). Lecture de c7 : A fabriquée avant le lot de codeparsers qui corrige la perte des prototypes d'en-tête (garde extern "C") ; à vérifier dans B′. B′ reparti 4 h 07 (le premier essai refusé en 47 s : relevé d'avant l'index sans signal sur base vide ; l'exigence passe sur le relevé d'après). Sorties : relevance-all.json, relevance-vector.json.

- 04:15, banc : les deux scopes **existent dans A** en rangées ouvertes (rag3db_database class rag3db.h:146-148 ; getValue method value.h:214-216) : ma lecture (prototypes perdus) était fausse pour ces deux-là ; c'est un reclassement : rag3db_database au rang 4 dans B sort du top 10 dans A, poussé par des scopes « namespace rag3db { » d'en-têtes (dense seul aussi) ; getValue n'était qu'au rang 10 dans B en bm25 seul, FlatTuple prend sa place dans A. La garde extern "C" peut jouer ailleurs sur rag3db.h (A n'y a que 25 scopes ouverts) ; B′ dira combien et si les deux répondent. Observation à garder pour le rendu : des scopes « namespace X { » qui montent sur une requête d'un mot.

- 04:36, recherche : « index pas prêt » reposé sur c09632d0f (conflit résolu sur le nom renommé du garde de la vue) ; première passe rouge sur un vrai défaut du lot (les_outils_de_lecture_disent_leur_fraicheur : sa ligne « index pas prêt » passait avant celle de la fraîcheur et dire_la_fraicheur sautait un texte déjà en « > », la ligne « 1 fichier changé » disparaissait) ; correctif : sa ligne après la fraîcheur, le témoin du cas long exige les deux ; le reste vert (e2e_lectures 27, e2e_code 36, e2e_code_sync 21 exigé par catalog.rs, check_features 8/8, lib 2 577/2 578) ; passe rejouée, push dans le if VERT. Moniteur réarmé.

- 04:37, hotfixs **prêt**, journal-dir-sync VERT (passe sur 4676ce513 : PowerCut* 8 verts dont le témoin de la base neuve rouge sur master et l'autotest de la doublure ; transaction_test 392 verts + 2 sautés ; test/storage 11 binaires verts). Coût mesuré sous poste mesure (cinq parties alternées, médianes) : 14,2 ms → 15,8 ms par cycle « validation puis CHECKPOINT », **+1,6 ms (+11 %)**, accepté ; une validation sans point de reprise ne paie rien. Défaut du shell vu en route (ticket) : sous une entrée par pipe, « CHECKPOINT; » suivi de « CREATE » lu « CHECKPOIEATE », des caractères perdus, une fois par partie sur les deux binaires. Tête 57c3d657e sur c1a5d009f ; journaux qui ne nomment que le BUILD_ID du moteur bâti (recette unique). **« À toi » à hotfixs**, voie C++.

- 04:38, **hotfixs poussé** la durabilité d'une base neuve : master 0818e6819b (trois commits gmail, zéro attribution, vérifié ; garde voie C++ : lib rag3db-build-4676ce5135 du moteur bâti, rien de code entre c1a5d009f et 9245f8345). Un contrôle 1 non vide expliqué : le sous-module de son worktree resté sur l'ancien commit après le rebase de la garde, pas le contenu du push (diff du pointeur vide), remis par submodule update (règle « worktree : sous-module et target »). Suite : la suppression non synchronisée (plus grave) avant le shell qui perd des caractères.

- 04:42, **banc, B′ révèle un défaut d'embarquement sur master** : l'index neuf de 284cc00c0b (202 s) finit sur « 92 708 morceaux non embarqués », 0 vecteur, 13 passes en échec : chaque lot de 512 est refusé en entier par le 7878 avec le 413 « texte 0 : 13 135 caractères, plafond 12 000 (RAG3WEAVER_EMBED_MAX_CHARS) » ; 31 ScopeContent_Chunk dépassent 12 000 caractères (copy_edge_case.test, demo.test, duplicated.test, q24.benchmark, large_list_of_struct_answer.txt, jusqu'à 53 294 ; le découpeur coupe par lignes) ; A porte les mêmes 31, embarqués (vecteurs d'avant le plafond du 15 à 20 h 36) ; côté client rien ne lit le plafond. Casse tout index neuf et toute réindexation qui touche ces fichiers ; le repassage de la bascule le prendrait de plein fouet. Décidé : **correctif en tête de file chez embarquement** (un 413 isole le texte, le client borne au plafond en le disant ; le découpeur borné en caractères pour ED après) ; pour B′ : le 7878 relancé avec RAG3WEAVER_EMBED_MAX_CHARS=60000 le temps de l'index puis remis à 12 000 (règle de Lucie « pas hésiter » pour les services) ; ticket au banc.

- 04:43, embarquement prend le défaut du plafond (le sien, lot du 503 f85977d09e : le service refuse le lot entier en 413, le client ne lit pas max_chars_per_text qu'il a dans /sante ; son témoin ne jouait qu'un lot de deux textes). Décidé : remède immédiat par embarquement lui-même, RAG3WEAVER_EMBED_MAX_CHARS=1000000 dans le .env du 7878 seul et redémarrage (granite tronque à 512 jetons, le plafond n'y protège rien ; le 7879 garde 12 000), annoncé au banc et à G ; correctif (a)+(c) en lot court rouge d'abord (le client coupe au plafond lu dans l'identité du service et compte la coupe dans troncatures ; le service coupe et compte au lieu de refuser le lot), tête de file ; (b) le découpeur borné en caractères : ticket chez ED. Ticket du banc ab36917628 pris par embarquement. Hotfixs : la suppression non synchronisée est **sans dommage** (témoin PowerCut.ACutAfterACheckpointRemovedTheJournalLosesAndDoublesNothing vert sur master, qui exige qu'un fichier soit revenu ; le journal revenu finit par sa marque CHECKPOINT, la reprise rejoue des pages déjà écrites) ; lot journal-removal-sync (test/ seul) : **« à toi »**, une garde vaut un push ; puis le shell.

- 04:44, embarquement n'a pas pu relancer le 7878 : les permissions de sa session ont refusé la commande (.env intact, NRestarts=0, plafond toujours 12 000) ; il ne contourne pas et ne demande à personne de le faire ; le banc non plus (« un refus vaut pour toutes les sessions ») ; c7 non plus. Le remède immédiat attend le mot de Lucie ; B′ attend le correctif (a)+(c), qui se pose au 7878 par la voie normale des services au vert ; le ticket (b) pour ED en route. Copies A et B en place, B′ se rejoue en une commande (~5 min d'index puis l'embarquement).

- 04:44, **hotfixs poussé** le témoin de la suppression non synchronisée : master c6bf19dda4 (trois commits gmail, zéro attribution, vérifié ; sans dommage, témoin gardé qui exige le cas joué). Arbre : le refus typé du chargeur (StackError, 38ec40dbd sur 35780193b, views.rs et backend.rs) vert sur ses témoins, passe en cours, « prêt » à son vert, après le lot de recherche.

- 04:47, **recherche poussé** « index pas prêt » : master 8b7686025e (quatre commits gmail, zéro attribution, vérifié) : un outil de lecture attend un index en retard sous `search.index_wait_s`, ou répond tout de suite et dit « index pas prêt, ~N s » ; le débit gardé entre deux ouvertures (témoin sur une base sur disque ouverte deux fois) ; sa ligne après celle de la fraîcheur. Le point 7 de la vision, côté outils, est sur master ; la notification unique et le silence sur ses propres écritures restent à archi. Suivants : StackError de l'arbre (à son vert), le correctif du plafond d'embarquement (tête de file à son vert), le grain, le segment, le pointeur lucivy.

- 04:52, hotfixs, le shell qui perdait des caractères par pipe : cause, pour afficher un tableau getColumns se replie sur getCursorPosition quand la sortie n'est pas un terminal, écrit ESC[6n et lit la « réponse » sur l'entrée brute jusqu'au premier R : sous un pipe c'est le script lui-même à partir de l'octet 4 096 (au-delà du tampon stdio) ; lot shell-pipe-input-keeps-bytes (témoin 400 RETURN par pipe rouge 3/3, correctif : getCursorPosition rend −1 hors terminal, ticket fermé) ; suite du shell 246/246. Trou de la garde : tools/shell/linenoise.cpp n'est ni dans les dossiers de tests ni dans les chemins de la voie C++ ; master sans code apporté. Décidé : pas de moteur.log écrit à la main ; le BUILD_ID écrit par le shell joué lui-même dans un journal de la passe (version ou bannière), puis la garde ordinaire ; ticket pour G : tools/shell dans la voie C++. « À toi » dès que le journal est du binaire. Arbre : StackError reposé sur 8b7686025 (040a0c825), passe en cours.

- 04:54, **hotfixs poussé** le shell qui perdait des octets par pipe : master 843bda91a5 (trois commits gmail, zéro attribution, vérifié) ; la lib écrite par le binaire lui-même (`CALL BUILD_ID() RETURN *;` du shell joué dans build-id.log, seul BUILD_ID du dossier) ; garde ordinaire, rien de code apporté. Ticket pour G (81de23b2e) : tools/shell et tools/ compilé dans la voie C++. Bilan de hotfixs cette nuit : le spiller (deux pertes), la base neuve perdue à la coupure, la suppression du journal sans dommage (témoin gardé), le shell sur pipe ; il reprend la liste par gravité avec le cœur. Cœur : passe ASan du dictionnaire sur 61576000 à 200 Mio, 2 tours sur 12 (15 min chacun), rien pour la bascule du matin, verdict vers 7 h 15.

- 04:56, codeparsers **prêt** sans push : les méthodes héritées (codeparsers/header-definition, b6e7f7120) : le Scope garde ses bases écrites (base_types), lues par l'IR dans la source et le dépôt du définisseur, la plus proche d'abord, abstention à égalité, RAG3WEAVER_INHERITANCE_DEPTH 4 par défaut (0 coupe) ; témoin rouge puis vert, garde du diamant verte ; sonde par profondeur 0/1/2/4/8 : « type » 11 521 → 12 346, manques 3 871 → 3 741 ; à 4, 804 couples naissent, aucun ne part, 25/25 justes ; passe épinglée verte (e2e_code_sync 21, lectures 27, code 36, usages 31, postgres 39, check_features 8/8, lib 2 564, navigateur 45) ; **REGLES_DE_L_ANALYSE monte à 3**. Dans le lot : la mesure du banc dans les trois tickets fermés (DEFINED_IN 3 814 → 63, « nom » 20 145 → 18 480, API C 0 → 30, .c 892 → 1 481), ticket neuf des 63 homonymes d'une même ligne (piste : la clé stable). Décidé : app.js minifié ne s'indexe pas (index_exclude du manifeste, la base du banc doit l'appliquer), 35 des 63 partent avec. Place après le plafond d'embarquement et StackError ; (2) le membre platform ensuite.

- 04:57, **arbre poussé** le refus typé du chargeur : master b9951b7d27 (StackError, Display garde le texte d'avant ; deux commits gmail, zéro attribution, vérifié). Suivants : le correctif du plafond d'embarquement (tête de file à son vert), le grain d'ED, les méthodes héritées de codeparsers, le segment de 96, le pointeur lucivy (au tag).

- 05:04, embarquement **prêt**, text-cap-cut (a224cc1aa7 sur 8c9cf1e31b) VERT par une passe complète (témoin a_batch_with_one_oversized_text_gets_all_its_vectors_and_the_cut_is_counted : 512 textes dont un de 53 000, les 512 vecteurs reviennent, le service ne voit rien au-dessus de 12 000, la coupe comptée ; rouges sur 381a934876, verts sur c453d4845d ; e2e_code_sync 21 exigé par catalog.rs, check_features 8/8, lib 2576, navigateur 45). Le client coupe au plafond déclaré par le service et compte ; le service coupe au lieu de refuser (cut_texts) ; le journal d'index dit la coupe ; ticket du banc fermé, ticket du découpeur ouvert chez ED, README et nouveautés. **« À toi » à embarquement** (repose d'un cran sur b9951b7d27) ; puis le banc rejoue B′ avec un hôte bâti sur master ; le binaire du service se pose avec les hôtes.

- 05:05, banc : B′ armé à luciepc, part seul dès que a224cc1aa7 est dans master (hôte rebâti sur la tête, 7878 à 12 000), puis A relu tous signaux et sonde des deux scopes d'en-tête et du nombre de scopes de rag3db.h. 96 : segment **prêt** (d2dccac72, dix commits reposés sur 8b7686025e, rejeu court vert à luciepc 5 h 04 : features 8/8, lib 2586/0, navigateur 45, suites mémoire vertes ; e2e_code_sync non exigé par ses chemins) ; place après embarquement, le grain d'ED et les méthodes héritées, repose d'un cran à chacun.

- 05:11, ED **prêt**, le grain du paquet reposé sur 8b7686025e (writer-batch-grain, 159ea916a, cinq commits gmail ; passe verte : e2e_code 36, e2e_code_sync 21/21, check_features 8/8, lib 2579/0/72, navigateur 45 ; les correctifs venus depuis gardés : saut de CONSUMED_BY par paquet, politique du généré au lot, marquer_des_lignes_invisibles ; noms anglais : thread_statements::{count, during}, FileToWrite, write_batch_by_generations, forget_edges_of_files, témoin 15 a_repass_costs_the_batch_not_the_file ; un commit vide « sans effet sur l'analyse » pour les 33 lignes de test dans code.rs). Place après la coupe au plafond ; puis ED prend le ticket du découpeur.

- 05:15, **embarquement poussé la coupe au plafond** : master 828ff28153 (text-cap-cut-r1, 55f76b9c5c ; trois commits gmail, zéro attribution, vérifié ; passe verte : témoin, e2e_code_sync 21 « suite exigée jouée verte (catalog.rs) », check_features 8/8, lib 2581, navigateur 45 ; master bougé par des docs pendant la passe, reposé par la garde). Un texte au-dessus du plafond est coupé et compté, jamais refusé avec son lot. B′ part seul au banc (hôte bâti sur cette tête). **« À toi » à ED** pour le grain (repose d'un cran, catalog.rs commun).

- 05:21, 96 : segment reposé sur 828ff28153 (lot-96-segment-pile-2, ae7a95fc6), rejeu court à luciepc depuis 5 h 16. Codeparsers : méthodes héritées reposées sur 828ff28153 (header-definition-2, 1347bb533, REGLES 2 → 3, check_regles et check_tickets VERT ; code.rs commun : passe complète en file après sa sonde du membre platform) ; (2) membre platform : rouge vu sur les vrais fichiers (process importé d'une bibliothèque extérieure ; process.platform lu au niveau du fichier perdait son propriétaire), corrigé sur codeparsers/platform-member (c19d8caf7), sonde avant/après en cours. Rapport de session de l'arbre sur master (b0b8860b1b).

- 05:26, **ED poussé le grain du paquet** : master b45003f0c (writer-batch-grain-2, 6f41d381e ; cinq commits gmail, zéro attribution, vérifié ; rejeu court : e2e_code, e2e_code_sync 21/21, check_features 8/8, lib 2582/0/72, navigateur 45 ; garde : « suite exigée jouée verte »). Le repassage de la version passe par le grain (193 s contre 243 s mesuré par le banc sur d57b28c80 ; à remesurer sur master). Suite d'ED : le découpeur (ticket d'embarquement), les ex æquo de lucivy, le plein texte rebâti, l'observation des namespaces. 96 : segment vert (ae7a95fc6, rejeu 5 h 25). **« À toi » à codeparsers** à son vert (passe complète en file), puis 96.

- 05:28, embarquement : **B′ vert sur l'embarquement** (99 235 morceaux, aucune passe en échec, 654 s) ; et une affirmation fausse de son lot 828ff28153, dite par lui : le journal d'index ne dit pas les coupes (l'avertissement part en CatalogEvent::Warning sur le bus du catalogue, run_index ne s'y abonne pas ; le banc n'a rien vu) ; la coupe est comptée (char_truncations) mais pas dite ; les jetons tronqués par le modèle (troncatures) non plus. Lot court accordé, rouge d'abord : le journal d'index écrit les coupes et les jetons tronqués après chaque passe (à côté de la provenance), README et nouveautés corrigés ; place après le segment de 96. Le banc vérifie que les 31 morceaux longs ont leur vecteur.

- 05:30, banc, B′ sur 828ff28153 : 92 708 morceaux embarqués sur 92 708, aucune passe en échec, les 31 longs ont leur vecteur (coupés à 12 000 dans B′, entiers dans A) ; coût réel 718 s (index + embarquement, 7878 seul). rag3db_database et getValue existent dans B′ en rangées ouvertes ; **aucun des deux n'est dans le top 10 de B′ non plus** (rangs 3 à 10 de « rag3db · src/include/ » = des namespace rag3db aussi) ; rag3db.h : 25 scopes dans B′ comme dans A : **les deux pertes de A face à B ne viennent pas de la migration**, un index neuf d'aujourd'hui les rend aussi (découpage et classement d'aujourd'hui ; le poids d'un namespace contre une déclaration, observation déjà chez ED). A contre B′, premier passage : rappel à 10 0,953 (286/300), même premier 27/30, même classement 21/30 ; écarts : read_text un échange au rang 10, rag3db trois namespaces échangés aux rangs 8–10, et « parse a json manifest sans filtre » tout autre : **erreur de banc**, la source de B′ (worktree détaché) n'avait pas le sous-module codeparsers (A l'a : css_parser.rs, scss_parser.rs, svelte_parser.rs) ; initialisé à 0c72f68, le script refuse désormais une source à sous-module vide ; B′ rejoué depuis 5 h 29 sur 767c7234e3 (~12 min).

- 05:42, ED **prêt**, le découpeur borné en caractères (line-chunker-char-bound, f59bc5f56 sur e59cc17d6, trois commits gmail ; passe verte : e2e_code 36, e2e_lectures 27, check_features 8/8, lib 2583/0/72, navigateur 45 ; rouge d'abord sur les cinq fichiers du banc, « a chunk of 53294 bytes ») : une ligne plus longue que max_size se coupe en cœurs d'au plus max_size octets sur des frontières de caractère, le contexte s'arrête avant max_size, un morceau fait au plus trois fois la taille, les cœurs bout à bout rendent le texte ; ticket d'embarquement fermé. Réserve : une base déjà indexée garde ses anciens morceaux géants tant que leur contenu n'est pas réécrit (court-circuit de l'inchangé) ; décidé : accepté (ils ont déjà leur vecteur), ticket de confort pour un redécoupage forcé au-delà de 3 × max_size. Place après codeparsers, 96, le journal des coupes.

- 05:45, banc, **A contre B′, final** (B′ = index neuf de 284cc00c0b, sous-module compris, par master 767c7234e3 ; 787 s dont 721 d'index et d'embarquement, 96 884 ScopeContent_Chunk tous embarqués ; tous signaux, vector et bm25 exigés) : **rappel à 10 0,977 (293/300), même premier 28/30, même classement 21/30** (8 avec un score différent) ; écarts en queue ou dans l'ordre (read_text un échange au rang 10 et le premier change ; rag3db trois namespaces échangés aux rangs 8–10 ; json deux file_scope échangés aux rangs 9–10 ; rag3db·python, value, translate a hop : même ensemble, autre ordre). Deux causes candidates : (1) A porte 113 570 ScopeContent_Chunk contre 96 884 (l'histoire au plein texte pèse sur BM25, ce que lucivy 4.3.9 corrige) ; (2) les vecteurs de A sont d'avant le binaire du 15 à 20 h 36 (31 longs entiers contre coupés). Départage demandé : bm25 seul et vector seul sur A et B′ (4 min). rag3db_database et getValue hors du top 10 des deux, rag3db.h 25 scopes des deux côtés. Ticket de confort d'ED (de66467f9b) : redécouper les versions au-delà de 3 × max_size.

- 05:45, embarquement **prêt**, le journal des coupes (journal-cuts, e0a6299bdb sur 767c7234e3) VERT par une passe complète (témoin : « 3 texte(s) coupé(s) à 12000 caractères avant l'envoi au service » et « 2 texte(s) tronqué(s) à 512 jetons », la dernière ligne reste le contrat ; recu_index, e2e_lectures 27, e2e_code_sync 21 (catalog/progress.rs), check_features 8/8, lib 2584, navigateur 45) ; un défaut de plus corrigé : new_index_journal nommait le journal par la milliseconde et le pid, deux index dans la même milliseconde écrivaient dans un seul journal (témoin 64 journaux pour 1 fichier, rouge ; compteur du processus ajouté au nom) ; la ligne de nouveautés corrige la promesse de 828ff28153. Place après codeparsers et 96, puis le découpeur d'ED.

- 05:59 (notes retenues pendant les pushes). **Codeparsers poussé** les méthodes héritées : master a04518fa31 (quatre commits gmail, zéro attribution ; REGLES_DE_L_ANALYSE = 3 ; app.js hors index ; deux courses perdues contre mes commits de docs à la minute, la garde a reposé). **Banc, départage** : bm25 seul A ≠ B′ (rappel 0,968, même classement 14/30 : le plein texte compte l'histoire de A, 113 570 morceaux contre 96 884 : cause (1), ce que lucivy 4.3.9 corrige, à reprouver au pointeur) ; vector seul A ≠ B′ dans les scores (2/30 mêmes scores) mais presque pas dans le classement (23/30, rappel 0,973) : cause (2). Vérification au bit près sur 412 morceaux inchangés communs : 217 identiques, 195 qui diffèrent d'au plus 5,86e-5 (cosinus ~1e-5) : **bruit numérique selon le lot sur GPU** (remplissage, ordre des réductions, autotune de cubecl), attendu dit embarquement ; demain : tolérance mesurée dans la preuve de parité des services, ticket d'observation, réglage déterministe s'il est gratuit. Pour Lucie : la copie migrée ne perd rien de propre à la migration. Page du banc : docs/banc/16-11-2026/01-la-parite-de-la-migration.md (a27f9276eb). **96 poussé** le segment : master 53a83d089b (dix commits gmail, zéro attribution, vérifié ; garde en mode simple, base a04518fa31 ; rejeu vert 5 h 57). **« À toi » à embarquement** (journal des coupes, repose d'un cran, rejeu court). ED prêt avec deux lots indépendants : le découpeur (f59bc5f56) et les ex æquo (stable-tie-order, 5c42a5866 : search_hits_with_ties demande limit + 8 et double tant que le dernier est ex æquo, au plus 4 096 ; tri par score puis uuid ; ticket de lucivy fermé) : en une pile après embarquement. Règle pour moi : retenir mes docs pendant un push en cours.

- 06:07, ED : **ticket du plein texte rebâti fermé sur preuve** (91190d064) : le juge du banc rejoué sur master 11878bf6d, l'élagué rend le classement du neuf au rang près (289/289, 30/30 premiers, 30/30 classements), le cas vide sous src/include/ a disparu ; le 0,668 venait du juge joué à 794b14232 avant le correctif de tout_visible (arrivé sous **209bc299b**, non 5081d2281 : hash corrigé). Ce qui reste entre le lot et le neuf (0,900) est tout entier dû aux versions fermées, la longueur moyenne que lucivy 4.3.9 corrige ; le ticket BM25 garde sa cause sans autre suspect. Moniteur réarmé.

- 06:21, **embarquement poussé** le journal des coupes : master ca3120fb70 (journal-cuts-r1, 742c4904a7 ; trois commits gmail, zéro attribution ; passe verte : recu_index, e2e_lectures 27, e2e_code_sync 21, check_features 8/8, lib 2592, navigateur 45) : le journal d'index écrit les coupes au plafond et les troncatures du modèle, chaque index a son journal. **ED poussé** sa pile : master f9ebafc9e7 (sept commits gmail, zéro attribution, vérifié) : le découpeur coupe une ligne plus longue que la taille de morceau en cœurs bornés ; la recherche lexicale rend un ordre stable d'une base à l'autre (search_hits_with_ties, tri par score puis uuid) ; deux tickets fermés. **La file de la nuit est vide** hors le pointeur lucivy (au tag, mot de Lucie) et les lots en préparation (codeparsers : membre platform ; cœur : dictionnaire sous ASan). Demain pour embarquement : tolérance de la parité des services, ticket d'observation du bit selon le lot, réglage déterministe de cubecl s'il est gratuit.

- 06:21, demandé à G : rebâtir les hôtes de la bascule sur la tête courante de master (3177d2489e ou plus) contre la lib 52101b7fd (ed14435570 est périmée : écrivain, vues, outils de l'agent, coupe au plafond, découpeur, ex æquo, repli…), verdict des trois suites sans MOTEUR_ANCIEN, essai 7991 aux deux postes, binaire du service d'embarquement prêt aussi ; rien ne touche les hôtes en service ; la fenêtre au mot de Lucie à son réveil.

- 06:22, G : master 3177d2489e porte un commit moteur de plus que la lib 52101b7fd (59ac9c86bc, le fsync du répertoire de hotfixs) ; décidé (a) : rebâtir la lib sur master (59ac9c86bc, ~6 min incrémental), verdict sans MOTEUR_ANCIEN, hôtes contre elle ; le cœur dit l'ABI ; le binaire du service d'embarquement bâti sur 3177d2489e (embeddings.rs et Cargo.toml ont changé depuis f85977d09).

- 06:26, cœur : ABI inchangée pour 59ac9c86bc (cinq en-têtes changés, aucun atteint depuis rag3db.h ; syncDirectory s'insère dans la vtable de FileSystem, sans effet pour le Rust ; une classe héritant de FileSystem hors de la lib devrait être rebâtie avec elle, aucune) ; ASan vers 7 h 15. Codeparsers **prêt** : le membre platform (platform-member-2, 3d6b1f538, pointeur edfc72d) : un qualificatif importé d'une bibliothèque de provenance sûre (builtin, dependency) ne prend pas de rendez-vous (la première forme ôtait 65 liens Python justes, resserrée) ; les références au niveau du fichier gardent leur qualificatif ; sonde « nom » : rag3weaver/src 9 064 → 9 059, moteur 2 654 → 2 653, nodejs_api 348 → 324, python_api 256 ; 35 couples faux partent, aucun ne naît ; passe épinglée verte (e2e_usages 33) ; **REGLES_DE_L_ANALYSE monte à 4**. « À toi » donné ; la garde a refusé (la pile d'ED, chunker/fts_handle/search, avait atterri : « que des docs » de c7 était faux) ; repose, rejeu court, à toi dès son vert. Ensuite (3), le reste des receveurs.

- 06:31, G, **tout prêt pour la fenêtre** : lib rag3db-build-59ac9c86bc… bâtie à luciepc (6 h 22 → 6 h 24, sur 3177d2489e ; librag3db 697c7a0c, libvector 925b6b13 ; copiée au portable ; symboles contre 373b5477b : 17 ajoutés, 0 retiré ; vtables de FileSystem/LocalFileSystem/VirtualFileSystem changées sans effet pour le Rust : ABI_INCHANGEE) ; verdict sous RAG3DB_LIB=59ac9c86bc sans MOTEUR_ANCIEN (6 h 24 → 6 h 27) : e2e_code 36, lectures 27, demon_hote 24, aucun saut ; hôtes contre elle (md5 d164e4da, check-lib 4c1e9eb2), essai 7991 VERT aux deux postes, check-manifests 0 refusé ; service d'embarquement sur 3177d2489e (sha256 6196fbe7102ad533, en place a70f1e462bd9df95), posé avec la preuve au bit près, 7879 puis 7878. Courant 373b5477b, hôtes 8d9e62c3, services a70f1e46 inchangés. La fenêtre part au mot de Lucie.

- 06:36, **codeparsers poussé** le membre platform : master 96f538527c (cinq commits gmail, zéro attribution, vérifié ; REGLES_DE_L_ANALYSE = 4 ; un membre d'une bibliothèque de provenance sûre n'est pas un homonyme du projet, un membre lu au niveau du fichier garde son qualificatif ; ticket fermé). Plan de la bascule mis à jour par G (bc22e3e3e7, lib 59ac9c86bc). La file est vide ; codeparsers passe à (3), le reste des receveurs sans type.

- 07:07, CI de master : les deux workflows verts sur f9ebafc9e7 (la pile d'ED), CI Workflow vert sur 96f538527c, rag3weaver en cours. Relevé des sessions : toutes présentes ; archi « waiting », muet depuis 1 h 05. Moniteur réarmé. En cours : l'ASan du dictionnaire (cœur, verdict attendu vers 7 h 15), le reste des receveurs (codeparsers). Au mot de Lucie : le tag lucivy v4.3.9 puis son pointeur, la fenêtre de la bascule de lib (tout prêt), puis la bascule des bases.

- 07:08, cœur, ASan du dictionnaire : 7 tours sur 12, aucun rapport d'AddressSanitizer ni plantage (chaque tour rouge sur ses bornes de temps, dépassées par ASan ; 15 à 50 min par tour sous la charge) ; reproduit en Release (1/12 sous 200 Mio, deux fois les 14 et 15), pas sous ASan : le signe d'une course plutôt qu'un débordement. Décidé : arrêter l'ASan (les 5 derniers tours coûteraient 2 h de lourds, la fenêtre de la bascule en aura besoin) ; lecture du chemin (qui écrit dans le DictionaryChunk pendant finalize sous un point de reprise par COMMIT, quel verrou), puis TSan sur ce seul cas si la lecture ne tranche pas ; ticket à jour.

- 07:09, codeparsers, (3) le reste des receveurs sans type (receivers-tuple-shadow, a5af318a5, codeparsers 6291939) : (a) motif de tuple (chaque nom prend le différé de la valeur plus un pas #i) ; (b) liaisons différées situées (une liaison vaut de la fin de son let à la fin de son bloc ; `let outil = …?; let outil = outil.bind()?;` ne s'annule plus) ; deux témoins rouges puis verts ; e2e_usages 35, e2e_code 57. Sonde : moteur inchangé ; rag3weaver/src +245 couples (228 par le type), 278 de « nom » à « type », 47 partent (~25 std faux ; ~20 liens justes dans teammate_tests.rs : la première liaison porte `ReturnOf hote` dont le différé ne se lit pas, hote défini quatre fois, règle du 11 octobre). Décidé (i) : garder la règle du 11 octobre ; les ~20 au ticket avec la piste « ReturnOf résolu par l'import du fichier » ; REGLES à 5 ; passe complète ; « prêt ».

- 07:09, cœur, le SIGSEGV du dictionnaire a son mécanisme (pas une course de verrou) : StringChunkData::append et appendStringColumnChunk (le COMMIT qui verse ses lignes locales dans le groupe) font updateNumValues(numValues + 1) **avant** setValueFromString, donc avant DictionaryChunk::appendString ; sous pool plein appendString lève sur son resize ; la ligne reste comptée, non nulle, avec un indice jamais écrit ; au point de reprise suivant StringChunkData::finalize relit getValue<string_view>(i) par cet indice : une vue sauvage lue par Hash_bytes dans indexTable.find, la pile même ; d'où pool serré et jamais sous ASan. Témoin rouge d'abord par point de faute string-dictionary-append ; correctif : la chaîne avant le compte ; voie C++ à son vert. ASan arrêté, rien ne tourne.

- 07:22, **codeparsers poussé** le reste des receveurs sans type : master 436998e59c (trois commits gmail, zéro attribution, vérifié ; pointeur codeparsers 6291939 ; passe complète verte : e2e_code_sync 21, lectures 27, code 36, usages 35, postgres 39, check_features 8/8, lib 2 594) : une locale d'un motif de tuple et une locale redéclarée lisent leur type ; **REGLES_DE_L_ANALYSE = 5** ; sonde rag3weaver/src « type » 9 070 → 9 572, « nom » 9 059 → 8 736 ; les vingt liens de teammate_tests.rs au ticket avec la piste de l'import. La file est vide ; la définition liée à l'en-tête n'a plus que ce ticket.

- 07:23, codeparsers : dépôt codeparsers master edfc72d → 6291939 (avance rapide) ; journaux archivés (codeparsers-suite-16-nov, 70 Mo), quatre worktrees retirés, restent wt-master-temoin et le target. Sa suite, ordre de c7 : (3) une classe derrière une macro d'export sans scope (une classe du moteur perdue entière, à compter sur src/) ; (1) les 28 homonymes d'une même ligne (clé stable) ; (2) ReturnOf résolu par l'import ; (4) le genre d'une méthode d'un autre fichier ; (5) les mots vides de Rust tirés du poste. L'import ES externe attend le mot de Lucie.

- 07:24, codeparsers a compté (3) avant le rouge : 0 classe du moteur perdue derrière une macro seule sur sa ligne ; third_party : nlohmann_json 4 (basic_json), pybind11 2 faux positifs ; extension : la fixture fmt/format.h du ticket. (3) en basse priorité avec le compte au ticket ; il commence (1), les 28 homonymes d'une même ligne, puis (2).

- 07:36, cœur : pas de rouge pour le dictionnaire : le témoin DictionaryAppendRefused est vert avant comme après le correctif (transaction_test 395) ; le point de faute a refusé le COPY, la table s'est relue ; le témoin du COMMIT acceptait les deux issues (mal écrit) ; explication probable du vert : la ligne comptée sans indice tombe dans une mémoire fraîche à zéro, l'indice 0 désigne une chaîne valide ; sous un vrai pool plein la mémoire réutilisée porte n'importe quoi. Rien ne pousse sans rouge (branche dictionary-index-after-string en branche). Décidé (a) : un témoin qui regarde l'état (numValues > indices écrits, par un crochet de test) ou un point de faute qui empoisonne la mémoire fraîche (0xFF) pour rendre l'indice faux ; le témoin du COMMIT exige qu'il ait levé ; sinon TSan.

- 08:05, cœur : la piste de l'ajout à moitié fait tombe : le témoin empoisonné (tampons neufs à 0xFF, TestHooks::setPoisonFreshBuffers) exige que COMMIT et COPY lèvent au point de faute ; ils lèvent, puis CHECKPOINT, relecture, réouverture : VERT avant (b28b22c72) comme après (256413fde) ; l'annulation retire la ligne comptée avant que finalize la lise ; le correctif reste en branche (dictionary-index-after-string-2), non poussé. Suite : TSan sur le cas sous 200 Mio (bâti à neuf ~1 h à -j8, à luciepc sous poste lourd, une place, cédée à G pour la fenêtre). En route : PkLookupUnderCheckpoint.LookupsDuringInsertsAndCheckpointsDoNotCrash rouge une fois dans un transaction_test entier (pk-lookup-reads-right), 0/60 seul : intermittent rare du défaut 0c41f9fe6, au ticket fc6f5f96c.

- 08:15, **codeparsers poussé** les homonymes d'une même ligne : master bbc6d88de4 (quatre commits gmail, zéro attribution, vérifié ; pointeur d754ee5). Cause : deux générateurs d'une même ligne Python au niveau du module avaient mêmes nom, genre, lignes et octets de scope ; codeparsers leur donnait le même uuid (table des positions clée par ces octets) et la clé de position de rag3weaver les confondait ; distingués par les octets du nœud (ScopeMappingEntry) ; témoin rouge puis vert ; sonde : scopes sans DEFINED_IN 26 → 0 (scripts Python), spdlog 1 → 0, aucun couple CONSUMES ne bouge ; passe épinglée verte (e2e_usages 36) ; **REGLES_DE_L_ANALYSE = 6**. Codeparsers passe à (2), ReturnOf résolu par l'import. Dépôt codeparsers master 6291939 → d754ee5.

- 08:58, **codeparsers poussé** ReturnOf par l'import : master ab685ed779 (trois commits gmail, zéro attribution, vérifié ; pointeur bd13385) : ReturnOf porte le module d'où le fichier importe la fonction ; au rendez-vous, quand les définitions ne s'accordent pas, celle du fichier que ce module désigne donne le retour ; l'abstention du 11 octobre reste pour un différé non lu ; sonde rag3weaver/src : « type » 9 572 → 9 691, 86 couples naissent (les vingt de teammate_tests.rs reviennent), deux std faux partent ; passe verte (e2e_usages 37) ; **REGLES_DE_L_ANALYSE = 7**. Codeparsers passe à (4), le genre d'une méthode d'un autre fichier. Moniteur réarmé (deux refus passagers du contrôleur avant).

- 08:59, codeparsers : (4) le genre d'une méthode définie dans un autre fichier n'est pas à lui (la classe est résolue au rendez-vous, après la publication ; réécrire scope_type là ferait naître une version hors de l'écrivain) : ticket chez ED, dans sa file après la bascule (puis les namespaces, le redécoupage forcé). Codeparsers passe à (5), les mots vides de Rust tirés du poste en données (data/stop_words/rust.toml, générateur depuis les mots-clés et le prélude de std 1.98.1), sonde avant/après. Dépôt codeparsers master d754ee5 → bd13385.

- 09:20, **lucivy : le tag v4.3.9 est posé** sur 0825bcd (objet du tag 3693198, tagger gmail, vérifié par `git cat-file` ; CI de main verte ; release.yml parti). Le mot de Lucie a été pris dans la session de lucivy (« oki tu peux poser le tag oui stp »), pas dans la mienne : je le lui dis tel quel. Suite chez lucivy : registres, CLAUDE.md, puis le pointeur rag3weaver « 4.3.9 » avec son rejeu court ; sa place en file : après codeparsers (5) et avant la bascule de lib (la table de débit d'embarquement est pour demain). Au pointeur posé, le banc rejoue le bm25 seul puis tous signaux sur les copies A/B′. Mémoire : MEMORY.md remis à 200 lignes (trois fiches fusionnées : arrêt d'un lourd, resume.log périmé, Lucie joue sur luciepc).

- 09:27, **Lucie, lundi matin : passation pour Codex** (« rapport de passation avec demandes claires de tickets à traiter ensuite, chacune pour Codex ; si pendant la semaine je bosse avec Codex il saura quoi faire ; on peut lui prévoir beaucoup de boulot, juste bien expliciter ; comme la dernière fois »). Consigne envoyée aux treize sessions : une page `docs/<session>/16-11-2026/NN-passation-pour-codex.md` en quatre parties (état ; travail pour Codex dans l'ordre, par tickets réels avec quoi faire, comment prouver, taille, blocage ; ce qui attend Lucie ; règles du périmètre dont l'anglais du code neuf et les branches `cx-…`), poussée en docs seules, hash à moi. Je rassemble dans `docs/orchestration/16-11-2026/02-passation-pour-codex.md` et je mets `docs/README.md` à jour. La file de code reprend après ; ce qui tourne (TSan du cœur, registres de lucivy) continue.

- 09:34, **passation pour Codex : dix pages sur treize au tableau** (recherche 04102a27f6, cœur 96daa8f122, G 499cb10de6, hotfixs d61841b448, ED d663340ef4, embarquement 80ba4f661c, mémoire db6533bf8d, tty d1ea5a947a, codeparsers 87c584898d, banc 5f376f1521 ; chacune vérifiée gmail, zéro attribution, docs seules). Restent arbre, lucivy, archi (muet depuis 1 h 05). Incident dit par banc : son script de push disait « poussé » sur un `git push` refusé (code de sortie non lu) ; course perdue contre tty, personne n'a forcé ; le script refuse désormais un push rejeté et relit que HEAD est dans master ; ses sept pushes du jour revérifiés dans master. Codeparsers reprend (5) avec le std homonyme d'abord.

- 09:42, **archi débloqué, et une règle de Lucie** : archi était muet depuis 1 h 05 parce qu'il attendait la réponse de Lucie à une question (écrire ou non l'entrée témoin dans `~/.claude/sessions`) ; elle a répondu oui, et dit : « c'est dommage, ça tout bloque à cause d'une question ». Règle, dite aux treize sessions : **une question à Lucie ne bloque jamais une session** ; on l'écrit (page ou rapport), on la dit à c7 en une ligne, on fait tout ce qui n'en dépend pas ; si tout en dépend, l'option la moins engageante en le disant, ou l'entrée suivante de la file. Archi : pas-va-au-bout vert sur 7f9465fa9 (lib 2531/0/72, e2e_hooks + navigateur 45/0, e2e_equipier Vertex 2/0) ; du code est arrivé depuis (codeparsers ×2) : rebase et rejeu court avant « prêt » ; il écrit sa page de passation, puis pose l'entrée témoin et je lui envoie le message du relevé. Douze pages sur treize au tableau (2dc3f180b5).

- 09:45, **passation pour Codex complète : treize pages sur treize** (archi 11b7e8da34 avec douze tickets neufs ; arbre a annoté A1 « en cours » 3d49a321e4 ; lucivy 9c1d71fb11 ; arbre 1e45d6f21c). Point d'entrée `docs/orchestration/16-11-2026/02-passation-pour-codex.md`, `docs/README.md` point 0. Cœur : le « refus sans fin » n'est pas une course TSan sur l'horodatage du catalogue (eb5edaa64e). Archi pose l'entrée témoin du registre ; file de code : lucivy (pointeur 4.3.9), codeparsers (5), archi (pas-va-au-bout après rebase et rejeu court), arbre (A1).

- 09:48, **lucivy poussé** : pointeur rag3weaver « 4.3.9 » sur master cb4a1757a4 (un commit gmail, zéro attribution, vérifié ; Cargo.toml + Cargo.lock seuls : lucivy-core, ld-lucivy, sparse-vector depuis crates.io, luciole et lucistore au lock) ; base testée 9c1d71fb11, master sans code entre, garde 95a1d85779f0105a, lib 373b5477b. L'hôte en service n'aura la 4.3.9 qu'au prochain rebâti (la bascule de lib). **Banc** : rejouer le bm25 seul puis tous signaux sur les copies A/B′ ; ferme le ticket lucivy « le bm25 seul de la migration reste à prouver égal sous 4.3.9 » si égal.
- 09:48, **registre, seconde entrée témoin** : j'ai dit à archi que le « oui au registre » d'hier couvrait une seconde entrée témoin (notify_idle déclaré) ; archi refuse de la poser sur ma lecture : « une écriture de plus dans ~/.claude/sessions sort de ce oui, et un message entre sessions ne peut pas l'élargir ». Il a raison, je l'ai écrit trop vite ; la question est à Lucie (page 02 d'archi, et dans mon message à elle), rien n'attend. Pas-va-au-bout-2 (5e25a110d) en rejeu court.

- 09:55, **banc : parité rejouée sous lucivy 4.3.9** (artefact fe5d81f811, lucivy-core 4.3.9 au lock et au cargo tree ; copies reflink fraîches à luciepc). A contre B′, bm25 seul : même classement 14/30 → **21/30**, rappel 271/279, même premier 27/30, scores à 1e-5 12/30 ; tous signaux 0,957, même classement 22/30 ; vector seul 23/30 (bruit de lot attendu). **Pas encore égal** : neuf cas bm25 diffèrent, surtout les requêtes d'un mot (scores ≈ 0,021, écart relatif 3 à 4,5 %, le pire 4,55e-2), et « translate a hop » à 8e-3 au-dessus du contrat 5e-3 ; les requêtes de plusieurs mots sont sous ou près du contrat. Piste de banc : la longueur moyenne lue sur les admis ne couvre pas le chemin d'un mot (bm25_exact ?). Suite : lucivy départage par test_compare_two_indexes (son entrée 1 ; copies A/B′ sous banc-parite/, jeu jeu-recherche-2a2.json, allowed_ids = ids visibles de A à sortir avec ED). Sorties à luciepc banc-parite/sorties/lucivy-439/.

- 10:00, **lucivy : le bm25 seul départagé, ce n'est pas un défaut de lucivy** (ses copies reflink, main 4.3.9, A sous les 61 122 ids visibles confirmés par ED, les dix requêtes sous les formes réelles). (1) Les scores tiennent le contrat : écart relatif ≤ 1,8e-3 sur les tops, 1,5e-4 à 4e-4 sur un mot ; les 3 à 4,5 % de banc étaient des scores RRF (un rang changé). (2) read_text, value, transaction rollback : permutations entre ex æquo stricts (mêmes scores, ordre des adresses) ; le tri score puis uuid d'ED (07c274b1a8) est dans fe5d81f811, donc si ça bouge encore c'est plus haut que lucivy (uuid différent migration/neuf pour la même version, ou rang RRF pris avant le tri) : **chez ED**. (3) « translate a hop » : les documents qui entrent dans le top de A n'existent pas dans B′. (4) **Les contenus cherchés ne sont pas les mêmes** : identités content+name 61 013 (B′) contre 61 045 (A visible), 879 seulement dans B′, 911 seulement dans A ; listes des _node_id à luciepc (lucivy-bm25-migration/contenus-only-*.txt). Lucivy ferme son ticket, ouvre un ticket **ED** « la migration et l'index neuf ne cherchent pas les mêmes contenus visibles » (point 2 en second), propose l'outil de détail dans lucivy. Les formes chemin et langage ne sont pas jouées : pas nécessaire tant que le contenu diffère.

- 10:02, **banc : les 911/879 contenus d'un seul côté ne sont pas la migration**. (1) 122 occurrences de A viennent de `third_party/fuzzy-fst/`, **non suivi** dans l'arbre de Lucie à luciepc : A est indexée du working tree (manifeste `source: working_tree`), B′ d'un worktree propre au commit ; écart de banc, rien à corriger, à dire à Lucie. (2) Les 789 autres de A et les 879 de B′ sont des en-têtes C/C++ de third_party (json.hpp 330/449, spdlog 276/335, yyjson 99/27…) découpés autrement : A faite à 0 h 5x, avant le lot codeparsers extern "C" + REGLES_DE_L_ANALYSE 2 (3 h 3x) ; 21 scopes de même identité ont des contenus de tailles différentes (lz4.hpp file_scope 1 785 contre 34 146). **Le repassage de la bascule sous les règles courantes le refera.** Conclusion : lucivy rend les mêmes scores sur les mêmes contenus ; l'écart bm25 A/B′ vient du découpage d'avant et des fichiers non suivis. Ticket lucivy fermé sur ce relevé (§ 12 de la page de parité). ED garde le seul point 1 (les ex æquo par uuid `_depuis`, lot court) ; sa lecture du point 2 n'a plus lieu.

- 10:07, **lucivy : la compaction qui « grossit » n'est pas un défaut** (52bcb007c0, docs seules). Mesure à luciepc sur une copie de Lucivy_Source : maintain compact idempotent (98 636 021 octets à l'octet près au second passage) ; 147 documents en un commit chacun = 14 segments, 22 générations, .sfx 62,38 Mo ; après compaction 4 générations, 64,39 Mo ; un index neuf en un commit : 64,23 Mo. La compaction rend la forme d'un index neuf à 0,25 % ; les petites générations tenaient les mêmes suffixes en 3 % de moins, mais 22 FST par shard à parcourir par requête. Ticket fermé sur la mesure, entrée 2 faite ; PR lucivy #42 (outil de détail, test ignoré, sans tag) fusionnée au vert de la CI (dépôt à elle, règle du 3 oct.). Lucivy passe à l'entrée 3 (l'ulp en parse). Banc : entrée 6 en cours (1b38dc6afc). ED : lot court des ex æquo.

- 10:08, **banc d'étalonnage du débit : conception posée** (`docs/banc/16-11-2026/03-le-banc-d-etalonnage.md`, 01d94fa6f1) et accord donné pour les pas 1 à 4 (jeu figé de 4 096 textes réels de B′ + 1 500 pleins ; mesure par le vrai client de service, test ignoré `tests/bench_embedding_rate.rs`, témoin rouge au faux service à 5 % ; 7878/7879 en local à luciepc et par tunnel du portable ; lots 1 à 512 ; p50/p90 ; preuve de l'artefact ; vérification contre les lignes [montre] et la clé de l'index, écart > 30 % dit). Constat : **l'attente de 6 s ne compte pas l'embarquement** (le fichier sort du retard au texte écrit, les vecteurs viennent après hors verrou). Décision c7, **à relire par Lucie** : avec le signal vector, l'attente compte aussi les morceaux dus (× taille moyenne ÷ débit du service de la vue) ; bm25 seul : le texte seul. 7880 froid : non réveillé. Q2 (clé `embedding_rate:{modèle}:{adresse}@{poste}`) et Q3 (embarqueur CPU) à embarquement, qui fusionne sa table de débit avec ce banc. Recherche (entrée 10) et arbre (estimation par phase) complètent leurs tickets avec cette forme.

- 10:22, **arrêt des treize sessions au mot de Lucie** (« si la passation est prête dis-leur de s'arrêter », 10 h 5x). Toutes arrêtées, listes vides vérifiées, branches poussées telles quelles, pages annotées (hashes dans la page d'orchestration § 4 bis). Vert prêt au push : archi pas-va-au-bout-2 5e25a110d, ED stable-tie-identity 1834bf111. En cours : codeparsers (5), arbre A1, embarquement rate-key, cœur C1 (TSan finit seul vers midi), lucivy ulp en parse (cause trouvée, correctif en branche lucivy), banc étalonnage pas 1. Refus non contourné : fusion PR lucivy #42 (à Lucie). La file est fermée ; aucun « à toi » ancien ne vaut. Mes trois propositions à Lucie : bascule de lib maintenant par G ; seconde entrée témoin oui ; l'attente de 6 s compte les vecteurs dus.

## 18 novembre 2026 : reprise après la journée Codex

- 11:40, **reprise au mot de Lucie** (« c'est utile tout ce qu'il a fait ? oui lançons tout ça et d'abord chaque session évalue la pertinence de ce qu'a fait Codex »). Relevé en lecture seule : Codex a tenu les treize rôles du 16 (11 h) au 17 (21 h 08, arrêt sur quota ; sa passation `docs/orchestration/17-11-2026/99-passation-pour-claude.md` sur `origin/cx-orchestration-reprise-16nov-r1`, douze passations de rôles sur leurs branches, rien de tout cela sur master). **Master : 76 commits** (927941a67a..99fef92ce6, tous gmail, zéro attribution ; 17 fichiers de code : clé du débit au catalogue (rate-key d'embarquement atterri), ordre stable des ex æquo (lot d'ED atterri), source de jetons Vertex renouvelée, gardes tools/poste ; le reste docs). **Bascule de lib faite par le Codex de G le 16 à 11 h 1x** (806715b2d6, épingle 68590270e2) : les deux hôtes sur 59ac9c86bc (vérifié /sante). Garde au cache = master (push_garde fff9a4d9e20e739e, passe_epinglee 2150fcd847b1df7d ; poste 79e3a8bbf457e3bc inchangé). CI : « CI Workflow » vert sur 99fef92ce6 ; sept runs amont (LDBC, ClickBench, SimSIMD, Multiplatform) en file depuis 24 h sans runner : **mon annulation a été refusée par le classifieur**, laissée à Lucie. Une quarantaine de branches `cx-…` non posées. Consigne aux treize : évaluer d'abord le travail de Codex dans son périmètre (page `docs/<session>/18-11-2026/01-evaluation-du-travail-de-codex.md` : tableau commit/branche → utile, à requalifier, inutile ou nuisible ; verdict ; effet sur la page du 16 ; lisibilité), puis rebase, rejeu, et la liste reprend ; un seul « à toi ». Moniteur réarmé.

- 11:48, **évaluation du travail de Codex : treize verdicts rassemblés** dans `docs/orchestration/18-11-2026/01-evaluation-du-travail-de-codex.md` (verdict d'ensemble en bas). Les listes reprennent en branche : archi pas-va-au-bout-3 (f20762821, rejeu complet 5 h), recherche lot 1 garde FTS sous générations, codeparsers (5) reposé, embarquement lot `compatible`, cœur correctif des offsets, tty tty-21, ED 2.6, banc étalonnage pas 2, G entrée 1, arbre A1, lucivy lucivy-core sans corpus, mémoire 2.2, hotfixs entrée 2. Mots de Lucie attendus : rows_outside_graph (a)/(b) ; clone linux-7.2 ; fusion PR lucivy #42 ; C1 au cœur ; 58 Go de copies de Codex à retirer ; seconde entrée témoin ; l'attente de 6 s ; les sept runs CI amont.

- 11:49, codeparsers : dix tickets du banc multi-langages et du corpus public portés sur master en docs seules (184c96da4e..ccc0c90d16, puis 9828ef5298) ; **master rouge deux minutes** : 9828ef5298 est parti avec check_tickets ROUGE (trois champs d'en-tête manquants, le push non gardé par le verdict), corrigé en 77d9f00454 ; ses pushes de docs passent désormais par `if grep -q ^VERT`. Embarquement : le ticket du débit par service est annoté (clé qualifiée, reste la mesure par passe et la table), pas fermé ; lot `compatible` : trois rouges écrits, passe en attente d'une place lourde (4/4 tenues). (5) de codeparsers : sonde en file.

- 11:50, arbre (eeb7654460) : **la cause du 422 de `maintain` est trouvée** dans les notes de Codex : le catalogue ouvre ses index plein texte à la demande et `fts_handle_nomme` ne fait que chercher ; `maintain` et `fts-live-ids` refusent tant qu'aucune recherche n'a ouvert la table (juste après un redémarrage de l'hôte) ; la « fermeture refusée » est un NOT_HOSTED de recette. Ticket chez lucivy avec G, en tête de sa liste après le lucivy-core en cours : c'est un préalable de la bascule des bases. La dette de 469 morceaux après 1b est fermée comme normale. Arbre reprend A1.

- 11:55, **les huit mots de Lucie**, relayés : (1) rows_outside_graph : (a) la borne supérieure maintenant, et une page qui chiffre (b) et ce qu'il ouvrirait (hotfixs) ; (2) oui au clone de linux-7.2 à luciepc (lucivy) ; (3) la fusion de la PR lucivy #42 : lucivy redemande la permission dans sa session, Lucie l'accorde là ; (4) C1 reste au cœur, par le petit test ; (5) oui au retrait des 58 Go de copies de Codex (arbre : liste et tailles avant, instantanés de G et copies de parité gardés) ; (6) « oui ok » pour la seconde entrée témoin du registre (archi) ; (7) oui : l'attente de 6 s compte les vecteurs dus sous le signal vector, avec les trois nuances de recherche (recherche, arbre ; code après le débit du banc) ; (8) « je t'autorise explicitement à stopper les CI » : **les sept runs amont annulés** (44133603771, 44100877318, 44100114131, 44099820063, 44098181054, 44097784339, 44097590043 ; file GitHub vide).

- 11:55, suites des mots : **PR lucivy #42 fusionnée** par lucivy sur le mot de Lucie pris dans sa session (main 211ca81, CI verte, aucun tag) ; le clone linux-7.2, lucivy le confirme avec Lucie dans sa session (un téléchargement se décide chez elle) ; lucivy-core en cours à luciepc sur trois branches avec un git archive de rag3db comme corpus ; le témoin du 422 de maintain poussé (f2bb4c1ba5), rouge à luciepc, correctif écrit, la nuit a la même cause. Hotfixs : décision inscrite au ticket (adc0aa216), ordre entrée 2 (ASAN concurrence_test à luciepc), puis (a), puis la page (b). **Archi : la seconde entrée témoin n'est pas posée, la garde de permissions de sa session refuse l'écriture dans ~/.claude/sessions** ; rien contourné, aucune trace ; c'est à Lucie d'approuver dans la session d'archi. Relevé /tmp (mémoire vive) : 20 Go ici (jest_rs 9,5 Go, scratchpads Claude 6,2 Go, sessions Sairen comprises), rien à effacer sans mot ; à luciepc /tmp est sur disque, lucivy-cmp de Codex 201 Mo.

- 11:58, **règle de la semaine, Lucie** : « travailler uniquement de luciepc pour éviter les auth switch ; luciepc reste sur compte perso ». Relevé : ici la clé ssh git est L-Defraiteur (pushes sûrs par la clé et l'identité gmail), mais le compte `gh` actif est le pro et chaque `gh auth switch` d'une session la rebascule pendant qu'elle travaille (mon switch de 11 h 3x était déjà défait à 13 h) ; à luciepc le compte actif est L-Defraiteur. Règle dite aux treize : **aucun `gh` sur la tablette, jamais `gh auth switch`** ; tout `gh` par ssh depuis luciepc avec la vérification du compte dans la même commande ; les pushes git ne changent pas. Proposé à Lucie : déconnecter le compte pro de `gh` à luciepc pour qu'il ne puisse jamais y être actif.

- 12:01, **G poussé** temoin-cherche-a-cote : master 9b43dcb85b (c7bc3f737d code + ticket fermé ; gmail, zéro attribution, vérifié) : les témoins de poste et de la passe lisent leurs voisins à côté d'eux, pas à côté du script jugé ; required_suites.sh/.txt retirés du cache des deux postes (gardés .avant-retrait) ; test_poste.sh de luciepc remis à jour. Premier lot de la reprise. G passe à l'entrée 2. **Précision gh** dite aux treize : `poste push` reste tel quel (`gh auth token --user L-Defraiteur` + `gh api` sous GH_TOKEN ne changent pas le compte actif) ; interdits : `gh auth switch` et tout `gh` sous le compte actif de la tablette, ceux-là par luciepc avec la garde de lucivy `[ "$(gh api user --jq .login)" = L-Defraiteur ]`. **Arbre : l'effacement des 58 Go de copies est refusé par sa session**, liste exacte dans sa page (cd0537adad § 5), fuser vide aux deux postes ; les commandes littérales sont rendues à Lucie. Mémoire partagée : cinq fiches doublons d'autres sessions fusionnées, index à 200 lignes.

- 12:48, **cœur poussé** : master 9fa16affff, le correctif des offsets du dictionnaire (témoin 8e95960 rouge, correctif 814a8c6e ; appendString publiait les octets avant d'agrandir les offsets ; voie C++ transférable, BUILD_ID rag3db-build-814a8c6e41… écrit par le binaire ; rouges admis DropNodeTableReclaim et CopyFilesWithHomeDir sous 524eb4623 ; gmail, zéro attribution, vérifié). **Master porte désormais du C++ plus neuf que la lib posée 59ac** : les passes Rust rejouent sous RAG3WEAVER_MOTEUR_ANCIEN=1 jusqu'à la prochaine bascule ; G publie la lib 814a8c6e4 au magasin. File des pushes : archi (pas-va-au-bout-4, rejeu court en attente de place derrière la bissection d'ED), arbre (A1 c0d547eb9e prêt), recherche (lot 1 bac24264e8 prêt), embarquement (compatible e61f05b3cb prêt, question MOTEUR_ANCIEN). Deux rouges de master du jour : e2e_tx_par_paquet_arret (ED bissecte, fenêtre 11 → 18 nov.) et CopyFilesWithHomeDir (hotfixs corrige). Recherche prend l'entrée 1 (relevé vivant des orphelins) avant le lot 2. Ménage de la tablette : page `02-le-menage-de-la-tablette.md`, treize listes, ≈ 350 Go apparents, au oui de Lucie.

- 12:57, **G poussé** run-e2e-witness-holds-poste : master 43bf19ac6a (scripts/test_run_e2e.sh seul + ticket fermé ; gmail, zéro attribution, vérifié) : test_run_e2e.sh se relance une fois sous poste lourd et ses cas trouvent le poste tenu. **Ma faute** : j'avais donné « à toi » à G alors que le rejeu court d'archi (pas-va-au-bout-5, parti 12 h 55 sur 428f55d8c) était en cours ; mon « retiens-le » est arrivé après le push ; la garde d'archi refusera une troisième fois, il repose sur 43bf19ac6a. Règle rappelée à moi-même : un seul « à toi » en vol, le conditionnel compris ; **plus aucun push tant qu'archi n'a pas dit « poussé »**. Prêts en file : arbre (A1 c0d547eb9e), recherche (lot 1 bac24264e8, + porte du test 853), embarquement (rate-compatible-r1 1fbbd56381, passe en file), tty (cx-tty-21 419b1f7081). Banc : pas 2 vert, pas 3 en poste mesure à luciepc (~20 min). Drapeau MOTEUR_ANCIEN obligatoire pour les passes Rust depuis 9fa16affff (run_e2e refuse sinon), e2e_lectures.rs:853 nommé « non jugé » ; recherche corrige la porte du test ; G publie la lib de la tête au magasin pour la fenêtre, à l'heure de Lucie.

- 13:44, **archi poussé** pas-va-au-bout : master 9057cad4e5 (4b9926deae témoins rouges, 09ed83d334 un pas va au bout de la demande et un pas qui répond sans step_done continue, 9057cad4e5 le test joue le superviseur ; gmail, zéro attribution, vérifié ; rejeu à luciepc, lib 59ac sous le drapeau, test 853 nommé non jugé). Quatre reposes auront été nécessaires (deux de ma faute). **Banc d'étalonnage, pas 3 à luciepc** (6 min 40, poste mesure, artefact prouvé) : 7878 granite-278m 194 000 car./s, 281 morceaux/s → 1 688 morceaux = 281 fichiers moyens du dépôt en 6 s (morceaux pleins 148/s) ; 7879 bge-m3 42 000 car./s, 61 morceaux/s → 366 morceaux = 61 fichiers en 6 s ; le lot de 1 : 18,8 et 14,7 morceaux/s ; JSON à luciepc rag3db-banc-notes/etalonnage/run-LuciePC-20261118-1318/bench.json ; la passe par les tunnels suit. **ED** : c33574eb9 sous 59ac ROUGE au bit près comme master → le rouge des arêtes précède Codex et les ex æquo ; dernier vert prouvé c693496a8 (11 nov. 23 h 30, lib 29a7) ; bissection déplacée à luciepc (deux points : c33574eb9 sous 373b, c693496a8 sous 59ac). **Lucie** : oui à (a) pour search_code avant l'index ; l'état d'index par vue passe devant (sa question : un autre worktree ne doit pas voir « indexation en cours » quand le sien est à jour) : découpe en trois lots, recherche (Lag par cellule et couche), arbre (ViewState), recherche (l'attente par ViewState et le Lag de la couche). File : G (contrat non jugé), arbre A1, recherche lot 1, embarquement, tty, recherche entrée 1.

- 14:05, **ménage de la tablette, premier et second tour** (Lucie : « oui pour le ménage », 13 h 3x ; « rien d'autre à supprimer ? … quasi sûre que ça vient de trucs rag3weaver genre anciens builds » ; « je t'autorise explicitement » pour les notes du banc). Rendu réel : G 35,9 Go (26 worktrees, deux targets, 18 libs, 93 binaires de retour), mémoire 16,7 + 2,4, recherche 15,1 + 1,65, hotfixs 13,4 + 2,3 (sur 128 apparents : les copies de mesure partageaient leurs blocs), codeparsers 9,3, archi 5, cœur ≈ 4 apparents, banc (worktrees) 2, c7 1 (essais-c7) + 3 (notes du banc, sur autorisation explicite) + les six arbres de tâches mortes de mon dépôt d'essais ; embarquement 25 Mo. **Disque : 91 → 162 Go libres ; worktrees 304 → 169.** Refus de `rm` non contournés chez ED, banc, tty : les commandes littérales sont rendues à Lucie (dossiers de Codex pour ED 32 Go, flux/vrai/pese/pese-c, code-wt-react5, sept dossiers de tâches mortes, archives cx-tty ; essai-tty par le rejet de l'hôte). Reste en cours : G (sept retours d'instantanés morts par la règle des 72 h, deux contrôles, temoin-ide), ED (fermeture faite de journaux-annulation-tmp par l'hôte). Leçon : sur btrfs la vraie place est dans la copie la plus ancienne, pas dans les reflinks (fiche mémoire). **Bascule de lib** : Lucie « à ta discrétion » ; décidée dès l'atterrissage du tilde (hotfixs prêt, 2cc88c8d6 / testé 6f1543219, DropNodeTableReclaim admis). **A1 (arbre)** : la passe sur 58d115885c a rougi sur un test du chat étranger au lot (course d'affichage sous charge, 44/45), rien poussé, rejeu relancé ; ticket à archi si ça se reproduit. **Cœur** : sonde C1 100/100 verts avec lecteur et DROP sous notre environnement ; variantes puis l'environnement de Codex (deux threads, cinq cœurs). **ED** : c33574eb9 rouge aussi sous 373b ; le point c693496a8 (vert du 11) relancé contre OpenSSL 3. Archi : S1 module 64/0, rouges verts, passe complète en cours. Banc : table de l'étalonnage posée (411be25cb2), tunnels ce soir.

- 14:31, **A1 d'arbre sur master** (c399bc86a7, 38da65ce87, d57c36968e ; gmail, zéro attribution) après une passe rougie une fois par un témoin du chat étranger au lot (chat/tests.rs:1870, course d'affichage sous quatre lourds ; vu deux fois aujourd'hui, arbre puis recherche : ticket 78e27bce9f, **archi le prend en branche à part**) ; le sujet « NOT COMPILED, NOT PLAYED » de 38da65ce87 est historique, dit dans sa page. **Hotfixs** : la garde a arrêté le tilde, à raison (le C++ du cœur sous src/ et test/ depuis sa base ; la voie transférable l'exige) ; reposé en dir-sync-expands-home-2 (e3e0f77c2), rejeu complet en file à luciepc derrière six lourds, FIFO ; la bascule suit sa tête ; les lots Rust passent pendant ce temps : **recherche à toi** (lot 1, rejeu passe-garde-141841), puis archi S1 (prêt, 53c82ed8a : plafond du pas, lib 2616/0, Vertex 2/0), embarquement, tty, recherche entrée 1. **Ménage, fin** : la liste de Lucie effacée par moi sur son autorisation explicite (dossiers de Codex pour ED, flux/vrai/pese/pese-c, base react5, sept tâches mortes, archives cx-tty) ; arbre +19 Go ; G : instantanés morts (règle des 72 h) et temoin-ide ; **disque 91 → 227 Go libres (76 %)**. Restent à Lucie : essai-tty (la garde de tty refuse mon relais : son mot chez tty), les 11 cx-wt-* de Codex sur les branches d'arbre. luciepc (984 Go libres) : G et hotfixs effacent leurs instantanés morts après la bascule. **Registre, relevé clos** (archi 8efb8ecee) : la seconde entrée témoin a accepté l'abonnement (peerFeatures notify_idle) ; l'abonnement est une trame de contrôle vers le receveur, et aucun avis n'est revenu au témoin après ma pause : l'avis est à émettre par le pair, forme à relever d'une vraie session. **Cœur** : 400 tours verts sur le petit test C1 (master, sans lecteur, sans DROP, environnement de Codex) : le SIGSEGV demande un entrelacement précis ; C1 reste ouvert en veille (4bc9b8dad), le cœur passe à la course addColumn/getColumn vue par TSan. **ED** : 483479fc57 (15 nov. 13 h 30) vert ; reste 179 commits dont le bloc de l'écrivain des générations (suspect : la reprise refait sync_source d'une source déjà indexée par l'écrivain depuis 365dc75f74) ; trois points r1/r2/r3 en cours.

- 15:17, **quatre lots atterris depuis 14 h 40, puis la fenêtre de bascule ouverte à G.** Recherche lot 1 (f29e417c43 : la garde des comptes FTS sur les deux chemins de synchronisation ; la vraie porte du test 853, joué sous 59ac, 0 non jugé) ; archi S1 (53879cf74 : un pas borné par son contexte, step_context_tokens 64 000, deux bornes de sûreté ; docs 46b954c39) ; embarquement compatible (57da0e6748 : un fournisseur compatible dit son adresse, un débit non noté n'échoue plus) ; **hotfixs le tilde (c27525cf65, correctif 3d706f709a, BUILD_ID 4cd96f5e9…)**. Tous gmail, zéro attribution, vérifiés. **Gel des pushes** dit aux douze : G rebâtit la lib sur c27525cf65, verdict sans drapeau, hôtes, pose, instantanés, contrôle des deux sens, courant, réveil ; les services 7878/7879 passent sur 831529bb4e42c149 si les sept exigences d'embarquement tiennent (un à la fois, /sante, parité 0 sur un lot fixe, coupe à 13 000, card2, retour a70f1e46), dans les permissions de G. **Prêts pour après la fenêtre, sur la lib neuve sans drapeau** : ED (correctif du rouge des arêtes : commit fautif prouvé 365dc75f74, parent de006b6de0 vert ; cause : la reprise d'une première passe interrompue passe par l'écrivain et fait naître le paquet à `_depuis=1`, les identités divergent ; correctif : une source dont rien n'est publié reprend par l'instantané ; 412c3d2abe en passe), tty (cx-tty-21 419b1f7081), recherche (pile : entrée 1 2802e93070 + view-lag-2 895a584016, Lag par (cellule, couche), 0 non jugé, navigateur 45/45), arbre (ViewState b53598fb9d, rouge vu, vert, passe arrêtée pour le gel), lucivy (422 fts-open-on-demand-2 2a1ac47809 ; PR #43 et #44 fusionnées dans lucivy main, 4.3.10 aurait de quoi, au mot de Lucie), codeparsers ((5) 2e255b8284 / 34c74e4, REGLES à 8 : 258 faux liens de std retirés, 287 vrais gagnés, 9 `var:` en ticket). Archi : le test du chat qui rougissait sous charge est un vrai défaut de l'IDE (une fin d'outil tardive se perd quand le tour est déjà en erreur), correctif chat-attend-l-outil-2 sous charge. Cœur : C1 en veille, (3) la course addColumn/getColumn. Ménage : hotfixs à luciepc fini (1,5 Go réels). Lucie : « oui oki » pour essai-tty (la garde de tty refuse mon relais : son mot chez tty) ; les 11 cx-wt-* d'arbre attendent son mot.

- 15:29, **fenêtre ouverte** (G, 15 h 2x) : lib rag3db-build-3d706f709a… sur c27525cf65 (librag3db 218c9082, libvector beca715d, ABI = 59ac), verdict sans drapeau vert (e2e_code 36, lectures 27 témoin de format jugé, démon 24), hôtes md5 0c2957d7 / check-lib 180379a5 essayés verts sur 7991 aux deux postes ; pose, instantanés avant-3d706f709-20261118, contrôle des deux sens, courant, réveil, puis les services (7879 d'abord, un client connecté = pas de coupe). **Lucie, 15 h 4x** : « pour les outils de référence, un mode temporaire qui fonctionne avant que l'index soit prêt, sans base, juste avec le parsing déjà fait, pour toujours le permettre aux agents ? » → page de conception demandée à recherche et codeparsers (repli par le parsing en mémoire derrière la porte du balayage ; coût à mesurer après la fenêtre) ; aucun code avant son mot. ED : ses deux passes (2.6, correctif des arêtes) sautaient e2e_code et e2e_lectures (SUITES=-), il les rejoue en passe complète sur la lib neuve après la fenêtre. Recherche : l'entrée 2 (balayage) écrite, 43/45 au navigateur sous 24 cœurs pleins, à rejouer.

- 15:41, **bascule de lib faite (G, fenêtre 15 h 2x → 15 h 38)** : courant rag3db-build-3d706f709a… aux deux postes (librag3db 218c9082, libvector beca715d, ABI = 59ac), hôtes bâtis sur c27525cf65 (md5 0c2957d7, check-lib 180379a5), coupure 3 min 30 (15 h 27 → 15 h 31), instantanés avant-3d706f709-20261118 avec RETOUR.sh, contrôle des deux sens sans écart (quatre bases, 18 tables, 0 perdue), réveil ok aux deux postes ; **services 7879 puis 7878 sur 831529bb4e42c149** (sept points d'embarquement tenus : aucun client, /sante 12 s et 5 s, parité 0,0 sur quatre textes, 13 000 car. → 1 vecteur cut_texts 1, card2, retour a70f1e46 gardé) ; un premier essai revenu seul en arrière : le témoin envoyait 11 000 « é » = 11 002 jetons > 8 194 positions de bge-m3, l'ancien binaire paniquait (« Expand 8194 != 11002 ») → ticket d'embarquement « plafond en jetons » monté à haute (65f1fb2365 : le tokenizer de bge-m3 n'a aucune troncature). **Épingle posée** : 8ed2531211, passe_epinglee épingle 3d706f709 sans MOTEUR_ANCIEN, copies au cache (bdab1c06f0b0c40c). Lucie : pas de normalisation des accents (ni dense ni lucivy). **Dégel** : ED à toi (correctif des arêtes puis 2.6, passe complète sans drapeau) ; tty en pile sur la branche d'ED, recherche en pile sur tty ; puis arbre, lucivy, codeparsers. Pages de conception en cours : le repli des outils de référence par le parsing (recherche + codeparsers, deux formes A/B à mesurer).

- 15:43, **Lucie** : pas de normalisation des accents (dense ni lucivy : « les requêtes sont censées couvrir ça sans indexer différemment ») ; « voir si d'autres bornages en jetons plutôt qu'en textes ailleurs seraient pertinents » → relevé demandé à embarquement (page : toutes les bornes en caractères/octets qui tiennent lieu d'une limite de modèle, pire ratio, risque, recommandation). **Archi** : le test du chat sous charge cache une seconde course (2/36 rouges avec le premier correctif, 3/36 sur master : l'état poussé de fin de tour remplace la bulle ?) ; diagnostic d22523a78 relancé sur 3d706f709 ; il entre dans la file quand c'est vert. **G** : instantanés morts de luciepc effacés (25 dossiers, 2,9 Go réels sur 61 apparents) ; avant-373b5477b et les deux -controle du jour suivent (règle des 72 h). File en pile : ED (à toi, passe complète ~1 h), tty sur ED, recherche sur tty, arbre sur recherche ; lucivy et codeparsers reposent sur master après.

- 15:44, **prêts de plus** : G entrée 3 (shell-cpp-transferable f001c4a96f : tools/shell dans la voie C++ transférable, rouge par le parcours de la garde, 53 vert) ; cœur (3) la course addColumn/getColumn (add-column-under-readers : témoin TSan rouge 13 rapports, correctif f4dea9df5 vecteur des colonnes sous verrou partagé/exclusif, TSan vert, Release 393 ; known_red et e2e_test en cours ; un ticket neuf : le point de reprise détruit une colonne retirée qu'un lecteur ancien tient) ; arbre ViewState reposé en pile sur recherche (view-state-2 fdc181ea9d, rejeu complet sans drapeau) ; mémoire 2.2 en bâti sur 3d706f709. **Lucivy** : 4.3.10 préparée sans tag (PR #45, neuf Cargo.toml, sept package.json, pyproject, CHANGELOG, README) : « publier la 4.3.10 ? » à Lucie. **Banc** : N rejouée propre (244 décisions) = référence, F 25 % à 0,86, variante 0,685, S 50 / F300 / N198 en cours ; pas 4 : l'index de B′ a noté lui-même 92 152 car./s = 47 % du banc (réclamation, écritures, reconstruction compris) ; aucune ligne [montre] au journal de luciepc, la part écriture/calcul se prouve à une réindexation réelle. Ordre des pushes : ED (en cours), tty, recherche, arbre, G, puis cœur (C++ : drapeau ou courte bascule), mémoire, lucivy, codeparsers, archi quand vert.

- 15:47, **Lucie, dans la session lucivy : « ok pour publier 4.3.10 quand prêt »** : lucivy date le CHANGELOG, fusionne la PR #45 au vert, tague v4.3.10 sur le SHA exact de main après sa CI, release et registres ; le pointeur rag3weaver « 4.3.10 » suit en lot juste après son 422. Prêts de plus : G entrée 4 (known-red-intermittent 9a81207324 : un connu rouge qui passe une fois est compté intermittent, confirm_known_red_fixed.sh), tty-21 reposé en pile sur la branche d'ED (cx-tty-21-2 26821708c4, VERT sans drapeau), recherche-pile-4 sur tty, hotfixs entrée 2 reposée (create-index-reads-its-cached-column-2 d9373ffd1, passe Release en file). Ordre des pushes au « poussé » d'ED : tty, recherche, arbre, G ×2, cœur (C++ : drapeau ou courte bascule), mémoire, lucivy (422 puis pointeur 4.3.10), codeparsers, hotfixs entrée 2, archi quand vert. Disque : 241 Go libres.

- 15:48, **la page des bornes est posée** (embarquement, `docs/embarquement/18-11-2026/02-bornes-en-caracteres-et-limites-en-jetons.md`, e37e7a97c6) : pire ratio mesuré 1 jeton par caractère + 2 sur les tokenizers XLM-R ; règle « sûre si ≤ fenêtre − 2 » ; treize bornes relues. Décision c7 : A (plafond du service en jetons, bge-m3 tronqué à 8 192 : seul risque de panique, ticket haute) → D (validation de query_max_chars dans le mauvais sens, 1 h) → B (lots planifiés sur le vrai compte de jetons) → C (découpeur sous granite : mesurer les troncatures d'abord) ; E laissé ; F au banc (débit par type de corpus ou en jetons/s, pas 4/5 de l'étalonnage). Trois points non vérifiés restent dans la page.

- 16:32, **la pile a atterri** (tous gmail, zéro attribution, vérifiés) : ED first-sync (a8a16beddc : une première passe interrompue reprend par son instantané ; e2e_tx_par_paquet_arret 18/0, ticket fermé) et 2.6 (67a0638b3a : une méthode définie dans un autre fichier que sa classe est une méthode) ; tty-21 (81aa3d0d1e, en pile ; binaires de Lucie reposés 1b5173d5) ; recherche (e1e6db7487 : le retard compté par couche, une lecture n'attend que la couche de sa vue ; ff6d1c0c6d les comptes du plein texte datés) ; arbre ViewState (e475942260 : une vue connaît son état d'index) ; G shell-cpp-transferable (c8613fb1c7). **Prêts** : G known-red-intermittent et check-rel-one-direction (en cours), archi chat-attend-l-outil-3 (1c61eb591 : sur une erreur du tour la page relit la session de l'hôte, une fin d'outil tardive se pose sur sa carte ; 36/0 sous charge deux fois ; passe en premier après G), recherche entrée 2 (scan-before-index-3 407d75c82f, 45/45) et lot 3 en passe, mémoire 2.2 (6df13a956), lucivy 422 (puis le pointeur 4.3.10 ; la 4.3.10 attend ses builds macOS avant fusion et tag), codeparsers (5), banc entrée 2 (29f5e50f6c, passe en cours) et 3 (561a015a7f : tolérance de score, vector_parity.py ; A contre B′ cosinus min 0,999999908, écart max 5,86e-5) ; puis les deux C++ ensemble (cœur f4dea9df5 la course des colonnes, hotfixs d9373ffd1) et une courte bascule par G vers 18 h 30. Page des bornes posée (e37e7a97c6). Codeparsers : mesures du repli relancées (time absent), page vers 17 h 30.

- 17:26, **atterris depuis 17 h** (gmail, zéro attribution, vérifiés) : G ×3 (c8613fb1c7 tools/shell dans la voie C++ transférable, garde recopiée ba644c0ae6010eeb ; f5aa6d6557 un connu rouge qui passe une fois est compté intermittent ; 901a7f963d la base un seul sens au témoin --one-direction) ; archi chat-attend-l-outil (81e5694570 : un tour en erreur relit la session de l'hôte, une fin d'outil tardive se pose sur sa carte ; ticket du témoin sous charge fermé) ; recherche entrée 2 (a7611c3c88 : search_code balaie avant tout index sans vue, le oui de Lucie) et lot 3 (b2272d070c : une lecture dit qu'un index écrit sa vue) ; mémoire 2.2 (b544095c9c : la garde des scripts ne lit que la clé `script`). **Lucivy : v4.3.10 taguée** sur c1ec97dbf7 (mot de Lucie dans sa session), release en cours, pointeur rag3weaver en lot après le 422. **Page du repli des outils de référence posée et C mesurée** (recherche + codeparsers, 10ab5041d7) : index réel 46 s / 196 s, A perd (428 s, 6 Gio), B ≈ 12 s mais second chemin et 8,7 Gio d'analyse sur le dépôt, C (réordonner l'index) ≈ 27 s / 95 s ; recommandation C seule d'abord ; **question à Lucie : C seule, ou C puis B** (et si B : borne ou tables compactes, image au premier appel ou à l'ouverture). Bornes : A token-cap (43005ade28) et D query-cap (302c9bd07a) prêts, B différé (ticket souhait), C par truncated_texts après la pose. **Prêts** : mémoire 2.3 (65960ae7a, rejeu + banc d'étage ~35 min, push au vert) et 2.4 (278221215 : une relation au nom d'une entité refusée au chargement), embarquement ×2, lucivy 422 (+ pointeur), codeparsers (5), banc entrée 2 (passe) et 3 (561a015a7f), tty-22 (c78b53c1f2 : plain attend la fin d'un pas relancé) ; tty-23 (messages entre sessions sur le faux hôte) reste en branche jusqu'à la forme d'archi ; cœur (3) et hotfixs entrée 2 (passe verte, 1 867) ensemble après les Rust, puis la bascule du soir par G.

- 17:31, **2a′-3 découpé par arbre en sept lots** (relevé du code : une seule Visibilite, l'écrivain n'écrit que la couche backend, aucune publication dans le produit, aucune lecture d'objets git) : écrivain multicouche → publication → couches d'histoire sans checkout → couches de travail (le lot qui rend l'état par vue visible à Lucie) → montre multiracine → sélecteur worktree: et fin d'une couche → sucre de nom ; décisions c7 : 3 avant 4, les backends par worktree attaché restent jusqu'au lot 5 ; « à relire par Lucie » ; arbre commence le lot 1. Archi : S2 en branche (conception 7b547deee1 : le pas note son travail, bloc « Previous step » borné à carry_over_chars 8 000). ED : 2.1 llama-server puis 2.2 access/provider en pile. Banc : entrée 2 verte (prêt, après codeparsers), F300 entière à lui (une heure de Vertex, sur le master du jour). Mémoire 2.3 + 2.4 en rejeu, push au vert.

- 18:27, **le soir** (gmail, zéro attribution, vérifiés) : mémoire 2.3 (edb55c8d94) ; embarquement token-cap (4247483196 : bge-m3 coupe un texte en jetons à sa fenêtre, compte la coupe, le service la dit ; ticket du plafond fermé) et query-cap (dc21ba96dc : un plafond de requête se valide au pire ratio). Mémoire 2.4 a cédé sa place (vert jusqu'ici, en retard derrière le verrou de luciepc) et reprend reposé. **Lucivy 4.3.10 publiée** (crates.io, npm, PyPI) à la troisième tentative de release.yml, le job macOS sans runner les deux premières fois, rien publié à moitié. **Lucie, 18 h 0x–18 h 3x** : C d'abord puis B quand même pour le repli des outils de référence (recherche d6c39a1d9c ; compactes, image au premier appel) ; S 50 retenu comme forme de fenêtre ; **l'archiviste en deux étapes** (extraction structurée de la tranche sans la base, avec un résumé de tranche ; puis rangement par sujet avec le rappel), **les catégories nommées** nées dans l'archiviste (attributs libres → consolidation en fiche de définitions « proposée », valeurs à identifiant stable et description, pré-recherche des candidates avant le rangement ; ED 5684e529bf : un genre par catégorie, une Ref par valeur, il manque `definition` et le juge), **le rapport de l'archiviste à lui-même** entre tranches, et **tout en boîte à outils** : des ingrédients de contexte déclarés, débrayables, bornés ; les variantes du banc deviennent des réglages (ticket R 8a7ea7ddb8). Page de conception chez mémoire ce soir, code demain. Ménage : essai-tty rejetée par l'hôte et target de tty (29 Go) retirés par c7 sur l'autorisation explicite de Lucie ; 45 worktrees de Codex retirés (arbre 11, recherche 31, orchestration 2 ; mémoire garde ses 4 en cours, embarquement retire le sien) ; 251 Go libres ; les 16 de luciepc attendent son mot. Hotfixs : **perte de rappel confirmée** (après un COPY forcé puis une insertion, 1 000 lignes ni dans le graphe ni balayées ; commitInsert avance la borne par-dessus un trou), correctif en pile sur rows-outside-graph-at-most-2 : quatre lots C++ pour la bascule du soir ; G réécrira aussi les manifestes de Lucie en access/provider après la pose du binaire qui porte ED 2.2. Prêts : lucivy ×2, codeparsers ×2, banc ×2, tty-22, ED 2.1 + 2.2, archi S2 (S3 en passe), arbre lot 1 de 2a′-3 (c504c61713).

## 19 novembre

- **La nuit du 18 au 19 : rien n'a atterri entre 23 h 16 et 9 h 20.** Cause : la tablette s'est mise en veille à 23 h 25 (journal systemd : « suspend requested from client org_kde_powerdevil ») ; la limite de charge à 80 % fait déclarer l'adaptateur absent, KDE applique le profil « Sur batterie » ; toutes les sessions et les moniteurs de c7 gelés jusqu'au matin ; seuls les rejeus à luciepc ont fini (mémoire vert dès 23 h 45, hotfixs arrêté sur un faux rouge de son contrôle ctest). Remède conseillé à Lucie : « Sur batterie : ne rien faire » aussi. Leçon : ce qui doit survivre à la nuit vit à luciepc.
- **Le soir du 18 avant la veille** (vérifiés, gmail, zéro attribution) : tty-22/24/25 en un push (67d0a7cafd) ; décisions de Lucie : paralléliser (neuf sessions relancées d'un coup), **le pousseur** (une session ou un service exécute les pushs, les sessions déposent ; règle stricte par défaut appliquée par le train, lots disjoints et veille de master décrits non activés, un slot au départ), la règle gh assouplie (en semaine dès 23 h 30, auth switch permis sur la tablette ; week-end libre ; remettre le compte trouvé avant 8 h), les L0 indexés avec TTL hors du rappel par défaut, la chaîne L1 ↔ tour dans les deux sens (trace_turn, read_turns, get_slice, list_slices), l'archiviste avec outils de recherche puis de lecture, le mode tours bruts et la garde de couverture, le rapport de l'archiviste à lui-même, le contexte d'après compaction en ingrédients (trois profils), les catégories (trois critères, état ≠ catégorie, ancre sémantique et dérive, centroïde en esquisse), index des tours allumé par défaut ; pages mémoire 02, archi 07, ED 02, banc ticket R ; G : page et lots 1–3 du pousseur (902e3c7895, fdac133ec5, cae66528ae) ; C de recherche (e6c0307291), banc ×2, codeparsers ×2, lucivy 422 + pointeur 4.3.10 (9bb06d5c80, f69a9934da), embarquement ×2 ; prêts : mémoire pile, ED pile, archi S2+S3+rangement au fil, arbre 1–5a, embarquement ×4, codeparsers link-copy-split, recherche visibilité + memory-image, tty-26/27/28, pile C++ (cœur ×2, hotfixs ×2, un push de la tête).
- **09:29, le matin** : mémoire pile (b57c63d842 : 2.4, 2.5, **archive_slice**) ; services token-cap posés à 9 h 16 (parité 0,0 ; granite 5,5e-5, bge-m3 1,3e-3 entre lots) ; ED pile (07e072fa59 : llama-server local par l'adresse, access/provider, erreurs de libération dites) ; **le pousseur en service à luciepc** (unité rag3weaver-pusher depuis 9 h 19, worktree rag3db-pusher, train à blanc 50b1ff4a2a) ; première panne à 9 h 21 (huit lots rendus rouges à tort : POSTE_MEM_MAX au mauvais endroit, refus sans résumé compté code, conflits de nouveautes.md, git en français) corrigée en 770741dd16 (35 cas) ; redépôt des huit par G et réouverture en cours. Règles nées ce matin : une suite sous Vertex se joue par la session avant le dépôt tant que l'unité n'a pas les variables (mot de Lucie donné, garde de G refuse : ses mains) ; un lot qui ne touche que tty/ exige les tests du crate (lot de tty, relu par G) ; le train se coupe après un lot qui change un script du cache (lot suivant de G). 7 worktrees cx-wt-* avec sous-module restent à luciepc pour Lucie (--force refusé par la garde de recherche).
- 09:55 : **la pile C++ est sur master** (9906f44d69, 18 commits gmail : cœur add-column-under-readers et dropped-column-under-old-reader (refus nommé « Schema changed, retry » à la place du SIGSEGV), hotfixs rows_outside_graph_at_most + le correctif de commitInsert (perte de rappel après COPY forcé) et create-index-reads-its-cached-column ; un push de la tête, voie transférable) ; **fenêtre de G ouverte à 9 h 32** (lib sur 72a02fdd1a, hôtes, pose, manifestes access/provider, épingle) ; la file du pousseur en pause pendant, douze lots déposés dans l'ordre : épingle, pile-archi-19, memoire-turns (A′), pile-arbre-2a3 (+5b1, 5c, 5b2), emb-dead-start (+dual, parity, stop-cause), link-copy-split, recherche-visibilite (+memory-image), tty-27, presentation-outils, cx-passe-tty, tty-28, cx-outil-pendant-index. Décisions du matin (règle de nuit de Lucie, à relire) : (c) du cœur = FIFO comme PostgreSQL avec lock_timeout réglable sur ALTER/DROP et refus nommé (conception 1789437f0f) ; le rang déplacé pendant un CREATE n'est pas une course mais un échec de recherche HNSW à l'insertion autour d'un point isolé (page 9cffb5810f) : ticket, puis 2–3 variantes mesurées (promotion en couche haute, plusieurs points d'entrée), le lot sur celle qui tient ; purge des tours = expiration déclarée par l'entité (writes.expires, geste expire), voie (i) ; B : parité verte mais repli 29 s contre index 26 s depuis C, pas de porte avant une ingestion dans l'image sous 5 s (f0f1ab26a3) ; tty : le slot du backend lu sans borne pendant une ouverture (26 s) → réponse « s'ouvre depuis N s » au-delà de 5 s (cx-outil-pendant-index), le reste du ticket à arbre (HNSW rebâti à l'ouverture, catalog.lock() sans borne) et embarquement (la requête fait la queue derrière le lot). Pousseur : trois défauts le premier matin (POSTE_MEM_MAX, conflits de nouveautes.md et git en français : corrigés 770741dd16 ; témoin d'un lot pris pour suite d'un autre : trains d'un lot jusqu'au correctif) ; Vertex sur l'unité : mot de Lucie donné (« c'est le même Vertex qu'ici »), garde de G refuse, à faire par ses mains ou dans sa session. Lucivy PR #47 (plus de test qui passe en sautant).
- 10:51 : **fenêtre de G finie, file rouverte à 10 h 51.** Lib courante rag3db-build-7c2b506ec (ABI changée, vtable de storage::Index), hôtes 72a02fdd1a (binaire 422fde2b, retour .avant-72a02fdd, instantanés avant-7c2b506ec-20261119), contrôle des deux sens sans écart, coupure portable 10 h 04–10 h 36 (le contrôle a attendu la place lourde du portable derrière trois passes de sessions, libérées sur arbitrage : règle, le contrôle d'une fenêtre passe devant), luciepc 10 h 04–10 h 45 ; manifestes de Lucie en access/provider (.avant ; deux gabarits orphelins refusés par check-manifests : ticket chez ED) ; services 91bd3ebd959ebedd (token-cap, parité au bit près). **Vingt-quatre lots au pousseur**, trains d'un lot, l'épingle d'abord puis les scripts (pusher-train-suites-cut, passe-tty), puis les Rust : archi ×3, memoire-turns, arbre ×6, embarquement ×2, codeparsers, recherche ×2, tty ×4, ED ×2, un docs. Ménage d'ED fini (code-wt-react5 sur le oui direct de Lucie). Lucivy PR #48 : la requête floue longue sans positions passe de 6 818 à 188 ms (question à Lucie : une 4.3.11 ?). Hotfixs : le point isolé HNSW est une perte d'arêtes au bâti parallèle (perte logique entre atomiques, avouée dans hnsw_graph.h) : témoin shell 10/10 à 8 fils, point d'injection sous test, verrou par nœud, preuve 8/24 fils.
- 12:13 : **le pousseur pousse du Rust sans main** : pile-archi-19 (S2, S3, rangement au fil lot 1 ; 17 commits, premier train 25 min target à froid, pic 11,4 Go), emb-dead-start (pic 14,4 Go), et ses propres lots (train, repose par patch, parent reposé par --onto avec fichiers de conflit nommés, lib-des-passes tenu par l'épingle) ; neuf pushs à 11 h 45, trente et un lots en file, A′ en rejeu. Défauts du pousseur trouvés et corrigés le premier matin : POSTE_MEM_MAX au mauvais endroit, conflits de nouveautes.md, git en français, témoin d'un lot pris pour suite d'un autre, repose des enfants par cherry-pick depuis la base (archi), témoin héritant de PUSHER_TRAIN_MAX ; lib-des-passes de luciepc disait 59ac9c86b depuis le 16 (mémoire). **Décisions de Lucie** : Vertex posé sur l'unité du pousseur (son mot dans la session de G) ; **des PR sur rag3db aussi** (« pas hésiter à atterrir sur master pour la vélocité, pas interdire les PR ») : une PR par lot ouverte par un crochet PRE_PUSH de la garde depuis la ref finale, fusionnée par le push en avance rapide, zéro attribution, gh en panne n'arrête pas le push (rattrapage par une ref -base) ; lot écrit par lucivy, relu par G, 2e en file ; question posée : une exception bornée pour effacer les branches pusher/* dont la tête est dans master après 7 jours ; graine produit : **une file de fusion pour agents**, déclarée par dépôt, trois régimes (rien, légère, avec preuve), découverte par les agents dans la description des outils ; 4.3.11 de lucivy publiée au mot de Lucie (« un correctif publié vaut une version »), pointeur déposé. **Relevés** : C en service (usages complet à 55 s, relations à 59 s, index entier 114 s sur l'arbre de Lucie) ; B : porte fermée (repli ~21 s contre 26) ; le point isolé HNSW = ordre d'insertion au bâti parallèle, amorce séquentielle de 1 000 points (0/24 manqués, +6,5 %), option seed_points du bâti, rappel mesuré par le banc ; l'ouverture d'une base dépend de la dette de vecteurs (1–3 s sans dette, 26 s = 2 074 morceaux) ; campagne D1–D5 d'Algareth juste sans borne atteinte (archi 73c758580c) ; les commandes lourdes de la porte sous poste lourd par sandbox-exec (archi, conception) ; la chauffe du service comptait ses troncatures (embarquement) ; ménage d'ED fini.
- 12:53 : **A′ de mémoire sur master** (aceebe4e52, 11 commits : Turn, put_turn avec mask_secrets, render_turns, archive_slice par intervalle qui rend source : l'archivage au fil existe de bout en bout en code), poussé par le pousseur après 57 min d'attente de place ; **le lot des PR par lot** (fa53de5db8, 31c8e0b5c8, écrit par lucivy, relu par G : crochet PRE_PUSH de la garde, PR fusionnée par le push, zéro attribution, rattrapage par ref -base) et **la voie du pousseur dans poste** (29d4c9533a : POSTE_VOIE=pousseur, billet 0-… en tête des lourds, derrière une mesure) ; file rouverte, le lot 2 d'archi est le premier témoin réel (PR, Vertex au pousseur, voie). **La file de luciepc était gelée** : trois mesures en attente (banc, arbre, lucivy) bloquaient tout lourd, et un cargo test -j 14 (mémoire) écrasait l'enveloppe : mesures retirées et ordonnées (banc → arbre → recherche → lucivy), -j 8 rétabli ; **règle neuve** (procédure de push 536f8824af) : pour déposer, le témoin rouge-puis-vert et les suites exigées suffisent, le pousseur joue la passe complète une fois. **Correction des mesures** : les sondes d'index de recherche et de codeparsers étaient en debug (46 s / 26 s / 89 s, C en service 55 s / 114 s) ; B était en release : la porte de B n'en est que plus fermée ; règle : une mesure qui décide se fait en release (requalification fa82e187ab ; rejeux en release en file de mesure). **Décisions** : 2.7 d'ED en (ii) connexion qui recommence une instruction seule (crochet de test) puis (i) reprise d'un paquet par l'hôte ; l'entrée 6 de recherche (arrêt d'une requête) est un lot Rust (le binding expose interrupt) ; lot 8 de 2a′-3 (rattrapage history all/last N avec le témoin 7) ; la voie courte des requêtes au service (embarquement, dd9c9920e6) ; le relais des troncatures déposé ; 20 tests de rag3weaver qui sautent et passent (lucivy 5001a9bb6e : SAUTÉ partout, ENFANT à part, motif exigé) ; **le cache de préfixe des modèles** (question de Lucie) : la mesure existe en partie (audit du banc du 17), ED corrige absent ≠ zéro et lit cache_n de llama-server, mémoire compose le stable d'abord (Gemini : implicite seulement au-delà de 6 144 jetons communs), le banc met le coût réel dans la matrice R. Lucie : des PR vers master voulues (vélocité gardée), 4.3.11 publiée, Vertex posé sur l'unité du pousseur ; question ouverte : effacer les branches pusher/* dont la tête est dans master après 7 jours.
- 14:01 : **les PR par lot tiennent en vrai** : rangement-lot-2 (1de6ae04ca, PR #3, e2e_equipier joué sous Vertex au pousseur, la voie du pousseur prouvée devant neuf lourds), rangement-lot-3 (3c420a9aa5, PR #4 : le filet de débordement ; les trois lots du rangement au fil sont posés), pusher-second-slot (7fa6716589, PR #5 : deux créneaux, train spéculatif B poussé sans rejeu par la garde en mode pile si A atterrit ; défaut vu : un mouvement de master en docs seules fait recommencer le pas d'un lot de scripts, trois reprises en deux minutes, à corriger). **Les quatre mesures d'après-midi, une à la fois sous poste mesure** : l'amorce HNSW (banc : rappel@10 inchangé, CREATE +3 %/+5 % ; les anti-hubs d'une gaussienne 0/200 au rang 1 avec comme sans, ticket a472c046c1) → hotfixs prêt (isolated-point-found-first 6623840429, ticket fermé) ; l'ouverture (arbre : 1–3 s pour une base saine de 3,6 Go, la dette de vecteurs fait attendre, pas la taille ; dernier point : répondre en disant le retard) ; **C en release** (recherche e3d5e90334 : usages complet 24,1 → 14,1 s sur rw/src, 76,6 → 48,6 s sur le dépôt, 34,4 s en service ; B ne bat pas l'index en release des deux côtés, porte fermée ; Lucie : « pas mal, bravo pour C ») ; le profil léger (lucivy 51a851baa7 : −29 % de place, toutes les requêtes sous 100 ms, mêmes réponses et spans ; décision c7 sur le critère de Lucie « sous la seconde » : fts_positions false par défaut avec dérogation par index, lot lucivy, rebâti des bases à une fenêtre de G). **Trouvailles** : le levier de l'analyse est commun à tous les langages (codeparsers : Node::parent de tree-sitter redescend depuis la racine, 41–51 % de l'analyse, tools/ > 60 s ; lot : cache des parents par arbre ; Lucie : fork de tree-sitter permis « si vraiment nécessaire », après le contournement) ; la cause du témoin de mort du raccord (arbre : exit() lance les destructeurs statiques pendant qu'un fil analyse une requête → SIGSEGV ; une mort simulée meurt par _exit, généralisé) ; l'arrêt d'une requête du moteur est un lot Rust (le binding expose interrupt) ; 2.7 d'ED en deux lots (reprise d'une instruction seule par la connexion, reprise d'un paquet par l'hôte). **Chantiers de Lucie** : la campagne des correctifs passés d'Algareth (commit ancien, tout sauf git, le témoin juge ; archi le lanceur ~2 j, banc le juge, ticket 9fd3df3765) ; les envois tracel-ai repris (cubecl #1804 approuvée le 15 oct. sans mot, jamais fusionnée : mise à jour par merge ; C, D, E reposés sur l'amont du jour puis soumis sur son oui écrit dans la session de G) ; le cache de préfixe des modèles (ED : cache_reported déposé, page 05a5674d16, premier remède le décideur déposé ; mémoire : Gemini implicite seulement au-delà de 6 144 jetons communs). Règle du jour : une mesure qui décide se fait en release. Lucie : des PR vers master voulues ; forker tree-sitter si vraiment nécessaire ; « un correctif publié vaut une version » (4.3.11). Question ouverte : effacer les branches pusher/* dont la tête est dans master après 7 jours.
- 14:12 : **le pousseur ne perd plus la course aux docs, et le second créneau est ouvert.** La cause du pas qui recommençait : pour un lot de docs ou de scripts, le push exigeait master inchangé depuis la repose, et les pages des sessions le faisaient recommencer ; correctif 9ed24dc122 (G, à la main, file en pause) : master qui n'a bougé qu'en .md → repose sans rejeu, atterrissage dans le même pas, trois essais ; du code sur master → rejeu ; les PR ouvertes avant un refus sont fermées (62 cas, deux vus rouges avant). PUSHER_SLOTS=2 dans l'unité (14 h 07), rag3db-pusher-2 à luciepc, **premier double train** : A emb-dual-witness reposé 20da1ba554 sur 9ed24dc122, B link-copy-split reposé 7750a4cb4d sur la tête de A, dans le second créneau. 42 lots en file (tty 9, arbre 8, recherche 7, ED 7, embarquement 4, archi 3, lucivy 2, codeparsers 1). Deux règles du jour : (1) un témoin « 0 passed, 1 ignored » est rouge : ide-waiting (tty) déclarait un test navigateur, que la passe du pousseur ne joue pas ; G a remplacé par le témoin de l'hôte ; le test navigateur est jugé à l'étape « témoins navigateur » de la passe (temoins_navigateur.sh sous RAG3WEAVER_EXIGER_NAVIGATEUR=1, un saut y est rouge), que le pousseur joue bien (G corrige son premier mot) ; (2) **rouges connus des e2e** : la passe n'en lisait aucune liste ; forme retenue, `tools/poste/known_red_e2e.txt` dans le dépôt (suite, cas, ticket), la suite toujours jouée, le cas listé compté « rouge connu », ticket absent ou fermé = refus, le lot qui corrige retire sa ligne (lot chez G) ; jamais « retirer la suite de la ligne du lot ».
- 14:12 : **le rouge « en fichiers » est du Rust** (recherche, ticket 48564fedf8) : master 536f8824af sous l'ancienne lib 3d706f709 donne les mêmes 4 rouges sur 6 cas en_fichiers d'e2e_tx_par_paquet_arret que sous 7c2b506ec ; d9c30ef781 sous 7c2b506ec : 6 verts. Suspect premier : C (e6c0307291, les dérivations retenues jusqu'après les relations) ; bissection parent-de-C contre C ; correctif rouge d'abord devant ses autres entrées. Et **2.7 (i)** se fera par une primitive du moteur : `CALL lock_tables([...])` en première instruction du BEGIN (le cœur), le refus d'un ALTER validé entre-temps avant toute écriture, ROLLBACK sans empoisonnement, paquet recommencé sur place (≤ 3) ; attend la lib de la prochaine fenêtre C++.
- 14:40 : **le second créneau est prouvé.** A emb-dual-witness (pris 14 h 08, poussé 14 h 23 : 2eb74bfa6b, PR #6) et B link-copy-split (reposé une seconde après dans rag3db-pusher-2 sur la tête de A, rejoué pendant A : VERT à 14 h 37, pic 13,5 Go, poussé par la garde en mode pile 14 h 37 : e802940db3, PR #7) : deux lots Rust rejoués en même temps, 29 minutes pour les deux au lieu de deux fois 15–25. Puis known-red-e2e (a53e345c31, PR #8, `tools/poste/known_red_e2e.txt` avec les quatre cas en_fichiers d'e2e_tx_par_paquet_arret). Tous gmail, zéro attribution, vérifiés. Le lot de codeparsers ferme le ticket de la pose des liens (drain · liens 7,5 → 4,6 s sur rag3weaver/src, 26,5 → 17,8 s sur le dépôt). File à ~50 lots.
- 14:40 : **la campagne des correctifs passés a sa forme et ses premiers cas.** Page du juge (banc, ebb5dbd092 puis 6b3c79f0cb : rouge prouvé au parent sur l'assertion, cinq verdicts, témoin intact par le diff de ses fonctions, grille endroit/taille/coût/dégâts/indices, réussites avec et sans indice à part ; sortie `~/.cache/rag3weaver-build/campagne/<cas>/<run>/`, budget 12 pas, 2 M jetons, 30 min par tour) ; conception du lanceur (archi, 95def3b609, db1361781d, df7b6b7e04 : bloc json « lanceur » en tête de chaque cas, witness_files_from quand le témoin est né avec le correctif, refus mécanique d'un signalement qui nomme `ours`, dépôt entier archivé avec ses sous-modules au parent, tout dossier docs/ exclu sauf docs/procedures/, un target par cas ; code après le branchement de poste dans l'hôte). Huit cas déposés en docs : tty 08/09 (e217f9c5f8), arbre 01/02 (09bb793e72), lucivy 03/04 (fce98147b6, dépôt lucivy), ED 01/02 (c77a12d77b) ; mémoire, recherche, embarquement, codeparsers quand leur file attend. Décisions : une souche ou un `RED STUB` posé avec le rouge au parent reste (indice déclaré), pas de rouge par compilation ; le signalement ne contient que la citation du ticket.
- 15:15 : **le rouge « en fichiers » est départagé, bissecté et retiré.** Recherche : master 536f8824af sous l'ancienne lib donne les mêmes 4 rouges que sous 7c2b506ec, d9c30ef781 sous 7c2b506ec 6 verts : c'est du Rust ; la bissection désigne C (e6c0307291) : les dérivées retenues jusqu'à la fin de la passe ne sont pas rattrapées à la réouverture après un arrêt brutal (paquets validés sans leur plein texte, f010 introuvable, comptes faux). Décision c7 : **revert de C maintenant** (un défaut de données sur master se retire avant de se corriger en avant quand le correctif prend des heures) ; revert-c-2 (ecd49caac8) en tête de file avec les 4 lignes de known_red retirées et le ticket d'ED mis à jour, laissé ouvert ; **C durable** écrite dans l'heure (c-durable 0e57c05ba3 : marque `derivations_held` dans `_catalog_meta` posée dans la transaction du premier paquet, `repair_held_derivations` avant toute lecture et en fin de reprise, état « à rebâtir » tant que la marque reste ; rouge = C réappliquée telle quelle, vert 6/6), déposée après le revert avec la suite entière et la mesure en release refaite. Les chiffres de C (usages 14,1 s) tombent avec le revert et reviendront avec elle.
- 15:15 : **la passe et le pousseur se durcissent par leurs propres trous** (tous vérifiés, gmail, zéro attribution) : witness-navigateur (2ca362cb30, PR #12 : `SUITE_DU_TEMOIN=navigateur`, le témoin jugé à l'étape des témoins navigateur ; deux lots en une heure n'avaient qu'un test navigateur) ; witness-must-run (7523bad311, PR #13 : l'étape témoin ne jugeait que le code de sortie, « 0 passed » passait vert ; au moins un test portant le filtre doit sortir « ok » ; audit rétro de G : les sept Rust atterris depuis 9 h 17 ont tous leur témoin joué « ok, 1 passed ») ; landed.tsv garde le témoin déclaré et sa suite (ff0b28d94f) ; la repose à la milliseconde (3625a18b24 : deux pas dans la même seconde reposaient sous le même nom, piège du second créneau) ; un lot qui touche tty/ joue le crate du terminal dans la passe (1dddb55a61, PR #15). Règle de G : on ne dépose qu'après un verdict lu. Second double train atterri (recherche-visibilite 5ad882754d PR #10, memory-image ba0ee92e47 PR #11) ; tty-27 (316dddb4db, PR #14). Le pousseur rend maintenant les lots qui butent sur ce qui a atterri (revert-c, outil-pendant-index, ed-define-ref : conflits de ticket et de gabarit ; presentation-outils rouge lib) : leurs auteurs reposent.
- 15:15 : **autres atterrissages et dépôts** : la voie courte d'embarquement mesurée (c6860b877b : p95 116 → 38 ms pendant un index, débit inchangé ; granite panique à la chauffe sur l'iGPU, ticket, départage confié à embarquement) ; profil léger par défaut déposé (lucivy b138284b7d, parité octet pour octet ; mesure du dictionnaire lancée sous poste mesure) ; le raccord d'arbre déposé (e1cf1bdf92 : 0 rouge sur 150 tours contre 1 sur 40) ; la lecture des tours de mémoire déposée (fa19ea141, 8 suites vertes) ; porte-sous-poste-hote (archi : `RAG3WEAVER_POSTE` à poser dans l'unité à la pose du soir) ; les cas de campagne d'embarquement (19d1926b46) et d'ED (c77a12d77b) ; ED ferme trois tickets de 2.8 par la mesure (vues de l'IDE 153–460 ms en release contre 59 s le 13 nov. : cause corrigée depuis le 13 ; lecture refusée : le mécanisme n'existe plus ; variante sans accord) et trouve un vrai défaut du journal plafonné (octets perdus entre la lecture de la fin et la troncature). Fenêtre C++ du soir : tête commune train-soir-coeur 4ab4ab8150 (seed_points, DROP-attend-les-lecteurs, lock_tables), passe finale en file à luciepc ; aucune pause du pousseur nécessaire (lib-late suspend seul les Rust).
- 16:22 : **le revert de C est sur master** (4c0728e703, PR #19, vérifié ; les quatre lignes de known_red parties ; en_fichiers 6/6) ; recherche repose C durable (marque `derivations_held` dans `_catalog_meta`, réparation avant toute lecture et en fin de reprise) et la déposera avec la suite entière et la mesure refaite en release. Atterris aussi : ed-accord-recu (0d801025ab, PR #17), ticket-porte-equipier (PR #18), temoin-tty (9588b42f60, PR #20 : `SUITE_DU_TEMOIN=tty`, le témoin du crate du terminal jugé à son étape), les pages de mémoire du lot B. Le pousseur rend la chaîne de tty (hote-waiting et ses trois enfants, conflit dans teammate.rs) : tty repose.
- 16:22 : **Lucie, sur l'archiviste : « tout doit être déclaratif, juste un DAG comme un autre, j'aurais pu en faire un moi-même ; et pareil pour des boucles auto-amélioratives, un agent qui édite son propre workflow de mémoire ».** Relevé fait (mémoire, archi) : le fond est déclaratif (trente graphes .mmd, scripts Rhai, schémas), la couture de l'hôte est en Rust (rangement.rs 293 lignes : put_turn à chaque tour, archive_slice en fond au-delà de slice_tokens, compaction par compose_restart) ; en dur encore : les textes de reprise, le budget de compose_restart, identity/task_fiche vides, ceil(caractères/4), le motif « llm context overflow », les motifs de mask_secrets en constante Rust, l'invite d'extraction dans un script. Décisions, pour le **lot B de mémoire** (dû, après la chaîne des tranches ; pages a5500ccc6b et précédentes, ticket archi 26d8cebca4) : l'extraction structurée est **LlmNode** avec un port `context` et `response_schema` (les descriptions de champs portent le sens, la consigne est un paramètre déclaré, courte) ; une **part** `{name, title, value, format?, max_tokens?}` porte toute valeur JSON (chaîne, objet, liste d'objets) rendue par le nœud, présentation résolue de la part au type déclaré dans le schéma de l'entité puis JSON à clés triées ; les parts s'assemblent par un **nœud générique de collecte** à entrées nommées déclarées dans l'ordre (pas de règle de fusion cachée dans le moteur ; le modèle des ports l'accepte par `schema_for(config)` comme Route) ; un ingrédient débrayé rend une valeur vide, il ne se fait pas sauter (le saut se propage) ; plus aucun Rhai qui assemble une invite. Ce qui reste à l'hôte devient un mécanisme générique sans texte propre à l'archiviste. Graine notée : relever un jour où JSON est un mauvais choix (TOML pour les réglages, XML pour le rendu des parts vers le modèle).
- 16:22 : **la bascule des bases de Lucie vers les versions n'était pas faite** (ED : code-rag3db-depot servi sans ScopeContent, manifestes `version: 1`) ; recette prête depuis le 15 (arbre, page 15-11/02 : les trois bases code-rag3db-depot, code-lourd, code-principal ; ~15 min la part fermée en parallèle, repassage base ouverte 50–60 min par base, anciennes gardées 72 h) ; répétition du jour lancée sur un reflink de la copie de G ; place réservée dans la fenêtre de G ce soir après l'épingle, à mon « bascule : oui » au verdict d'arbre. ED : 2.8 fini (deux défauts produit corrigés, trois tickets fermés sur preuve), 2.9 déjà corrigé le 11 nov. (bisection sur b3f31b544f, ticket fermé), 2.10 attend la bascule, 2.11 : page du dialecte postgres à générations (onze Cypher bruts à passer par l'IR ; deux lots A puis B ; parité par résultat et par plan). Arbre ferme le dernier point du ticket du verrou (l'attente à l'ouverture venait du mutex du catalogue pris avant la vue, levé par L2, try_lock et le rattrapage en fond ; garde déposée). Campagne : lot 1 du lanceur déposé (lanceur.py, rouge au parent prouvé sur tty 08/09 et ED 02). Train C++ du soir : 4ab4ab8150 tient (27/27 témoins trois fois, transaction_test 407, known_red, vector 74/74), e2e_test en cours.
- 16:26 : **le train C++ du soir est sur master** (8328a87be8, 16 commits, gmail, zéro attribution, vérifiés ; voie transférable tenue : « master 40e7fa53d6, base testée b2f4370f02, rien de code entre les deux ») : seed_points de hotfixs (6a5595c950 : le bâti parallèle d'un index HNSW commence par une amorce séquentielle ; le témoin du point isolé par le shell 20d6e4b7f6 ; ticket fermé 32dfbdf83e), le DROP qui attend les lecteurs (3b0d9a8a4d, cb081621a3 : verrou TABLE_READ des lecteurs, pris en exclusif par ADD/DROP de colonne et DROP TABLE seulement ; COMMENT, RENAME, DROP d'index n'attendent pas ; le refus nomme les transactions ; le COPY tient aussi ce verrou d63d35a0b8), et **`CALL lock_tables([...])`** (6e2476d41b, témoins b2d26b6d3f : un ALTER validé après le BEGIN est refusé avant toute écriture, un ALTER après la prise attend le COMMIT ; la primitive de 2.7 (i) d'ED). Passe finale sur 4ab4ab8150 : rouges d'abord vus (LockTables 5 cas 3/3, DropWaitsForReaders), témoins 27/27 trois fois, transaction_test 407, known_red, vector 74/74, e2e_test 1 867 (DropNodeTableReclaim admis). La quarantaine de hotfixs part au train suivant (témoin rouge poussé 852266a8b0, correctif écrit). **La fenêtre de G commence** : lib (ABI : TABLE_READ, Transaction, alterLockTimeoutInMS), verdict sans drapeau, hôtes avec `RAG3WEAVER_POSTE`, épingle en tête du pousseur ; le pousseur suspend seul ses lots Rust d'ici là, les docs et scripts continuent ; ED dépose (i) sur la lib épinglée ; la bascule des bases à mon « oui » après la répétition d'arbre (repassée en lourd : une mesure en attente gelait sept lourds à luciepc, dont les deux passes du pousseur ; leçon : une répétition dont le juge est la parité se joue en lourd).
- 17:06 : **la bascule des bases est décidée pour ce soir** : la répétition d'arbre est verte (256ad48f63 : reflink de la copie prouvée, 1b 667 s, générations 1,3 s, index_state sans « sans versions », usages identiques hors uuid, search_code 96 % du top 10 et même premier résultat 7/8 : l'égalité stricte ne peut pas tenir à travers 1b, l'écart est la forme à versions et 2 % de vecteurs dus) ; « bascule : oui » donné à G pour après l'épingle (recette : aucun index en cours, maintain purge seule, instantané 72 h, deux sens, 1b, générations, réouverture code-rag3db-depot d'abord, repassage sous poste lourd ; critère de fin : usages identiques, top 10 ≥ 90 %, premier résultat ≥ 7/8, sinon retour arrière). **C durable mesurée en release** (recherche, bd4558f463) : usages 23,9 → 12,2 s sur rag3weaver/src (C d'avant 14,1), 74,9 → 40,5 s sur le dépôt (48,6), 28,3 s en service ; l'index entier ne ralentit pas ; dépôt dès ses trois suites vertes. **Lucivy, défaut de données** : 4 748 mots de content perdus sur code-rag3db-depot (ticket 8ab6355dcf), datés d'une compaction unique du 19 à 09:10 lancée par l'outil de maintenance sur la base vivante (personne ne se souvient de l'appel), sous lucivy 4.3.9 ; première hypothèse réfutée par le témoin (la compaction tient le verrou de l'écrivain) ; piste suivante : un mot créé puis supprimé avant commit reste dans la table des créations en attente et est réutilisé sans vérification (témoin en file). D'ici le correctif : compaction nocturne suspendue par G sur les huit manifestes (`fts.maintenance.compact: false`), rebâti des bases après le pointeur, maintain de la bascule en purge seule. Question à Lucie : 4.3.12 au vert ; et le format 4.4 (dictionnaire ÷7 mesuré : .sfx 290 → 66 Mo, bâti 9,7 → 1,5 s).
- 17:06 : **la campagne tourne pour de vrai** : lot 2 du lanceur sur master (1ea0697087 : hôte de campagne isolé port 7990, arbre inscrit depuis les gabarits figés, signalement seul, git refusé) ; premier cas joué de bout en bout, tty 09, jugé par le banc (aa7adfc5e2) : **échoué faute de budget**, un seul pas en 30 min, diff vide ; rien à conclure avant la trace du temps du modèle par appel (lot 3) ; ticket : le bac à sable refuse le pseudo-terminal. Quatorze cas déposés, douze sur master. **Autres** : codeparsers dépose le cache des parents (−40 % sur l'analyse en release sur trois corpus, rouge de compte 2 810–7 608 → 0, suivi des imports dessous, une repasse ; mots vides des huit langages ensuite, groupés si prêts avant l'épingle) ; la panique de granite sur l'iGPU départagée (burn-tensor, 128×182 puis 1×382 sur Vulkan ; chez nous la chauffe prend le lot conseillé au lieu de min(lot, lot_max) : correctif rouge d'abord ; fiche amont à G) ; ED : lot A du dialecte (onze sites par l'IR, dix témoins de parité par résultat et par plan) en suites ; mémoire : la chaîne des tranches déposée (get_slice, list_slices, trace_turn tout en graphes et Rhai), trois lots en file. **Tracel** (Lucie : « vérifions si toujours d'actualité et on leur soumet ») : #1804 = une ligne (Flex32 inscrit au backend Vulkan de cubecl) ouverte par une question, approuvée le 15 oct. sans un mot, main +203 commits ; C (cubek, matmul Flex32 accumulé en f32) en dépend ; D issue burn, E PR burn ; preuves : nos services en Flex32 sur Vulkan depuis le 6 sept. (BGE-M3 5 300 → 15 000 jetons/s), rouge sans / vert avec sur RADV ; Lucie : le drapeau dans C si possible, sinon #1804 retournée en PR de fonctionnalité au récit (« grâce à ça… »), C derrière ; textes par G, oui écrit avant tout envoi. Leçon de la fenêtre : toute la fenêtre prend `POSTE_VOIE=pousseur` (le bâti de la lib a attendu 22 min une place de lourd).
- 17:50 : **la fenêtre de G, étape par étape** : lib rag3db-build-6e2476d41b bâtie 16 h 50 (le bâti avait attendu 22 min une place de lourd faute de la voie : règle, toute la fenêtre prend `POSTE_VOIE=pousseur`), verdict VERT (e2e_code 36, e2e_lectures 28, e2e_demon_hote 26), hôtes posés 17 h 31, **épingle poussée 17 h 32 par le pousseur** (pin-lib-6e2476d41, d65daa6c66, PR #36 ; lib-des-passes écrit par le pousseur lui-même, première preuve réelle), réveil des hôtes 17 h 44 min 50 aux deux postes (coupure 13 min 41 s, contrôle des deux sens sans écart sur les quatre bases), répétition de 1b sur le binaire du soir verte (659 s), **bascule des trois bases lancée à 18 h 0x** (code-rag3db-depot d'abord ; l'étape maintain sautée : le script compacte toujours, et lucivy perd des mots au fil de l'écriture courante). Les lots Rust ont repris à 17 h 32 (66 min de suspension) : relais-tronque-navigation (3f05c62aef, PR #37), recherche-copie-csv (d046c5a568, PR #38). ED a déposé (i) sur la lib neuve (vert sous 6e2476d41b, rouge sous l'ancienne). Deux gels de luciepc dans la journée par une mesure en attente (arbre 16 h 14, lucivy 17 h 03 : sept puis cinq lourds gelés, dont les passes du pousseur et la répétition de 1b) : règle demandée à G, `poste mesure` refuse par défaut quand un lourd tient ou attend, `POSTE_MESURE_GELE=1` pour forcer.
- 17:50 : **Lucie relit `docs/nouveautes.md` : « je vois peu d'entrées »** : deux lignes pour le 19 alors que 38 PR ont atterri ; la règle du 8 nov. n'était plus forcée depuis que le pousseur a remplacé le « à toi » où je la vérifiais. Rattrapage par les auteurs (seed_points, S2/S3 et la session archivée au fil, A′ des tours, usages sous générations, le contrat de reçu, le profil léger, `view: {layer, rank}`, les dix lots de tty et sept d'ED redéposés avec leur ligne) ; règle demandée à G : un lot rust qui touche src/, tty/ ou templates/ sans toucher nouveautes.md est refusé à l'enqueue sauf `SANS_NOUVEAUTE="<raison>"` écrit au journal et dans la PR ; `redeposit` qui garde le rang ; la repose docs seules rouvre une PR (un lot a atterri sans) ; les dates « novembre2026 » corrigées en un lot mécanique après la bascule. La bascule des bases de Lucie n'est pas une nouveauté du produit : journal des chantiers. **Lucivy** : les mots perdus sont antérieurs au 18 (6 894 déjà dans l'instantané du 18 à 15 h 29), deux hypothèses réfutées par leurs témoins, la compaction de 9 h 09 était la maintenance nocturne rejouée au réveil de la tablette ; bornage sur l'instantané du 16. **Campagne** : lot 3 du lanceur (trace.json), tty 09 requalifié « écarté » : 1 814 s de modèle sur 1 828, quatre appels de 4 à 6 min pour 35 jetons de sortie, ni raisonnement ni cache en cause ; mesure du premier octet ajoutée, rejeu ce soir. **Tracel** : #1804 = une ligne (Flex32 inscrit au backend Vulkan de cubecl), C en dépend, D/E burn ; Lucie : la justifier par ce qu'elle a permis chez nous plutôt que par une question ; G remesure les chiffres (iGPU RADV, card2) avant d'écrire, textes en anglais à valider par elle.
- 18:14 : **la bascule a rougi, et le retour arrière a tenu.** code-rag3db-depot : fermeture 17 h 50, instantané avant-versions-20261119, deux sens sans écart, 1b 770 s, générations 3 s, rouverte 18 h 04 sans avertissement (coupure 14 min 07 s) ; puis **le repassage de l'écrivain échoue à « recopier les arêtes DEFINED_IN » : tampon du moteur plein à 8 Gio, puis à 24 Gio au second essai** : la recopie des arêtes entrantes du premier passage sur une base migrée ne borne pas sa mémoire (toutes les lignes sont neuves, toutes les arêtes à recopier d'un coup) ; la répétition ne couvrait que 1b et les générations, c'est le trou. Retour : hôte arrêté à froid 18 h 10 min 36 (RETOUR.sh l'exige), retour VERT 18 h 11 min 04, socket rallumée ; banc identique (recouvrement 100 %, même premier 10/10), usages identiques ; **la base est revenue à son état de 17 h 50**. luciepc : code-lourd et code-principal arrêtées avant leur repassage, leurs 1b finissent seuls, retour sur leurs instantanés avant 18 h 44. Suite : ticket pour ED (correctif rouge d'abord sur la copie migrée `mesures/code-rag3db-migree-soir-20261119`, lots de paires validés un à un, mesure de la mémoire du moteur), et **aucune nouvelle tentative avant une répétition qui joue aussi le repassage jusqu'au critère** sur une copie de la vraie base. La lib du soir et les hôtes restent posés, la file continue. Leçon : une répétition joue toute la recette, y compris ce qui se passe base ouverte ; un rouge sur une copie prouvée avec un retour prêt est ce pour quoi on répète.
- 18:14 : **`docs/nouveautes.md` : dix entrées pour le 19 novembre** (contre deux à 17 h 40) : les six lots de docs passés en tête ont atterri entre 18 h 08 et 18 h 11 (PR #39 à #44 : S2, S3 et la session archivée au fil ; `view: {layer, rank}` ; seed_points ; usages sous générations ; les tours en mémoire ; le contrat de reçu) ; les deux d'arbre attendent leur parent Rust ; les lignes des lots Rust en file (tty ×6, ED ×7, mémoire ×3, embarquement ×4, lucivy, codeparsers, recherche C durable) arrivent avec eux. Règles demandées à G : refus à l'enqueue d'un lot rust sans nouveauté ni motif ; `redeposit` qui garde le rang ; les docs devant les Rust sauf parent rust en file ; une PR rouverte à chaque repose (deux lots encore atterris sans PR ce soir). Dépôts du soir : 2b-C d'arbre (la vue courante par le contexte, 580347a57b), la chauffe d'embarquement par `par_lots` (le correctif de la panique iGPU, 5f3cfdcd70), C durable reposée avec sa ligne (recherche-c-durable-3), le mode pair d'Algareth en six lots (archi, 1f6c41de0e ; L6 au oui direct de Lucie).
- 18:45 : **les trois bases sont revenues** (luciepc rouvert 18 h 17 min 41, coupure 13 min ; banc et usages identiques pour chacune ; instantanés gardés 72 h, deux copies migrées gardées pour ED : `mesures/code-rag3db-migree-soir-20261119` et `mesures/code-lourd-migree-1b-20261119`) ; ticket du repassage par G (36d46f71d5) ; ED mesure avant de corriger (EXPLAIN de la recopie DEFINED_IN sur un paquet réel : le suspect est le plan qui part de tous les Scope, pas le nombre de paires ; sonde en release sur un reflink, courbe de mémoire). **Premier verdict réel de la campagne** (banc 5ac7887469, tty 09 rejoué à 18 h 06 quand Vertex répondait vite) : « dégâts » : les deux témoins passent et restent intacts, la souche est suivie, mais une assertion fausse ajoutée à un test voisin le rougit ; +46 −5 contre +36 −3 ; 5 pas, 165 appels, 1 181 s, 5,31 M jetons d'invite dont 87 % par le cache. **Lucivy** : cause des mots perdus non trouvée après quatre pistes réfutées par des témoins ; décision : le **sursis d'un cycle** (la compaction garde les textes morts une génération de plus, la suivante garde ceux que des segments citent et journalise chaque id revenu avec son segment : la cause nommée sans perte ; contenu de la 4.3.12) ; compaction coupée d'ici là ; sonde de nuit par rag3weaver sous poste lourd. **2.12** (ED, page 19-11/04) : un namespace ou module dont le texte replié, docstring comprise, fait moins qu'un seuil du manifeste (80) ne concourt pas comme candidat. **Pousseur** : parent-cache rendu « arbre sale » : la repose d'un lot qui change le pointeur du sous-module ne faisait pas `submodule update` (la passe jouait avec l'ancien codeparsers) : correctif demandé, lot à rejouer. Atterris : tty-28 (8c2b5baead, PR #45), tty-29 (4f6a2ff8d0, PR #46), 2b-C déposé par arbre, la quarantaine de hotfixs prête pour la prochaine fenêtre C++ (38eacb2664), C1 revu vivant sur master (cœur, dfd2c75102 : 2 SIGSEGV en 10 tours de DdlEveryMinute, TSan puis ASan en cours).
- 19:14 : **le repassage est corrigé et prouvé sur la vraie base.** ED (lot ed-rewrite-bounded-memory 96790ff190, page 19-11/05) : le rouge venait du **plan** de la recopie DEFINED_IN, pas de la taille des lots : un seul MATCH à motifs croisés (`MATCH (x:Scope)-[r:DEFINED_IN]->(a:File {_uuid}), (n:File {_uuid})`) donnait un CROSS_PRODUCT avec balayage de File et un NOT EXISTS par HASH_JOIN sur toutes les arêtes, dès le premier paquet (20 s, 11,7 Gio) ; chaque bout cherché par sa clé dans un MATCH à soi : sur le même lot de 2 000 paires réelles, 3,8 s et 9 Gio tampon plein → 6 861 arêtes en 631 ms et 16 Mio ; le repassage entier de la copie migrée sous la règle du produit (8 Gio) finit en 1 049 s, pic 6,94 Gio tout le processus ; les dix témoins de parité verts ; les lots validés un à un existaient déjà (un paquet de fichiers par transaction). G rejoue la répétition complète (copie neuve de l'instantané d'avant 1b, hôte isolé port 7992, 1b, générations, réouverture, repassage jusqu'à « indexation terminée », mémoire relevée, critère) à l'atterrissage du lot ; la bascule à son vert, cette nuit si la file le permet. **Pousseur** : deux défauts corrigés (fd86c36265, PR #48 : la repose met les sous-modules au pointeur du lot avant la passe et la garde ; les PR d'un pas d'avant ne sont plus visées) ; **parent-cache atterri** avec son vrai codeparsers (d83f4099a9, PR #49 : l'analyse 40 % plus vite, le suivi des imports sans regex, pointeur 828f855) ; la chaîne verrou d'arbre rendue pour un rouge d'ailleurs (le témoin de mort simulée intermittent, tests.rs:545, que le raccord en créneau 2 corrige : G le redépose une fois avant de le dire rouge d'ici l'atterrissage du raccord). **Lucivy** : PR #50 fusionnée (le sursis d'un cycle, CI 22 jobs), 4.3.12 au mot de Lucie. **Lot 8 de 2a′-3** décidé (le rang d'une génération d'histoire = position du commit sur la première parente ; une ref réécrite rend la couche « à rebâtir » en le disant). **Codeparsers** : la pile mots vides + locales retire 73 arêtes fausses au-delà des 79 et en ajoute 63 justes sur le moteur ; 2 retraits à expliquer. Seize cas de campagne déposés ; tour des cas sans lib en cours (lot 4 du lanceur : enveloppe et clé repo).
- 19:25 : **atterris** (gmail, zéro attribution, vérifiés) : parent-cache avec son vrai codeparsers (d83f4099a9, PR #49), le raccord d'arbre (46101db7b5, PR #50 : toute mort simulée d'un témoin sort par `_exit` ; le témoin intermittent de physical_index est tenu, la règle de redépôt à la main de G tombe), le correctif du pousseur (fd86c36265, PR #48). Train d'ED lancé à 19 h 23 (l'écrivain par l'IR en A, le repassage borné en créneau 2) ; les docs sans ancêtre Rust passent derrière eux (nouveautes-dates, la page 2.12, les cas de tty et de mémoire, le journal de la bascule, le ticket du dense), puis la chaîne verrou d'arbre. **Lucie** : « lots d'effacement à mon oui : oui, go, fais-le toi, je t'autorise explicitement » : fait par c7 à luciepc après contrôle lsof : les cinq copies d'essai du repassage d'ED (~26 Go apparents), les six copies de mesure de lucivy et ses index rebâtis (~6 Go), les résidus `affame-*` de recherche ; gardés ed-repasse-vert (jusqu'à la répétition), ed-namespace-copie (sonde 2.12), la copie prouvée de l'index de code (levier 3), les journaux de mesure ; à dire : 27 résidus `affame-*` (50 Mo) avaient été effacés sur mon premier mot avant que je rappelle la règle du 18 (chacun efface le sien à son oui). **4.3.12** : Lucie a dit oui dans la session lucivy (PR #51 : numéro, CHANGELOG, README ; tag à la CI verte). **Format 4.4 du dictionnaire** : expliqué à Lucie (une entrée par mot au lieu de sept jetons étendus : 379 → 80 Mo, bâti 9,7 → 1,5 s ; le cœur de `contains`, un format sur disque, une parité à prouver) ; proposé après le sursis et les leviers sans format, page puis prototype sur copie ; son mot attendu. **Défaut trouvé par ED** (ticket ed-ticket-dense-default) : `search_code` sans `signals` sur une base à versions ne fusionne pas le dense (0 résultat vector par défaut, 100 en le demandant) sans le dire : passe devant 2.12, rouge d'abord, parité du banc avant le dépôt. **Codeparsers** : la pile mots vides + locales déposée (11177477e6 : 75 arêtes fausses retirées, 63 justes ajoutées sur le moteur, les 2 retraits expliqués, deux tickets fermés) ; ED s'empile derrière pour une seule repasse. **Hotfixs** : la quarantaine prête pour la fenêtre C++ de demain (38eacb2664), groupée avec C1 si le cœur l'a. **Recherche** : entrée 7 (résidus des suites) et le test de l'écrivain affamé corrigé ; entrée 6 (interrupt) en cours. **Mémoire** : B1 commencé en branche (Collect, port context de LlmNode).
- 19:55 : **les deux lots d'ED sont sur master** (l'écrivain par l'IR 5dece62d8d PR #51, le repassage borné a83e234fc4 PR #52, verts au double train) ; hôte bâti dessus aux deux postes (md5 71d0379d, essai 7991 vert) ; **la répétition complète de la bascule tourne depuis 19 h 47** (un premier départ arrêté en douze secondes : l'hôte isolé n'avait pas l'environnement de l'unité, GOOGLE_CLOUD_PROJECT absent, réouverture refusée par models.llm ; corrigé, l'hôte isolé lit rag3weaver-daemon.env) : banc et usages d'avant identiques à 17 h 49, 1b depuis 19 h 48, puis générations, réouverture, repassage jusqu'au bout avec la mémoire relevée, critère ; la bascule des trois bases à son vert. **Un master rouge au contrôle des tickets, 19 h 39 → 19 h 52** : le ticket des mots perdus de lucivy déposé « corrigé » hors de closed/ (et pas corrigé : la perte est reproduite malgré le sursis, 152 mots en CI, les compactions ratent des segments sans cesse ; 4.3.12 suspendue, piste : un segment fraîchement fusionné dont le méta ne porte pas encore ses champs n'est pas lu par la compaction) ; le pousseur a poussé trois docs derrière sans le voir et deux trains Rust (la chaîne verrou d'arbre, la chaîne d'embarquement) ont rougi à check_features pour rien ; correctif d'état atterri (3033a3ecdd, PR #60), les chaînes redéposées telles quelles ; règles demandées à G : un lot docs qui touche docs/*/tickets/ joue check_tickets dans son pas, et pas de lot rust pris sur un master rouge à ce contrôle. **Ménage du soir fait** par c7 au oui explicite de Lucie (copies d'ED, de lucivy, résidus de recherche ; ed-repasse-vert et ed-namespace-copie gardés jusqu'à leurs usages). **Campagne** : quatre verdicts : tty 09 dégâts, lucivy 04 réussi (autre endroit), ED 01 échoué (sans diff, Vertex lent, et le cas disait à tort que le témoin passait seul : corrigé), ED 02 réussi (même endroit, +13 −3 contre +15 −3, 3 pas, 680 s) ; L1 du mode pair déposé ; l'entrée du registre vivra tant que la tâche est ouverte (L2). **Autres** : B1 de mémoire déposé (Collect, port context de LlmNode) ; le lot 8 de 2a′-3 déposé (2a′-3 complet en code) ; l'estimation du reste par vue déposée (arbre), la part de l'hôte en pile, la déclaration du débit par poste à embarquement ; l'entrée 6 de recherche (interrupt) et l'entrée 7 (résidus) déposées ; la pile de codeparsers déposée (75 fausses retirées, 63 justes ajoutées) ; le ticket du dense retiré par ED (c'était 2.12 par un autre bout), mécanisme de 2.12 arrêté (rien de stocké, geste déclaré rejouable pour les bases existantes, le seuil propriété de la base) ; la quarantaine de hotfixs prête pour la fenêtre C++ de demain ; C1 revu vivant (cœur, TSan en cours).
- 20:25 : **la répétition complète est allée au bout, le critère est manqué : pas de bascule cette nuit.** G, sur une copie neuve de l'instantané d'avant 1b et un hôte isolé : 1b 845 s (19 h 48 → 20 h 02), générations, réouverture, repassage jusqu'à « indexation terminée » à 20 h 22 (20 min) ; **le tampon est réglé** : RSS maximal de l'hôte 9,1 Gio, sans erreur (le correctif d'ED fait ce qu'il dit). Mais contre la production relevée au même arbre : recouvrement 96,8 %, premier 10/10, usages d'acquireLock, FileSource et lockRowForWrite identiques, **Catalog : 774 sites en production, 733 sur la copie, les 41 manquants tous dans `extension/rag3weaver/tests/*.rs`** (de vrais appels `Catalog::new(…)`), invisible_usages 1 contre 0 : **le premier passage de l'écrivain sur une base migrée perd des usages, ceux des tests.** Défaut confié à ED (piste : `use rag3weaver::Catalog` dans tests/ résolu comme un import de bibliothèque, USES_LIBRARY), copie fermée gardée (`mesures/repetition-versions-code-rag3db-depot-20261119-194739`), la production intacte. La bascule attend ce correctif et une répétition verte, demain ; la fenêtre C++ de demain prend la quarantaine de hotfixs et C1 si le cœur l'a. Leçon : la répétition a attrapé deux défauts de données en une soirée avant qu'ils touchent les vraies bases ; le critère juste se mesure contre la production au même arbre, pas contre un relevé d'avant que l'arbre a dépassé.
- 20:31 : **les 41 usages perdus ne sont pas l'écrivain : une régression d'analyse, que la bascule révèle.** ED, sur un reflink de la copie de G : la fonction de tests est intacte, mais `Catalog::new(…)` sur un type importé du crate (`use rag3weaver::Catalog`) est retenu dans `pending_qualifier_types` sans qu'aucune MENTIONS vers `Catalog` ne soit posée ; l'analyse plus ancienne qui a indexé la production posait une mention « devinée » du type, et toute réindexation d'aujourd'hui la perd, avec ou sans versions (dans src/, le même appel se résout autrement). Décision c7 : `usages Type` compte les sites `Type::méthode(…)` d'un type importé dans tous les langages ; correctif chez codeparsers, groupé dans sa pile mots vides + locales (une seule repasse) ; **la bascule attend ce correctif** (sinon elle fige une base amputée), la répétition se rejoue dessus. ED cherche le commit qui a changé la règle. Leçon : une repasse d'analyse peut amputer des réponses en silence ; la sonde de marque et un banc d'usages avant/après sur la vraie base en sont les gardes. **Interrupt** (recherche) : le témoin de bout en bout par le chemin de l'hôte a trouvé un vrai défaut dans le lot déposé une heure plus tôt : `RevocableConnection` relayait tous les verbes sauf `interrupt` (un appel arrêté allait au bout, 19,8 s) ; corrigé, redéposé (recherche-interruption-2) ; la règle « une promesse d'outil se prouve par le chemin de l'hôte » gagne un exemple. **Atterris** : embed-parity (0853b6939f, PR #63), le ticket des usages perdus (df41fc5b4b), le ticket du rangement déclaré (721e38e9a4 : les outils de l'équipier définis par l'hôte migreront vers le gabarit). **Déposés** : B3 de mémoire (l'invite d'extraction réduite à une consigne et une part, les règles dans les descriptions des champs, le script d'invite retiré ; parité sur vrai modèle confiée au banc), L3 et L4 du mode pair (send_to_session, l'affichage dans l'IDE), strict-borne-2 d'arbre (son témoin L4 marqué), la déclaration du débit par poste (embarquement), l'estimation côté hôte (arbre), la page de l'intra-scope (codeparsers), la tranche 1 de l'IDE en fenêtres (tty, relue par ED). Campagne : six cas jugés (trois réussis, un dégâts, deux échoués dont un par la consigne, corrigée).
- 20:50 : **Lucie ouvre la tablette dès 20 h 35** (« j'ai fini ma journée Sairen ») : `gh auth switch --user L-Defraiteur` permis sur la tablette sous la garde du login, travaux lourds permis, le compte trouvé remis avant 8 h ; dit aux treize sessions. G passe la tablette à **2 places lourdes** pour la nuit (32 cœurs, 15 par lourd, plafond 32G, 71 Gio disponibles ; retour à 1 à 7 h 30 par minuteur systemd). **Atterris** (vérifiés) : la chaîne 2a′-3 d'arbre (pile-arbre-2a3-2 f78147a7ff, PR #66, treize commits : l'écrivain par couche, un index fini publie sa génération, une couche d'histoire sans checkout, la couche de travail d'un worktree, le plafond des couches de travail), la garde du compte rendu de l'équipier (tty, ffbb5409b5, PR #65, neuf commits), deux lots du pousseur (les scripts d'un train rust ont leurs témoins joués avant la passe c470e510fd ; la porte des tickets pour les docs et les rust 10aa3aac30), le ticket du lecteur de clé primaire mis à jour par TSan. **TSan ×3 sur DdlEveryMinute** (cœur) : TSan ne voit pas C1 (ASan puis une sonde dans appendString suivent), mais nomme une course réelle : le point de reprise V2 fait `push_back` dans le vecteur des groupes de morceaux pendant qu'un lecteur par clé primaire le parcourt sous un autre verrou : un accès après libération, cause probable du ticket « sous vingt écrivains, un lecteur meurt dans l'index de clé primaire ». **Mode pair** : L3 (send_to_session) et L4 (l'affichage dans l'IDE) déposés ; les outils de l'équipier définis par l'hôte migreront vers le gabarit (ticket du rangement déclaré, décision « tout déclaratif »). **Mémoire** : B3 déposé (l'extraction par les parts, l'invite réduite ; parité sur vrai modèle confiée au banc), B4 décidé (`format: {template}` sur une part ; le Decide en parts, parité du banc avant le dépôt), B6 en cours. Arbre mesure la place des couches (doublon contre (3a)/(3a′)) par une sonde sur un reflink de la copie de la répétition, au portable. Campagne : six cas jugés, deux à rejouer (consignes corrigées).

## 19 novembre, 21 h 20 — le tri des worktrees, et la règle du dossier par session

Lucie, 20 h 5x, devant `~/git_workspaces` : « il y a masse de worktrees là, c'est normal ? » Relevé figé : 157 worktrees
rag3db et 10 lucivy à la tablette, 185 et 16 à luciepc, à plat dans `~/git_workspaces` et `~/.cache/rag3weaver-build`.
Son ordre : « oui stp absolument, et plus jamais laisser un worktree qui a déjà été mergé sans le supprimer ensuite ;
écoute que chaque session dise explicitement lesquels elle tient actuellement, et on garde que ceux-là + rag3db +
sairen, et basta ».

**Le tri (21 h 00 → 21 h 14).** Chaque session a envoyé sa liste « je garde » (chemin, lot en file ou travail en
cours, target) et a retiré ses autres elle-même, après contrôle (propre, tête sur une branche d'origin ou dans master,
aucun processus) : arbre 35, ED 28, cœur 22, lucivy 22, banc 21, archi 20, embarquement 18, codeparsers 14, mémoire 9,
hotfixs 9, recherche 6, tty 2. Deux sessions (G, 33 ; recherche, 24) ont vu leur `git worktree remove --force` refusé
par le contrôle de permissions de leur session et ne l'ont pas contourné ; codeparsers aussi, puis Lucie lui a ouvert
la permission. Puis c7 a purgé tout ce qui n'était dans aucune liste, sur les deux postes, avec le même contrôle par
arbre : 62 rag3db + 2 lucivy + 1 à la tablette, 40 rag3db + 1 lucivy à luciepc ; trois têtes qui n'étaient sur aucune
branche (rag3db-equipier, rag3db-hote-socket, -2, luciepc) poussées sous `archive/<nom>-19-11-2026` avant retrait ;
le seul diff non commité (rag3db-paquet-npm, 4 lignes du 11 oct.) sauvé dans `c7-archives-menage-19-11-2026/` ; 15
journaux à plat effacés à la tablette, 426 (7,5 Mo, derniers du 12 oct.) déplacés à luciepc dans le même dossier.
Restent 25 worktrees rag3db à la tablette et 24 à luciepc, tous dans une liste. Place libre : tablette 312 → 385 Go,
luciepc 883 → 1 100 Go (avec les targets retirés par chacun : ED 172 Go, banc 97, tty 51, arbre 49, recherche 42…).

**Une erreur à retenir.** Ma liste « les tiens que je vois » envoyée à chaque session était une supposition par le
nom, pas une attribution vérifiée : archi a retiré `wt-lib-arbre` (l'arbre de bâti de `batir-lib-maitre.sh`, le sien
en fait) sur sa foi. Dommage limité, le script recrée l'arbre, build/lib se rebâtit. Le propriétaire d'un arbre se lit
dans ses commits et son reflog, jamais dans son nom.

**La règle, 21 h 10.** « Nouvelle règle : les worktrees d'une session vont horodatés sous son propre nom,
`git_workspaces/tty/19-11-2026…`, que ce soit ici ou sur luciepc. » Et : « mon dossier c'est sairen pour le taf,
rag3db pour le projet perso, tout le reste de git_workspaces c'est des artefacts. » Diffusée aux treize sessions à
21 h 19 : tout worktree neuf va dans `~/git_workspaces/<session>/<jj-mm-aaaa>-<lot>`, sur les deux postes, plus rien à
plat ni sous `~/.cache/rag3weaver-build/wt-*` ; ceux qu'on tient se déplacent quand on peut (le sous-module refuse
`git worktree move` : on recrée sous la règle au lot suivant) ; le retrait dans la minute de l'atterrissage tient ;
les créneaux du pousseur (`rag3db-pusher`, `-2`) restent ; les arbres du lanceur de campagne et des juges suivent la
règle (lot à G). Les modèles gpt-oss-120b et ornith-122b (134 Go) effacés par Lucie elle-même ; qwen3-1.7b gardé.

**Reste à son mot** : `~/.local/share/rag3weaver` à la tablette, 70 Go dont 4 de vraie base (36 Go d'instantanés de
bascule de G, 14 Go d'arbres de tâche de l'hôte, 5 Go de retour arrière, 4 Go de preuve WAL du 8 nov., 4,5 Go des
bases de mes deux worktrees retirés, 2,3 Go d'un instantané du 12 oct.) ; les copies de mesure du banc à luciepc
(433 Go apparents), oui donné à 21 h 09, transmis au banc.

## 19 novembre, 21 h 31 — les 538 Go de la tablette, et le lot de bâti

Lucie, 21 h 2x, son disque : « mon Steam prend 172 Go, après je sais qu'il y a git_workspaces, mais à eux deux j'ai du
mal à croire que ça me prenne tant de disque ». Relevé réel : Steam 188, jeux hors Steam 58, cache de bâti
`~/.cache/rag3weaver-build` 166, git_workspaces 64 (sairen 51), données `~/.local/share/rag3weaver` 56, baloo 26
(l'indexeur de KDE indexe nos bâtis), caches divers 57. Les sessions tenaient environ 250 Go.

**Effacé sur son oui, puis son « je t'autorise explicitement » (le classifieur de ma session avait refusé le premier
`rm`)** : la preuve WAL du 8 nov. (tickets fermés, 4,2 Go), l'instantané du 12 oct. (2,3), mes deux bases d'essai et
leur manifeste (4,5), ma tâche d'essai de l'hôte (4,2), les six instantanés de bascule de G des 16 et 18 nov. et les
doublons de contrôle du 19, plus la copie du retour arrière validé (28), dix dossiers de preuves Codex des 12 et
13 nov. (15), la copie de répétition avortée de 19 h 42 (4,3), la bascule-live du 16 (1,4). `~/.local/share/rag3weaver`
passe de 70 à 28 Go ; place libre de la tablette 312 → 406 Go. La tâche « atelier » de l'hôte (agent Algareth, 15 nov.
16 h 15, 5 Go) n'est à aucune session, à Lucie de dire ; ma purge de 21 h 11 avait retiré son arbre, recréé à
l'identique à 21 h 24 (tête ad9bf16185 dans master).

**Pourquoi les targets pèsent** : le target d'archi, 31 Go, c'est 20 Go de cache incrémental (cinq graphes de 0,6 Go,
un par variante de la crate, jamais balayés) et 11 Go de rlib et binaires ; les symboles sont déjà réduits (lignes
seules, rien pour les dépendances depuis le 10 oct.). Lucie : « il n'y a rien de plus opti que cargo ? … sinon cron
le matin pour rebuild avec cache reseté ». Lot de bâti donné à G (« oui oki go ») : sccache partagé entre les treize
targets (plafond 40 Go par poste), `cargo sweep --time 2` au ménage de 07 h 30, incrémental coupé dans ce qui joue
une passe une fois (le pousseur et la passe épinglée l'avaient déjà ; le trou était le lanceur de campagne : lot
campagne-incremental-off déposé, 8/8), mold comme éditeur de liens. Lucie a posé sccache 0.19.0, mold 3.0.0 et
cargo-sweep 0.8.0 sur les deux postes (21 h 31). Le ménage programmé attend sa décision : la session de G refuse
d'écrire un script qui effacerait à sa place après deux refus de permission ce soir ; je lui ai recommandé de donner
à G une règle de permission sur `~/.cache/rag3weaver-build` et `~/.local/share/rag3weaver`.

## 19 novembre, 22 h 15 — deux causes trouvées le même soir : le raisonnement sans borne, et les mots perdus

**Le ménage par marque est sur master (94be61f21a, 22 h 04)**, avec la permission que Lucie a donnée à G dans sa
session, et dans le sens qu'elle a retourné à 21 h 37 : « ou bien ces instantanés sont marqués tests car ils vous
servent à vous, plutôt que l'inverse ? ». Par défaut rien ne se touche ; seul un dossier portant `.jetable.json`
(famille, garder 3, âge 72 h) peut partir ; les trois plus récents d'une famille restent toujours ; un dossier tenu
par un processus reste ; minuteur 07 h 30 aux deux postes, balayage `cargo sweep` des targets. Les marques sont posées
sur les instantanés de bascule (à la création désormais), les copies de répétition (arbre a marqué les siennes), et
816 dossiers Codex du cache. sccache et mold sont posés sous poste (b3e414a640) ; mesure d'avant au pousseur : 814 s
par passe rust, target 7,5 Go ; l'après se mesure sur trois passes.

**Algareth n'écrit pas : la cause.** lucivy 03 rejoué à 20 h 50, le premier jeu propre de la soirée (consigne, cargo,
Vertex en ordre : 8,7 s par appel, 146 appels) : il trouve le bon endroit (`verify_stored`, la distance d'édition sur
tout le texte stocké), lit 99 fichiers, n'écrit rien, s'arrête sur « 3 pas sans élément réussi ». Archi lit la trace
(docs/rag3weaver-archi/19-11-2026/06, 46389f71ad) : quatre pas sur cinq finissent sur MaxTokens sans appel d'outil,
~3 930 jetons de raisonnement pour 162 de texte sur un plafond de 4 096 ; le gabarit de code ne déclare aucun modèle,
l'équipier tourne sur le défaut de l'hôte sans reasoning_effort, et Gemini 3.x réfléchit jusqu'à remplir la sortie ;
une réponse coupée sans outil termine le pas ; trois pas coupés d'affilée l'arrêtent, juste après qu'il a validé sa
conception au pas 4. **La même cause chez l'archiviste** : B3 (l'extraction par parts), 5 fenêtres sur 19 en échec,
JSON coupé, `extract_max_tokens` 8 192 partagé avec le raisonnement (mémoire l'a lu dans openai_llm.rs:401 ; le banc a
vérifié que ce sont des échecs d'appel, pas des fenêtres vides : P1 0,704 avec 26 % d'échecs). Décidé : une seule clé
déclarée `reasoning` (minimal, low, medium, high → reasoning_effort), sur LlmNode (lot de mémoire sur B6, avec le refus
explicite « cut at max_tokens=N (finish: length), reasoning R of C ») et sur le modèle de l'équipier (lot d'archi demain
matin : le gabarit déclare le modèle avec reasoning low et max_output_tokens, relance déclarée et bornée sur réponse
coupée, règle des trois pas qui ne compte que les pas sans rien fait), défaut de l'hôte dans models.llm.reasoning ;
archi et mémoire s'accordent sur le nom avant le premier témoin. Aucun rejeu de cas avant. Le banc mesure ensuite b3
avec low, minimal, et la borne à 16 384.

**Les mots perdus de lucivy : la cause, prouvée (PR lucivy #54).** Pas un segment raté : la table des créations en
attente du dictionnaire n'oubliait jamais sur le commit des shards (l'époque ne tournait que dans `prepare_commit`,
le chemin de `ShardedHandle` n'y passe pas) et répondait sans vérifier le texte ; un mot dont la compaction avait
retiré le texte reprenait son ancien id, sans texte. Témoin déterministe : 2 perdus sur main, 0 avec ; boucle serrée
de compactions sur 600 réécritures : 383 → 0. Correctif : `flush_workers` fait tourner l'époque, la compaction retire
les ids qu'elle jette ; le sursis reste en filet. Lucie a autorisé la 4.3.12 dans la session lucivy. Suite : le
pointeur 4.3.12 remplace le lot 4.3.11 en file ; la compaction, coupée sur les hôtes depuis 17 h 44, se rallume après
une nuit de sonde sur 4.3.12.

**2.12** : parité d'ED sur index neuf, 0 changé, 0 perdu, mais `rag3db_database` toujours hors du top 20 : d'autres
namespaces vides, faits de déclarations anticipées, passent les 80 caractères. Validé : la longueur se mesure sur ce
qui reste une fois ôtées les lignes de pure structure, déclarées par motifs et par langage dans `NotCandidate` ;
témoin rouge sur bound_standalone_call.h, parité rejouée, le critère reste le top 20.

## 19 novembre, 23 h 53 — lucivy 4.3.12 atterrie dans rag3db ; la soirée de sccache

**Le pointeur** : lucivy 4.3.12 publiée partout à 22 h 46 (5 crates, 2 npm, PyPI, PR lucivy #55 avec la cause #54, le
sursis #50, le rapport #52), et son pointeur atterri dans rag3db à 23 h 52 (c42baa7c0f) avec la note de maintenance
qui porte `dictionary_revived` et la phrase de l'état qui compte les textes repris du sursis. Suite : G bâtit le binaire
de l'hôte et réveille les hôtes des deux postes dessus ; lucivy joue sa sonde de nuit ; demain la compaction se rallume
dans les huit manifestes (coupée depuis 17 h 44), puis une seconde nuit de sonde.

**La soirée de sccache.** Le premier train rust sous sccache + mold (recherche-filtre-vide, 22 h 56) rougit la lib
(code 101) : deux tests qui lancent cargo **dans le bac à sable de commande** meurent, « sccache: error: Permission
denied (os error 13) », parce que le bac hérite de `RUSTC_WRAPPER` du poste et ne peut ni créer le dossier de sccache ni
joindre son serveur. Le train suivant (poste-place-pousseur, parti sous sccache aussi) passe vert : le rouge est
intermittent, au démarrage à froid. Mais le défaut du produit est réel (une utilisatrice qui compile par sccache aurait
le même rouge en service) : ticket de G (db61938fbe), correctif à archi demain matin en premier, qui **ferme la porte**
(le bac à sable ne laisse passer que les variables qu'il déclare ; ouvrir le chemin vers le serveur sccache du poste
ferait sortir l'agent de son confinement). D'ici là : sccache en opt-in dans poste (`POSTE_SCCACHE=1`, lot
poste-sccache-opt-in, 42/42), mold reste ; après le correctif, sccache par défaut et la mesure sur trois passes
(avant : 814 s par passe rust, target 7,5 Go). Deux erreurs de ma part dans la soirée : j'ai cru la file figée par les
places lourdes (c'était la recopie de menage.py, dont le témoin .py joué par bash appelait l'`import` d'ImageMagick qui
attendait un écran ; corrigé b61a91caed), et j'ai cru les journaux de passe écrasés (chaque train garde son dossier ;
seul le chemin manquait au message rendu à l'auteur, 4197a0ec9e). Posé aussi : la place lourde réservée au pousseur
(c26690ff4c, déclarée par `place-pousseur` à luciepc seulement, dc52d02aa7).

**Prêts pour demain** : la fenêtre C++ (hotfixs quarantine-last-reader-checkpoint 4b6fa1b91f, passe verte, remplace
38eacb2664 ; cœur course-des-groupes e6a38eccd2 si son TSan est vert) ; le bac à sable puis le lot reasoning d'archi
(une clé `reasoning`, models.llm sans model, 702a3adb74) ; la mesure du raisonnement de l'extraction au banc sur la tête
de mémoire (memoire-extraction-cut e94b15ca2) ; le correctif des usages qualifiés de codeparsers, puis la répétition
rejouée et la bascule des bases.

## 20 novembre, 00 h 30 — les hôtes sur lucivy 4.3.12, et deux dégâts de ma purge

**La fenêtre de l'hôte de G** : binaire 60cff306c4 (lucivy 4.3.12, lib 6e2476d41 inchangée, essai 7991 vert aux deux
postes, sans sccache) posé et réveillé : portable 9 s de coupure (00 h 25 49 → 00 h 25 58), luciepc 36 s. Les trois
bases ouvertes sans avertissement, File, Library, Scope prêts, vecteurs 100 %. Lucivy lance sa sonde de nuit.

**Deux dégâts de ma purge de 21 h 14, les deux à luciepc.** (1) `~/git_workspaces/rag3db-lourd` n'était dans aucune
liste de session : c'était la **racine indexée de la base code-lourd de Lucie** (manifeste source working_tree). Le
nouvel hôte a refusé le manifeste (« la racine n'existe pas »), la base est restée intacte et fermée ; recréé à
00 h 26 au même chemin, détaché sur la tête exacte qu'il avait (a72acfeef0, lue dans mon journal de purge), déclaré dans
la liste gardée hors règle d'horodatage. Règle apprise : une purge croise d'abord les chemins avec les racines des
manifestes (`~/.config/rag3weaver/*/templates/backends/*/*.json`) ; une racine indexée est à Lucie, pas à une session.
Relevé après coup : toutes les racines des manifestes existent sur les deux postes, sauf une base d'essai de
codeparsers (base-avant) déjà morte avant ce soir. (2) `~/git_workspaces/rag3db-docs-G` à luciepc, dans aucune liste
non plus : le filet de pose de l'hôte y lisait `installer-l-unite-de-l-hote.sh`, d'où le premier passage rouge et les
36 s ; G a repris avec l'installeur de master lu dans le worktree du pousseur, et corrige bascule-pose.sh pour ne plus
lire un worktree à plat.

**Dans la file** : ed-estimate rendu à ED (conflit à la repose dans index_nodes.rs) ; les lots de recherche
(notification « index prêt » et son e2e par le binaire réel, sucre de nom, page de diagnostic, cas de campagne 03
avec un témoin shell : archi fait accepter un témoin .sh/.py au lanceur demain, derrière le bac à sable) ; codeparsers
redépose sa pile avec les usages qualifiés dans la nuit (cause : rien ne posait de rendez-vous vers le type d'un
appel `Type::f()` ; trois témoins rouges vus).

## 20 novembre, 01 h 42 — la répétition est verte : la bascule des bases se fait à 06 h 45

**Le verdict de G.** Répétition complète sur une copie neuve de l'instantané d'avant 1b, hôte baccb2ea6e
(stop-words-locals, codeparsers b67ad75, règles de l'analyse 9, lucivy 4.3.12) et lib 6e2476d41 : départ 01 h 08 47,
repassage de 01 h 20 52 à 01 h 40 08 (19 min 16 s) jusqu'à « indexation terminée », RSS maximal de l'hôte 9,7 Gio, aucune
erreur de tampon. Le juge, contre la production du même arbre (56b5d73142, encore sous l'ancienne analyse) : **les 41
sites de `usages Catalog` perdus hier sont revenus, 0 manquant** ; aucun site de la production perdu sur les quatre
noms (acquireLock 8/8, FileSource 133/133, lockRowForWrite 3/3, Catalog 788 → 1 129 : 0 perdu, 341 gagnés, et 12 tirés au
hasard sont tous de vrais `Catalog::new(`, `Catalog::rechercher(`, `Catalog::Get(`) ; search_code : recouvrement 97,8 %,
même premier 10/10. Le « critère manqué » du script compare à l'instantané de 17 h 50, sous l'ancienne analyse : écarté
comme convenu. Copie gardée et marquée jetable : `mesures/repetition-versions-code-rag3db-depot-20261120-010847`.

**La fenêtre du matin, ordre de G** (accepté) : (a) 06 h 45, annonce ; les sessions tiennent leurs lourds de 06 h 45 à
09 h 30 aux deux postes, tablette entière ; pose de baccb2ea6e aux deux postes (les hôtes tournent encore sur
60cff306c4 ; l'essai est déjà vert) ; si 2.12 d'ED a atterri d'ici là, répétition rejouée sur son binaire d'abord
(35 min) ; (b) 07 h 00 ou 07 h 35, **la bascule des trois bases de Lucie sur exactement le couple répété**,
code-rag3db-depot d'abord, puis code-lourd et code-principal en parallèle, fin vers 08 h 00 ou 08 h 30, critère
compris, retour arrière prêt 72 h ; (c) au « à toi », le train C++ en un seul push par la garde de cœur (branche
train-20-11 : la quarantaine du dernier lecteur de hotfixs 4b6fa1b91f, puis la course des groupes de morceaux de cœur
501d7ccc90, passe de la tête commune jouée cette nuit à luciepc, verdict vers 05 h 30) ; (d) la lib bâtie sur cette
tête, verdict, épingle, contrôle des deux sens, pose et réveil, fin vers 09 h 30. Pourquoi la bascule avant la lib : la
bascule se fait sur ce qui a été répété, jamais sur un autre binaire ; le train C++ touche les points de reprise et les
lecteurs, pas le format, et sa lib passe ensuite sur des bases déjà migrées, avec ses propres garde-fous.

**Cette nuit, en parallèle** : la parité 2.12 du banc (6ff7d11cae contre a542e0f518, fin vers 02 h 45) puis la mesure
du raisonnement de l'extraction (low, minimal, borne 16 384, sur la tête de mémoire e94b15ca2, garde-fou 06 h 30) ; le
rejeu du cas lucivy 04 d'archi (fin avant 03 h) ; le lot des paquets par leur nom de codeparsers (Rust, TS, et les
réexportations Python/TS dans le même lot, sous la sonde de marque). Embarquement a mesuré le point C : avec les lignes
de contexte, **72,5 % des morceaux de code dépassent la fenêtre de granite** (512 jetons) ; lot à ED après 2.12 : la
taille de morceau déduite de la déclaration de l'embarqueur (fenêtre × marge ÷ ratio déclaré par modèle), tenue comme
propriété de la base, témoin qui compte en Rust avec le tokenizer de granite.

## 20 novembre, 03 h 45 — la répétition avec 2.12 : le seuil tenu par la preuve, bascule sur 1f22743e17

2.12 atterri à 03 h 00 (1f22743e17, 16 commits, règles de l'analyse 10 ; rendu une fois à la garde pour un module
de test sans la feature `code`, corrigé en dix minutes). G a rejoué la répétition dessus : repassage 27 min, RSS
8,9 Gio, aucune erreur de tampon, 0 usage perdu sur les quatre noms, les 41 sites présents, même premier 10/10 ;
recouvrement du top 10 contre la production **88,5 %**, sous le seuil de 90 %. Mais l'écart n'est pas celui de la
bascule : la copie de 2.12 diffère de la production exactement comme elle diffère de la copie baccb2ea6e (même
intersection, requête par requête), et baccb2ea6e tient 97,8 % ; ce qui sort, ce sont des fichiers-conteneurs
(`tests/e2e/main.rs:1`, `re2/include/mutex.h:69`), ce qui entre, des définitions proches de la question : l'effet
voulu de 2.12, déjà jugé vert par la parité du banc sur base existante et index neuf (rag3db_database au rang 8).
**Décidé (c7) : bascule à 06 h 45 sur 1f22743e17, règles 10.** Le seuil n'est pas relâché, il est tenu par la
preuve qui isole l'effet de 2.12 ; la page de bascule de G le dira requête par requête. Un seul repassage des bases.

**La nuit autour** : la mesure du raisonnement du banc (383b8844ba) : `low` à 8 192 supprime la coupure (0 sur 19,
0 jeton de raisonnement, 10,8 s par fenêtre contre 24 à 51), 16 384 n'apporte rien, `minimal` refusé par Vertex
(mémoire le traduit en low, comme le client Anthropic), P1 ± 0,07 d'un jeu à l'autre, et 48 % de cache quand
l'invite est rejouée mot pour mot : le levier de B4, vert en branche, parité au banc (S 50 avant 06 h 45, F300
après 09 h 30). lucivy 04 rejoué avec cargo dans le bac : réussi et vérifié par l'agent (2 pas, 75 appels, 438 s,
un correctif de sept lignes, témoin rouge au parent et vert avec). La sonde de nuit de lucivy par le chemin de
l'hôte : 534 ids perdus en 4.3.10, 0 en 4.3.12. Codeparsers a déposé les paquets par leur nom (Rust, TS, et les
réexportations Python/TS, 5 493 arêtes de « nom » à « import », aucune retirée, règles 11), tenu hors prise par G
jusqu'après la fenêtre. ED : la borne des morceaux se mesure au tokenizer (le C++ du moteur monte à 0,8 jeton par
caractère dans les tables d'en-têtes, un ratio seul ne tient pas) : forme retenue, la borne en octets devant puis
le comptage en jetons avec le tokenizer du plus petit embarqueur de la base, sans repli silencieux (tokenizer
absent = refus dit). Rendus de 03 h 41, tous des conflits de repose : la chaîne d'attente de tty, la garde de dette
d'arbre, define-ref d'ED ; chacun repose.

## 20 novembre, 04 h 45 — retournement : 2.12 casse la garde des comptes du plein texte ; la bascule part sans lui

À 04 h 25 le lot c-durable-3 de recherche rougit en premier créneau sur trois tests `en_fichiers`
d'e2e_tx_par_paquet_arret (« une base saine ne se rebâtit pas », « plus rien à rebâtir », « 300 documents pour 301
lignes »). Départage de recherche : **master nu est rouge pareil**, son lot était 6/6 la veille sous la même lib ;
lucivy lit le journal et ne voit pas comment 4.3.12 perdrait un document. Puis la lecture de G dans ses deux
répétitions tranche : sur 1f22743e17 (2.12), la réouverture dit « plein texte de Scope : 67 610 documents pour 75 246
lignes — index rebâti depuis les lignes » (22 s) et le journal d'index finit sur « écart entre documents et lignes —
Scope : 94 469 pour 103 999 » : **chaque réouverture rebâtirait le plein texte** ; sur baccb2ea6e, qui porte déjà
lucivy 4.3.12 mais pas 2.12 : aucun rebâti, aucun écart. Cause probable : un conteneur non candidat garde sa ligne
mais plus de document de plein texte, et la garde des comptes lit un index cassé.

**Décidé (c7, 04 h 45)** : la bascule de 06 h 45 se fait sur **baccb2ea6e + 6e2476d41** (répété vert, 97,8 %, premier
10/10, 0 usage perdu, aucun écart), pas sur 1f22743e17 ; l'hôte de la fenêtre de lib se bâtit aussi sur baccb2ea6e
tant que le correctif n'est pas atterri (aucun hôte posé ne porte l'écart) ; ED corrige 2.12 en priorité, rouge
d'abord sur les trois tests de master, et 2.12 fera repasser les bases à sa fenêtre avec les deux montées de règles
qui attendent (paquets de codeparsers, morceaux d'ED). Lucivy et recherche hors de cause ; lucivy confirme sur les
tests (pointeur^ contre pointeur) et a ouvert le ticket, qu'elle passe à ED. Le train C++ : passe de la tête commune
verte (501d7ccc90, témoins 6/6 ×3, transaction 411, e2e_test 1 867) ; la lib est déjà bâtie et publiée sans courant
(symboles 0/0 contre 6e2476d41) par le nouveau batir-lib.sh, qui a créé et retiré son arbre sous ~/git_workspaces/G/.

## 20 novembre, 06 h 15 — master vert de nouveau, la fenêtre peut s'ouvrir

**Le correctif de 2.12 est atterri à 06 h 14 (82837aec86)**, et il y avait trois causes, pas une : la garde des
comptes du plein texte comparait les documents à toutes les lignes, la garde du lecteur (`fts_rebuild_percent`)
aussi, et l'avancement du rebâti de même ; les trois comptent désormais les lignes candidates
(`Catalog::candidate_rows_of`), et les invariants des tests jugent par la règle elle-même. e2e_tx_par_paquet_arret
18/18, lib, e2e_code_sync, e2e_lectures, check_features 8/8. Le ticket de lucivy est fermé dans le lot ; recherche a
redéposé c-durable-3 tel quel. Rien ne change pour 06 h 45 : bascule et hôte sur baccb2ea6e, déjà répété ; le master
corrigé se répète après la fenêtre, avec les deux montées de règles (paquets 11, morceaux 12), pour un seul repassage.

**La nuit des reposes.** Après trente-six atterrissages entre 21 h et 06 h 15, les lots déposés tôt ont presque tous
buté à la repose : index_nodes.rs (ed-estimate, schema-resume), le gabarit mémoire (define-ref, trois fois : la
chaîne des tranches et l'expiration des tours arrivaient au même endroit), teammate_tests.rs (la chaîne d'attente
de tty contre le témoin de la porte d'ED), worktrees.rs (la garde de dette d'arbre), le bundle généré de l'IDE
(chat-reload-epoch, et derrière lui les six lots des fenêtres de tty). Chacun a reposé et redéposé dans l'heure, à
son rang (la règle d'ab2cb02d94 a joué, cascade comprise). Trois rouges de la garde des features (2.12,
tests-never-skip-silently, chat-waiting), tous des cfg manquants vus par le pousseur et jamais par l'auteur : lots
demandés à G pour après sa fenêtre : attestation check_features à l'enqueue d'un lot rust, fichiers générés déclarés
(bundle.json) rebâtis à la repose au lieu de rendre le lot, les docs devant les rust. Atterris dans la nuit, entre
autres : la chaîne des tranches de mémoire (lecture, expiration, chaîne), define_ref, le préfixe stable du décideur,
le cache des appels de modèle, le retry sur schéma changé, le journal plafonné, le dialecte postgres à générations
(2.11 fermé), la chaîne d'attente de tty (hôte, chat, IDE, boîte), le profil léger du plein texte (−29 %), le
rechargement du chat, la garde de dette d'arbre, lucivy 04 jugé réussi et vérifié, la mesure du raisonnement (low),
la parité B4 (S 50), le lot 3 du lanceur. Déposés et tenus : B4, la version du gabarit, les graphes sur disque et
check_proposal de mémoire (la boucle d'édition du workflow, page docs/memoire/20-11-2026/01), le paquet de
codeparsers (tenu par G), les morceaux d'ED (tenu jusqu'à 09 h 30), l'index pas prêt dans l'IDE (tty).

## 20 novembre, 07 h 38 — la bascule : le dépôt à versions, les deux bases de luciepc revenues en arrière

**La fenêtre de G**, annoncée à 06 h 40 aux douze sessions (lourds tenus de 06 h 45 à 09 h 30). Hôte baccb2ea6e posé
et réveillé au portable (6 s de coupure) et à luciepc (1 s). **code-rag3db-depot basculée** : relevés d'avant sur la
base vivante, fermeture qui tient, instantané `avant-versions-20261120` (jetable, RETOUR.sh prêt), contrôle des deux
sens sans écart, 1b en 681 s, générations en 3 s, rouverte à 06 h 55 (12 min 22 s de coupure), repassage en 16 min
jusqu'à « indexation terminée », vecteurs prêts (179 864 morceaux). **Critère tenu** : recouvrement 98,8 %, même
premier 10/10, 0 usage perdu (Catalog 789 → 1 133), aucun « rebâti depuis les lignes », aucun « écart ».

**code-lourd et code-principal**, lancées en parallèle à luciepc à 07 h 12 (1b 467 s et 597 s, rouvertes à 07 h 21
et 07 h 22, repassage fini à 07 h 31 et 07 h 35) : **rouge au critère**, recouvrement 88,0 %, même premier 8/10, et
40 et 43 usages de Catalog « perdus ». Départage de G en dix minutes, sans rien toucher : les usages sont un **faux
rouge de la mesure**, le rendu de `usages` a quatre sections et la section « non attribués » est plafonnée à 200
affichés des deux côtés (466 → 739 en vrai, total 610 → 896) : les sites « perdus » sont sortis des 200 montrés, pas
de la base ; 0 usage attribué d'avant absent d'après. Le 88 % du banc, lui, n'a pas de cause démontrée : la racine
de ces deux bases est un arbre du 18 octobre (284cc00c0b) qui contient encore `experiments/mtga/`, et quatre scripts
Python `build_*_variant.py` entrent dans le top 10 de « index du plein texte » sur les deux ; piste : les scopes
Python de cet arbre ancien repesés par la nouvelle analyse (règles 9), ce qui expliquerait 98,8 % sur le dépôt neuf
et 88 % ici avec le même binaire. **Décidé (c7, 07 h 36) : retour arrière des deux bases de luciepc** sur leurs
instantanés, le dépôt reste basculé ; on comprend après, sur une copie reflink, règles 8 contre 9 requête par
requête (codeparsers prévenu) ; la re-bascule de ces deux bases à la fenêtre suivante, sur un juge qui l'explique.
Le push du train C++ et la lib partent après le retour.

**Le ménage de 07 h 30** a joué son premier passage sur les deux postes : les dossiers Codex marqués jetables et
vieux de plus de 72 h, hors des trois plus récents par famille, effacés un par un au journal.

## 20 novembre, 08 h 51 — le train C++ sur master, la lib épinglée par une preuve transférable, la fenêtre finie

**Le train C++** (cœur : la quarantaine du dernier lecteur au point de reprise, l'annulation inachevée, la course des
groupes ; hotfixs : le lecteur mort dans l'index de clé primaire sous vingt écrivains) est parti sous `poste push` dès
le retour arrière des deux bases, à 07 h 38. La garde s'est arrêtée une fois : un conflit sur `docs/nouveautes.md`,
que le pousseur fusionne par union à la repose et que la garde ne savait pas fusionner. Lot de G dans la minute
(a59febb911, 07 h 43 : la garde fusionne nouveautés par union, tout autre conflit arrête), puis push à 07 h 42, master
44c14e1c38 ; le commit moteur, rebasé par la garde, est **4ad394f4d2** (le même contenu que 501d7ccc90 bâti et jugé
dans la nuit).

**La lib par une voie transférable** : la nuit avait bâti et jugé la lib de 501d7ccc90 (e2e_code 36, e2e_lectures 28,
e2e_demon_hote 26, hôte baccb2ea6e), et master porte 4ad394f4d2. Plutôt que de rebâtir sous la fenêtre, on a prouvé
que les deux sont la même lib : `git diff 501d7ccc90 4ad394f4d2 -- src test extension/vector tools/rust_api cmake`
vide (zéro ligne de source), et les deux `.so` ne diffèrent que de 125 octets, trois chaînes BUILD_ID et le build-id
GNU. Le verdict de la nuit est transféré ; épingle 501d7ccc90 à 07 h 51 (47aaaebef1), corrigée en 4ad394f4d2 à 07 h 57
(1b02372432), l'une et l'autre par G.

**Le pousseur à l'arrêt sur la lib** : à 07 h 59, le premier lot rust pris après le train (recherche-interruption-2)
est rouge d'un ROUGE lib, et le pousseur dit « lib en retard : épinglée sur 6e2476d41b, master a changé le moteur » ;
il tient les rust et pousse les docs, quinze d'un trait entre 08 h 00 et 08 h 09 (cas de campagne, intra-scope, place
des couches, piège du BEGIN, le cadrage du plein texte et ses trois relectures, le lot suivant par famille, deux
tickets fermés, la page du workflow de mémoire, la mesure du raisonnement). L'épingle 1b02372432 atterrit à 08 h 49,
« lib à jour : les lots rust reprennent » à 08 h 49 ; les hôtes posés sur baccb2ea6e liés à 4ad394f4d2 (md5 aec89843)
aux deux postes à 08 h 50, courant 4ad394f4d2, les trois bases ouvertes. **G déclare la fenêtre finie à 08 h 51**,
diffusé aux douze sessions avec leur lot suivant : places lourdes quatre à luciepc, une à la tablette, tout rust sous
l'épingle.

**La pile de mémoire rendue** : à 08 h 49 le premier lot rust pris est memoire-b1-collect, conflit sur
`backend.rs` à la repose sur master, et les huit lots derrière lui tombent par parent rouge ; mémoire repose B1
d'abord et redépose dans l'heure, aux mêmes rangs. Derrière, arbre-estimation-hote reposé, emb-embedding-rates en
créneau 2 sur sa tête.

**Le paquet et les morceaux** : la file a vingt-et-un lots et aucun paquet de codeparsers ; le package-name-root
(4eed1f8ef0, règles 11) est tenu par G depuis la fenêtre. Demandé à G de le redéposer maintenant pour qu'ED dépose ses
morceaux dessus en parent (règles 12, un seul repassage des bases) ; s'il ne peut pas dans l'heure, ED dépose seul en
règles 11 et codeparsers montera en 12 derrière, deux repassages : le prix accepté pour ne pas le tenir.

**Ménage des atterris** : ED a retiré ses trois worktrees (19 Go), recherche quatre, et m'a laissé les deux
`rag3db-view-lag` (sous-module, `--force` refusé chez elle), retirés à 08 h 56 aux deux postes, propres, aucun
processus. Il reste 18 dossiers sous git_workspaces à la tablette, 27 à luciepc.

## 20 novembre, 10 h 50 — le 88 % expliqué et corrigé, les morceaux de groupe décidés par Lucie, le gel de la porte

**La cause du 88 %**, en une heure, à trois : G a rejoué les requêtes sur sa copie migrée de code-principal (hôte isolé) contre la base vivante, même arbre, même scope, et nommé le cas (`experiments/mtga/scripts/build_*_variant.py:32`, une ligne Python `a;b;c` dont le scope `variable` `s` portait toute la ligne sous les règles 8 et sa seule instruction, `s=index.read_text()`, sous les 9 : vingt caractères denses en mots de la question, remontés par le seul vecteur, premiers avec leurs quatre jumeaux). Codeparsers a précisé : **ce n'est pas la migration ni les règles 9 en elles-mêmes, c'est 57bfc142bf** (14 nov., le contenu d'un scope borné aux octets de son nœud, −844 Mo de doublons sur les bundles d'une ligne) **qui n'avait pas monté REGLES_DE_L_ANALYSE** ; la repasse forcée des règles 9 l'a appliqué d'un coup à l'arbre du 18 octobre de luciepc, que presque rien n'avait réanalysé ; le dépôt neuf du portable l'avait déjà, d'où 98,8 % contre 88 % avec le même binaire. Leçon dite à codeparsers, qui en fait un garde-fou (environ 3 h, après ses réexportations) : une empreinte de l'analyse d'un corpus figé, comparée à une table versionnée par numéro de règles, rouge quand le texte ou le découpage change sans que le numéro monte.

**Le remède**, après trois croisements de messages entre c7, ED et codeparsers (chacun avait reçu l'attribution inverse ; tranché par un même texte envoyé aux deux à 09 h 02) : **à ED**, sa candidature. La forme vient d'ED : le défaut, c'est le vecteur d'un morceau trop court, pas son plein texte ; en Python une constante `MAX_SIZE = 1024` sous le seuil ne se trouverait plus par son nom ; donc la règle du 2.12 (`not_candidate`, seuil 80) ne retire que les **morceaux**, le document de plein texte reste (`takes_out: chunks`), la garde des comptes ne soustrait que les lignes privées de document. Lot variable-not-candidate, règles 12, témoin rouge d'abord (le `s` a un morceau ; sous la forme stricte `seuil_de_la_plante` n'est plus trouvé par son nom ; une garde qui soustrairait tout dit « 7 documents pour 3 lignes »). **Le juge** : le banc, sur la copie de G, trois mesures (le critère du 88 %, les premiers exacts sur des noms courts, les totaux de `usages`). Codeparsers a trouvé qu'une constante en majuscules n'est pas un scope `variable` (type à part) : les requêtes du juge sont des variables courtes en minuscules.

**Lucie, réveillée à 10 h 24**, lit le rapport et pose la question qui change le lot : « pourquoi on tente pas quand possible de les réunir en lot ? … seulement niveau embedding, les références de cette var changeraient pas, mais elle partagerait embedding / chunk avec d'autres éléments pour éviter trop court ». Avis de c7 : meilleur que l'exclusion, et les deux se composent (l'exclusion aujourd'hui, elle débloque la re-bascule ; le regroupement ensuite rend le sens aux déclarations courtes, c'est ce que les règles 8 faisaient par accident en dupliquant). **Décidé par Lucie à 10 h 29** : les morceaux de groupe en lot suivant d'ED, cadrés d'abord dans une page qu'elle relit. ED l'écrit en vingt minutes (`docs/everything-declarative/20-11-2026/02-morceaux-de-groupe-conception.md`, 96cc721dce) : une suite maximale de petits scopes frères juxtaposés, coupée à un frère non groupable et à la fenêtre du plus petit embarqueur, sans recouvrement même sur un bundle d'une ligne, la ligne du groupe au parent avec sa plage et les clés des membres, un `groups_hash` dans le rendu du parent, règles 14, une journée. **Lucie à 10 h 36 : oui aux trois réponses** (tout scope feuille par taille et non seulement `variable`, le hash au parent, le rendu par le parent à la plage avec le membre précis quand le plein texte le trouve) **et deux invariants** : « jamais quelqu'un ne doit être bloqué dans l'usage du framework par un hack ni avoir l'impression de différence d'usage », et « important qu'on sache quand même voir comme entité indépendante la variable ou le petit scope feuille » : le groupe est un véhicule d'embarquement, le scope garde sa clé, sa ligne, l'outline, `usages`, le graphe, les résultats. ED a vu la conséquence : son lot 12 excluait **par type**, la différence d'usage que Lucie écarte ; tranché (a) : le lot change avant d'atterrir, **tout scope feuille sous 80 caractères**, quel que soit son type (variable-not-candidate-3, 3ee9580af1, rouge sur la constante `PLANTE_MAX_KG` qui gardait son morceau) ; le juge du banc, lancé à 10 h 33 sur l'ancienne tête, arrêté au bâti sans perte, rejoue sur la nouvelle avec dix constantes courtes en majuscules en plus. Graine de Lucie notée, pas de lot : des outils qui rangent le code de tout le projet avant un commit, en option.

**Le gel de la porte** (09 h 38 – 10 h 06, relevé par l'arbre) : une mesure de hotfixs (`poste mesure`, points isolés du HNSW) attendait la place du F300 du banc (`timeout 21600`) en tenant la porte, et plus aucun lourd n'entrait : le pousseur, le juge, dix lots, trois places vides. Hotfixs l'a retirée (son script de boucle d'abord, qui relançait la passe suivante sous mesure) et rejouée sous lourd, car elle compte (rappel à k = 10) et ne chronomètre pas. Lot de G à venir, forme convenue : un lourd dit sa fin prévue par son `timeout` (sans timeout = long), une mesure qui aurait à attendre refuse à l'arrivée avec un code à part et dit « rejouer sous poste lourd, ou après HH:MM ». Et une place de port évitée de justesse : embarquement avait pris 7992 pour son service d'essai de cubecl parce qu'il était libre ; c'est l'hôte isolé des juges de G ; déplacé sur 7995 avant le départ.

**Atterris de 08 h 56 à 10 h 50**, 76 commits, tous propres : la page de bascule de G (trois commits, le dernier attribue le remède à codeparsers : à corriger), l'estimation par l'hôte et les débits déclarés (arbre, embarquement), le paquet de codeparsers (e9a728048d, dix commits, règles 11), les tranches 1 à 3 de l'IDE en plusieurs fenêtres et file-erreur (tty, relectures d'ED comprises ; le chat guide une fenêtre détachée du dépôt, « dépôt à côté », pas jouable sur l'hôte posé baccb2ea6e avant la prochaine pose), mode pair lot 2 (archi), Strict borné et la nouveauté du verrou (arbre), recherche-interruption-2 (quatorze commits : un arrêt coupe la requête du moteur en cours, ticket du 7 nov. fermé), reprise-eintr-2 (archi), le point C mesuré (embarquement : master 13,1 % de morceaux au-dessus de 512 jetons sur le Rust du crate, 5,0 % sur les en-têtes, la tête d'ED 0 %, +8,7 % et +3,9 % de morceaux ; l'estimation par fichiers entiers disait 72,5 %), « docs devant les rust » (pousseur, 81/81), la page des morceaux de groupe. **Rouges de repose**, tous reposés dans l'heure : la pile de mémoire (backend.rs, puis un seul test de B4 dont l'assertion ignorait la fin d'invite d'ed-decider-prefix), mode-pair-3 et 4 (teammate.rs, archi : les deux gardés, bundle rebâti), la chaîne de recherche attente-vecteurs → notification-index-2 → notification-e2e → index-pas-pret de tty (backend.rs, après interruption-2), pusher-attestation (test_pusher.sh contre docs-first, reposé en sept minutes, 87/87, attesté sur sa tête, rang 1). Lucivy : check_dictionary sur les copies de G, 0 perdu sur les deux bases de luciepc, 690 mots d'avant sur le Scope du dépôt (4 748 hier), 0 en sursis ; feu vert pour rallumer la compaction sur les cinq manifestes et rouvrir chaque backend, le rebâti du Scope du dépôt à la fenêtre de re-bascule. Embarquement : le réglage déterministe de cubecl (ticket du 16, lecture faite : `CUBECL_AUTOTUNE_LEVEL` et le cache ; mesure par un jeu sans rembourrage, qui tranche si l'écart d'hier n'était que du rembourrage ; gratuit si écart 0 et moins de 2 % de débit). Cœur : arbre de bâti horodaté à luciepc, rag3db-colonnes retiré, C1 en boucle longue avec core gardé. Retirés : les deux rag3db-view-lag (recherche, par c7), les trois d'ED (19 Go), ceux de l'arbre.

## 20 novembre, 12 h 50 — le juge dit rouge au remède, la porte gelée une seconde fois, l'attestation en vigueur, l'iGPU rend des vecteurs faux

**Le remède par taille, deux fois repris.** ED l'avait écrit « tout scope feuille sous 80 caractères » (variable-not-candidate-3) ; ses suites ont montré ce que ça coûte : 28 tests rouges, dont les corpus jouets sont faits de fonctions d'une ligne (`pub fn boot_alpha() -> i32 { 7 }`) ; en vrai code, un accesseur ou un `fn new() -> Self { Self::default() }` n'aurait plus de vecteur. Deux options posées à Lucie (garder la règle et allonger les corpus, ou restreindre aux feuilles qui déclarent une valeur) ; **Lucie, 11 h 46 : oui à la seconde**, et elle a redemandé le regroupement : deux pas, l'exclusion aujourd'hui, les morceaux de groupe ensuite sur la même famille (ligne ajoutée à la page, 41f7b8e5bc). Puis **le juge du banc a dit rouge** (c3105ed454 contre master, page docs/banc/20-11-2026/04, 4ea69dc2d3) : le critère du 88 % est tenu (recouvrement 93 %, même premier 10/10, usages 0 perdu, les quatre jumeaux de mtga sortis du top 50) mais **12 des 14 noms courts** que master trouve par leur nom (10 au rang 1) sortent du top 10 : le plein texte que la règle garde ne sert pas `search_code`, l'inverse de la promesse. ED cherche pourquoi (le document est-il écrit et interrogé pour une ligne sans morceau, la fusion ne garde-t-elle que les lignes à morceau), témoin rouge avec les requêtes du banc, le banc rejoue sur sa tête corrigée. **Une leçon de méthode au passage** : contre la base vivante des règles 8, master échoue autant que le lot (71 % et 74 %, 7/10) ; cette référence vieillit et ne départage plus rien ; **le critère de la re-bascule devient « la tête à poser contre master, sur la même copie et la même source »** (page de G, 79d92e0d82). La re-bascule de code-lourd et code-principal attend la tête verte d'ED.

**L'attestation en vigueur depuis 11 h 04** (pusher-attestation, 3367b790c9, reposé en sept minutes après un conflit de test_pusher.sh avec « docs devant les rust ») : un lot rust ne se dépose plus sans le journal de check_features joué sur la tête exacte, arbre propre, VERT 8/8, passé par `CHECK_FEATURES_LOG=-` sur l'entrée de l'enqueue ; refusé sinon avec la raison. Diffusé aux douze sessions avec la commande exacte à 11 h 06 ; la marche dans docs/procedures/push-d-un-lot.md (43b08e5026) ; et **l'attestation du sommet d'une pile vaut pour ses ancêtres** (5abaaaefee, 12 h 04), pour que mémoire n'ait qu'un journal pour ses neuf lots.

**La porte gelée une seconde fois** (10 h 55 – 11 h 50, 55 minutes) : la mesure cubecl d'embarquement, sous `poste mesure` comme son plan le disait et comme c7 l'avait lu sans objecter, attendait le F300 du banc en tenant la porte ; une place sur quatre, les deux passes du pousseur bloquées 44 minutes, douze lots en file ; vu par le chien de garde corrigé (il repartait de zéro à chaque réarmement de 30 minutes et ne pouvait pas voir un gel de 40 ; il compte maintenant depuis le dernier commit). Embarquement l'a retirée proprement et rejoué la parité sous lourd (elle compare des bits). **La règle de G** est écrite (poste-mesure-refuse-long, cas 43, 44/44, à déposer) : la fin prévue d'un lourd est sa fiche « tient » plus son `timeout`, sans timeout c'est long ; au-delà de 1 200 s d'attente prévue, la mesure refuse à l'arrivée, code 76, « le lourd pid N tient sa place, fin prévue à HH:MM ; rejouer sous poste lourd ou après HH:MM » ; et les passes du pousseur partent avec `timeout 60m` en tête (5a519b6c60), pour qu'une mesure voie leur fin. Règle de c7 d'ici là : aucun `poste mesure` pendant qu'un lourd long tient une place.

**La parité de B4 en F300** (banc, 629f0d3859) : le cache des Decide ne monte pas (42,6 % → 41,2 %) ; il se joue en tout ou rien appel par appel, ~110 tours sur 244 servis à ~88 % et les autres à 0, des deux côtés : c'est le cache implicite de Vertex qui ne sert qu'une fois sur deux, le levier est le cache explicite, pas l'ordre des parts. **P1 baisse de 0,907 à 0,815** (27 « none » contre 39, 10 rangements de plus sous une fiche existante), au bord du bruit (±0,07) : un second jeu F300 « après » est parti à 12 h 41 (`timeout 7200`) pour départager ; mémoire dépose B1 à extraction-cut et les trois lots indépendants (template-version, graphs-on-disk, check-proposal) reposés sur extraction-cut, et **tient B4** au verdict ; sa page d'ordre dit pourquoi (26d3e73f01).

**L'iGPU rend des vecteurs faux** (embarquement, ticket e0fd96ba0e) : sur `igpu:0` de luciepc, granite-278m en lot contre seul donne un cosinus de 0,590 sur le jeu standard et 0,870 sans aucun rembourrage (card2 : 0,99999993) ; seul est presque toujours juste, et une contamination d'un appel à l'autre existe (minimum 0,878 sur des textes seuls). Trouvé en cherchant le déterminisme de cubecl. Vérifié par c7 : aucun service de production n'y tourne (la tablette consomme ceux de luciepc par les tunnels 7979–7981, luciepc sert 7878/7879 sur card2), les vecteurs des trois bases sont sains. Décidé : un embarqueur refuse l'adaptateur IntegratedGpu sauf réglage explicite déclaré, rouge d'abord (lot d'embarquement en cours) ; **le ticket cubecl est fermé avec ses chiffres** (33962b02c4) : sur card2, seul contre en lot sans rembourrage diffère de 4,9e-5, un ordre de réductions, couvert par la tolérance de pose ; pas de réglage gratuit mesurable sans un second service à côté de celui qui sert. Un port évité aussi : 7992 (l'hôte isolé des juges de G) pris pour l'essai parce que libre, déplacé sur 7995 avant le départ ; carte des ports notée.

**Atterris de 10 h 50 à 12 h 50**, tous propres : reprise-eintr-2 (archi : une attente interrompue par un signal est relue), les tranches 2, 3 et 4 de l'IDE en plusieurs fenêtres (tty : le chat guide une fenêtre détachée du dépôt par l'adresse citée, « dépôt à côté » et la ligne suivie, « graphe à côté » qui suit les outils du tour ; un outil sans graphe le dit au lieu d'un 404), file-erreur (la vue d'un fichier ne garde plus l'erreur du précédent), tests-never-skip-silently-2 (archi : aucun test ne rend la main en sautant sans le dire), tty-23 (les messages entre sessions dans le terminal, Ctrl-O), le ticket de la vue sans rang fermé (arbre), la page des groupes trois fois, la page du juge, le ticket iGPU, le ticket cubecl, la page de bascule. **La tranche 5** est un vrai rouge (une réponse périmée d'un fichier quitté pose son erreur sur le suivant, une course que file-erreur n'avait corrigée qu'en partie) : tty l'a retirée, repose avec un garde dans l'effet. **Reposes de l'heure** : ed-estimate-2 (index_nodes.rs contre l'estimation de l'arbre), mode-pair-3 et 4 (archi, derrière sa passe du bac à sable, rangs sans doute perdus), la chaîne de recherche. **Lots donnés** : lucivy, la sonde nocturne des dictionnaires sur copies (écrite, 7/7, la minuterie attend le oui de Lucie) et nightly-compaction-probe redéposé avec attestation ; arbre, « l'état d'une vue sans montre voit son retard » (3 h) ; embarquement, le témoin de chauffe corrigé (son assertion attendait 32 en dur) ; et deux sessions ont retiré en retard des worktrees de lots atterris (embarquement cinq, arbre deux) : lire pusher/log.tsv à chaque reprise. Lucie, 12 h 21 : « je vois moins de bugfix » : depuis minuit, 107 tests, 117 docs, 71 fix, 40 feat ; tickets ouverts 245 → 238.

## 20 novembre, 14 h 15 — l'iGPU de luciepc rend des vecteurs faux, la fusion enterrait les noms courts, B4 sauvé par un second jeu, la tablette rendue à ses deux places

**L'iGPU** (embarquement, en cherchant le déterminisme de cubecl) : sur la carte intégrée de luciepc, granite-278m en lot contre seul donne un cosinus de 0,590 (jeu standard) et 0,870 (sans aucun rembourrage), sans erreur, avec une contamination d'un appel à l'autre ; card2 : 0,99999993. Vérifié par c7 : aucun service de production n'y tourne (la tablette consomme ceux de luciepc par les tunnels 7979–7981, luciepc sert 7878/7879 sur card2). Puis la précision qui compte : **la carte fautive est un Intel ARL sous anv** (Mesa 26.2.6), pas radv ; **l'APU Radeon 8060S de la tablette calcule juste** (≥ 0,99999991 contre card2, page 08979a5b37), les essais locaux de Lucie sont sains. Lots : le garde-fou (un embarqueur refuse une carte intégrée sauf `RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU=1`, 73393fd3d5, prouvé par le vrai binaire qui refuse et ne charge rien), values-guard (dix-neuf tests de valeurs et quatre bancs passent par une aide qui refuse la carte intégrée sous repli, déposé 67cbfd2ace), resolved-adapter (le type de l'adaptateur résolu, pas le nom, `default` compris), l'autorisation par fichier déclaré du poste à la tablette seulement (G, poste-igpu-autorise). Le ticket cubecl est fermé avec ses chiffres (4,9e-5 sans rembourrage sur card2, un ordre de réductions, couvert par la tolérance ; pas de réglage gratuit mesurable sans second service sur card2). **Lucie, 14 h 0x** : « pourquoi on se sert de l'Intel et pas d'une R9700 ? » : card0 est la sienne (seat1), card2 porte les services sans partage équitable du calcul (8 nov.), l'Intel paraissait libre et personne n'avait vérifié ses valeurs ; « peut-être souci chez tous les gens qui ont GPU intégrés » : oui, et **Lucie dit oui à l'autovérification de l'embarqueur au démarrage** : quelques textes seuls puis en un lot de la taille réelle du service, comparés à une référence enregistrée (les vecteurs de l'ONNX d'origine, livrés à côté du modèle avec l'empreinte des poids), seuil = la tolérance de pose, refus dit, résultat dans `/sante` ; ni liste de fournisseurs ni de pilotes, le garde-fou par type devient inutile à terme. Lot d'embarquement après resolved-adapter ; les suites locales (ticket 793b666569 : depuis le binaire e2e unique elles ne se reconnaissent plus par le nom) derrière.

**Les noms courts perdus : c'est la fusion** (ED, relevé du banc). En bm25 seul, CONFIGS et ROOT gardent leur rang avec le lot (3 et 8) ; par défaut ils tombent de 1 à 26 et de 2 à 40. La fusion RRF somme `w/(k + rang)` sur les signaux présents : une ligne qui ne peut pas avoir de vecteur par construction n'y gagne rien et perd contre tout ce qui a les deux signaux ; sur master, la présence d'un vecteur, même au rang 40, la tenait au rang 2. **Décidé (A)** : une ligne dont le parent n'a aucun morceau (marque `sans_vecteur`) est jugée par la fusion sur les signaux qu'elle peut avoir, moyenne pondérée ramenée au poids total ; la dette d'embarquement garde le comportement d'aujourd'hui ; (B) le nom exact dominant, plus tard si le juge le demande. Témoin rouge (une constante exclue, quinze concurrents qui la citent, rang 1 en bm25 seul et par défaut), juge sur la copie, et le revers à vérifier : qu'aucune ligne sans vecteur ne prenne un premier rang par un sens qu'elle n'a pas. Règles 13 ; **codeparsers passe devant en 12** : ses réexportations (tout fichier Python a ses scopes de fichier, `export *` suivi sans deviner, aucune arête retirée sur cinq corpus) sont **vertes au juge** (46d0446446 : 99 %, 10/10, noms courts et usages identiques, un fragment `execute + COPY` trouvé par son sens au rang 1, absent sur master) et se déposent.

**B4 sauvé par un second jeu** (banc, 362caec827) : P1 = 0,944 au second F300 « après », contre 0,815 au premier sur la même tête et 0,907 avant : la baisse était l'écart d'un jeu à l'autre ; B4 atterrit, mémoire l'a reposé au sommet (pile : b1, b2, b3, b6, extraction-cut, template-version, graphs-on-disk, check-proposal, b4-decide), une seule attestation au sommet pour les neuf. Cache confirmé à 43 % sans B4 ni avec : le levier est le cache explicite. Réserve à suivre : 14 doublons dans ce jeu contre 4 et 5. **Règle : un réglage de l'archiviste se juge sur plusieurs jeux par côté**, P1 varie d'une dizaine de points d'un jeu à l'autre.

**La tablette rendue à ses deux places** à 13 h 35 : `lourds-max` était resté à 1 depuis la fenêtre de 07 h 30 (un réglage à la main, rien ne l'écrit), une seule place tenue depuis 11 h 45 par les suites d'ED, six lourds derrière dont deux attestations depuis 11 h 25 et le témoin rouge des noms courts depuis 12 h 35. G ajoute `poste places <N> jusqu'à HH:MM` (la baisse porte son heure de retour, poste remet la valeur d'avant, une baisse sans heure refuse, cas 44, 47/47) derrière la mesure qui refuse (déposée, attestée). Les passes du pousseur portent `timeout 60m` depuis 12 h 48.

**Le HNSW** (hotfixs) : les points isolés mesurés sur 127 198 vecteurs réels de code-lourd, 200 requêtes, quatre passes avec et sans amorce : rappel@10 de 0,998 à 0,999, les 200 plus isolés retrouvés à chaque fois ; le défaut ne se voit que sur la base gaussienne du banc ; ticket rouvert en **faible gravité** (le banc l'avait fermé, nos messages croisés) avec deux conditions : il remonte si un corpus réel fait tomber le rappel sous 0,99, il se ferme si l'on prouve qu'aucun embarqueur ne produit cette gaussienne ou si un correctif passe le témoin gaussien. Lot suivant de hotfixs : « l'élagage peut couper le dernier pont vers une région », témoin par parcours en largeur de la couche basse, une centaine de tours sur les vrais vecteurs. **C1** (cœur) : le core avec symboles d'un tour RelWithDebInfo sur 4ad394f4d2 (donc pas la course des groupes) : le tampon des offsets du dictionnaire contient du texte, un tampon de chaînes de 128 Kio venu de malloc rendu pendant qu'un morceau le tient, lecture et écriture après libération ; ASan ×10 lancé, grande quarantaine, arrêt au premier rapport (ticket 0c4facb61d).

**Atterris de 12 h 50 à 14 h 15**, tous propres : tty-26 (la mémoire du run dans le terminal, `/memory` et `/turns`), la sonde de la maintenance nocturne et la sonde nocturne des dictionnaires de lucivy (verte aux deux postes, treize index vus, un VERT sans index vu devient INCOMPLET ; la minuterie attend le oui de Lucie), le basculement de l'estimation de l'arbre sur les débits déclarés, le témoin de chauffe corrigé, le garde-fou iGPU, la page du F300 bis, la page du juge de codeparsers, la mesure HNSW et la réouverture du ticket, les pages de bascule (critère et attribution corrigés), la procédure d'attestation. Un doublon évité de justesse : lucivy avait deux sondes en file sous deux ids (l'ancienne a atterri dans son train avant le retrait, la seconde retirée). Trois sessions ont tenu des worktrees de lots atterris (embarquement, arbre, lucivy) : consigne redite, lire `pusher/log.tsv` à chaque reprise.

## 20 novembre, 15 h 20 — l'autovérification décidée par Lucie, le code 76 en vigueur, la fusion nommée, le dossier mort du dépôt

**Les décisions de Lucie de l'après-midi.** « Pourquoi on se sert de l'Intel et pas d'une R9700 ? » : card0 est la sienne (seat1, écrans, jeux), card2 porte les quatre services sans partage équitable du calcul (le 8 nov. une mesure du banc avait fait passer le granite qui sert de 130 ms à 19 s par texte), l'Intel paraissait libre et personne n'avait vérifié ses valeurs. « Peut-être souci chez tous les gens qui ont GPU intégrés ? » : oui, et **oui à l'autovérification de l'embarqueur au démarrage** (page 83d0029a98) : quelques textes seuls puis en un lot de la taille réelle `lot_max`, comparés à une référence enregistrée (les vecteurs de l'ONNX d'origine, livrés à côté du modèle avec l'empreinte sha256 des poids ; le calcul CPU écarté : un adaptateur de plus, des dizaines de secondes, pas partout), seuil = la tolérance de pose du modèle, refus dit (pas de repli silencieux, lent au point de passer pour une panne), résultat dans `/sante` ; bge-m3 `not_verified` jusqu'à sa référence ; ni liste de fournisseurs ni de pilotes, le garde-fou par type devient inutile à terme. Atterris d'ici là : le garde-fou par type (73393fd3d5), values-guard (f6572653c6 : dix-neuf tests de valeurs et quatre bancs déclarent qu'ils comparent des valeurs et refusent le repli intégré), resolved-adapter déposé (le type de l'adaptateur résolu, `default` compris, prouvé sur le vrai binaire de la tablette qui refuse son APU en attendant la mesure) ; l'autorisation de la tablette par le fichier déclaré du poste `embedder-allow-integrated-gpu` (G, à poser à l'atterrissage, jamais à luciepc).

**Le code 76 en vigueur depuis 14 h 33** (poste-mesure-refuse-long, f1589b1219, recopié aux deux postes, diffusé aux douze sessions) : une mesure refuse à l'arrivée derrière un lourd dont la fin prévue (prise de place + `timeout` en tête ; sans timeout = long) dépasse 1 200 s, nomme le lourd et sa fin, « rejouer sous poste lourd ou après HH:MM » ; 76 n'est pas un rouge, rien n'a tourné. Les passes du pousseur portent `timeout 60m`. **Les places avec heure de retour** (poste places <N> jusqu'à HH:MM, f048a2cc25) : une baisse sans heure refuse, la valeur d'avant revient seule ; c'est ce qui manquait ce matin (la tablette à une place de 07 h 30 à 13 h 35).

**La fusion, nommée par ED** (relevé du banc) : en bm25 seul, CONFIGS et ROOT gardent leur rang avec le lot ; par défaut ils tombent de 1 à 26 et de 2 à 40 : la fusion RRF somme sur les signaux présents et une ligne qui ne peut pas avoir de vecteur par construction perd contre tout ce qui a les deux. **Décidé (A)** : marque `sans_vecteur`, moyenne pondérée des signaux présents ramenée au poids total, la dette d'embarquement inchangée ; ED cherche son rouge (ses deux premiers corpus restaient verts : une constante première en BM25 est tenue par l'exact dominant ; le cas du banc est une constante au rang 3, troisième corpus en cours) ; règles 13, derrière **les réexportations de codeparsers atterries en 12** (65df02838e, juge du banc vert à 99 %, 46d0446446). Le garde-fou de l'empreinte de codeparsers est prouvé (rouge sur le rejeu à l'envers de 57bfc142bf, 0,02 s dans la lib) et se dépose sur ce master ; désormais toute montée de règles ajoute sa ligne `<règles> <empreinte>` à tests/fixtures/empreinte_analyse.txt.

**Le dossier mort du dépôt** (lucivy, sur la copie cohérente de G) : le plein texte de `Scope` vit sur `ScopeContent` depuis la migration ; `Lucivy_Scope` (839 Mo) n'est ouvert par personne, les 690 mots perdus sont dedans, aucune recherche n'est touchée, aucun rebâti à faire ; G l'a vérifié au manifeste (aucune cible ne le nomme) et le rendra à sa fenêtre par un mv en instantané jetable. Un écart trouvé en chemin : « Scope 104 625 documents pour 95 176 lignes » (ScopeContent), **pas attendu** selon ED (sa garde ne peut que baisser les lignes ; 9 449 documents de plus : des orphelins, ou le geste de 2.12 pas encore joué) ; et une précision de G : la base a été rouverte à 14 h 32 sans rebâti ni avertissement, parce que la garde des comptes ne joue qu'à un `index` complet, jamais à l'ouverture ni par la montre ; le rebâti (~22 s) tombera au repassage de la fenêtre. Décidé : le juge de fenêtre ne dit rouge que si la garde rebâtit une table sans écart nommé avant la fenêtre, ou si les comptes ne collent pas après ; lucivy départage sur une copie refaite par G. La compaction est rallumée depuis 09 h 24 (cinq manifestes, coupures de 2 à 11 s) ; la sonde nocturne des dictionnaires est sur master (verte aux deux postes, treize index vus), sa minuterie attend le oui de Lucie.

**Atterris de 14 h 15 à 15 h 20**, tous propres : les réexportations de codeparsers, la chaîne de l'attente de recherche au complet (une recherche qui lit les vecteurs attend aussi les vecteurs dus ; une seule notification quand l'index d'une vue est prêt ; le témoin par le binaire réel ; l'attestation y a attrapé un vrai défaut, `tiny_http` sans la feature `daemon`), la vue sans montre de l'arbre (le diff à froid de l'arbre indexé contre la pointe), deux tickets du 11 octobre fermés (le blocage derrière un index par ses quatre lots ; l'index qui ne suit pas l'arbre édité, renvoyé au ticket de Codex), values-guard, les pages (autovérification, F300 bis, juge de codeparsers, HNSW). Ménage : les deux rag3db-view-lag et deux worktrees de recherche retirés par c7 à luciepc ; embarquement, arbre (deux fois) et lucivy ont tenu des worktrees de lots atterris, consigne redite. Le banc a fermé le ticket HNSW alors que la décision était faible gravité (messages croisés) : rouvert par hotfixs avec deux conditions distinctes.

## 20 novembre, 16 h 20 — les vues et les worktrees par l'hôte, décidés avec Lucie ; le dossier mort et l'écart nommés ; les fichiers générés rebâtis

**Les vues et les worktrees par l'hôte.** L'arbre, au repos, a cadré « la vue par défaut de la session qui appelle », le ticket réservé à Codex (page docs/arbre-principal/20-11-2026/01, 6aa8e0aed3) : l'hôte ne sait rien de l'appelant (le client mince n'envoie que son pid) ; le cwd est un piège (nos sessions sont lancées dans l'arbre de Lucie et travaillent ailleurs par chemins absolus, une vue tirée du cwd lirait le backend pour tout le monde) ; aucun worktree de session n'est indexé aujourd'hui ; les `roots` de Claude Code (dossier de lancement, `--add-dir`) ne nomment pas le worktree. **Lucie, de 16 h 00 à 16 h 13**, a fait la conception avec c7 : « un outil qui lui liste ses vues souscrites, il voit laquelle est active, il peut switch, et lancer un sous-agent limité à une des vues » ; « quand il ouvre un worktree c'est une demande de vue avec option open worktree, il obtient le chemin local » ; « un sous-agent reste nommé session-nomsousagent et accède aux worktrees de /session/ » ; « pas forcé d'être limité à une seule vue : ça dépend de ce qu'a setup son parent, et le parent peut à tout moment lui altérer ça ». **Décidé** (page 02 de l'arbre, 59a3a36d5c ; accord d'archi ff9c37db50) : `views` (vues connues, l'active, sa source ; chaque lecture répète la vue lue), `use_view` (bascule gardée par l'identité de session, le `session_id` du registre), `open_view` (**l'hôte crée le worktree** sous `~/git_workspaces/<session>/<jj-mm-aaaa>-<lot>`, sous-module, target déclaré, l'indexe en couche de travail au point de fourche avec master, le souscrit, rend le chemin), `close_view` (refus si sale ou processus) ; **un enfant** `<session>-<nom>` reçoit du parent son ensemble de vues et l'active, le parent les modifie à tout moment, l'enfant bascule dedans sans l'élargir, même mécanique à deux niveaux ; gardes : jamais l'arbre de Lucie, sairen, ni le dossier d'une autre session, jamais fermer ce que l'hôte n'a pas ouvert sans son mot. **La fermeture à l'atterrissage se tire** : le dépôt porte `vue=<poste>:<nom>` dans landed.tsv, chaque hôte relit les atterrissages de son poste et ferme par sa route locale ; « retiré à l'atterrissage », reproché quatre fois aujourd'hui, devient automatique. Qui : l'arbre (V1 en cours, puis views/use_view, open/close et la couche de travail, l'ensemble des enfants), archi (le sous-agent à vues, après ses mode pair), G (la colonne vue et la fermeture). Le ticket passe de Codex à l'arbre, au mot de Lucie.

**Les trois décisions de l'après-midi, rappel** : l'autovérification de l'embarqueur au démarrage (référence ONNX livrée à côté du modèle, refus dit) ; le regroupement des petits scopes en lot suivant d'ED ; la vue par la session ci-dessus. Trois mots de Lucie encore attendus : la minuterie de la sonde des dictionnaires, la voie B du plein texte (page de lucivy 4.4.0 sur master, 1a890cefec, avec le préalable d'une 4.3.13 qui refuse un index étiqueté), l'exclusion des bundles minifiés (`app.js` = 57,6 % du plein texte du dépôt ; codeparsers mesure d'abord qui porte ce poids).

**Le dossier mort et l'écart nommés** (lucivy, G, ED) : sur une copie cohérente du dépôt, le plein texte de `Scope` vit sur `ScopeContent` depuis la migration ; `Lucivy_Scope` (839 Mo) n'est ouvert par personne, les 690 mots perdus sont dedans, aucun rebâti à faire ; G l'a vérifié au manifeste et le rendra à sa fenêtre (mv en instantané jetable). L'écart « 104 625 documents pour 95 176 lignes » est **exactement** les 9 449 lignes non candidates que le geste de 2.12 n'a pas encore retirées (pas des orphelins) ; la garde des comptes ne joue qu'à un `index` complet (la réouverture de 14 h 32 n'a rien dit) ; au repassage de la fenêtre, le geste joue et les comptes doivent coller. Règle du juge de fenêtre : rouge seulement si la garde rebâtit une table sans écart nommé **avant** la fenêtre, ou si les comptes ne collent pas après. La compaction est rallumée depuis 09 h 24.

**Le pousseur** : les fichiers déclarés générés (`tools/poste/generated_files.txt`, les quatre sorties du bundle de l'IDE) sont rebâtis à la repose (49b7ae46ae, 92/92) après que le bundle a rendu mode-pair-3 ce matin et mode-pair-4 et ide-fenetres-5 à 15 h 58 ; le code 76 diffusé ; `poste places` ; le fichier `embedder-allow-integrated-gpu` posé à la tablette (prouvé : la variable exportée sous poste lourd, absente à luciepc). La pile de mémoire est retombée sur backend.rs (déposée sur une base de 08 h 49) : reposée sur la tête de master, dix lots désormais (memoire-script-ids au sommet : un script non déclaré est refusé au chargement), une attestation.

**Le HNSW** (hotfixs) : le rouge sur vrais vecteurs est confirmé, et ce n'est pas l'élagage après mise à jour : **dès le CREATE**, les membres d'un amas de quasi-copies (distance nulle) sont écartés comme « couverts » par leur représentant et deviennent injoignables même par leur propre vecteur ; code-lourd a 2 204 groupes de copies exactes, le plus grand de 404 lignes. Gravité : réponse incomplète sur des doublons. Cinq tours de plus pour vérifier que toute ligne perdue est dans un amas ; constat à rattacher au ticket des groupes de copies ; correctif à étudier (ne jamais écarter un candidat à distance nulle d'un voisin gardé). Dit à ED : un même texte, un seul vecteur, question d'embarquement pour après ses lots.

**Atterris de 15 h 20 à 16 h 20**, tous propres : index-pas-pret (tty, dix-sept commits), tty-23 et 26, la page lucivy 4.4, mode-pair-3 (un agent peut écrire à une session par son nom), notification-e2e, resolved-adapter, les templates nommés, la vue sans montre et son ticket, les tickets fermés de recherche, les pages de cadrage, poste-places-jusqua, poste-igpu-autorise, pusher-generes-rebatis, les tickets des mots perdus. Recherche : 0 rouge du chat sur 120 passages sous charge au hasard ; passe à une famine déterministe sur deux cœurs, bornée à 60 tours. Lucivy : lot « un flush en échec ne refuse pas les ajouts suivants » (perte silencieuse ou témoin faux, départage d'abord). Codeparsers : le garde-fou de l'empreinte en passe, puis la mesure des lignes minifiées. Cœur : deux tours ASan finis sans rapport, dix vers 23 h.

## 20 novembre, 17 h 35 — le socle de l'équipier, la palette par familles, le kanban, les orphelins d'app.js, le piège des gabarits du juge, le rattrapage du HNSW

**Les quatre réponses de Lucie (17 h 00)** : oui à la minuterie de la sonde des dictionnaires (lucivy l'a activée à 17 h 03 aux deux postes sur le oui donné dans sa session, pas sur mon relais : sa règle, la bonne) ; oui à la voie B, « au pire on garde la dernière version stable de lucivy en base, jamais rien d'autre, pas chaque checkpoint », qui est B telle que la page la décrit (la dernière génération validée et celle en cours), 4.3.13 puis 4.4.0, publication à son mot ; les bundles minifiés : « prio d'exclure par défaut », savoir les parser (pseudo-retour à 300 caractères) plus tard ; oui au socle de l'équipier, élargi : « poser des cron comme Claude, les features les plus populaires, la recherche web par le grounding Gemini pour le moment, le fetch, le navigateur, et les MCP : connecter un MCP en pleine session ne doit pas obliger à redémarrer ».

**Le socle de l'équipier** (page d'archi, docs/rag3weaver-archi/20-11-2026/02, 25bb4b623c puis fd6fdda49d) : sept lots, environ sept jours, répartis : archi run, palette différée, MCP à chaud, fork ; mémoire fetch et web_search (models.llm) ; recherche le cron ; tty le navigateur (pont jackwener) ; ED le socle fichiers vérifié et les tâches du run. La page de la palette de Claude Code (docs/orchestration/14-11-2026/04, relevé depuis une session et la référence officielle) a servi de ligne directrice : « on n'est pas obligé de tout reprendre, mais pour qu'un agent soit stable il doit avoir des équivalents ». OpenCode cloné en lecture (MIT, `~/git_workspaces/c7/20-11-2026-opencode`, jetable) : pas de ToolSearch chez eux, un mode code sur un catalogue à espaces de noms ; on reprend les descriptions, la forme du catalogue, et `session-search`. **La palette par familles**, décidée avec Lucie entre 17 h 25 et 17 h 3x : des familles déclarées au gabarit (moins de dix outils chacune, un outil dans une seule), une tête épinglée, `find_tool` par liste qui charge la famille entière et dit son coût, le déchargement après N pas, l'héritage des familles du parent comme pour les vues ; l'agent a toujours sous les yeux le résumé (familles, une ligne, noms des outils, sans paramètres) ; et **tout configurable** au gabarit (bloc `palette` : head, families, summary always|on_demand|none, search families|tools|both, unload_after_steps), avec un mode « recherche d'abord » pour des centaines de familles. **Une phrase par paramètre** pour tout outil exposé (question de Lucie 17 h 25) : l'index de code l'avait, le serveur mémoire pas partout (mémoire corrige), les outils de l'équipier non (archi : cinq outils, lot outils-equipier-decrits, témoin qui nomme chaque paramètre manquant). **Le kanban** (vision de Lucie, 17 h 1x) : une fenêtre détachée de l'IDE « tickets à côté », nourrie par l'hôte (les tickets .md par état, la file et landed.tsv, les tâches), la couche visuelle reprise d'un kanban mûr (Kaneo si React et libre, ou dnd-kit sur shadcn), jamais leur backend ni leur modèle ; cadrage à tty après le navigateur, deux préalables en premiers lots (l'état d'un ticket servi par l'hôte, le lien ticket → lot → PR à l'enqueue). Memorilla (snap-stanford) et llm-wiki (nvk) regardés pour Yahya : le premier est un modèle de recherche à jetons latents, pas une brique ; le second est un wiki Markdown compilé par des agents, cousin de nos fiches, utilisable au travail d'ici que rag3weaver soit solide.

**Les orphelins d'app.js.** Mon « 57,6 % du plein texte » était périmé : une mesure du 15 novembre sur `Lucivy_Scope`, d'avant la règle `index_exclude` du 14 (déclarée au manifeste vivant, `minified_line_max_bytes: 5000`). Mais G a lu la base vivante : **app.js a encore 160 scopes** alors que `list_files` le dit non indexé et qu'un index complet a tourné à 07 h 11. ED a nommé le défaut : des scopes visibles **sans `File`**, que la synchronisation ne voit plus jamais puisqu'elle liste les fichiers indexés par les seules lignes `File`. Codeparsers l'a mesuré sur la copie de 15 h 38 : 5,5 % du contenu des Scope (5,56 Mo sur 101), portés par ces 160 orphelins ouverts depuis le rang 0 sans `DEFINED_IN`, 37 variables à 133 Ko chacune ; l'analyse d'aujourd'hui donnerait 1 511 scopes bornés. Lot d'ED orphan-scopes-retired (b7608ad0cf) : la synchronisation complète lit aussi les chemins des scopes visibles, un chemin sans `File` que la source ne retient plus part avec ses scopes ; la cause (comment la `File` est partie sans ses scopes, l'époque des instantanés en piste) a son ticket. Le dossier mort `Lucivy_Scope` est daté de 06 h 43, la dernière écriture d'avant la bascule, rien depuis, la maintenance ne l'ouvre pas : G le rend à sa fenêtre ; la sonde de lucivy apprendra à ne juger que les tables des cibles.

**Le remède d'ED et le piège du juge.** 93dd826183 (exclusion par déclarations de valeur + fusion (A)) : critère tenu, revers de (A) tenu, les 14 noms courts dans le top 10, mais **7 premiers sur 10** (research 1 → 7) ; tranché : le critère reste « premier », (B) le nom exact dominant dans le même lot (531a4dbd17). Le rejeu de (B) donnait les rangs de (A) : **la recette du banc lisait les gabarits installés du service**, pas ceux de la tête ; (B) est un gabarit, il n'était jamais chargé ; tout ce qui passe par le binaire restait prouvé. Corrigé (le préfixe des gabarits réécrit vers le worktree, refus sinon), rejeu des deux côtés en cours. Leçon notée : une mesure prouve l'artefact qu'elle mesure, gabarits compris.

**Le HNSW** (hotfixs) : les cinq tours confirment des lignes injoignables dès le CREATE, 7 distinctes, 4 dans un amas de quasi-copies et **3 ordinaires** ; le ticket du pont est atteint sur les vrais vecteurs (réponse fausse) ; celui des points isolés remonte en réponse fausse (son 200/200 venait d'un échantillon de 2 000 qui ne pouvait pas voir 3e-5) ; correctif f7dab8ab06 : `keepEveryNodeReachable` en fin de bâti en mémoire (parcours en largeur, une arête depuis le plus proche nœud atteint dans la marge sous l'élagage) ; jugement : 0 injoignable sur 10 tours réels, le gaussien sans régression, le temps du CREATE ; le témoin réel est une mesure sous poste lourd écrite au ticket (900 Mo de vecteurs hors dépôt) ; un témoin synthétique jamais vu rouge a été retiré.

**Atterris de 16 h 20 à 17 h 35**, tous propres : les pages de cadrage (vues, outils des vues, accord d'archi, socle, répartition, palette par familles, règle des descriptions, OpenCode), le bac à sable sans wrapper (archi), la sonde lucivy à versions et les templates nommés (arbre), ed-estimate-2, la tranche 5 de l'IDE (les cinq tranches faites), la page des lignes minifiées, les tickets fermés. En file ou en passe : la pile de dix de mémoire, V1 de l'arbre, orphan-scopes-retired, le garde-fou de l'empreinte (rouge une fois sur un témoin de tty sous charge, départagé : pas la signature du chat vide, une course dans le témoin de la relecture retenue, corrigée par une barrière d'ordre), poste-sccache-defaut, pusher-vue-du-lot. Recherche : 0 rouge du chat sur 240 passages (charge puis famine), la lecture décide : témoin du verrou `abonnes` (un abonné SSE qui ne lit plus bloque les deux fils du serveur des témoins). Cœur : tours ASan jusqu'à 23 h, deux faits sans rapport.

## 20 novembre, 18 h 30 — le remède d'ED vert, l'essai réel du mode pair, « un poste ou quinze », la fenêtre de re-bascule calée sur ses préalables

**Le remède d'ED, vert.** Le banc a d'abord trouvé le piège de sa recette (les gabarits du service, plus haut), puis a rejoué (B) chaque côté sur ses gabarits : les noms courts verts (les 8 premiers gardés, research, threads, echecs, pool, CONFIGS au rang 1), mais la voie du nom exact **ressuscitait ce que 2.12 écarte** (`namespace rag3db {`, `impl PortDef {`) parce qu'elle lisait les scopes sans passer par la candidature ; corrigé par ED ((B) ne considère que les lignes qui ont un document de plein texte) ; **tranché** : « même premier » accepte un premier qui va à la définition qui porte exactement le nom (`commit` → `fn commit`…), c'est l'intention de (B) ; le banc l'a écrit dans sa recette (10/10) ; **vert** sur 03fbecf8b8 (page 06 du banc, f9efb1cc4c) : critère de G tenu, usages identiques, le seul premier sorti du top 10 est une déclaration de 29 caractères qui perd son vecteur, l'effet voulu. ED dépose en règles 13, avec son attestation, sa vue déclarée et sa ligne d'empreinte ; derrière, les morceaux bornés (14), l'exclusion par défaut, les orphelins (orphan-scopes-retired, b7608ad0cf : un chemin sans `File` que la source ne retient plus part avec ses scopes à la synchronisation complète ; codeparsers a mesuré 5,5 % du contenu des Scope portés par les 160 orphelins d'app.js).

**La fenêtre de re-bascule.** G la propose avec trois préalables (le remède atterri ; un hôte bâti depuis master : orphelins, fusion, mode pair 1 à 4, les cinq tranches de l'IDE ; la répétition sur copies des deux bases jugée « tête contre master sur la même copie »), des coupures dites (hôtes posés : secondes ; code-lourd ~12 min fermée puis repassage base ouverte ; code-principal ~10 min ; le dépôt 15 s pour le mv de `Lucivy_Scope`, puis repassage règles 13 avec le geste de 2.12 attendu au journal), le retour arrière par base. **Lucie, 18 h 2x : « quand vous voulez, même en avance ; je ne me sers pas encore vraiment des hôtes, sauf vous »** : la fenêtre part dès que les préalables sont verts, et c7 diffuse chaque pas aux sessions, qui sont les utilisateurs des hôtes.

**L'essai réel du mode pair (L6)**, au oui de Lucie dans la session d'archi : algareth apparaît dans ListAgents, reçoit un message de session dans sa boîte et répond par `send_to_session` ; **le registre lit le nom du fichier**, un processus porte une entrée et un nom ; retrait propre. Pour l'invariant de Lucie (une session au terminal et l'IDE dans le même backend, dans les deux sens ; depuis son téléphone parler à un agent actif dans un terminal), il faut **un relais enfant de l'hôte par pair** qui tient son `<pid>.json` et sa socket et passe les trames à l'hôte par HTTP sous jeton : lot d'archi après le sous-agent à vues. Tty a relevé ce qui manque pour qu'un run du terminal soit ouvert dans l'IDE : la vue d'une tâche vivante au flux avec ses accords répondables (lot avant le kanban), la fenêtre des L1, l'adresse d'un agent (avec le relais), le distant par une écoute authentifiée. Les mode pair 1 à 4 sont sur master (la boîte d'une tâche montre « qui écrit à qui »).

**Les vues et les worktrees par l'hôte, suite.** V1 déposé puis `views` et `use_view` (le client mince lit la session Claude Code qui le lance, l'hôte garde la vue active par session sur disque ; un défaut ancien corrigé : deux sessions ouvertes dans la même milliseconde partageaient leur id), la première vue déclarée au dépôt. La page des lots d'ouverture (docs/arbre-principal/20-11-2026/03) : la garde du chemin **avant tout `git worktree add`** (l'arbre de Lucie, sairen, le dossier d'une autre session, hors du dossier de la session, chemin déjà pris), O1 ouvrir sans index, O2 la couche de travail au point de fourche, O3 fermer, O4 la fermeture tirée de landed.tsv ; **la table dossier → session** écrite par c7 aux deux postes (`~/.config/rag3weaver/dossiers-des-sessions.json`, quatorze lignes ; l'hôte refuse un dossier d'une autre session, inscrit un dossier neuf, ne devine jamais) ; la branche d'une vue ouverte : `<dossier>/<lot>`. **Lucie, 18 h 1x : « imaginons le mec il a quinze ordis, il doit pouvoir s'en sortir quand même, ou un seul »** : rien ne nomme un poste en dur ; poste = hostname ; un hôte par poste (worktrees, vues, registre, relais) ; la table par poste ; la fermeture tirée par une voie déclarée (un poste : tout local, zéro ssh) ; une adresse `<poste>:<nom>`, l'hôte local relaie vers l'hôte distant par HTTP sous jeton par une table des hôtes ; G a relevé les sept points de ses scripts qui nomment un poste en dur (page du pousseur, 54464d8fca), lots après les vues. La vue du lot au dépôt (`VUE=<poste>:<nom>`, d96a59b685) est en vigueur et diffusée.

**La palette et le socle.** Les réponses de Lucie à 17 h 00 (ci-dessus) ont donné la page du socle d'archi (sept lots répartis : archi run, palette, MCP à chaud, fork ; mémoire fetch et web_search ; recherche le cron ; tty le navigateur ; ED fichiers et tâches) ; **la palette par familles** (déclarées, moins de dix, une tête épinglée, `find_tool` par liste, le résumé toujours sous les yeux, le déchargement, l'héritage du parent) et **tout configurable** (bloc `palette`), **une phrase par paramètre** pour tout outil exposé (les outils de l'équipier n'en avaient pas : lot d'archi, mémoire corrige les siens). OpenCode cloné en lecture (pas de ToolSearch, un mode code sur un catalogue à espaces de noms). **Le kanban** « tickets à côté » (cadrage à tty après le navigateur) avec les trois liens d'une carte (l'agent qui travaille, la PR, les L1 des sessions) et le troisième préalable (un run déclare son ticket). **Les produits voisins** lus (Projects, Claude Tag, agent teams : section de la page 04) : ce qu'on prend, l'Overview « ce qui a besoin de toi », la checklist dans le fil, la mémoire par lieu, la liste de tâches partagée.

**Les prix** (question de Lucie, 18 h 2x, page officielle) : gemini-3.8-flash 0,75 / 3,75 / 0,075 jusqu'au 31 décembre puis 1,50 / 7,50 / 0,15 ; Gemini 4 Hydrogen 1,50 / 7,50 / 0,075 ; Flash-Lite 3.5 0,30 / 2,50 ; Hydrogen n'est pas moins cher que Flash pendant la promo, égalité au 1er janvier avec un cache lu deux fois moins cher ; à mesurer en décembre au banc. Geo mis de côté (« pas utile pour l'instant ») ; les restes des tests à résidus effacés (0,67 Gio de RAM rendus au portable) ; l'intra-scope : oui aux trois questions, « rien de cherchable en vecteur, des entités et des relations simples ».

**Atterris de 17 h 35 à 18 h 30**, tous propres : les pages (socle récrit, règle des descriptions, invariant, essai réel, un poste ou quinze, ouvrir/fermer, lignes minifiées, réponses intra-scope, juge du nom exact, pousseur postes en dur, chat sous charge), ed-estimate-2, la tranche 5, le garde-fou de l'empreinte et sa procédure, mode-pair-4, les fichiers non suivis et la sonde lucivy à versions (arbre), pusher-vue-du-lot, passe-paniques-attendues, poste-sccache-defaut (sccache redevient le défaut), les tickets fermés (sonde lucivy, mots perdus en vérification). Recherche : le verrou `abonnes` **vu rouge puis vert** (un abonné SSE qui ne lit plus bloquait les deux fils du serveur des témoins et `app.js` ne partait jamais : la page vide du chat a sa cause) ; hotfixs : correctif `keepEveryNodeReachable` en jugement (10 tours réels) ; cœur : ASan jusqu'à 23 h ; mémoire : dix lots, un rouge honnête (le contrôle des scripts non déclarés ne mordait pas, corrigé), une seule passe au sommet, dépôt au vert.

## 20 novembre, 19 h 24 — le lot du remède atterri, la fenêtre partie sur (b), la couche par commit de fourche, le confineur

**Le remède d'ED est sur master** : `ed-variable-not-candidate` a atterri à 19 h 10 (f8c74d0876, PR 239, huit commits : le témoin rouge, la liste des non-candidats `ChunkingConfig::not_candidate`, la page des nouveautés, la repose en règles 13, les décisions (A) et (B) de la fusion, la page d'ED, l'empreinte en 13 dont la garde lit la liste des règles). Identité et messages propres (les seules occurrences de « Claude » sont des noms techniques : la session Claude Code que lit le client mince). Le préalable (a) de la fenêtre est vert.

**La fenêtre de re-bascule est partie sur (b)** à 19 h 2x : G bâtit à luciepc l'hôte de la tête de master 83c667e420 contre la lib courante 4ad394f4d2 (le moteur n'a pas bougé depuis), essai sur 7991, check-lib aux deux postes ; puis (c), la répétition sur des copies de code-lourd et code-principal avec ce binaire et le juge étendu ; il prévient dix minutes avant le premier pas et à chaque pas, c7 diffuse. **Le choix de G sur les règles** : la fenêtre passe en règles 13 ; la seule exception est que les morceaux bornés d'ED (règles 14, à reposer sur master, environ une heure) atterrissent avant que (c) démarre, alors un rebâti et un seul repassage ; une fois (c) parti, plus de changement, 14 viendra au repassage suivant, base ouverte, sans coupure. L'ordre d'ED est réglé : les morceaux d'abord (ils montent les règles), l'exclusion par défaut des fichiers minifiés ensuite (elle ne monte pas les règles, c'est l'admission, pas l'analyse ; son ordre est libre). Dans la même fenêtre : le mv de `Lucivy_Scope` (839 Mo) avec son écart nommé, et `reference.json` posé sur les services. Rien de `tracked: master` à déclarer. Banc rejouera le juge du nom exact sur la base basculée, à un seul côté, comparé aux relevés de feuille-5d, chaque côté sur ses gabarits.

**La couche par commit de fourche (O2) est déposée** par arbre : `arbre-fourche-a-la-demande`, 91adbd364d, en pile sur O1 (`arbre-open-view`, 9535776336). Les rouges : deux worktrees fourchés au même commit partagent `fourche:<commit>`, une seule génération ; chacun lit son écart et le g de sa fourche, le backend ne lit rien des deux ; `open_view` sur une base sans `tracked` rend sa couche active. Le vert : lib entière 2779/0, e2e_lectures 28, e2e_code 36, e2e_code_sync 27. La tête déposée est le correctif de check_features lui-même (`open_view` appelait `history_layers`, qui n'existe que sous `code` ; sans `code`, le worktree s'ouvre et la réponse dit que la couche demande la feature) ; la tête rouge de check_features n'a jamais été déposée ; l'ancienne branche `couche-de-fourche` (qui suivait master) n'est pas déposée, worktree et target retirés. **Le coût d'une fourche** est sur master (83c667e420, page d'arbre) : sur la copie de G, 361 fichiers d'écart entre la racine et la tête de master s'écrivent en 557 s, soit 1,5 s par fichier ; un gros lot de 73 fichiers, 43 s ; +4,3 % de base. La suite d'arbre : `open_view` qui rend le worktree tout de suite et écrit la fourche puis la couche en fond (réponse « index pas prêt »), rouge d'abord, en pile sur O2 ; puis la mesure des 1,5 s par fichier, pour ED. **Lucie, 18 h 5x**, à la question « c'est pas un gros changement de comportement tout ça, ça veut pas dire tout refacto ? » : non, un genre de couche de plus, additif ; les risques sont la résolution de la vue à trois étages et la garde du catalogue, tenus par deux rouges et le juge sur copie.

**Le confineur de validation a une course** : G a vu à 18 h 50 un faux rouge « descendants laissés après la commande » avec `restes: []` ; la garde compte la session `dbus-run-session` avant sa fermeture, rejeu identique vert. Le ticket est écrit par G (20226b5487) ; le lot (la garde attend la fin de la session, ou réessaie une fois après une courte grâce, rouge d'abord sur une fausse commande dont la session se ferme en retard) va à recherche, c'est la famille de son ticket du jour (« e2e_mcp_hote_designe laisse parfois des descendants »). D'ici là, G a déposé `pusher-course-du-confineur` : un tel rouge est rejoué une fois au lieu d'être rendu, noté « course du confineur » au journal (96/96, rouge d'abord).

**Atterris de 18 h 30 à 19 h 25**, tous propres : views/use_view (7c377c06d0, avec le correctif des deux sessions ouvertes dans la même milliseconde), les orphelins retirés par la synchronisation complète (e1f170fb43), les outils de l'équipier décrits à la phrase (6ffee49f96), le ticket R du banc (60be449abe), le ticket du confineur (20226b5487), le lot ED (f8c74d0876), la page du coût d'une fourche (83c667e420), la page d'ED des déclarations courtes (dc60f584dd : deux des trois premiers changés sont des scopes variable qui recopient toute la ligne, le défaut est chez codeparsers). **En file** : arbre-open-view (pris à 19 h 11), arbre-fourche-a-la-demande, cx-relecture-retenue-2 (créneau 2), emb-self-check, et index-fenetre de tty (37322f305c, l'avis dans la fenêtre du dépôt, reposé sur master, avec la fermeture de son ticket EINTR) dès son attestation. **tty** passe à la vue d'une tâche vivante au flux, accords répondables depuis la page comme le terminal les répond en o / n ; il a amendé un commit déjà poussé, push refusé, retour sur la tête distante sans rien forcer. **Worktrees tenus déclarés** : ED quatre (estimate-2, orphelins en file, groupe-page, morceaux-3 ; variable-not-candidate retiré avec 9,3 Go de targets), arbre deux à la tablette (open-view, fourche-a-la-demande), tty un par poste (index-pas-pret), banc deux de juge et leurs copies à retirer après sa lecture pour codeparsers.

## 20 novembre, 21 h 12 — la fenêtre de re-bascule jouée, les gabarits installés périmés, le critère remplacé, le confineur dans sa propre portée

**Le critère de la fenêtre a été remplacé, pas relâché.** La répétition de G sur copies (c) manquait le recouvrement à 90 % contre l'hôte posé (71 % sur les deux bases, même copie, même source, référence rejouée par baccb2ea6e à 20 h 03) ; banc l'avait vu l'après-midi (71 % sur master contre la base des règles 8) : entre les règles 9 et 13, le classement change par construction, ce seuil mesure le changement, pas une régression. Décision c7 (20 h 2x) : le juge devient 0 usage perdu (Catalog 0 perdu, 8 gagnés), journal propre, versions rendues ouvertes et rang publié ≥ 1 (lus par Cypher : 99/99 et 98/98 scopes ouverts, rang 1), et le juge du nom exact de banc sur la copie repassée (10/10, noms courts 13/13, trente cas sans écart, les trois premiers changés sont les bonnes définitions). Remplacer un critère exige la preuve qu'il ne départage pas, et un autre qui le fait.

**Les gabarits installés du service étaient périmés** (trouvé par banc à 20 h 1x : son premier juge sur la copie de G donnait 5/10, 43 %). Le manifeste de code-principal et de code-lourd citait ses graphes d'outils dans `~/.config/rag3weaver/<base>/templates/tools/*.mmd`, copies de l'installeur datées du 11 octobre, neuf sur quatorze en dérive : la fusion servie n'avait ni `dominant='bm25_exact'`, ni `exact_name`, ni `exact_name_leaf`, quel que soit le binaire posé ; les hôtes de Lucie servaient la recherche d'il y a cinq semaines. La tablette n'était pas touchée : ses manifestes désignent déjà les gabarits livrés du binaire (`builtin:`, Lucie, 19 octobre et 8 novembre). Décision : dans la fenêtre, les deux manifestes de luciepc passent au livré (sauvés en `backend.json.avant-builtin-20261120`, `templates drift` à 0) ; et un lot chez ED (ticket 0cddbdfc9a) : l'installeur pointe `builtin:` par défaut, une copie qui différait est sauvée à côté et dite, `--keep-copies` pour qui l'a modifiée exprès, la dérive dite au démarrage et à `/sante`. Leçon pour toute mesure par un hôte de service : lire d'abord la date et la fusion du gabarit installé contre `templates/` de la tête mesurée.

**La question des versions fermées est tombée, le revers de (A) est réel.** Banc et codeparsers avaient vu, sur la copie du juge d'ED, des versions fermées de scopes (la ligne entière d'avant le bornage aux octets) rendues en tête par le sens ; relu par l'hôte avec le contenu de chaque résultat : les scopes rendus sont les versions courantes et les générations 0, 1, 2 sont publiées, la clé `chemin:ligne` les confondait (page 06 de banc corrigée, 7a0f26d973 ; page 04 d'ED et son ticket pour codeparsers à corriger comme faux défaut). Mais des déclarations courtes sans vecteur prennent le premier rang de requêtes par le sens sur le seul BM25, parce qu'un mot de la requête est leur nom : « parse a json manifest » rend `manifest` (variable, 51 caractères) aux rangs 1 et 2, « rebuild the vector index » [python] rend quatre `index = ROOT/'README.md'`. Sur la référence ancienne (baccb, même source), la variable courte est au rang 3 ou 4 : la bascule apporte ce revers. Cause : la mise à l'échelle de (A) (×2,2 du BM25 d'une ligne sans vecteur). Décision : la fenêtre part quand même (ce qu'elle porte est le long et le risqué, le nom exact est gagné), le revers reste un défaut connu à ticket, et ED corrige (A') dans le graphe de fusion seul : plus de mise à l'échelle, le dense imputé au rang qui suit le dernier de sa liste, et, après le tri des dominants et avant le nom exact, toute ligne sans vecteur placée devant le meilleur candidat à vecteur est reportée juste derrière lui ; la règle du nom exact ne vaut que pour la requête entière après trim, jamais pour un mot parmi d'autres ni par le contenu (premier essai rouge à moitié chez banc : `manifest` et `s` restaient premiers par `bm25_exact`). Livré par un hôte neuf posé par G, sans repassage. Leçon de banc : juger le scope rendu (nom, genre, taille), jamais la ligne source.

**La fenêtre, jouée par G** (préavis 20 h 46, diffusé aux douze sessions ; Lucie : « quand vous voulez, même en avance »). Pas 1, 20 h 49 : services sans bail, aucun backend ouvert à luciepc. Pas 2 : hôte 83c667e420 posé à luciepc (réveil vert 20 h 50 min 16 s, aucune base ouverte, pas de coupure vue) et à la tablette (7970 coupé de 20 h 50 min 25 s à 20 h 50 min 54 s). Pas 3, luciepc : code-principal fermée 8 min 11 s (rouverte 20 h 59 min 26 s), code-lourd 10 min 56 s (21 h 02 min 11 s) ; instantanés `avant-versions-20261120-soir-<base>` avec retour arrière, contrôle des deux sens sans écart, 1b en 464 s et 613 s, générations en 1 s, manifestes au livré ; repassages en règles 13 base ouverte, la recherche répondant sur l'état publié. Pas 4, tablette : code-rag3db-depot fermé de 21 h 02 min 47 s à 21 h 02 min 53 s, `Lucivy_Scope` (885 Mo, mort depuis 06 h 43) déplacé en instantané jetable 72 h avec sa commande de retour, le .fts ne garde que File, Library, ScopeContent et Symbol, repassage 13 base ouverte (écart « 9449 versions écartées » attendu). Pas 5, 21 h 03 : `reference.json` posé à côté des poids de granite-278m à luciepc (24 textes, sha256 des poids, tolérance 5e-4, onnxruntime 1.30.0 CPU f32) ; rien pour bge-m3 tant que le lot de la mémoire libre n'a pas atterri (sa vérification au démarrage enverrait 32 textes longs, la forme qui a fait tomber le 7879) ; les services gardent leur binaire ce soir, `/sante` reste « non vérifié » jusqu'à une repose ≥ e2725d9f6c ; la tablette n'a pas de service local (7979 et 7980 sont les tunnels). La fenêtre se fait en règles 13 : les morceaux d'ED (14) ont atterri à 20 h 41, après le départ de (c), et iront au repassage suivant, base ouverte. Restent, à 21 h 1x : le jugement des trois journaux et des usages, puis le juge de banc sur les bases vivantes (pas 6).

**Le 7879 est tombé douze secondes** (20 h 35 min 45 s à 20 h 35 min 57 s, relancé par systemd) : une requête d'essai d'embarquement de 32 textes longs a voulu réserver 4,36 Gio sur card2, « can't allocate buffer », la chute nommée du 15 novembre (code 70). Ticket d57df0dd08 : un lot accepté peut dépasser la mémoire libre de la carte et le service tombe au lieu de recouper ; lot derrière la référence bge-m3 (chaque lot borné par la mémoire libre lue avant l'appel, recoupé). Règle rappelée à embarquement, écrite le 14 novembre : rien n'est envoyé aux services en place (7878, 7879), ni essai ni mesure ; les essais vont sur son propre service (7995) ou sur une carte sans service. La tolérance de bge-m3 a été fixée sur l'APU de la tablette : écart 8,4e-4 à l'ONNX, retenu 5e-3.

**Le confineur de validation vit dans sa propre portée plafonnée.** À 21 h 05, Lucie a vu plusieurs fenêtres « Pénurie de mémoire évitée : le service d'arrière-plan run-p…-i….scope a été interrompu par le noyau » : c'était le témoin de portée réelle du confineur (recherche, `scripts/test_confiner_validation.py`) joué sur la tablette, qui allume exprès le tueur OOM dans une portée limitée à 100 Mo, et le bureau notifie chaque portée tuée ; 66 Go libres, rien ne manquait. Arrêté ; règle : aucun témoin qui provoque un OOM, un SIGKILL de portée ou une faute injectée au niveau du système sur un poste où une session graphique est ouverte, luciepc seulement. Ce que le journal a montré (`journalctl -k`, à luciepc et à la tablette) : deux victimes en 3 ms dans la même portée, l'enfant qui alloue puis le confineur lui-même (le pid qui nomme la portée `run-p<pid>`) ; `oom_score_adj` choisit l'ordre des victimes, pas leur nombre (branche `confineur-oom` écartée). Lucie, 21 h 0x : « relou ces notifs, il n'y a pas un moyen de s'arrêter nous-mêmes quand la mémoire qu'on cherche à allouer n'est pas dispo ? ». Décision (voie B de recherche) : `poste` pose `Delegate=yes` sur sa portée (G, après la fenêtre, lourd et mesure) ; le confineur crée deux feuilles, `superviseur` (lui) et `commande`, celle-ci avec `memory.high` = le budget (le noyau ralentit, ne tue pas) et `memory.max` = le plafond moins une marge (garde-fou) ; le confineur surveille `memory.current` et `memory.events` de la feuille, arrête lui-même la commande au budget (SIGTERM, grâce, SIGKILL) et écrit « mémoire dépassée » ; l'enveloppe de `poste` se lit à la feuille. Témoins à luciepc seulement : une commande au-delà du budget est arrêtée par le confineur avec verdict écrit, sans oom_kill ; un OOM réel du noyau dans la feuille laisse quand même un verdict ; rouge sur master (confineur en -9, verdict absent). Ticket écrit (d1c4fb42bf, à déposer en docs après l'atterrissage de confineur-grace), code demain matin. À part : `recherche-confineur-grace` (la grâce de fermeture d'une seconde, 790f199e4c) passe tel quel ; son rouge de créneau 2 (20 h 59) est indécidable, le pousseur écrase `last-witness.log` au train suivant (manque dit à G).

**La tablette.** Lucie, 20 h 5x : « on peut bosser sur la tablette à partir de ce soir jusqu'à lundi matin, en addition de luciepc » ; à 21 h 1x, encore devant son écran, elle la trouve lente : deux `cargo test` en parallèle (ED gabarits, embarquement bge-reference) et le repassage du dépôt, charge 15 sur 32, 43 Go en zram (swappiness 150) avec 68 Go libres, pression d'entrées-sorties nulle (pas le SSD, le TRIM était l'affaire de luciepc). Décision : deux lourds et régime doux tant qu'elle est devant ; G monte les places à son « je quitte l'écran ».

**Les faux pas et les règles du soir.** c7 a poussé un commit de chronologie direct sur master à 19 h 24, pendant le push d'arbre-open-view par le pousseur : garde « push refusé », repose au pas suivant (treize minutes de passe), créneau 2 de relecture-retenue jeté ; ticket de G (d561ac1a32, correctif après la fenêtre : sur « push refusé », relire master, et si l'arrivée n'est que des docs, rebaser et repousser jusqu'à trois fois) ; d'ici là, la chronologie se dépose au pousseur en train docs depuis un worktree à c7. Codeparsers a déposé les noms intégrés en règles 13 alors que master y était déjà et que 14 était en passe : retiré, à reposer en 15 avec l'empreinte de la tête reposée ; c7 tient l'ordre des numéros, aucune session ne monte les règles sans le dire avant. La chronologie du soir : atterris de 19 h 25 à 21 h 10, tous propres : open-view O1 (19264108f2), relecture-retenue (d5fe995216), ticket de la garde (cc25f4396c), emb-self-check (e2725d9f6c), course du confineur (68fe32d396), O2 (b720859a29), index-fenetre (bdae042db9, tickets EINTR et avis fermés), ticket des gabarits (0cddbdfc9a), morceaux bornés règles 14 (a44474d874), ticket du 7879 (d57df0dd08), open-view-en-fond (86666fbde1), page du coût d'une écriture (08b9a150b9 : 1,0 s par fichier, la couche n'est pas en cause, le temps dans « entités · dérivations », question à ED par les plans), exclusion par défaut aux motifs bornés (ed9568a811 : `*bundle*` écartait des sources comme `bundle.rs`), flush de lucivy fermé (2b7e2a7ee6). Déposés : O3 close_view (9812875892 : refus dits, générations gardées lisibles par id après une fermeture à la main, purge et retrait à l'atterrissage avec O4), sonde lucivy sans connus, confineur-grace. lucivy 4.3.13 (le refus `min_lucivy`) est prête, publication au mot de Lucie, posée sur les hôtes à un hôte neuf avant 4.4.0.

## 20 novembre, 21 h 37 — la fenêtre close, la frontière gabarit / script / Rust posée par Lucie, la tablette à plein jusqu'à lundi

**La fenêtre de re-bascule est close à 21 h 3x, sans retour arrière** (page de G, 4c85099941). Pas 3 : code-principal repassée à 21 h 11 min 14 s, code-lourd à 21 h 15 min 29 s, journaux propres (ni rebâti, ni écart, ni garde), aucun usage perdu (acquireLock 5 = 5, FileSource 99 = 99, lockRowForWrite 3 = 3, Catalog 348 → 900 et 348 → 859 sites : le `usages` livré) ; le recouvrement, relevé mais qui ne juge plus, vaut 31 et 33 %, attendu puisque le graphe servi a changé. Pas 4 : le dépôt repassé en 10 min 27 s, « 12 373 versions de conteneurs repliés écartées », comptes qui collent (ScopeContent 97 539 = 97 539, File 26 007, Symbol 93 140, Library 482), sans rebâti ; l'écart avec les 9 449 nommés (2 924) est, lu par ED dans le code, la somme des gestes des deux règles de non-candidats (les conteneurs repliés et, pour la première fois en règles 13, les déclarations courtes) sous un libellé qui ne nomme que la première (ticket chez ED) ; la preuve par Cypher se joue sur la copie de G, zéro morceau restant attendu. Pas 5 : granite seul. Pas 6, le juge de banc sur les bases vivantes par 7970 et les gabarits livrés : code-principal 10/10, 100 %, 13/13, 30 cas sans écart ; code-lourd et le dépôt verts à un côté. **Un point ouvert, qui n'est pas la fenêtre** : sur le dépôt, cinq noms courts (`__all__`, `lib`, `result`, `con`, `PORT`) absents du top 10 sans homonyme ; banc a lu la cause : ni fichier écarté, ni rang publié, le plein texte existe et `search_code` filtré sur le fichier les rend premiers ; **la promotion du nom exact ne voit qu'une liste fusionnée bornée** (`lib` et `result` ont 13 et 32 porteurs du nom dans le top 50 BM25, aucun dans le top 10 ; `con` et `__all__` hors du top 50 ; une méthode `result` avec vecteur n'est pas promue non plus). Ticket (2) reformulé chez ED : le nom exact se cherche par le nom, une lecture directe et bornée des scopes dont le nom est la requête entière, injectée avant la fusion ; dans (A') ou juste après. (A') lui-même, premier essai rouge à moitié chez banc (`manifest` et `s` restaient premiers par `bm25_exact`), est repris : après le tri des dominants et avant le nom exact, toute ligne sans vecteur devant le meilleur candidat à vecteur est reportée derrière lui, et le nom exact ne vaut que pour la requête entière.

**La frontière gabarit / script / Rust, posée par Lucie.** Sa question de 21 h 3x : « à quel point le code, pour des features et non des correctifs, devient juste l'écriture de gabarits, et à quel point on doit encore hardcoder ? ». Relevé du jour : assembler et régler se fait en gabarit, ajouter une capacité se fait en Rust ; deux ou trois lots sur trente étaient du gabarit seul. Les primitives (read, replace, grep, fetch, web_search) sont en Rust pour les gardes, le contexte de l'hôte, le contrat typé et l'autonomie du binaire, la performance venant en dernier. Puis sa seconde question : « on avait déjà géré le TypeScript comme langage de script dans les nœuds ? » : oui, `ScriptedNode` (`src/dataflow/scripted_node.rs`, déclaration avec ports et schémas, script `.ts`, `.js` ou `.rhai`, dépouillé par swc et joué dans QuickJS embarqué), employé par le seul proto `templates/proto/boutique` (deux nœuds, trois vues, « rien ne se compile »), aucun gabarit livré ne s'en sert ; le TypeScript du dépôt est sinon l'IDE (37 fichiers), la liaison Node et des fixtures. **Sa règle, 21 h 4x : « ce qui a besoin de flexibilité d'implémentation → script ; ce qui a besoin de performance et qui est une primitive → Rust ».** Décidé : une page courte d'ED et archi, « ce qui reste codé en dur, et pourquoi », par domaine, en trois colonnes (gabarit, nœud scripté, Rust) avec le coût de chaque passage ; le défaut pour un nœud neuf de fonctionnalité devient le script, sauf primitive ou performance mesurée ; archi a posé sa part (2991ec0c3a : système et sûreté en Rust, textes, bornes, choix et compositions en déclaré), ED ajoute la sienne avec l'état des gardes du nœud scripté (temps, mémoire, ni fichier ni réseau).

**La tablette, finalement à plein.** Lucie, 21 h 2x : elle joue sur luciepc, seat1, card0 (la carte qu'on ne touche jamais), pas sur la tablette : « vous pouvez abuser de la tablette autant que luciepc, comme d'hab, pas grave si elle rame un peu ». Réglage de G, 21 h 3x : trois places lourdes à la tablette jusqu'à lundi 23 novembre 8 h (retour automatique à deux par `lourds-max.retour` ; `poste places` ne sait dater qu'une baisse, lot à venir pour une hausse datée et le mot « réglées »), `embedder-allow-integrated-gpu` remis (suspendu quelques minutes pendant qu'on la croyait devant son écran), le régime doux des passes reste écrit en dur dans `passe_epinglee.sh` hors luciepc (lot de déclaration par poste), le pousseur reste à luciepc (un créneau sur un autre poste est le chantier des N hôtes) ; consigne diffusée : attestez et jouez vos témoins à la tablette. Cœur y a pris une seconde série ASan ×10 de C1. luciepc inchangé : card2 pour nos services, rien sur card0. Ce qui ramait à 21 h 08 : deux `cargo test` en parallèle et le repassage du dépôt, charge 15 sur 32, 43 Go en zram (swappiness 150) avec 68 Go libres, pression d'entrées-sorties nulle ; pas le SSD.

**Le reste de l'heure.** Le confineur (recherche) : le ticket est sur master (be7c2e6352, la voie B et ses témoins, le code demain matin, après `Delegate=yes` de G) ; le témoin (2), un OOM réel dans la feuille, va dans la suite à faute injectée à part, avec une garde qui refuse en le disant quand `loginctl` montre une session graphique active, jamais un saut silencieux. Recherche se voit refuser `git worktree remove --force` par le contrôle de permissions de sa session (nécessaire pour un worktree à sous-module) et ne contourne pas par `rm -rf` : la demande d'une autorisation explicite de Lucie sur ses worktrees sous `~/git_workspaces/recherche/` est portée, un worktree propre reste en place à luciepc d'ici là. Embarquement : `emb-bge-reference` déposé (dfe9e303ff, tolérance 5e-3 mesurée sur l'APU) ; la conception du lot de la mémoire libre validée avec deux ajouts (une allocation refusée recoupe le lot de moitié si elle n'empoisonne pas le serveur de la carte, à vérifier sur l'APU d'abord ; le calibrage prouvé passivement sur card2 par sysfs autour d'un lot réel du 7879). Mémoire : la pile de dix déposée (b1-collect atterri 6f62de7734 : Collect, le contexte déclaré de LlmNode ; b2 en passe), lot suivant `fetch` et `web_search` par le grounding Gemini. Atterris aussi : la page du juge de banc (31e9459ca4), la sonde lucivy sans connus (091d8ad3e7), la chronologie de 21 h 13 par le pousseur (327534c972). Arbre : O3 `close_view` déposé (9812875892), O4 ensuite. Codeparsers : les noms intégrés à reposer en règles 15. lucivy 4.3.13 attend le mot de Lucie.

## 20 novembre, 23 h 16 — les remèdes de la recherche verts, le nœud scripté à valider, le 7879 expliqué, l'essai de Lucie préparé pour demain

**La recherche, après la fenêtre.** (A'), `ed-vectorless-below-best` (e5f1f8052d), est vert au juge de banc et déposé à 21 h 21 : après le tri des dominants et avant le nom exact, toute ligne sans vecteur placée devant le meilleur candidat à vecteur est reportée juste derrière lui, et le nom exact ne vaut que pour la requête entière après trim ; les douze noms exacts restent premiers (et `ENTETE` passe premier), « parse a json manifest » rend `parse_declaration` (`manifest` au rang 2), « read_text » ne rend plus `s`, « rebuild the vector index » rend `file_scope_08` (`index` au rang 3), quarante cas sans ligne sans vecteur en tête ; le lot porte la page 04 corrigée, le faux ticket pour codeparsers fermé, le ticket du revers fermé. **Les cinq noms courts absents du dépôt** (`__all__`, `lib`, `result`, `con`, `PORT`) ont leur cause, lue par banc : ni fichier écarté, ni rang publié, le plein texte existe et `search_code` filtré sur le fichier les rend premiers ; la promotion du nom exact ne voit qu'une liste fusionnée bornée (`lib` et `result` ont 13 et 32 porteurs du nom dans le top 50 BM25, aucun dans le top 10 ; `con` et `__all__` hors du top 50 ; une méthode `result` avec vecteur n'est pas promue non plus). Ticket reformulé chez ED, lot suivant : le nom exact cherché par le nom, une lecture directe et bornée des scopes dont le nom est la requête entière, injectée dans les candidats avant la fusion (`ExactNameNode`, témoin à rendre fidèle d'abord : sur un petit index la déclaration reste dans la liste bornée). **Les gabarits qui suivent le binaire** sont déposés (`ed-templates-follow-the-binary`, 0c14e99360) : l'installeur pointe `builtin:` par défaut, une copie qui différait est sauvée à côté et dite, `--keep-copies`, `/sante` donne `gabarits.livres` et la `derive` par backend, la ligne `[gabarits]` au journal d'ouverture ; ticket fermé. Le compte du dépôt (12 373 contre 9 449 nommés) est la somme des gestes des deux règles de non-candidats, la seconde jouant pour la première fois en règles 13 ; libellé à corriger, ticket chez ED ; preuve par Cypher sur la copie de G.

**La frontière gabarit / script / Rust, suite.** Lucie, 21 h 4x, devant la page d'archi en trois colonnes : « ce qui a besoin de flexibilité d'implémentation → script ; ce qui a besoin de performance et qui est une primitive → Rust » ; puis « il faut d'abord valider le scripting, qu'il fonctionne bien, on ne sait jamais, et qu'il est pratique là où il est placé » ; puis, 21 h 5x : « si c'est un truc en construction qu'on n'est pas sûr, comme l'archiviste, si c'est scripté ou entièrement des nœuds, on reteste différentes configurations sans tout rebâtir ». Décidé : un lot de validation du nœud scripté chez ED, sur une vraie petite fonctionnalité (la mise en forme de « Previous step », `StepWork::block` / `carry_over`, avec les témoins S2 d'archi comme oracle à l'octet), gardes prouvées rouge d'abord (temps borné, mémoire bornée, ni fichier ni réseau depuis le script, un script qui boucle arrêté et dit), erreurs rendues avec fichier et ligne du `.ts`, rechargement à chaud, deux configurations du même script rechargées sans rebâti, mesure en release contre le nœud Rust, page d'ergonomie ; avant le mécanisme d'archi (le pas appelle un script à des points déclarés : composer la frontière, juger un pas, rendre un résumé, choisir une famille ; état JSON entrant, décision JSON sortante, défaut Rust ; environ deux jours ; la voie de la graine « un agent édite son propre workflow »). Archi a posé sa part en trois colonnes (4b2594ab93, puis 52e3ed7890 : ce qui bouge encore va d'abord en script) ; le proto `boutique` est vivant (chargé par les tests de rechargement, `code_gabarits.rs` et trois e2e MCP), à garder ; les gardes passent par `ScriptLimits`, à lire. Question de Lucie sur un terminal piloté (FIFO) pour un agent : problème de riche, tmux suffit pour un débogage, « l'extension navigateur est bien plus prio » (23 h 1x).

**Le 7879, expliqué par la mesure** (embarquement, 21 h 4x) : sous radv, une allocation unique est limitée à 4 Gio (`maxMemoryAllocationSize`, card2 comme l'APU) ; le tampon d'attention demandé à 20 h 35 (4,36 Gio) dépassait cette limite, pas la mémoire libre (12 Gio libres) ; et une allocation refusée empoisonne le serveur de la carte pour de bon (« Parent device is lost », « The server is in an invalid state », les petits lots suivants paniquent), donc pas de réessai possible. Conception acceptée (page 3a519b1db1) : deux bornes estimées avant l'appel (le plus grand tampon du lot, lot × têtes × T² × 4 octets, calé exactement sur les deux refus mesurés, sous la limite d'allocation de l'adaptateur avec marge ; le total sous la moitié de la mémoire libre), les jetons comptés par le tokenizer, un texte seul qui ne tient pas reçoit un 503 nommé (bge-m3 ne peut jamais calculer sa pleine fenêtre de 8 192 jetons sous radv), estimation majorante prouvée sur l'APU puis passivement sur card2. La référence de bge-m3 est sur master (c3f809fe9a, tolérance 5e-3, six fois l'écart mesuré sur l'APU) ; les suites locales déposées (`suite_locale()` lit le nom du test, plus celui du binaire).

**Le confineur, voie B.** Recherche a confirmé à luciepc que `oom_score_adj` ne suffit pas (le confineur vit dans la portée plafonnée, le memcg resté plein continue jusqu'à lui) ; la voie B est tranchée avec le mot de Lucie (« s'arrêter nous-mêmes quand la mémoire n'est pas dispo ») : `poste` pose `Delegate=yes` sur ses portées (lot de G `poste-delegate`, 66eae5e4f1, rouge d'abord, déposé), le confineur crée deux feuilles, `superviseur` et `commande`, celle-ci avec `memory.high` = le budget et `memory.max` = le plafond moins une marge, surveille `memory.current` et `memory.events`, arrête lui-même la commande au budget et écrit « mémoire dépassée » ; ticket sur master (be7c2e6352), code demain matin ; le témoin (2), un OOM réel dans la feuille, va dans la suite à faute injectée à part, avec une garde qui refuse en le disant quand `loginctl` montre une session graphique active. La grâce de fermeture (`recherche-confineur-grace`) a atterri (da55e9d7a6). Le flux sans verrou du chat est déposé (82c141b1bd : chaque fenêtre SSE a son fil d'écriture et une file bornée, 131 tests, 54 témoins navigateur). Recherche se voit refuser `git worktree remove --force` par sa session et ne contourne pas : autorisation demandée à Lucie pour ses worktrees sous `~/git_workspaces/recherche/`.

**Le poste et le pousseur.** Lucie, 21 h 2x : elle joue sur luciepc seat1 (card0), « vous pouvez abuser de la tablette autant que luciepc, pas grave si elle rame » : trois places lourdes à la tablette jusqu'à lundi 23 novembre 8 h (lot `poste-places-datees` de G pour la hausse datée et le mot « réglées »), `embedder-allow-integrated-gpu` remis, consigne « attestez à la tablette » diffusée ; cœur y a pris une seconde série ASan de C1. La garde du pousseur qui repousse sans rejeu sur une arrivée de docs seules est déposée (`garde-repousse-docs`, ticket fermé) ; le créneau 2 interrompu s'écrit « jeté » avec sa raison (`pusher-creneau2-jete`) ; un créneau 2 mort sans trace (memoire-extraction-cut, 22 h 01) a son ticket (8e74eefa0f). lucivy 4.3.13 est publiée (tag v4.3.13, 21 h 27, au mot de Lucie dans la session lucivy : le refus `min_lucivy`), pointeur déposé (200b36a177). Mémoire : la pile de dix a atterri jusqu'à extraction-cut (b1 Collect 6f62de7734, b2 présentation déclarée 02e4efea74, b3 extraction 6bbbd36b18, b6 secrets 54eeb12e64, extraction-cut 1922f262ad), puis template-version est rouge sur la lib (un test de master comparant deux `metadata.template.hash` désormais différents, et l'aide `manifeste_jouable` réécrivant tout champ `template`) : les cinq lots restants retirés, corrigés, reposés sous des noms neufs, rejoués, à redéposer. Codeparsers : les noms intégrés en règles 15 sont sur master (466b974d3c) ; les bases de Lucie restent en 13, 14 et 15 iront au repassage suivant ; le lot 1 de l'intra-scope mesure la couverture (Rust 12,0 % de références, 17,5 % d'écartés, 52,8 % de structure sur 1,8 M de nœuds) et, pour le corpus de l'empreinte, règle posée : un corpus qui change remplace la ligne du numéro courant avec la preuve de l'ancienne valeur, une analyse qui change monte le numéro. Hotfixs (22 h 25) : seconde version du correctif HNSW (49afee87fe, rattache au plus proche voisin atteint), 0 injoignable sur 16 tours réels, CREATE 67 à 78 s sous charge contre 153 s pour la première, coût à mesurer sous `poste mesure` des deux côtés ; un rouge neuf sur le chemin de suppression (2 injoignables après `deleteFromGraph`, 1 tour sur 16) en ticket à part ; dépôt vers 23 h 30. Arbre : O3 `close_view` atterri (fc300b6447), O4 en cours. Archi : raisonnement borné déposé (b298b3ef7).

**L'essai de Lucie, demain.** Lucie, 23 h 0x : « tu penses que demain je pourrais essayer une expérience en terminal plus aboutie ? et sur l'IDE aussi ? ». Oui : G pose dans la nuit un seul hôte neuf quand (A'), le pointeur lucivy 4.3.13, le flux sans verrou et le raisonnement borné ont atterri (la tâche vivante de tty en cinquième si elle est en file à temps, sinon un second hôte demain matin), (b) seul avec `LUCIVY_ATTENDU=4.3.13`, sans repassage ; tty joue un essai réel par le chemin de l'hôte dans le terminal, recherche dans l'IDE de la tablette ; page « parcours d'essai » de tty et archi (`docs/tty/20-11-2026/03-parcours-d-essai.md` ; la part d'archi est posée, c7df32e881) : quoi lancer, quoi essayer, l'attendu, et les manques connus (pas de fetch ni de recherche web, pas de cron, pas de navigateur, la palette par familles pas encore, le sous-agent à vues en cours). Atterris aussi de 21 h 4x à 23 h 15 : la chronologie de 21 h 4x (1992412616), le juge de banc (3e5279cef0), la page d'embarquement (3a519b1db1), close_view, la référence bge-m3.

## 21 novembre, 00 h 37 — la nuit : les scénarios de Lucie, le pousseur en pause, l'identité inventée, les docs par le pousseur

**La consigne de nuit de Lucie** (20 nov., 23 h 3x, avant de dormir) : « demain il faut que je puisse tester terminal et IDE plus aboutis, avec gestion des vues pour l'agent, et les capacités relatives d'un agent de code standard ; dans la nuit, faire des tests comme si : 1. un dev qui connaît les lacunes et veut travailler avec rag3weaver sur un projet ; 2. un dev qui ne connaît rien au projet et veut lancer un projet de zéro (dont création du dépôt git, sans push) ; 3. des scénarios qui reproduisent vraiment des correctifs que vous aviez faits ». Puis, 23 h 4x : « l'IDE n'est pas fait du tout pour rechercher mais pour travailler ; c'est un environnement de travail encore plus complet que le terminal, avec gestion de tous les agents actifs ou non, savoir tous les dossiers sur le PC ouverts dans rag3weaver ». Et : « l'extension navigateur est bien plus prio » qu'un terminal piloté (FIFO, tmux : problème de riche). Répartition : tty (terminal, scénarios 1 et 2), recherche (IDE, 1 et 2, jugés sur le travail : agents, dossiers ouverts, pilotage terminal ↔ IDE, accords, tâche vivante), archi et banc (scénario 3 : l'équipier rejoue par le lanceur, hôte de campagne isolé, trois correctifs réels depuis leur commit parent, rouge prouvé au parent : A la réponse coupée de mémoire, B le bac sans sccache, C la chaîne Rust du bac ; le motif `*bundle*` attend le lot `needs_lib` du lanceur, ticket 726676b9ea), arbre (les vues), ED et mémoire (recherche, index, mémoire), G (deux hôtes neufs : après les cinq lots, puis avant 8 h avec les correctifs de la nuit ; le pousseur vivant toute la nuit). Règles : par le chemin de l'hôte que Lucie prendra, sur copies et dossiers neufs, jamais ses bases ni son arbre ; chaque manque un ticket avec le pas exact ; petit et sûr se corrige dans la nuit ; relevés à c7 avant 7 h 30 ; page du parcours d'essai de tty (`docs/tty/20-11-2026/03-parcours-d-essai.md`, la part d'archi posée c7df32e881).

**Premiers manques, avant même l'essai.** `rag3weaver-tty` n'est installé sur aucun poste (ni PATH ni ~/.local/bin) : les scripts de pose de G bâtissent désormais le terminal de la même tête que l'hôte et le posent avec lui aux deux postes ; ticket produit (3b6d12c2e1) : l'installeur de l'hôte doit installer le terminal (règle de Lucie, « les gens n'installeront rien en plus de rag3weaver »). **L'hôte « isolé » de tty ne l'était pas** (23 h 43) : lancé avec `RAG3WEAVER_CONFIG_DIR` et `RAG3WEAVER_DATA_DIR`, il a repris les sessions de chat de la tablette et l'équipier de l'atelier de Lucie pendant neuf secondes, et son journal d'arrêt a annoncé « daemon.json retiré » alors que le fichier du service était intact (G : mtime 20 h 50 min 54 s, pid du service, clients MCP qui le trouvent). Cause lue par archi (ticket 88a3d937e2, corrigé 1a640ebc76 : les faits d'abord) : les tâches et les chats sont partagés par racine, `DATA_DIR` ne couvre que `code_tree::Roots`, et `reprendre_les_equipiers` relance au démarrage tous les équipiers de la racine ; 38 lectures de HOME dans `src/`. Règle : un hôte d'essai se lance avec un HOME à lui et `RAG3WEAVER_DAEMON_DIR`, comme le lanceur d'archi ; lot de la nuit chez archi : un hôte ne reprend jamais un équipier qui n'est pas le sien ; puis jamais un retrait de `daemon.json` qui n'est pas le sien ; la racine unique `RAG3WEAVER_HOME` en lot d'un jour.

**Le scénario 2 dans l'IDE** (recherche, 00 h 35, relevé 34cd6a0a80 : hôte isolé à HOME, Chrome sans tête par CDP, le chemin de Lucie) : vert au bout (dépôt local, commit, worktree attaché et indexé), trois manques. (1) Bloquant au premier pas : l'IDE n'ouvre ni ne crée un projet ; sa page vide dit « un dépôt paraît ici quand un agent ou l'atelier l'ouvre », sans commande, aucune route `/ide/*` n'ouvre un dossier ; contournement `rag3weaver-backend atelier` depuis le dossier. Décidé : le petit correctif (la page vide donne la commande) et la vraie pièce cette nuit, recherche et arbre : ouvrir un dossier depuis l'IDE (route `/ide/ouvrir`, l'atelier posé et attaché, un dossier neuf créé avec `git init` sans push, la liste des dossiers ouverts du poste par la table des dossiers et les vues, gardes de l'arbre de Lucie et des dossiers d'autres sessions). (2) **Sérieux : l'équipier a fabriqué une identité.** Le bac à sable des commandes pose `GIT_CONFIG_GLOBAL=/dev/null`, le premier commit échoue, et l'agent a posé `Lucie <lucied@example.com>` par deux cartes `git config user.*`, que le commit porte. Décidé sans attendre Lucie (sûreté) : le bac à sable reçoit l'identité git de la config globale de l'hôte (lue une fois, `GIT_AUTHOR_*` et `GIT_COMMITTER_*`), surclassable par une identité déclarée dans le manifeste de l'atelier ; une carte `git config user.*` est refusée en le disant ; l'agent n'invente jamais une identité ; chez archi cette nuit, rouge d'abord, l'hôte du matin doit le porter. (3) Confort : cinq cartes d'accord pour un premier commit, `git add .` en double, l'agent écrit `Cargo.toml` pendant que `cargo init` attend son accord (tickets). Pour la page de tty : le premier pas hors de l'IDE, refuser les cartes `git config`, accorder la plus récente d'abord.

**Le pousseur dans la nuit.** À 23 h 15 il s'est mis en pause tout seul (`pusher/paused`, « appeler c7 ») : la recopie de `push_garde.sh` après le lot garde-repousse-docs voyait son témoin rouge contre la copie, parce que ce lot avait atterri par le créneau 2 dont `spec_settle` exporte `PILE_AVANT`, et la recopie jouait `test_push_garde.sh` dans cet environnement (8 rouges « la tête d'avant n'est pas un ancêtre » ; 63/63 en environnement propre) ; le message citait un chemin faux. À mon oui, G a recopié la garde à la main après rejeu propre (23 h 27) et retiré la pause ; lot `pusher-recopie-env-propre` atterri (2e6f0bc556). Puis deux rouges de `check_tickets` sur master par un ticket déplacé dans `closed/` avec un État « ouvert », le `git mv` commité sans le `git add` du fichier corrigé : G à 23 h 15 (passé par le créneau 2, qui ne jouait pas la garde des tickets ; corrigé 8ba1d3f4f8), embarquement à 00 h 20 (6de327eb49, un push direct, hors de vue du pousseur ; corrigé ae6e86cbe0 ; la file rust retenue dix minutes). Décidé et diffusé à toutes (00 h 2x) : plus aucun push direct sur master, docs, tickets et pages compris ; tout passe par le pousseur (train docs, une minute) ; le contrôle d'un ticket se joue sur ce qui est commité (`git show HEAD:`, `git archive HEAD`), jamais sur l'arbre de travail ; la règle est dans `docs/procedures/push-d-un-lot.md` (e862e1dafc, déposée par le pousseur depuis mon worktree). Lots de G : `pusher-tickets-partout` (la garde des tickets dans le créneau 2 et dans un train rust, 99/99), `pusher-creneau2-jete` (une passe interrompue s'écrit « jeté » avec sa raison, jamais « rouge » sans motif ; un créneau 2 mort sans trace à 22 h 01, ticket 8e74eefa0f), `poste-places-datees-2` et `pusher-creneau2-jete-2` reposés après conflits avec ses propres lots (`test_poste.sh`, `test_pusher.sh`). La chronologie de c7 passe désormais par le pousseur, une branche par entrée.

**Atterris de 23 h 15 à 00 h 36**, tous propres : la garde qui repousse sur des docs (aba24ff6ea, ticket fermé ec3e13fdcd), la recopie en environnement propre (2e6f0bc556), le ticket du terminal (3b6d12c2e1), les gabarits qui suivent le binaire (a51356d472 : `builtin:` par défaut, copies sauvées et dites, `--keep-copies`, `/sante`), `Delegate=yes` sur les portées de poste (820a2d76ce), le flux sans verrou du chat (a60f9f7a71 : un fil d'écriture et une file bornée par fenêtre, 131 tests, 54 témoins navigateur ; ticket fermé 9d575fb99a), le pointeur lucivy 4.3.13 (2bfcc013d2), les suites locales d'embarquement (fd7a80df79), (A') (e1845431d7 : le plafond après le dominant, le rang imputé, trois tickets), les tickets C1 de cœur (80f0edbce1, 57ce578a66 : le core dit un débordement du tampon de chaînes d'un dictionnaire vers ses offsets au point de reprise, les métadonnées des enfants OFFSET et DATA d'un segment publiées séparément : témoin le 21), le ticket de l'isolement (88a3d937e2, 1a640ebc76), la page de la mémoire de la carte (3a519b1db1), le relevé du scénario 2 (34cd6a0a80). Déposés : la pile de mémoire reposée après son rouge (template-version : un test comparant deux `metadata.template.hash` désormais différents et une aide réécrivant tout champ `template`), ed-exact-name-by-name (7b76c260b6, juge de banc vert : les noms absents reviennent premiers, +2 % de coût), raisonnement-borné (b298b3ef7, jeté deux fois en créneau 2, son train propre à venir), la tâche vivante de tty (b51a77e3d0), la couverture mesurée de codeparsers (eaa9bd4dd7 ; règle du corpus d'empreinte : un corpus qui change remplace la ligne du numéro courant avec la preuve de l'ancienne valeur ; lot 2 des occurrences autorisé, numéro 16 réservé, coût d'écriture mesuré d'abord), O4 (eae48333cb : la fermeture tirée des atterrissages par la voie déclarée de `atterrissages.json` ; garde demandée avant le service : ne fermer une vue que si HEAD est la tête déposée du lot atterri, les sessions gardant leur worktree pour le lot suivant), emb-memory-bound (5028c60dde : deux bornes avant l'appel, prouvé sur l'APU, wgpu limite à 2 Gio, un texte de 8 192 jetons sur bge-m3 reçoit un 503 nommé ; la référence de bge-m3 ne se pose à côté du 7879 qu'avec ce binaire). lucivy 4.4.0 a sa PR (#59 ; `commit()` refusé sur un index étiqueté, à porter dans le pointeur de rag3weaver) ; tag au seul mot de Lucie après la pose de 4.3.13. Hotfixs (22 h 25) : seconde version du correctif HNSW (49afee87fe), 0 injoignable sur 16 tours, coût du CREATE à mesurer sous `poste mesure` des deux côtés, un rouge des suppressions en ticket à part. Cœur : treize tours ASan sans voir C1 ; la série de la tablette continue.

## 21 novembre, 02 h 09 — l'hôte sans compteur, le moteur HNSW atterri, la lib épinglée, les scénarios et leurs manques

**L'hôte du soir posé sans la feature `token-count`** (01 h 29). Le premier pas du scénario 1 de recherche l'a vu : dans l'IDE de la tablette, le chat de `rag3db · master` restait sur « Connexion… », `/ide/chat` répondait « le découpage pour granite-278m compte ses jetons, et ce bâti n'a pas le compteur : il faut la feature token-count » ; puis `search_code` par le MCP code-rag3db de la tablette échouait de même, `rag3db open: false` : toutes les sessions de la tablette ont perdu l'index de code de 01 h 29 à 01 h 33. Cause : le script de bâti de G (`bati-lucie-luciepc.sh`) bâtissait `rag3db-native,code,daemon,openai-llm`, sans `burn-embedder`, ce qui suffisait jusqu'aux morceaux bornés (règles 14, 20 h 41) ; son essai n'ouvrait ni recherche ni chat sous granite. Retour arrière de G sur l'hôte de 83c667e420 (tablette 01 h 33 min 28 s à 30 s, luciepc 01 h 33 min 52 s à 53 s) ; le script bâtit désormais avec les features de service (`rag3db-native,burn-embedder,burn-ocr,code,daemon,openai-llm,anthropic-llm`), refuse tout binaire où `strings | grep -c "il faut la feature token-count"` n'est pas 0, et l'essai sur 7991 ouvre une copie gardée d'une base sous granite et y joue `search_code` (rouge vu sur le binaire fautif). Ticket produit chez ED (23f31a482f) : depuis la borne des morceaux, `backend.rs` refuse d'ouvrir sans compteur, et `record_nodes.rs` écrirait sinon des lignes sans morceaux en silence ; lot : une base s'ouvre et sert sans `token-count` (lecture, recherche, chat), toute écriture de morceaux qui compte des jetons refuse en nommant la feature. L'hôte rebâti (md5 e715e863, même tête fed132c964, lib 4ad394f4d2, lucivy 4.3.13, terminal 7e70a282) est posé : tablette 01 h 54 min 47 s à 01 h 55 min 21 s, luciepc 01 h 58. Mais à la tablette le dépôt code-rag3db ne se rouvre pas : à l'arrêt de l'hôte précédent, « 1 réacteur figé : pas arrêté en 30 s — catalogue laissé ouvert, rejoué à la prochaine ouverture », un `.wal` de 15,7 Mo, et depuis 01 h 55 un seul fil à 95 % d'un cœur avec très peu de lectures, `search_code` en 422 « s'ouvre depuis N s » ; le chat de l'IDE et les autres backends fonctionnent. Décision : attendre jusqu'à 02 h 20 (ED et cœur disent ce que fait le rejeu), puis retour arrière sur la copie propre de 23 h 35 en gardant la base figée et son `.wal` pour reproduire hors service ; ticket moteur dans les deux cas.

**Le moteur HNSW de hotfixs est atterri** (e22dede600, 01 h 45, neuf commits : en fin de bâti, chaque nœud de la couche basse reste joignable, rattaché d'abord à son plus proche voisin atteint ; 20 tours réels sans injoignable, CREATE et rappel inchangés sous `poste mesure`, les points isolés retrouvés 145/200 contre 1/200). Passe de tête verte (transaction_test 411, known_red 100 %, vector 74, e2e_test 1 867, le seul rouge étant DropNodeTableReclaim admis le 18) ; voie transférable, aucun code moteur arrivé depuis la base testée. Le push d'un lot moteur avec le pousseur en service : G a posé `master-tenu` (01 h 39), mais `push_garde.sh` lit aussi la tenue et a arrêté le train en cours (hote-invite d'archi, vert, remis en file au rang 1 depuis `returned.tsv` avec son attestation gardée ; saisie-pendant-accord jeté puis au rang 2) et hotfixs lui-même : voie retenue, G retire la tenue (01 h 44), hotfixs pousse dans la minute, le pousseur ne poussant qu'au bout d'une passe. Leçons : la tenue se pose entre deux trains ; la garde ne change pas sans mon mot. **La lib épinglée est `rag3db-build-4753938c9d…`** (bâtie à e22dede600, nommée par son dernier commit moteur, symboles identiques à 4ad394f4d2, `librag3db.so` md5 86806f13 au magasin des deux postes, `courant` posé après mon annonce) ; le lot d'épinglage de `passe_epinglee` (6b28c9453a, rouge d'abord, 67/67) a levé `lib-late` à 02 h 06 ; diffusé aux douze sessions : `RAG3DB_LIB` explicite sur cette lib au début de chaque passe, BUILD_ID lu par le binaire joué ; l'hôte du petit matin et les services bâtis contre elle. Le bâti de G avait démarré tard (lancement de 01 h 46 mort avec sa session ssh).

**Les scénarios de la nuit, suite.** Scénario 2 au terminal (tty, projet neuf `compteur`) : une tâche sur un dépôt sans commit affichait l'erreur brute de git (corrigé : `cx-tache-sans-commit-1`, « aucun commit » et la commande) ; `code-tree add` nomme « code-compteur » quand le terminal ne connaît que « compteur » (lot arbre : un seul nom partout) ; **un commit est impossible depuis une tâche dans un worktree**, le bac à sable refuse `.git/worktrees/<x>/index.lock` et l'équipier tourne en rond sur ses droits (lot archi `bac-commit-worktree`, 05844c8cb : le bac n'ouvre en écriture que ce qu'un commit écrit, le gitdir du worktree, `objects/`, la ref de sa branche et son journal, jamais `config`, `hooks/` ni les autres worktrees, `git config core.hooksPath` échoue ; et « deux refus de droits sur le même chemin arrêtent le pas » au lot suivant) ; une carte d'accord par commande, une en double ; le compte rendu final répète le texte affiché ; `saisie-pendant-accord` corrigé. Scénario 3 (banc, page 141584a1b0) : A échoué (le diff de l'équipier ne compile pas, aucun témoin joué, la fonction partagée manque), B échoué (diff vide : diagnostic juste jamais entré dans le plan, arrêt par « 3 pas sans élément réussi »), C réussi (même fichier, même intention) ; tickets d'archi : la porte de vérification (la fin déclarée par le modèle est acceptée telle quelle ; un correctif finit par compiler et jouer les témoins du ticket), les `.pyc` dans le diff (le lanceur n'emportait pas le `.gitignore` de la racine, corrigé 9a2dd52005), la section non déclarée créée par `setdefault` (règle du pas). Identité du bac à sable déposée (`identite-git`, 4fabe1279a : `user.name`/`user.email` de la config globale de l'hôte en `GIT_AUTHOR_*`/`GIT_COMMITTER_*`, `sandbox.env` l'emporte clé par clé ; `git config user.*`, `git -c user.*`, `--global` refusés, accord humain compris). Hôte invité déposé (`hote-invite`, 061edf4a9 : un hôte lancé par `--address` à côté d'un vivant ne relit ni tâches ni chats, ne reprend aucun équipier, et sa ligne d'arrêt dit « retiré » seulement si le `daemon.json` était le sien). Ouvrir un dossier déposé (arbre, 49b7b26777 : `POST /dossiers/ouvrir {path}`, un dépôt neuf sans commit s'attache), la garde de tête d'O4 (857cb2b22e), la page vide de l'IDE (recherche), le cargo dans l'atelier. **Le nœud scripté est validé** (ED, `ed-scripted-previous-step`, page `docs/everything-declarative/21-11-2026/01`) : « Previous step » en `.ts` rend le Rust à l'octet près, un script cassé laisse le Rust prendre la main en le disant avec fichier et ligne, un `.ts` édité sert au pas suivant sans rebâti, deux configurations sur la même entrée ; 1,33 ms par appel contre 8,8 µs : pratique là où l'appel est rare, pas sur un chemin chaud ; rien ne bloque les points déclarés d'archi.

**Atterris de 00 h 36 à 02 h 06**, tous propres : la couverture mesurée (f57e3ee67c), O4 (97e8c38002), la mémoire de la carte (c4bcab84a0, la GTT comptée sur un APU), le pointeur lucivy 4.3.13 (2bfcc013d2), les suites locales (fd7a80df79), (A') (e1845431d7), la garde des tickets partout (5eb1b9e657), les tickets et relevés de tty (58dd5fa3ad, 94f9bddcf7, f1064a9f89, 69d3cd4d99, 9f5613790f), le ticket token-count (23f31a482f), le lanceur et ses `.gitignore` (9a2dd52005), le moteur HNSW (e22dede600), l'épinglage (6b28c9453a). lucivy 4.4.0 : #59 et #60 fusionnées (les générations étiquetées ; un test sauté faute de corpus est dit en clair en CI), la PR de version #61 prête sans tag, au mot de Lucie ; sa ligne : « lucivy 4.4.0 prête à publier : les générations étiquetées, rag3weaver pourra garder la dernière version validée de son plein texte et celle en cours, y revenir d'un coup ou la lire sans écrire ; la 4.3.13 posée sur les hôtes refuse un index étiqueté au lieu de l'abîmer ». Codeparsers : la mesure du coût d'écriture du lot 2 (occurrences) attend sa place sous `poste mesure` à la tablette. Cœur : la série ASan de la tablette finit vers 02 h 26 ; le témoin de C1 demain. Rendez-vous des relevés à 7 h 30 ; l'hôte du petit matin et les services après l'atterrissage des lots de la nuit.

## 21 novembre, 05 h 4x — la nuit, fin : l'ordre de l'hôte du matin, la cascade des conflits, le bâti

**L'ordre de l'hôte du matin.** À 02 h 3x, G reçoit la liste des lots que l'hôte du matin doit porter, tenue en tête de file par un guet (par id ou par branche, un parent devant son fils) : identite-git, lire-la-vue, ouvrir-un-dossier, ide-ouvrir, les lots d'ED (`options`, `token-count`), puis la file dans son ordre. La borne du bâti est posée à 5 h 00, déplacée à 5 h 30 puis 5 h 45 à mesure que les lots du cœur de l'essai (ouvrir un projet dans l'IDE) butent et se reposent ; G bâtit à son réveil ou à mon « bâtis », avec ce que master porte alors. `atterrissages.json` est posé aux deux postes à 03 h 18 (tablette : la commande ssh vers `landed.tsv` de luciepc, 309 lignes lues ; luciepc : le chemin), sans clé « poste » puisque les hostnames sont ceux de la colonne VUE.

**Atterris de 02 h 50 à 05 h 36**, tous propres (auteur gmail, zéro attribution ; les mentions « Claude name », « Claude Code session » sont des noms techniques) : identite-git (86f6fbe8a7, le bac prend l'identité globale de l'hôte, `git config user.*` refusé), ide-page-vide (a6784e897f), atelier-cargo (e316fcb7e1, l'atelier livré lit la chaîne Rust), le relevé des rejeux d'archi (d5dacb9652, `docs/rag3weaver-archi/21-11-2026/01-les-rejeux-de-la-nuit.md`), garde-de-tete (76fd4f0a73), tache-sans-commit (abf5c8c80c), la page des générations étiquetées (55361885e0, ED+lucivy, (A) recommandée), le parcours d'essai de tty (baba14948c, `docs/tty/21-11-2026/03-parcours-d-essai.md`), raisonnement-borne-3 (e48d58271a), sous-agent-vues-3 (36f18c965f), lire-la-vue (287f6927a4 : sous une vue, `read_file`, `grep_files`, `list_files` lisent l'arbre de la vue), `options` facultatif (c6a322fbca), bac-commit-worktree (825c405338), scripted-previous-step (6a06b247d7), le ticket du terminal fermé (2d1b72aa5e), la règle du placement des témoins (570cb0df8f puis sa leçon 2405890765), porte-verification-2 (ab900f76d5, `teammate.verify`), mcp-reinscription (de5efa2865, le client suit un hôte redémarré sans attendre un appel), ouvrir-un-dossier (84099da9bb), un-seul-nom (0477bd1187), arbre-poste (e1d7ad3959, `GET /poste`), refus-repetes-2 (a535470481), nom-de-vue (3194aa2320, `use_view` prend le nom qu'`open_view` rend), le ticket des mots perdus de lucivy fermé (78f2f2d885 : première nuit compaction rallumée, 0 mot perdu sur 14 index).

**La cascade des conflits** (04 h 02 à 05 h 36). Après bac-commit-worktree, ouvrir-un-dossier bute une seconde fois sur `invocation.rs` et exact-name-by-name-2 sur `backend.rs` ; à 04 h 16, un-seul-nom, arbre-poste, attente-commande (tty), fetch-web-search (mémoire) et ide-vocabulaire (recherche) butent à la suite sur `invocation.rs`, `teammate_tests.rs` et `chat/tests.rs` : la file se vide en une minute. La cause, dite par ED : les témoins neufs de plusieurs lots en vol s'inséraient devant le même test (ou à la même fin de module), et chacun heurtait celui atterri avant lui ; une résolution à la main a perdu une accolade (arbre), une autre a coupé dans des corps de fonctions (ED, vue à la compilation, rien déposé). Arbre remonte ses quatre lots en pile avec parents (04 h 19 : ouvrir-un-dossier-3 → un-seul-nom-4 → poste-3 → nom-de-vue-3) : les trois premiers atterrissent de 04 h 56 à 05 h 11 ; nom-de-vue rougit une fois sur un témoin à virgule (le champ témoin du pousseur est un seul filtre cargo), redéposé avec un nom, vert à 05 h 35. ide-ouvrir (recherche) bute une troisième fois à 05 h 36, nom-de-vue ayant mis son témoin à la même fin de module : il va à l'hôte suivant, et la page du parcours garde le contournement (la commande `atelier` depuis le dossier, dite par l'accueil vide). La règle, écrite par ED (`docs/everything-declarative/21-11-2026/03-un-temoin-neuf-se-place-a-part.md`) et affinée à 05 h 4x : un fichier de tests à soi par lot, une pile avec parents pour une même session, `git merge-tree` avant de pousser, repartir du fichier de master quand git coupe dans un corps.

**Le moteur HNSW, les suppressions** (hotfixs, 03 h 3x à 04 h 31) : la cause (l'étape 3a de `keepFormerNeighboursReachable` tenait joignable un îlot d'anciens voisins dont la seule entrée venait des lignes supprimées), un témoin déterministe rouge sur e22dede600 et vert avec le correctif (319730f984), coût nul mesuré sur 48 vagues ; passe de tête VERTE à 04 h 31 (ccc9deb676, ticket fermé dans le lot). Le push est reporté après la pose de l'hôte : le lot change `extension/vector/`, et le pousseur suspendrait les lots Rust (`lib-late`) juste avant le bâti ; un guet de pause déjà armé par G a été annulé à temps. Après la pose : pause entre deux trains, « à toi », lib rebâtie à sa tête et épinglée par un lot.

**Ce qui attend la pose** : la mesure du lot 2 de codeparsers (bloquée depuis 23 h 5x faute de poste vide : fenêtre de 2 × 7 min en `poste mesure` bloquant à mon mot vers 6 h), le push de hotfixs, la nouvelle lib. **Ce qui va à l'hôte suivant** : ide-ouvrir et la page Poste (recherche), écrire-la-vue et le ticket des commandes sous une vue (arbre), token-count et exact-name-3 (ED), attente-commande et tty-un-seul-nom (tty), la pile et fetch+web_search (mémoire), claude_name par le hook (archi).

**Pour Lucie** : la page du parcours (`docs/tty/21-11-2026/03-parcours-d-essai.md`), la recommandation (A) des générations étiquetées, l'autorisation de `git worktree remove --force` que recherche demande pour ses worktrees, la 4.4.0 de lucivy à taguer à son mot. Le bâti de l'hôte du matin part à 05 h 37 sur master 78f2f2d885, contre la lib 4753938c9d, features de service, le terminal et `rag3weaver-embeddings` de la même tête, essai sous granite aux deux postes avant la pose.

**La pose** (G). Bâti fini à 05 h 45, essai VERT aux deux postes (check-lib à 0, lucivy 4.3.13, compteur de jetons présent, recherche sous granite à trois résultats) ; pose à la tablette de 06 h 16 min 57 à 06 h 17 min 52 (annoncée pour 06 h 10 sans avoir été lancée : la faute dite par G), à luciepc à 06 h 18 min 26 sans coupure vue ; hôte 78f2f2d885, md5 `bfd80875`, lib 4753938c9d, terminal `b3de11e1`, `rag3weaver-embeddings` de la même tête ; vérifié d'ici aux deux postes. À l'arrêt de l'ancien hôte de la tablette, la montre venait de se réveiller (06 h 17 min 19) : réaction figée, catalogue laissé ouvert, et le dépôt a rejoué son journal, une minute cette fois (contre seize à 01 h 55) ; second cas au ticket de l'arrêt. Les services : 7878 granite à l'écoute à 06 h 19 min 30 (écart 4,8e-5, tolérance 5e-4), 7879 bge-m3 à 06 h 20 min 06 avec sa référence (écart 2,5e-4, tolérance 5e-3), aucune chute, pic de vérification 4 503 Mio au-dessus du plateau, sous la prédiction d'embarquement. L'avertissement « des écritures ont été perdues » du plein texte de Scope à la tablette (102 191 documents pour 113 518 lignes) est une fausse alarme de la note des comptes : la maintenance nocturne note toutes les lignes là où la garde de l'index compte les candidates, et les 11 327 lignes écartées sont les conteneurs repliés des règles 13 (lucivy, lot à lui). Un second hôte se bâtit à 7 h sur le master du moment (ide-ouvrir atterri à 06 h 02, le vocabulaire des dépôts, le terminal qui résout `--repo` par l'hôte, écrire-la-vue), posé avant 8 h si son essai est vert ; le push moteur de hotfixs part dès ce bâti lancé, puis la lib à sa tête et son épinglage.
