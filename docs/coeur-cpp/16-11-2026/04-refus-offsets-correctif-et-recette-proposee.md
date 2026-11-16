# Refus des offsets : correctif minimal publié, qualification proposée

Ticket fils [garantie d’exception du dictionnaire](../tickets/2026-11-16-un-ajout-au-dictionnaire-refuse-change-la-derniere-chaine.md). ParentC1crash reste ouvert et causalité non établie. Branche neuve cx-coeur-dictionary-append-atomic dans Wcx-coeur-c1-dictionary-diagnostic, issue du rouge publiécd7731186b/source nativeb82. Pas256 et aucun autre ancienWIP importé.

## Commits séparés avant qualification

- ac6124702002565edc885c66436db6cc91ade03d : ticket fils et témoin renforcé sans correctif. Les mêmes préconditions48octets/3offsets/3chaînes subsistent ; ASSERT de validité avant toute sonde du hash ; déduplication des trois anciennes chaînes, ajout delta réessayé et dédupliqué, quatre offsets et19octets finaux. Contrôles supplémentaires NON JOUÉS ; le rouge réellement vu surb82 demeure la preuve initiale, pas un résultat inventé surac612.
- 033ffb92068f31cdae6338db876d23e6bced3567 : seul src/storage/table/dictionary_chunk.cpp. Réservation des octets existante puis réservation des offsets déplacée avant memcpy/setNumValues. Aucun compte ni contenu ancien publié si une de ces réservations échoue ; une capacité interne accrue est permise. Les entrées du hash désignent des indices, pas des pointeurs vers l’ancien tampon : sa réallocation ne change pas les valeurs recopiées. Pas de rollback supplémentaire nécessaire pour les refus BufferManager des deux réservations avant mutation.

La garantie forte face à std::bad_alloc dans unordered_set::insert n’est pas démontrée ni incluse par cette permutation. Alias éventuel du string_view d’entrée et réallocation, concurrence et vie des dictionnaires pendant C1 : non qualifiés ici. Aucun code de StringChunkData, MemoryManager, test_hooks ou sanityCheck changé. Ce dernier a son [ticket propre](../tickets/2026-11-16-le-controle-du-dictionnaire-compare-un-compteur-a-lui-meme.md).

## Recette CPU proposée à C7, non exécutée

Proposition NEUVE1800s après acquisition du poste réel, distincte des deux unités historiques144,848+155,730s. Source et moteur033ffb92068f31cdae6338db876d23e6bced3567 ; le BUILD_ID restera lu de l’en-tête généré, présent dans l’ELF et émis par le témoin, jamais déduit du seul nom de branche. Sourcefigée propre en staging privé, bundlelocal et SHA publiés avant entrée ; aucune lib du magasin ou cache partagé.

Même profilC++ autonome déjà éprouvé : Release, BUILD_TESTSON, shellOFF/sharedOFF, runtimechecks/ASAN/TSAN/UBSAN/LTO/backtraceOFF, grammarupdateOFF, dependenciesdisconnected, ccacheOFF. Aucun service/GPU/PG/base réelle/7970. Compiler/linker et options effectifs gardés. SourceRO après staging, build/tmpRW dans scratch dur32G, cgroup32G/swap0, netnamespace sans route ; preuve des limites refusée si absente.

Unité1790+10stop=1800, contrôle1770. Configuration120s ; build1500s borné au solde−120 ; tests un appel chacun30s (total≤90, solde contrôleur prioritaire). Construire seulement les cibles dictionary_append_exception_test, string_finalize_test, string_split_test, j8, une seule invocation cmake--build ; zéro autre cible applicative/extension. Dépendances moteur nécessaires autorisées dans ce même bâti.

Commandes test, chacune unique, env vidé PATH/LANG/LC_ALL/TZ/TMPDIR privés :

```
<build>/test/transaction/dictionary_append_exception_test --gtest_filter=DictionaryAppendException.OffsetAllocationFailureKeepsExistingStringsUnchanged --gtest_color=no --gtest_output=xml:<out>/offsets.xml
<build>/test/storage/string_finalize_test --gtest_filter=StringFinalizeTests.* --gtest_color=no --gtest_output=xml:<out>/finalize.xml
<build>/test/storage/string_split_test --gtest_filter=StringSplitTest.* --gtest_color=no --gtest_output=xml:<out>/split.xml
```

Attendus stricts1+2+2cas, aucun failure/error/skip/disabled/filtre vide. Premier nouveau rouge ou timeout arrête sans replay ni hausse ; aucun vert globalC1 même si5casverts. Avant toute assertion post-build, conserver horsscratch header/3ELF/archive+SHA/manifeste source/engine/flags. Journaux argv/env/temps/codes et XML complets ; après sortie TERM/KILL3+3s si nécessaire inclus dans les10s, PID récoltés, MainPID0/cgroupvide/absence descendants attribués, poste rendu et espace final32G. Garder tout rouge et paquet deux postes avant quelconque correction.

Le lanceur exact adapté à ces trois cibles/commandes et ses empreintes doit être publié et reçu avant qualification, après admission fraîcheC7 et capacité/mesure exclusive vérifiées. Pas de lancement effectué par ce document. Après ciblés verts, suites moteur transférables transaction_test/known_redlongsexplicités/e2e_* et cas pertinents dans une recette séparée à proposer ; pas de master ni lib posée implicites.

## TSan indépendant

Dernier relevé : tours1–4finis code66/15,18,16,19rapports,0dictionnaire ; tour5rapport17warnings/0dictionnaire encorepartiel, pasFIN. Aucune relance/arrêt ; ce résultat ne prouve ni ce correctif ni son lien au crashC1. G/TSan demeurent prioritaires.
