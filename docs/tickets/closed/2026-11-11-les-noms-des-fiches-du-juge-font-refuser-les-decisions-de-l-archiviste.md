# Les noms des fiches du juge font refuser les décisions de l’archiviste

- **État** : corrigé par `9bf3bb33a`, sur la branche du lot ; livraison sur master en attente du rejeu après rebase et du tour de l’orchestration.
- **Famille** : produit
- **Gravité** : blocage partiel de l’archivage : des sections restent sans apport.
- **Atteignable en service** : oui, si archive rappelle des fiches dont les noms sont des bilans longs.
- **Touche rag3weaver** : oui, le gabarit memory et les étiquettes du Decide.

## Rouge observé

Le premier document figé, `extension/rag3weaver/docs/06-novembre-2026-orchestration/01-plan-du-week-end.md`, contient sept sections. Sans exclusion des fiches `via=juge`, cinq décisions rendent HTTP 400 INVALID_ARGUMENT ; deux passent. Le premier essai dure 44,8 s, ouverture, recherche, décisions et écritures comprises : ce temps ne mesure pas la durée de chaque décision.

Les requêtes refusées portent des valeurs enum de 1 811 à 2 371 caractères. Le corps Vertex dit seulement `Request contains an invalid argument.` : aucun seuil ni champ responsable n’est établi par ce message.

La copie du premier binaire a le SHA256 `ff61a5b97f6b8ec119b4f4f15928828a3e2202ccc2254b84ba1be51ee19c92ad`, mais sa provenance de compilation n’est pas certifiée. Cet essai est un diagnostic provisoire. Le rejeu sous le binaire de provenance vérifiée d1345a115 confirme le même rouge : 2/7, cinq HTTP 400, 46,9 s. Avec les alias courts sur une autre copie neuve : 7/7, zéro refus, 113,5 s. Les noms et descriptions restent dans le prompt. Les maps des deux processus confirment la lib complète épinglée. Ces temps couvrent tout le processus, sans exclusivité ; ils ne mesurent pas chaque section.

## Preuves et reproduction

Racine des essais : `/tmp/claude-1000/-home-lucied-git-workspaces-rag3db/25b7ff4b-dabf-42ef-8580-86661c846fb9/scratchpad/blanc/` (mémoire vive).

- `reprise-11-11/resultats.jsonl` : rapport des sept sections.
- `reprise-11-11/etat/requetes-refusees/` : cinq requêtes et corps complets ; ne pas publier leurs en-têtes.
- `reprise-11-11/lib-chargee.txt` : chemins lus dans les maps du processus.
- `diagnostic-etiquettes-courtes/` : essai avec des alias `candidate_N`, noms et descriptions conservés dans le prompt ; table inverse pour écrire sous le véritable nom de fiche.
- `validation-alias-d1345a115/` : comparaison verte sur sept sections avec les alias courts sous le binaire vérifié.
- `validation-d1345a115/` : copie neuve, gabarit original et binaire fourni par optimiseur, SHA256 `597f6a29de486a9161996e8e4f460731802e090c71f0859a7ad80808ad7da7ee`.

Chaque copie part de l’instantané `~/.local/share/rag3weaver/instantanes/avant-29a7bc52a/memoire-longue/` (base, extensions, fts), avec copie séparée du dossier des fiches. Aucun essai ne modifie la base ni les fichiers de mémoire en service. La lib chargée est `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.

## Pour fermer

Isoler la cause par comparaison, poser un témoin rouge puis vert avec un binaire de provenance vérifiée, et livrer le correctif après la passe. Des identifiants courts doivent se résoudre vers les noms exacts des fiches, sans créer de fiches sous les alias ni écarter silencieusement les fiches du juge. Les cinquante documents attendent ce diagnostic ; la mémoire réelle attend toujours la décision de Lucie.

## Correctif et preuves du lot

Les alias courts sont résolus vers les noms exacts avec refus explicite des identifiants inconnus ou ambigus. Rouge ciblé (un échec attendu), puis cinq témoins verts ; essai réel sur copie neuve et binaire de provenance vérifiée : sept sections sur sept, zéro erreur, zéro refus. Passe historique sur base 133b9013e : features 5/5, lib 2094/0 (58 ignorés dans cette étape), navigateur 33/0/0, e2e_archiviste 5/0/0 et e2e_postgres 34/0/0. Le Rust arrivé sur master impose encore un rejeu court. La [note du lot](../../memoire/11-11-2026/04-alias-candidats.md) garde les journaux et le point de reprise.
