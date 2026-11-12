# Préflight privé de saturation : configuration seule, poste rendu

12 novembre 2026. Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. Source attestée :
`6bfba5b4cd081c0bbe10bda045fdcccfb6290594`.
**Aucune compilation moteur/test, aucun scénario exécuté, aucune lib distribuée
modifiée.** Les seules compilations éventuelles sont les sondes de plateforme
ordinaires de CMake configure. Build 1800 s et runtime 1200 s restent non autorisés.

## Résultat et bornes observées

Deux unités courtes, sorties et refus initiaux conservés :

| Unité | Résultat | Durée service | Pic mémoire | Swap |
|---|---|---:|---:|---:|
| cx-coeur-config-saturation-12nov-01 | collecteur arrêté après configure réussi | 4,101 s | 774,9 Mio | 0 |
| cx-coeur-config-saturation-12nov-02 | configure/préflight rc0 | 3,563 s | 543 Mio | 0 |

La première s'arrête en cherchant build_id.h, qui n'est pas produit par configure.
Le collecteur corrigé déclare cet en-tête non généré et n'exécute pas sa cible.
Aucun changement des sources moteur ni compilation pour résoudre ce refus.
La seconde unité repart d'un staging neuf, et non d'un build partiellement réutilisé.

Poste lourd acquis avant démarrage des budgets ; cgroup effectif MemoryMax 8 Gio,
MemorySwapMax 0. Scratch tmpfs dur 2 Gio, réseau sans route, source et référence
RO dans le namespace de configuration. Seuls build/tmp/output privés et les deux
répertoires de sortie extension privés sont RW. Source de staging + configure
occupent 472207360 octets sur scratch à la fin du contrôle. Les preuves exportées
restent hors scratch pour survivre à la fin du namespace.

Unités sans processus restant ; seconde inactive, MainPID=0, Result=success ;
poste libre, **0/4 lourds**, aucune mesure réservée. Voir
[état final](preuves/preflight-saturation/etat-final.txt),
[limites effectives](preuves/preflight-saturation/limites.json),
[journal unité](preuves/preflight-saturation/unite.log).

## Staging et absence de fetch/installation

Un pack Git de 78460 octets contenant seulement les nouveaux objets depuis0511
est transmis par SSH, SHA256
`5a1e0ef373897b5c34dfde234b9c2da1bac759fc1c7520aa2b6143621e12d9de`.
Dans le scratch borné, un clone local partagé sans checkout lit le dépôt déjà
présent ; unpack-objects écrit uniquement dans le clone privé, puis checkout
exact du commit et status propre. Aucun fetch, aucune installation ou mise à
jour de sous-module. Les objets de référence ne sont pas modifiés.

La configuration s'exécute dans un deuxième namespace où ce clone devient RO,
avec les seules exceptions `extension/vector/build` et `extension/geo/build`.
GIT_ALLOW_PROTOCOL est vide pendant configure, réseau isolé ; FetchContent fully
et updates disconnected sont demandés. CMake prévient que ces deux options ne
sont pas utilisées par ce projet : la preuve d'absence de téléchargement repose
aussi sur le namespace et l'absence de fetch/install dans le lanceur, pas sur
ces seules options. AUTO_UPDATE_GRAMMAR=OFF et ccache désactivé sont explicites.

Les caches, compile_commands, build.ninja et les trois listes `ninja -t commands`
sont exportés. **-t commands énumère, il n'exécute pas les commandes.** Le staging
et le build configuré étaient éphémères dans le scratch ; un futur build devra
recréer la même source et configuration privée, puis comparer ses empreintes et
commandes normalisées avant de compiler. Aucun artefact natif n'est à réutiliser.

## Comparaison des commandes et ABI de configuration

[Comparaison normalisée](preuves/preflight-saturation/comparaison-normalisee.json) :

- Référence : 1180 commandes ; candidate : 1181.
- Cœur src : 739 → 740 ; seul nouveau TU :
  `src/common/private_saturation_capture.cpp`.
- Aucune commande existante modifiée après substitution des seules racines
  source/build et retrait des arguments d'objet/dépendances/source.
- Aucun TU retiré. Le loader généré reste présent et sa commande est identique.
- Flags du nouveau TU commun exactement égaux à ceux de common/test_hooks.cpp.
- system_config.h généré byte-identique : log2 page 12, vecteur 11, groupe 17,
  segment 18. Release, moteur statique, tests ON, extensions vector;geo,
  runtime-checks/sanitizers/LTO/backtraces/malloc-BM OFF, multi-thread identiques.

Le rapport brut avait deux faux écarts de représentation conservés dans l'archive :
BUILD_TESTS TRUE/ON (même booléen) ; loader de référence classé sous la source parce
que build/moteur est à l'intérieur de la racine source, contre build candidat
extérieur. La relecture normalise les booléens et classe **d'abord** les fichiers
sous build comme générés. Aucun token de compilation n'est supprimé pour masquer
une différence. Le résultat corrigé n'est pas une nouvelle configuration.

build_id.h candidat reste absent à ce stade : généré par une cible de build non
exécutée. La référence porte un identifiant historique. La future génération doit
porter le commit privé6bf et la même configuration ; elle sera attestée avant
lien/exécution, sans prétendre à une identité binaire avec0511 ou la référence.
Une égalité de configuration/commandes ne prouve pas une ABI binaire testée.

Différences attendues nommées : source privée et crochets du diagnostic, un TU
supplémentaire, futur BuildID privé, désactivation des mises à jour de grammaire,
FetchContent disconnected, ccache désactivé et racines privées. Aucune différence
inattendue de définition/flag des TU communs observée.

## Dépendances : preuve disponible et limite

Tous les fichiers source de compile_commands candidats existaient pendant le
préflight. Les répertoires d'include existent, **sauf un ancien chemin Thrift**
`third_party/thrift/./include` également absent dans la référence, avec la même
commande de compilation. Pas de création d'un faux include ni de retrait de flag.

Audit lecture seule du Ninja de référence : 1210 entrées d'objet VALID,
4145 chemins uniques, zéro dépendance absente ; zéro dépendance sous le chemin
Thrift manquant. La sortie brute est conservée compressée et son SHA256 avant
compression est
`5db89ab016567b0eed1d0630ed5946154f76b38e92048f1ca45b9f68c280b893`.
CMake trouve les outils, Python et GTest système déjà présents ; aucune acquisition.
Le source vendored est celui du checkout attesté. Cela établit la disponibilité
observée du socle de référence, pas une résolution transitive fraîche du candidat :
aucun depfile de compilation candidate, aucune garantie de compilation du nouveau
TU ou des modifications. Le chemin Thrift préexistant et cette limite sont livrés
à la revue, sans classer le préflight en test moteur vert.

## Preuves complètes à relire

[Archive des deux tentatives](preuves/preflight-saturation/cx-config-saturation-12nov-preuves.tar.gz),
1261984 octets, SHA256
`9d69da01aa6d7cd16dfff6b5557904561d543f344ba9e84aaf1ecc46042bea6d`.
Les 33 fichiers du manifeste final de la seconde unité ont été rehashés après
copie locale, sans écart. L'archive garde aussi les rapports bruts qui précèdent
la normalisation, le premier arrêt, les scripts exacts, les argv configure,
cache/compile_commands de référence et candidat, build.ninja, générés disponibles,
commandes Ninja des trois cibles, versions d'outils, limites et état final.
[Reçu d'export](preuves/preflight-saturation/export-sha256.json).

Emplacement distant conservé :
`~/.cache/rag3db-moteur-notes/annulation/cx-config-saturation-12nov-02/` ; première
tentative voisine `...-01/`. Aucun droit de build ou runtime n'est inféré de ce
résultat de préflight. Le ticket reste ouvert ; sûreté0511 et coût restent distincts.
