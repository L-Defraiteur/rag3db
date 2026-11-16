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
