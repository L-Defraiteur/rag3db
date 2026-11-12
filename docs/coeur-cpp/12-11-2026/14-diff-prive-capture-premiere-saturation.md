# Diff privé : première saturation et contexte borné

12 novembre 2026. Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. **Préparation autorisée seulement ; aucune compilation,
configuration CMake, exécution ou réservation.**

Source : branche `cx-coeur-diagnostic-saturation`, commit `6bfba5b4c`, parent
fonctionnel 0511dece74f94db6937fb03e6ec45f0c58828c5a ; worktree neuf
`/home/lucied/git_workspaces/cx-coeur-diagnostic-saturation`.
Le worktree 0511 et ses artefacts restent séparés.
[Diff exact](preuves/diagnostic-saturation-prive/capture.patch),
[recette structurée](preuves/diagnostic-saturation-prive/recette-revue.json).
Cette branche n'est pas une candidate à publication de bibliothèque.

## Implantation et frontière

Un fichier commun privé, un header privé et des appels aux points suivants :

- BufferManager::reserve juste avant le retour false, avant de rendre la réserve :
  première capture POOL, octets demandés et usedMemory (réservations en cours incluses).
- VMRegion::addNewFrameGroup avant l'exception existante : capture FRAME_GROUPS.
  Ce point n'a pas accès au BufferManager ; used_bytes vaut UINT64_MAX (**inconnu**),
  requested_bytes vaut 0 (**non applicable**). Les échantillons voisins conservent
  la dernière mesure disponible, sans la présenter comme une mesure instantanée.
- AllocationSite scalaire thread-local distingue claimAFrame / mallocBuffer / inconnu.
  Scope thread-local distingue rollback et rollback PK. L'identifiant transaction
  est connu dans le scope rollback, sinon 0 (**inconnu**). Aucun nom de requête,
  texte de clé, pile allouante ou parcours des undo ajouté.
- Scope PK mesure chaque appel de rollbackPKIndexInsert avec steady_clock et
  additionne durée complète, même lors d'une exception ; compte aussi appels
  commencés et appels encore actifs. Le cumul est une somme de durées murales
  par appel, pas du CPU et pas nécessairement inférieur au temps mural global.
- Le manager republie, sous son verrou déjà tenu, nombre de transactions actives,
  nombre d'écrivains (recovery incluse) et plus petit startTS après BEGIN, retrait
  et refreshSnapshot. Aucun verrou nouveau. La valeur est le dernier état publié,
  pas une acquisition atomique du manager à l'instant du refus.
- NodeTable::commit compte les lignes transférées après append complet ;
  Transaction::commit compte la publication des versions après undoBuffer::commit.
  Les acteurs distinguent issues écrivains et ensemble des acteurs.

Le banc arme après chargement initial et avant lancement des acteurs, avec **la
même origine steady_clock** que le scénario. Le thread d'échantillonnage existant
reste à 100 ms ; au plus une capture supplémentaire par seconde est conservée.
Le calendrier, la charge, les graines, les DDL, les réglages et la taille du pool
ne sont pas modifiés. Les appels de diagnostic ajoutent néanmoins du coût :
aucune affirmation de perturbation négligeable n'est faite sans mesure.

## Bornes et lecture concurrente

Stockage fixe de 1280 enregistrements + une première capture ; tableaux touchés
avant armement et avant acteurs. Pas de vector, string, allocation de pool ou
formatage dans les callbacks de rollback. Compteurs uint64 atomiques dont le
caractère lock-free est exigé par static_assert ; horloge monotone et TLS scalaires.
La sortie utilise désormais un fichier privé ouvert avant les acteurs ; le sampler
écrit et vide hors rollback, puis le thread du test finalise après joins.
Voir le [delta de publication et de vidage](15-delta-publication-et-vidage-capture.md).

La première capture signifie **premier gagnant du compare_exchange** au site
instrumenté, pas minimum absolu des instants de refus physiques de tous les fils.
Son timestamp at_ns commence la lecture des métriques après ce gain ; read_end_ns
borne la fin. used_bytes est lu juste avant l'appel au callback. Les champs sont
lus indépendamment : ils ne forment pas un snapshot transactionnel. Les différences
entre échantillons donnent un contexte de fenêtre, pas une identité causale.

Le sampler est l'unique écrivain du tableau ; les données de première capture
sont publiées par état release/acquire, lues par le sampler puis après joins.
État 0 = aucun refus
capturé, 1 = capture non achevée donc résultat invalide, 2 = capture publiée.
Overflow est compté, jamais masqué. Un seul test dans un processus neuf : pas de
reset concurrent ni de réarmement prévus. Les compteurs ne bouclent pas dans les
bornes proposées ; les intervalles sont en ns, les volumes en lignes/octet.

La sortie CSV porte un schéma puis samples/status/first_meta/first. Le champ ddl
est une phase temporelle globale (0 hors DDL, 1–6 pendant le DDL correspondant),
**pas l'appartenance du refus au fil DDL**. Phase 0 signifie « hors scopes rollback
instrumentés », pas « BEGIN ». Sites inconnus restent inconnus.

## Volumes disponibles et absences assumées

- transferred_rows : nouvelles lignes locales transférées entièrement vers les
  tables de nœuds depuis l'armement, même si leur transaction échoue ensuite ;
  n'inclut pas la portion d'un append ayant levé. Ce n'est pas le nombre de lignes
  durables, ni un compteur de toutes les relations HNSW.
- published_commits : versions publiées, y compris commits internes ; pour un
  commit forcé, son checkpoint peut encore échouer. Ne pas l'étiqueter durable.
- writer_ok / writer_error : itérations terminées des acteurs écrivains, incluant
  MERGE sur Doc existant ; actor_ok/error inclut les lecteurs. Ce sont des volumes
  de travail réalisé, pas une cardinalité de tables.
- oldest_snapshot_ts : horodatage logique du plus ancien startTS publié, pas âge
  mural ni identité de transaction. Zéro quand aucune transaction n'est publiée.

Indisponibles sans autre mécanisme : mémoire par propriétaire, ventilation
évictable, âge mural des transactions, phase de requête précise hors rollback,
N/slots visités par scan PK, volumes durables par table. Aucune nouvelle API métier
créée pour les obtenir. La capture ne garantit donc pas à elle seule d'établir
une causalité de rétention ; elle peut éliminer un coût PK dominant ou localiser
un chemin qui justifierait une question plus ciblée.

Aucune instrumentation générale des préfixes : la présence d'un échec d'annulation
reste recherchée dans les messages existants. Si un refus est capturé hors scope
rollback, cela n'exclut pas une erreur d'undo ultérieure. On ne prétend pas que
l'absence de message prouve une récupération sûre. Le ticket quarantaine reste
séparé de ce diagnostic de pression.

## Recette de build et runtime à relire, pas à lancer

Les argv et limites proposés sont dans recette-revue.json. Le build doit être
neuf sur une copie privée du commit, avec ses headers, archives **et** extensions
assortis, aucun objet de l'archive 0511 réutilisé. Release/tests ON/static engine,
vector+geo comme le cache historique relu ; pas de sanitizer/LTO/backtrace ni
runtime-check supplémentaire. Construire seulement concurrence_test et les deux
extensions. Comparer aussi tailles de page/vecteur/groupe/segment et définitions
au manifeste historique ; arrêter sur différence non expliquée. Le nouveau TU
et les changements de diagnostic sont des différences explicites attendues.

Les extensions écrivent d'après CMake dans `SOURCE/extension/{vector,geo}/build`,
et loadVectorExtension charge le chemin de **ce source privé** compilé dans le
test. Il faut donc un checkout source privé protégé en lecture seule, avec seuls
ces deux sous-répertoires de sortie privés montés en écriture, ainsi que build et
TMPDIR privés. Interdire fallback vers une extension distribuée ou un autre arbre.
Les dépendances doivent être préparées et attestées avant le build ; pas de
fetch réseau implicite ou de modification de sous-module partagé. Si ce montage
n'est pas réalisable tel quel, revenir à la revue, sans déplacer les sorties vers
un emplacement partagé.

Proposition build : poste lourd, 1800 s + arrêt borné, -j8, cgroup 8 Gio/swap0,
stockage privé dur 2 Gio (ENOSPC = arrêt sans hausse). Proposition runtime distincte :
poste mesure exclusif **après** revue du manifeste de build, un processus filtré
DdlEveryMinute, 1200 s + 10 s d'arrêt, 8 Gio/swap0, stockage privé 2 Gio et journal
16 Mio. Budgets après acquisition du verrou, aucune réservation effectuée.
Le dossier temporaire des tests est sous TMPDIR privé (`temp_directory_path/rag3db`),
avec dossier de travail neuf. Source/extensions/build en lecture seule au runtime,
seul runtime/output en écriture ; réseau/GPU/services/PG inaccessibles.

L'environnement doit être relu et archivé explicitement : les historiques n'en
attestent pas tous les détails. SCENARIO_*, MAX_NUM_THREADS, GTEST_*, seuils/checksums
et variables du helper ne sont ni hérités silencieusement ni changés sans revue.
La recette n'autorise pas de rendre les deux passages historiques comparables
par déclaration. Un passage diagnostic localise un événement ; toute comparaison
base/correction à volumes fixés demanderait une décision ultérieure.

Le vidage après joins seulement est remplacé par le delta 15 : le sampler
vide périodiquement les échantillons et la première capture hors rollback, dans
un fichier privé créé avant les acteurs. Un timeout laisse des preuves partielles,
jamais un résultat global complet sans footer valide. L'état PK des appels encore
actifs doit être pris en compte : leur durée n'entre pas dans pk_completed_ns.

À produire ensuite seulement si autorisé : attestation source/diff et compile
commands, empreintes engine/binaire/extensions et BuildIDs, limites effectives,
argv/env, logs/XML/CSV, exit code et preuve d'arrêt. Aucun résultat natif n'existe
pour ce diff : vérification actuelle limitée à lecture, cohérence textuelle du
format CSV et git diff --check. La correction de sûreté 0511 et son coût restent
deux questions distinctes.
