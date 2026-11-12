# Une scope e2e imbriquée peut échapper au plafond du lanceur

- **État** : ouvert le 12 novembre 2026 ; porteur orchestration, faits ED et mémoire.
- **Pour** : orchestration et propriétaires des drivers de passe ; aucun changement du runner produit dans le lot des accords.
- **Famille** : produit
- **Gravité** : risque de dépassement du budget mémoire et d'arrêt incomplet des enfants.
- **Atteignable en service** : non, lanceurs de validation.
- **Touche rag3weaver** : oui, montage de run_e2e et attestations de cgroup.

## Faits et limite de la preuve ancienne

Dans `extension/rag3weaver/run_e2e.sh`, l'absence de `RAG3WEAVER_VERROU_TENU` relance la suite
sous poste lourd (64–102). `confined` (181–190) crée aussi par défaut une scope systemd
`MemoryHigh=16G`. Une scope ainsi créée ne constitue pas une preuve d'appartenance à l'unité
privée de l'appelant : elle peut être sœur, hors de son MemoryMax et de son KillMode.
MemoryHigh est une limite souple, pas MemoryMax.

ED a lancé sa globale b42 sous unité privée Max32G et KillMode=control-group, vérifiés sur
le parent. Les deux suites e2e sont fonctionnellement vertes, mais leur PID/cgroup réel n'avait
pas été attesté. Le ciblé lectures24 avait une unité enveloppante au pic9,4M, alors que le
runner lançait sa propre scope. Ces mesures du parent ne prouvent pas le plafond des enfants ;
on ne réattribue pas rétrospectivement un cgroup non observé. Mémoire a identifié le même piège
et interrompu préventivement une tentative avant e2e ; ses preuves restent dans son ticket M8.

Preuves ED : `~/.cache/rag3weaver-build/I-luciepc/accord-20261112/`
(`passe.sh`, `cgroup-passe`, `memorymax-passe`, journaux et verdict global b42-r2).
Source produit figée : `b42a4ada90ab7f2b80f1137e75692b3d00e26fcc`.

## Montage correct à composer

Après acquisition réelle de poste lourd, dans une unité privée effectivement Max32G avec
KillMode=control-group et budget après acquisition :

- poser `RAG3WEAVER_VERROU_TENU` pour ne pas reprendre un second verrou ; jamais avant un vrai verrou ;
- poser `RAG3WEAVER_BUILD_MEMORY_HIGH=0` pour éviter la scope intérieure quand le plafond extérieur
  est déjà garanti ; cela ne retire pas le MemoryMax extérieur ;
- relever le **PID du vrai binaire e2e**, son `/proc/PID/cgroup`, le memory.max de ce cgroup,
  son identité de suite et le KillMode de l'unité ; échouer si aucun vrai enfant n'est observé ;
- préserver les anciens passages, puis refaire seulement les étapes dont le confinement restait
  non prouvé, avec verdict composé explicite et source inchangée.

ED prépare cette vérification bornée pour lectures24 et démon13, autorisée par orchestration,
sans rejouer features/lib/nav déjà validés. Aucune désactivation de bac à sable, hausse du plafond,
nouveau service ou test de charge n'est nécessaire.

## Pour fermer

Consigner les preuves enfants ED/M8, prévenir les autres drivers et intégrer la recette dans
la procédure commune de passe. Vérifier que budget, arrêt et comptage des lourds couvrent les
vrais descendants. Toute évolution du runner lui-même est un lot séparé, relisible et testé ;
ce ticket n'en annonce aucune comme livrée.

## Preuve enfant ED acquise

Sur b42 inchangé, après autorisation et créneau7879 : lectures24/0/0 et démon13/0/0,
deux suites sur deux sans saut, unité600s après acquisition. PID2697195 et2702388
réellement observés dans cx-ed-accord-e2e-confines-20261112.service, avec
memory.max34359738368, RuntimeMaxUSec10min et KillModecontrol-group.
`I-luciepc/accord-20261112/{e2e_lectures,e2e_demon_hote}-enfant.json`,
`verdict-compose.json` et `quiescence-e2e-confines.json` ([]). Le verdict composé garde
features/lib/nav précédents et remplace seulement la preuve de confinement des e2e.
M8, prévention des autres drivers et procédure commune restent à orchestration : ticket ouvert.
