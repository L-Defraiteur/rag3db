# Le cas de chaînes isolé se termine sur SIGSEGV sans résultat

- **État** : ouvert (16 novembre2026), signal natif confirmé, cause inconnue
- **Famille** : moteur
- **Gravité** : arrêt du processus natif, aucune garantie de données jugée
- **Atteignable en service** : à établir, témoin privé isolé sans services
- **Touche rag3weaver** : indirectement, crash moteur potentiel dans un parcours de chaînes
- **Signalé par** : cœur C++, paire diagnostique50e2
- **Pour** : cœur C++, coordination C7, hotfixs

## Observation et preuve

[Bilan, core conservé et reçus](../16-11-2026/15-paire-diagnostique-arretee-sur-sigsegv-et-sonde-non-activee.md). Source ed0c3c8af57f25e83ae5c4ba004ae3393e77812e, header/ELF moteur888, Release sans sanitizers. StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops seul, wrapper139 sans timeout ; coredump journal natif PID4141127 signal11/SEGV_MAPERR, #0std::_Hash_bytes et #1ELF+0x1023a43. Pas de XML complet ni BUILD_ID stdout. Artefacts privés et core système existant conservés, aucun GDB ni nouvelle exécution.

Le [drapeau de sonde perdu](2026-11-16-le-confinement-efface-le-drapeau-de-la-sonde-de-chaines.md) empêche d'attribuer une absence de log à une étape précise. Aucun défaut de code de la sonde, corruption, OOM ou causalité append établi. Le sommet _Hash_bytes ressemble au [parent C1](2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md), sans suffire à identifier la même pile ou cause. Le précédent contrôle wrongRows228 n'est pas localisé par ce signal.

## Suite et fin

[Référence75/59 seule](../16-11-2026/24-reference-chaines-verte-une-fois-sans-attribution-causale.md) admise900s, même cas/sources test/profil32G/quota5 et drapeau absent effectif :1/1passé,922ms, aucun retry/candidat supplémentaire. Un vert unique ne prouve ni régression du delta append ni cause commune C1 ; aucun nouveau core du candidat acquis. Poste rendu225,250341s, ticket reste ouvert.

Analyse du core existant admise puis terminée : [pile et limites](../16-11-2026/21-core-lu-pile-du-checkpoint-dictionnaire-conservee.md). Thread crashé : Hashtable find → DictionaryChunk::appendString → StringChunkData::write → Column::checkpointSegment → NodeGroup::checkpointInMemAndOnDisk → Checkpointer::writeCheckpoint. Autre thread dans lecture DictionaryColumn/StringColumn. Pas de finalize dans les12frames obtenus, pas de variables DWARF ni cause démontrée. Aucun inférieur lancé, poste rendu en5,511915s. Ticket reste ouvert ; ni répétition du témoin ni attribution C1 acquise.

La [tentative de lecture du core120s](../16-11-2026/19-lecture-du-core-refusee-avant-gdb.md) est arrêtée avant GDB : option zstd refusée, aucun backtrace nouveau. Le présent ticket reste ouvert sans attribution supplémentaire.

Relire les sources et métadonnées déjà acquises ; si insuffisant, proposer une recette bornée de lecture du core avec ELF exact, sans replay du test. Tout outil natif d'analyse ou nouveau runtime requiert admission distincte. Préserver l'absence de comparaison référence/candidat, les trois autres cas non joués et les limites mémoire/maps non observées. Fin : cause démontrée, rouge pertinent avant correction, qualification et intégration autorisées ; pas de fermeture par simple similarité de pile ou correction du pilote.
