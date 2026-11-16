# Oracle final : préparation hors ligne du 16 novembre

Ticket : `docs/banc/tickets/2026-11-16-oracle-des-copies-finales-des-trois-bases.md`.
Aucun calcul joué par cette préparation. Attendre FINI G puis le créneau Arbre.
G seul possède les services. Le banc ne touche ni aux bases vivantes ni au produit.

## Entrées nécessaires par base

Arbre fournit les chemins des copies finales A, référence avant B et index neuf B′,
leurs manifestes et la source figée utilisée après REGLES7. Cette source doit
inclure les fichiers non suivis et les modifications locales : HEAD seul ne suffit
pas. Conserver l'inventaire des chemins inclus avec empreintes, les versions des
sous-modules, les exclusions, le manifeste et les règles effectives. Exclure les
secrets de toute copie ; ne pas copier aveuglément l'arbre de Lucie.

Avant ouverture, fabriquer une copie reflink neuve de chaque artefact de référence
vers un dossier neuf ; adapter uniquement les déclarations/gabarits copiés. Tous
leurs chemins de base doivent viser les copies privées, jamais les originaux de
service. Les chemins exacts restent à fournir : aucune déclaration n'est dite
prête à exécuter tant que cette vérification ne peut être faite.

## Provenance et outils disponibles

Source de la sonde historique et du jeu :
`3d422fad1145597b4b4bf3f2425c115b14ce3d23`, branche banc-parite-search-code.
Copies locales immuables préparées dans
`~/.cache/rag3db-banc-notes/oracle-final-16nov/tools/` :

- sonde_parite_search_code.rs SHA256
  `0bb347ec7db4ccd3884412e1ed827e85b94e9f62f39ce8e361b0ee64d22781b5` ;
- jeu-recherche-2a2.json SHA256
  `3d2fc0faa954bf41829b8abefaa8634f2906c39bb5323802c5693cbf658fa14d` ;
- parite_search_code.py SHA256
  `1f3be75a92b907e2458075d2f01d82f8a3d7f2ff9cb1bd8d140d01e4d0dae0de`.

À luciepc, outils existants relus par empreinte : vec-cmp.py
`59c0ad22e0457edfa37776652313e6544477ecfb9516d4d2042fa92ad5e180eb`,
et compaction/j_relevance.py
`66823e862434f574db9861668c8651f5cba8732700a3bc012c3abfd1685aac9a`.
Ils sont disponibles, pas nouvellement qualifiés sur les copies finales.
Le worktree rag3db-banc-vue est encore fe5d81f811 : il ne prouve pas le binaire927.
Ne pas exécuter les anciens lanceurs : ancienne lib373 épinglée, chemins fixes,
checkout forcé et suppressions ; leur logique est une référence de lecture.

## Recette après disponibilité

1. Prouver le lecteur : SHA exact des sources et binaires, BUILD_ID réellement lu,
   lib59ac complète annoncée par G, lucivy4.3.9 effective, sous-modules présents.
   Adapter la sonde seulement dans un worktree privé sur le code courant ; ne pas
   fusionner la branche historique entière. Compilation à qualifier au créneau.
2. Prouver chaque copie : forme, génération/rang visibles, manifeste et règles,
   modèle dense/creux configuré, colonnes/dimensions et vecteurs réellement écrits,
   dette0. Une absence de modèle ou un nombre de vecteurs ne suffit pas à prouver
   le modèle effectivement utilisé : vérifier journaux d'embarquement et identité
   du service après FINI G. Ne pas embarquer silencieusement pendant un relevé.
3. Sur A et B′, jouer séparément bm25, vector, puis tous signaux, même jeu de30cas,
   même k et mêmes filtres. `RAG3WEAVER_PARITE_NO_MIGRATION=1` ; exiger bm25 ou
   vector dans chaque régime, les deux en combiné via
   `RAG3WEAVER_PARITE_REQUIRE_SIGNALS`. Conserver les contributions par résultat.
   Avant/après B contre A est une évolution de réponses si le découpage change,
   pas une exigence d'identité de toutes les réponses.
4. Comparer identités de réponses/contenus, classement, rappel et premier rang.
   Le comparateur historique `reponses` est strict au bit : conserver son rouge,
   sans le transformer en vert par seuil improvisé. Mesurer séparément les
   vecteurs appariés par contenu : cosinus >0,99999 et maxabs <1e-4, bornes fondées
   sur les anciennes observations et à vérifier à nouveau, sans couvrir les
   contenus manquants ni les changements de rang. Pour le BM25 brut, la borne
   relative5e-3 concerne le départage lucivy sur mêmes contenus/ids visibles ;
   elle ne s'applique pas au score RRF de search_code, même en régime bm25.
5. Pertinence : union des réponses A/B′, bonnes réponses sourcées dans la source
   figée, étiquettes aveugles séparées de la parité. Conserver le juge et sa
   provenance ; pas de verdict sémantique nouveau sans jugement réellement joué.
   Pas de reprise N/F cloud implicite. Nommer séparément pertes nouvelles,
   contenus non comparables, ex æquo et bruit numérique.

## Créneau et limites

Prévoir après dette0 environ 10–20 min pour provenance et relevés si le lecteur
est déjà bâti ; un index neuf complet coûtait ~13 min au dernier banc, réserve
prudente20–30 min si B′ n'est pas fourni. Ces durées sont des estimations, pas
un délai autorisé ni une promesse. Qualification du lecteur et pertinence à
ajouter selon cache/volume ; annoncer avant acquisition. Une portée commune
32G avec preuves neuves et un budget global ; aucun verrou extérieur doublé,
aucun ancien GO réactivé. Les copies de parité sont conservées.
