# Un ajout au dictionnaire refusé change la dernière chaîne existante

- **État** : corrigé sur master `9fa16affff` (18 novembre 2026, cœur c++) : `DictionaryChunk::appendString` réserve ses offsets avant de publier les octets
- **Famille** : moteur
- **Gravité** : valeur fausse après refus d’allocation ; portée transactionnelle à établir
- **Atteignable en service** : à établir ; BufferManager::reserve peut refuser l’agrandissement des offsets sous pool plein
- **Touche rag3weaver** : par le moteur, garantie d’exception des colonnes de chaînes
- **Signalé par** : cœur C++, diagnostic C1 en reprise du16novembre
- **Pour** : cœur C++, coordination C7

## Rouge et invariant

Fils du [crash dictionnaire C1](2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md), qui reste ouvert et non attribué. [Preuve physique et limites](../16-11-2026/03-c1-refus-offsets-rouge-et-limites.md) publiée cd7731186b2bb7044df8441fe7a3e67bf3863f99 avant correctif.

Source témoin seulb82c06f2572892d2124c71dce87eba287f5e9f2b, moteur59ac,1testjoué/1échec/0skip. Réservation48octetsdesoffsets3→6 atteinte et refusée : gamma devient gammadelta ; compteur d’octets14→19 ; trois offsets toujours présents. L’ajout a donc modifié une valeur ancienne malgré son refus. BUILD_ID réellement émis, artefacts/sha/XML conservés deux postes dans ~/.cache/rag3db-moteur-notes/c1-offsets-02 et preuves du dépôt.

Invariant visé : un refus BufferManager pendant les réservations d’un append ne change ni anciennes chaînes, ni compteurs d’entrées/octets, ni identité des chaînes dédupliquées ; le dictionnaire accepte ensuite une nouvelle tentative. Une augmentation interne de capacité seule est permise. Pas de garantie de toutes les exceptions std::unordered_set::insert affirmée sur la seule preuve actuelle.

## Lot et fin

Branche neuve cx-coeur-dictionary-append-atomic dans le worktree possédé cx-coeur-c1-dictionary-diagnostic, issue du rouge publié. Renforcement du test avant correctif : trois anciennes chaînes, hash/déduplication, réessai delta et cardinalités, sans affaiblir l’assertion48octets. Ces contrôles supplémentaires ne sont pas encore joués ; ne pas leur attribuer rétroactivement le rouge enregistré.

Correctif minimal envisagé : réserver offsets et octets avant de modifier le texte ou ses compteurs, à analyser avec les primitives existantes. Ne pas reprendre256 ni modifier StringChunkData, MemoryManager ou test_hooks. Qualification ciblée puis suites pertinentes sous recette fraîche coordonnée, aucune lib posée ni master implicite. Fermer seulement après correctif qualifié et intégration autorisée ; C1 crash ne se ferme pas par ce ticket.

[Correctif et recette proposée](../16-11-2026/04-refus-offsets-correctif-et-recette-proposee.md) : test/hash/réessai séparéac612 puis correctifminimal033ffb92068f31cdae6338db876d23e6bced3567, src/storage/table/dictionary_chunk.cpp seul. Réservation avant memcpy/comptes, aucun256, garantie stdunordered_setinsert non démontrée. Ciblés proposés1+2+2cas/1800neufCPU32G/j8/quiescence, pas encore joués et lanceur exact à publier avant admission. Ticket ouvert.

## Ciblés verts, intégration non faite

La [référence75/59 seule, sans sonde](../16-11-2026/24-reference-chaines-verte-une-fois-sans-attribution-causale.md) passe le cas chaînes1/1 ; le candidat888 avait crashé lors d'une autre exécution. Cette seule paire historique ne prouve aucune responsabilité du correctif append et ne remplace pas la qualification globale rouge ni une intégration autorisée. Ticket reste ouvert.

La [qualification globale888 du16novembre](../16-11-2026/11-qualification-globale-arretee-sur-deux-rouges.md) s'arrête ROUGE sur deux nouveaux rouges de concurrence : offsets1/1 et transactions393passés, mais comparateurknown_red refusé. Chaînes dédiées/E2E non joués, poste rendu, aucune intégration. Le présent ticket reste ouvert ; ces rouges ne sont pas attribués au correctif append.

[Résultat cinq cas et preuves](../16-11-2026/05-refus-offsets-cinq-cibles-vertes.md) : source/moteur033ffb92068f31cdae6338db876d23e6bced3567,1offsets+2finalize+2split,0failure/error/skip/disabled. Assertions48octets/3strings/hash/retry gardées, BUILD_ID033 réellement émis. Unité145,114s success, poste rendu/MainPID0/cgroupvide/aucunrésidu,37fichierspaquet2postes+3ELF/archive/headerrehashés. Ticket reste ouvert avant suites/intégration, parentC1crashnonattribué, aucun stdunordered_setinsertgarantieforte/master/libposée.

## Fermé (18 novembre 2026)

Le témoin et le correctif de Codex (`70ee2719f`, `888c81825`), reposés sur master par le cœur
(branche `dictionary-append-reserves-first`) : `DictionaryAppendException.OffsetAllocationFailureKeepsExistingStringsUnchanged`
rouge sur le témoin seul (`8e9596069`), vert avec le correctif (`814a8c6e4`, BUILD_ID
`rag3db-build-814a8c6e414ecbe91e666bdc62560f6b9dbe11dc`). `transaction_test` entier 393 verts,
known_red vert, `e2e_test` 1 866 verts avec les deux rouges de master admis sous leurs tickets
(`DropNodeTableReclaim`, `CopyFilesWithHomeDir`). Poussé par la voie des suites C++ transférables,
master `9fa16affff`.

Ce correctif ne règle pas le SIGSEGV du dictionnaire au point de reprise (le ticket parent reste
ouvert) : Codex l'a reproduit avec ce correctif dans
`StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops`.
