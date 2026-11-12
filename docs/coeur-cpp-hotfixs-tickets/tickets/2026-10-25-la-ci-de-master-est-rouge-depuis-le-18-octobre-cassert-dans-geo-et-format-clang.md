# La CI de master est rouge depuis le 18 octobre : un `<cassert>` dans geo et le format clang

- **État** : ouvert — point 1 fait (sur master depuis le 25 octobre, 9bf7aca5c), point 2 à la
  fenêtre de formatage que l'orchestration ouvre la nuit du 8 novembre. Relevé par l'orchestration le
  25 octobre 2026
- **Gravité** : aucun run vert du « CI Workflow » dans l'historique récent : les autres jobs ne
  disent plus rien de master tant que ces deux-là rougissent.
- **Atteignable en service** : non (contrôle de CI).
- **Touche rag3weaver** : non (mais ses annexes de docs sont lues par le contrôle, voir 2).
- **Ouvert le** : 25 octobre 2026, session cœur C++, à la demande de l'orchestration.
- **Pour** : cœur C++.
- **Famille** : moteur

## 1. « sanity checks » : `scripts/check-no-std-assert.sh extension`

Le script refuse tout `#include <assert.h>` ou `<cassert>` hors du moteur (l'assert du dépôt est
`KU_ASSERT`, `common/assert.h`). `extension/geo/src/index/rtree.cpp` incluait `<cassert>` sans
l'employer (aucun `assert(` dans geo). **Fait** : l'inclusion retirée, le contrôle vert en local
(`bash scripts/check-no-std-assert.sh extension`), commit baf19b9e5 sur `ci-cassert-geo` ; vers
master quand l'orchestration ouvre la fenêtre.

## 2. « code format » : `run-clang-format.py` avec clang-format-18 puis `git diff --exit-code`

Le job reformate `src/`, `test/`, `tools/`, `extension/` en place avec `/usr/bin/clang-format-18`
puis refuse tout diff. Aujourd'hui environ 300 fichiers diffèrent.

**La version n'a pas changé** : la CI épingle clang-format-18 depuis l'amont (b3917d99e, « Bump
clang-format to v18 and enable auto format », bien avant la bifurcation ; `.clang-format` n'a pas
bougé depuis 2022). La dérive est la nôtre : des fichiers écrits ou retouchés depuis la bifurcation
sans passer par clang-format-18 (les postes n'ont pas la 18 : ici clang-format 23, dont la sortie
diffère de la 18 sur ce style, donc **ne pas formater avec la version locale**), et les **29
annexes `.cpp`/`.h` de `extension/rag3weaver/docs/`** (3 octobre 2026 et suivants), des sondes
et des fragments de pages que ce contrôle n'a aucune raison de lire.

**Ce qu'il ne faut pas faire maintenant** : reformater 300 fichiers entrerait en conflit avec
toutes les branches ouvertes (format 41, V2-1a, le rattrapage HNSW de hotfixs).

**Plan, dans une fenêtre que l'orchestration ouvrira quand les lots moteur du jour auront
atterri** :
1. un fichier `.clang-format-ignore` à la racine (le script le lit par défaut,
   `DEFAULT_CLANG_FORMAT_IGNORE`), avec `extension/rag3weaver/docs/*` ; ou l'option `--exclude`
   dans les quatre pas du job ;
2. un **commit de formatage pur, sans logique**, produit par clang-format-18 lui-même (un
   conteneur ou le runner, pas la 23 locale), sur `src/`, `test/`, `tools/`, `extension/` hors
   docs ; relu par `git diff --stat` et par un bâti + `transaction_test`, rien d'autre n'y change ;
3. ensuite, chaque lot passe `run-clang-format.py --dry-run` avec la 18 avant son push, ou le job
   reste le juge.

## Témoin

Le « CI Workflow » de master vert sur ses deux jobs (sanity checks, code format) après 1 puis 2 ;
`gh run list --workflow 'CI Workflow'` en garde la trace.

## Pour le fermer

Les deux jobs verts sur master, l'exclusion des docs posée, et la règle du point 3 écrite dans la
page du cœur sur les passes.

## Le 8 novembre 2026 (session hotfixs)

- **Le point 1 est sur master depuis le 25 octobre** : 9bf7aca5c, le même retrait que baf19b9e5
  sous un autre hash. `bash scripts/check-no-std-assert.sh extension` rend 0 sur master.
- **La CI de master ne dit plus rien, ni rouge ni vert** : ses runs récents sont tous annulés avant
  leur premier job (groupe de concurrence ; master reçoit un push toutes les quelques minutes). Le
  témoin « deux jobs verts » demandera un run qui finit : un push calme, ou un lancement à la main
  (`gh workflow run`).
- **Le point 2 se prépare** : clang-format **18.1.3**, celle de la CI, dans une image docker locale
  (`rag3db-clang-format-18`, Ubuntu 24.04) au portable. En attendant la fenêtre, chaque lot de la
  session hotfixs est passé à la 18 sur ses seules lignes, et n'ajoute aucun écart.

## Fenêtre de nuit du12au13novembre2026

Lucie autorise les correctifs utiles du cœur et le découpage des lots nocturnes.
Orchestration ouvre la préparation puis le formatage pur de ce ticket, à Cœur
C++ seul, après inventaire du périmètre et confirmation des sources réellement
concernées par la CI. Les autres lots de nuit sont Rust/Plans/fixtures ou revue ;
aucun autre porteur ne modifie en parallèle le C++ de master.

Utiliser clang-format18 exact (image existante si disponible), protéger annexes,
sous-modules et preuves forensiques ; pas de reformatage à la version23.
La cause cassert est déjà corrigée et ne doit pas être présentée comme nouvelle.
Source et recette publiées, formatage sans logique relu, puis bâti privé et
transaction_test selon bornes avant tour d'intégration. Aucun moteur posé,
ancien runtime bloqué ou job CI relancé. Les sources/private traces C++ de
quarantaine restent séparées ; un refus antérieur ne se contourne pas par ce lot.
La CI automatique devra encore confirmer les jobs avant clôture complète.

## Reprise de nuit du12au13novembre2026 — inventaire avant formatage

Porteur cœur C++, worktree neuf cx-coeur-ci-format18 depuisa783013f1, fenêtre
C++ pure exclusive ouverte par orchestration. cassert reste déjà corrigé ;
check-no-std-assert extension vient de passer sans modification de geo.
Les runs récents observés sont annulés, dernier43313396407 en attente ; dernier
CI Workflow échoué lu40246212509 : sanity checks vert, format rouge sur diff,
shell build rouge séparé. Aucun job relancé et aucun vert global revendiqué.

Image locale pinée d58d28d7…, clang-format18 version18.1.3-1ubuntu1 (paquet
1:18.1.3-1ubuntu1), binaireSHA55cc4a84… ; pas la version23 locale. Inventaire
exact du scriptCI :2094candidats, dont29annexes docs ;2065sources après exclusion
docs et sous-module. Sources privées0511/35ea et preuves forensiques hors lot.
.clang-format-ignore préparé ; seul le formatage des fichiers possédés est lancé
ensuite. Inventaire et faits dans docs/coeur-cpp/12-11-2026/preuves/ci-format18/.
Source/recette et revue avant lourd ; aucun build/runtime ancien contourné,
aucune bibliothèque distribuée modifiée et aucune clôture avant CI future verte.

Source publiée : exclusions25e5d653d, formatage pur253fichiers74ad9c25c.
[Inventaire, faits CI, revue et recette native](../../coeur-cpp/12-11-2026/22-ci-format18-inventaire-source-et-recette.md).
Contrôle réel idempotent/sortie vide et revue lexicale253sansécart sous les limites
exposées ; --dry-run du script imprime seulement les commandes, règle de passe
corrigée dans le knowledge dump. Bâti privé/transaction_test à suivre sous recette
2400s/j8/Max32G/scratch32G, source et lib distribuée séparées, aucun master autorisé
avant revue/tour explicite. Clôture toujours suspendue à CI future effectivement verte.

Premier bâti privé74ad :1099étapes/4cibles réussies, puis EROFS du runner créant
ses répertoires après gelRO ; **0test exécuté**, pas un rougeC++/transaction.
[Arrêt01 et delta de préparation](../../coeur-cpp/12-11-2026/23-ci-format18-arret-preparation-et-delta-runner.md) :
logs/manifeste conservés, poste rendu. Source/flags inchangés ; créer/binder les
seules sorties test privées avant gel, conserver artefacts avant tests. Budget
2400cumulé sansreset :161,855consommées, secondeunité2220+10max. Aucun replay
inchangé/libposée/tracebloquée. Ticket ouvert, pas de vert des tests avant exécution.

Second staging : quatre cibles bâties, mais transaction_test incomplet à sa borne540s ; aucun XML final ni vert revendiqué. [Verdict et preuves](../../coeur-cpp/12-11-2026/24-ci-format18-suite-incomplete-a-la-borne.md). Paquet834fichiers rehashé sans écart, unité arrêtée/MainPID0, cumul893,800s/2400. Aucun replay, lib posée ou publication master. Ticket reste ouvert, revue C7 attendue.
