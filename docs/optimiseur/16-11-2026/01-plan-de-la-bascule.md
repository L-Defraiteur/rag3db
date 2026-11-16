# Le plan de la bascule du 16 novembre : de la lib 373b5477b à la lib de master

Session G (optimiseur). Plan préparé dans la nuit du 15 au 16 novembre, au mot de c7. **Rien n'est bâti avant son
mot** ; la bascule se fait quand Lucie se lève, dans une fenêtre qu'il ouvre.

## Mise à jour de 6 h 30 : la lib 59ac9c86bc

Au mot de c7 (6 h 1x), on ne bascule pas vers une lib en retard d'un correctif de durabilité. La lib est rebâtie sur master
3177d2489e : **rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a** (librag3db 697c7a0c, libvector 925b6b13), le
fsync du répertoire parent à la création d'un fichier. Contre 373b5477b : 17 symboles ajoutés, 0 retiré. Les vtables de
`FileSystem`, `LocalFileSystem` et `VirtualFileSystem` changent de taille (syncDirectory s'insère) ; le cœur lit l'ABI
inchangée pour le Rust, qui ne voit pas ces classes. Verdict sans MOTEUR_ANCIEN sur 3177d2489e : 36/36, 27/27, 24/24. Les
hôtes bâtis contre elle (binaire d164e4da) ont été essayés aux deux postes. Le service d'embarquement sur 3177d2489e
(sha256 6196fbe7102ad533) attend dans `nuit-3177d2489e/` à luciepc. Ce qui suit valait pour 52101b7fd, et vaut de même.

## Ce que porte la lib neuve

Le magasin nomme une lib par son dernier commit moteur (`src`, `extension/vector`). Sur master, c'est aujourd'hui
**52101b7fd6** (une version de mise à jour naît à quatre lignes et double à la demande). Si le cœur pousse encore du
moteur avant la fenêtre, le nom suit. Le script l'écrit dans `lib58/id` ; toute annonce lit ce fichier.

Depuis 373b5477b, 22 commits moteur, dont :
- **ALTER et DROP TABLE** attendent les écrivains ouverts sur leur table (681b35b38). Une écriture refuse par un nom un
  schéma changé depuis le départ de sa transaction (« Schema changed, retry: », ae8f53d5d et suivants). L'index
  vectoriel prend le verrou de schéma en partagé (bc778d5e0).
- **La colonne ajoutée** : un ADD COLUMN refusé à mi-chemin ne laisse rien au stockage (1f6a52a51, f2b5cfe88).
- **Le pool** : DdlEveryMinute quitte known_red (2385b5f61). known_red est vert sur la tête du cœur.
- **Le planificateur** : un IN sur la clé primaire (7897f2453), LIST_CONTAINS sur une liste constante (dd43a2be3), le
  pliage des littéraux au binder (1d5580d74).

**L'ABI** : le lot du planificateur change la disposition de deux classes internes. `PrimaryKeyScanInfo` gagne un
`bool isList`, et `PrimaryKeyScanNodeTable` l'état du mode liste. Elles ne sont pas dans l'API publique, et l'extension
vector ne les emploie pas, d'après le cœur. Le relevé des symboles de `batir-lib.sh` ne voit pas une disposition : la
lib et l'extension se bâtissent ensemble, les binaires se rebâtissent contre elles, et l'annonce dit « disposition
interne changée, API publique inchangée ». **Le cœur confirme `ABI_CHANGEE`** avant l'étape 3.

## Les étapes et leur durée

Les durées viennent de la bascule du 14 (18 h 20 → 21 h 33). Il faut compter **environ 2 h**, dont **20 à 40 min sans
hôte à chaque poste** pendant les instantanés et le contrôle des deux sens.

| | quoi | où | durée | qui attend |
|---|---|---|---|---|
| 1 | `docs/procedures/scripts/batir-lib.sh <tête>`, sous `poste lourd`, `-j 8`, publié sans `courant` | luciepc | ~30 min | personne |
| 2 | lire `lib58/id` ; ABI : symboles ajoutés et retirés, vtables, `ABI_CHANGEE` dit par le cœur | luciepc | 2 min | |
| 3 | verdict sous `RAG3DB_LIB=<id>` explicite, **sans `RAG3WEAVER_MOTEUR_ANCIEN`** : e2e_code, e2e_lectures (ses trois témoins nommés), e2e_demon_hote | luciepc | ~10 min | |
| 4 | la lib et l'extension copiées au portable, md5 vérifiées, `courant` intact | les deux | 1 min | |
| 5 | les hôtes bâtis contre la lib explicite (`LIB=<id> preparer-binaire.sh <tête>`), essayés sur 7991 aux deux postes, `check-manifests` | les deux | ~10 min | |
| 6 | annonce aux sessions une minute avant ; hôtes et sockets arrêtés | les deux | | les passes qui lisent 7970 |
| 7 | instantanés `avant-<id>` (`instantanes-bascule.sh`) : memoire-longue, code-rag3db-depot ; code-principal, code-lourd | les deux | 1 min | |
| 8 | contrôle des deux sens sur les instantanés (`controle-des-deux-sens.sh`, outil bâti contre la lib neuve, sous `poste lourd`) | les deux | 20 min au portable, ~40 min à luciepc | Lucie (pas d'hôte) |
| 9 | `courant` aux deux postes ensemble (`courant-bascule.sh <id> <md5>`) ; lib-des-passes à luciepc | les deux | 1 min | |
| 10 | tunnels 7979 et 7980 vérifiés, puis réveil ; `daemon_status` : la lib neuve dite, chaque base rouverte sans refus | les deux | 2 min | |
| 11 | les passes épinglées sur la lib neuve (lot ci-dessous) | dépôt et cache | 10 min | les pushes |

Les services d'embarquement (7878, 7879) ne lient pas la lib : ils ne bougent pas.

## Les passes épinglées : la lib neuve, sans `RAG3WEAVER_MOTEUR_ANCIEN`

`tools/poste/passe_epinglee.sh` nomme la lib en dur (`RAG3DB_LIB=rag3db-build-373b5477b…`). Tant que les sources du
moteur sont plus neuves que 373b5477b, chaque passe pose `RAG3WEAVER_MOTEUR_ANCIEN=1` et le dit. Après la bascule :
1. un lot court de G : la ligne `RAG3DB_LIB` de `passe_epinglee.sh` nomme la lib neuve. Son témoin
   (`test_passe_epinglee.sh`) l'attend en première ligne du résumé. Le lot pousse au « à toi » de c7, en tête de file ;
2. la copie du cache recopiée aux deux postes par `install -m 755`, l'ancienne gardée en `.avant-<id>`, témoin joué
   contre la copie ;
3. l'annonce aux sessions : « `RAG3WEAVER_MOTEUR_ANCIEN` retiré, la lib `<id>` épinglée ». Une passe lancée avant
   l'annonce finit sur l'ancienne lib, et elle le dit.
Les scripts de G qui nomment 373b5477b (verdicts, preparer-binaire) prennent l'id de `courant`, ou de `lib58/id`.

## Ce que la fenêtre attend de c7

- **Le mot de bâtir**, avec la tête de master. Le bâti dure 30 min avant que quoi que ce soit ne bouge.
- **La fenêtre des hôtes**, 40 min à luciepc et 20 au portable, sans passe qui lise 7970, et Lucie prévenue.
- **Le verdict du cœur** sur sa tête (known_red, ×10). Un rouge du verdict de l'étape 3 se départage lib contre Rust
  par deux jeux croisés avant d'accuser la lib ; s'il vient de la lib, la bascule attend le cœur et `courant` ne bouge
  pas.

## Revenir

`RETOUR.sh` à côté des instantanés, `courant` reposé sur 373b5477b par `courant-bascule.sh`, et les binaires
`.avant-<tête>` reposés par `poser-binaire.sh`. Les deux postes reviennent ensemble, jamais un seul.

## Fenêtre autorisée du 16 novembre — reprise en préparation

Lucie a autorisé explicitement la lib neuve, la pose aux deux postes puis les
migrations des bases ; aucune nouvelle demande d’heure nécessaire. Source figée
`927941a67a5805450d49f1e04b39fdd542a6059b`, dernier moteur
`59ac9c86bc2b3529b590a7462ab7051e3ad26b8a`. Audit frais : aucun delta C++ ni
configuration CMake entre 3177d2489e et cette source ; magasins concordants,
17 symboles ajoutés, aucun retiré et trois vtables FileSystem agrandies.
Le cœur confirme ce delta nul ; son ancien TSan reste autonome, sans arrêt.

Les hôtes préparés sur 3177 sont périmés pour cette pose : rebâti sur 927 avec
lock lucivy 4.3.9. Première tentative refusée par la garde du target historique
appartenant à un autre arbre ; refus conservé, aucun artefact repris en silence.
Reprise dans `target-cx-optimiseur-bascule-16`, 32 Gio, huit jobs, sous poste et timeout.
Aucun hôte coupé à ce stade. Preuves hors dépôt aux deux postes :
`~/.cache/optimiseur-notes/bascule-16nov-927/`.

Porteur unique services/libs : optimiseur. Arbre prépare les migrations sur copies,
puis attend le FINI technique de pose : pins complets, réveil et ouvertures,
instantanés/RETOUR et contrôles des deux sens. Retour conservé 72 h minimum,
aucun retrait d’ancien index. Après réveil, fenêtres offline par base coordonnées
avec le porteur ; aucun outil historique lié à 373b ni purge sous lucivy ancienne.
L’épinglage des passes suit par le ticket existant
`docs/optimiseur/tickets/2026-11-16-les-passes-epinglent-373b5477b-apres-la-bascule.md`,
sous un nouveau tour de push. Les passes des autres lots attendent le pin neuf.

## FINI lib et pose — 16 novembre, 11 h 32 CET

Préavis 11:26:40, arrêt engagé 11:27:4x, réveil des deux hôtes à11:30:53.
Source exacte `927941a67a5805450d49f1e04b39fdd542a6059b`, lock SHA256
`090cc4e1330dc694127c0f08330d25acc656699763079400b502ba6607093fa7`, lucivy4.3.9.
Binaire identique aux deux postes SHA256
`0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288`,
check-lib `da8fe5f81a5defc98a56086364e7a824e6a755eeb27792cf90a1673b55afc295`.
Lib complète posée :
`rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
SHA256 lib `b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101`,
vector `1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561`.
`courant` concorde aux deux postes ; lib-des-passes PC mise à jour avec ancien conservé.
Maps des PID portable2698568/PC3136522 : lib ET vector59ac. Services7878/7879 et
tunnels7979/7980 inchangés et joignables ; aucune action card0, aucun sudo.

Verdict source927 sans MOTEUR_ANCIEN : code36/36, lectures27/27 et demon_hote24/24,
aucun ignoré ; trois témoins lectures nommés contrôlés, reçus de confinement verts.
Essais7991 verts aux deux postes, check-manifests5+2 acceptés.
Réouvertures vérifiées POST/open ET GET/etat : mémoire, code principal du portable,
code-essais-c7, essai-atelier, atelier ; code-principal et code-lourd PC, zéro refus.
Les anciens backends de tâches non ciblés ne sont pas déclarés restaurés par cette preuve.

Instantanés cohérents arrêtés, avec base et tous voisins présents/fts/checkpoints :
`~/.local/share/rag3weaver/instantanes/avant-59ac9c86b-20261116-927/` aux deux postes,
`RETOUR.sh` à la racine, conservation72hminimum, aucun retrait ancien index.
Copies de contrôle : même chemin suffixé `-controle`. Chacune des quatre bases a
18 tables de relations jugées ; sommes des unions (pas des relations uniques entre
bases) : portable code1148759/mémoire9054, PC lourd1205199/principal931405 ; zéro
perte d’un sens et aucun écart de propriétés. Contrôles bien exécutés, non vides.
L’ancienne extension enregistrée373b a été refusée à l’ouverture des COPIES ; cela
n’invalide pas le contrôle relationnel et ne prouve pas le vectoriel. Les hôtes
chargent ensuite leur extension59ac, confirmée dans les maps ; parité/oracle reste au banc.

Reçus privés aux deux postes `~/.cache/optimiseur-notes/bascule-16nov-927/` :
`runtime.json`, `opening-receipts.json`, `state-after-open.json`, `snapshot-identities.json`,
`live-stopped-identities.json`, `controls-summary.json`, journaux, recettes et rouges.
Le sélecteur hostname du premier snapshot PC a refusé avant copie (LuciePC/casse),
puis été remplacé par rôle explicite ; trace conservée. Aucun jeu réel perdu.
Le contrôleur est la source927 SHA f665671416a1eefa7b9e056445be1f2e8a5abf60c4ce3bcf07a32355915f8fc0.
L’outil requête RO privé d’Arbre, corrigé après rouge BUILD_ID, est
`read-only-query-v2` dans ce dossier aux deux postes : ELF
43cffb082a2487e9f33ffa62b5a246081e5c458683f4e6b7de50e46d60c01329,
source0f498190/SHA931267d266acf730a7699b9713c146e2ab17492d11b18ceafd651a5cc8bff125.
Fixture privée copiée : lecture1nœud, BUILD_ID59ac effectif, écriture refusée rc1,
SHA base inchangé. Aucun ancien outil de requête réutilisé.

FINI communiqué à c7 et arbre. Les migrations sont ensuite portées par arbre,
avec opérations services/hôtes exclusivement par optimiseur. La réouverture a pu
changer les identités des bases vivantes : ne pas appliquer une preuve de l’instantané
à une autre identité sans vérification. Préparation du pin des passes prioritaire :
témoin vu rouge sur l’ancienne ligne373b, correction59ac et rejeu court à suivre.

## Pin des passes terminé — 16 novembre, après11:52

Master `68590270e21d00285b29800bd9cdecf0cb921461`. Source/caches `passe_epinglee.sh`
identiques SHA2562150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d,
mode755, témoins des deux copies verts ; anciens conservés. Vraie passe candidate :
F8 8/8, preuve, lib2594/0/72ignorés, navigateur45/0/0, scope VERT sans reste.
72ignorés ne sont pas72verts :45 rejoués au navigateur,27non jugés (mesures,
services/modèles réels, anciens filets d’intégration explicitement ignorés).
Inventaire et journaux archivés ; ticket fermé sous `docs/tickets/closed/`.
L’ancien lib_seule est conservé sous `.avant-retrait-59ac-20261116` aux deux postes.

Correction de provenance outil query : l’ELF43cff compilé au portable ne fonctionne
pas à luciepc (ISA requise, rc127, fixture uniquement). L’exécutable NATIF PC est
`~/.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2-luciepc`, SHA256
`48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616` ; même source931267d2,
même lib59ac, count1/BUILD_ID et refus d’écriture rc1/SHAfixture inchangé vérifiés.
Le portable garde son ELF43cff. Les deux versions ne sont pas interchangeables sans qualification.

Maintenance portable d’Arbre ensuite ROUGE : premier dry_runFile422 ; client perdait
le corps, corrigé en recette distincte par son porteur. Une sonde distincte rapporte
« pas d’index plein texte ouvert pour File (ouverts:[]) » : cela ne prouve pas une
absence des fichiers. G n’a fermé aucun backend ni pris de snapshot migration ni
lancé d’outil natif sur une base vivante pour contourner ce refus. Journal existant
conservé, aucune ligne lors de l’appel : cause initiale exacte non retrouvée.
Les routes de maintenance ne font qu’employer les handles ouverts ; le chemin
ordinaire ensure_fts_handle d’un écrivain peut créer/rebâtir, donc aucun warmup
implicite n’est autorisé pour ce diagnostic. La primitive existing-only est un
lot distinct en coordination par c7/recherche. FINI lib/pose ne signifie pas FINI migrations.

Worktrees/target de cette fenêtre conservés, car Banc dépend encore des artefacts
release du source927. Pas de purge des targets, snapshots, anciens index ni magasins.

### Limite supplémentaire des sondes natives RO

Le filet froid d’Arbre sur fichier corrompu révèle qu’une ouverture moteur
readOnly peut créer le latéral `.readers` vide avant de refuser le format. Les
filets de G prouvaient le SHA inchangé du fichier BASE, pas l’absence d’effets sur
ses voisins ; aucun vert global de neutralité du système de fichiers n’en découle.
Les contrôles natifs restent sur copies privées, jamais sur base vivante détenue.
L’acceptation explicite du latéral sur COPIE et la primitive existing-only sont
arbitrées par c7 ; pas de relaxation, reprise maintenance ou migration déduite.

### Fin des lecteurs des artefacts release — 16 novembre, 12 h 13

Banc confirme ses scanners/admission terminés et ses exécutables autonomes hors
target. Arbre confirme les liens immutablepc01/idsetspc01 terminés, scopes verts
sans reste, aucun besoin futur déclaré de ces deps ; son ELF idsets est copié
hors target (SHA d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7).
Le relevé des exécutables et FD utilisateur ne voit aucune référence active au target.

Archive reflink fidèle de tout release à luciepc :
`~/.cache/optimiseur-notes/bascule-16nov-927/release-archive`.
Reçu `release-archive-verdict.json` : VERT,4806entrées ; SHA contenus, modes,
uid/gid, mtime et liens identiques, source avant/après inchangée. Inclut deps,
build, fingerprints et binaires. Aucun nouveau Cargo ou lien exécuté pour cela ;
l’égalité de l’archive ne promet pas un rebâti après relocalisation des chemins.
Originaux worktrees/target conservés, aucun nettoyage ni fermeture de backend.
Les services restent réservés au porteur pour la future fenêtre explicitement
qualifiée ; aucune maintenance verte ni migration déduite de ces faits.

### Fenêtre froide PC adaptée — 16 novembre, 12 h 29

À12h20 l’API avait changé de PID et ne connaissait plus aucun backend ; la
précondition de fermeture par alias a refusé AVANT tout POSTclose. Reçu privé
`diagnostic-code-principal-20261116-122008/`, aucun writer ouvert pour contourner.
C7 a adapté et Arbre publié le plan360bcd75db avant action : arrêt API7970PC
seule/service+socket, principal puis lourd sans réveil entre deux, aucune
maintenance API déclarée verte et aucune CLI par G.

Préavis≥60s, état immédiatement avant arrêt PID3761669/starttime117390374,
0backend ; arrêt à12:29:14, deux unitésinactive/dead/MainPID0. Portable,
embarquement et tunnels inchangés. Preuve FD/maps accessibles et verrous noyau
sur les deux bases/latéraux sans référence ; processus système protégés non
inspectables explicitement listés, aucun prétendu droit ptrace global.
Reçus PC `~/.cache/optimiseur-notes/bascule-16nov-927/pc-cold-window-20261116-122721/`,
complément après arrêt `post-stop-quiescence.json`, aucun nouvel HTTP7970.

Snapshot diagnostique FRAIS principal :
`~/.local/share/rag3weaver/instantanes/diagnostic-code-principal-20261116-122933/`.
733entrées, source avant/après et copie même arbreSHA
ca7e52e6e4483a0785a1d0e3bf72491748f8ec33fec80ce1eee8f587ce9c130e ;
identités source strictement inchangées. Reçu `code-principal/fresh-receipt.json`
SHA e21599096e1a6b93ea120fbddaae5dfcf7c9508a060a5e4a9e1aa3cc917d02f1,
retour72h. Arbre reprend ensembles/générations/deuxsens sur nouvelle copie,
aucune ancienne qualification reportée ; cli_authorized=false.

Le retour préparé préserve aussi les nouveaux voisins (dont un WAL né après
snapshot), après rouge/vert sur fixtures privées. La recette initiale est
conservée ; générateur non corrigé en ticket
`docs/optimiseur/tickets/2026-11-16-le-retour-d-un-snapshot-oublie-les-voisins-crees-apres.md`.
Aucune restauration réelle exécutée. Nettoyage worktree/target G différé par c7
jusqu’à fin de cette fenêtre froide, sans retrait de données ni de preuves.

### Retour V2 et preuve OS hors bus privé

Avant toute CLI live, c7 exige le retour V2 qualifié. Source publique bf0aba2cfc,
helper et cinq filets dans `bascule/` de ce dossier ; ancien comportement vu rouge,
nouveau vert sur fixtures aux deux postes, zéro moteur/service réel exécuté.
Principal : attestation séparée `RET_v2-attestation.json` SHA24eced05414acb1e505b49ff018519820cc48cbbb6597824bf7af47befda452b,
verdictVERT/scopefilesystem_fixture, cli_authorized=false ; ancien reçu e215,
ancien RET et données733 immuables. Détails et limites au ticket de retour.

Le préflight écrivain sur COPIE d’Arbre a ensuite refusé avant writer : systemctl
ne peut interroger le bus propriétaire depuis le bus privé du confineur. Cela ne
prouve pas l’API réveillée. G a fourni sans HTTP ni opération d’unité le relevé OS
hors confineur à12:47:38 : API/socketinactive/dead/PID0, ss7970 rc0 sans écoute.
Reçu `pc-cold-window-20261116-122721/api-off-os-attestation-20261116-124738.json`
SHA8f3a131fbaaa066a3ea34a9f2b4254bdc961915981a7de120d96323c1282f2a6.
Aucune CLI live par G, aucun réveil entre bases et aucune qualification verte
inventée pour ce refus. Arbre coordonne sa garde OS sans détourner le bus privé.
