# Livraison propre du refus des offsets : audit et proposition de passe

Préparation validée C7 ; aucun lancement global dans cette page. W `/home/lucied/git_workspaces/cx-coeur-dictionary-append-delivery`, branche **cx-coeur-dictionary-append-delivery-2**, base master frais après pinG68590270e21d00285b29800bd9cdecf0cb921461. Témoin porté70ee2719fe avant correctif888c81825e4cd6556261233f0e238af3b95375c9 ; **SOURCE ET ENGINE888**, pas033 ni59 dans le nouveau verdict à produire. Le premier essai de préparation de branche cx-coeur-dictionary-append-delivery/e0d973287a n’est pas le lot à qualifier et reste conservé ; aucun historique distant réécrit.

## Audit avant gel

Trois fichiers natifs uniquement, modes et OID complets strictement identiques aux trois de source033 qualifiée5/5 ; aucune importation deJSON/log/header/lanceur des branches de preuve. Ces preuves restent sur origin/cx-coeur-dictionary-append-atomic et origin/cx-coeur-c1-dictionary-diagnostic, aucune purge. Les tickets/liens sont MDseuls. Garde cpp_transferable actuelle : accepte MD/src/test/vector/rust_api, et exige aucun changement master sous ses chemins moteur depuis base. Les changements927→685 ne touchent aucun src/test/cmake/CMakeLists/vector, le pinG touche le Rust/outillage. Ne pas modifier la garde ni fabriquer un résumé pour contourner ces règles.

| Chemin | Mode | OID blob identique à033 |
|---|---|---|
| src/storage/table/dictionary_chunk.cpp | 100644 | 9f3b26d3165fb10af405e6b3162f6c2acb2ebf42 |
| test/transaction/CMakeLists.txt | 100644 | 1da36643a14db053bd4a0e98b53bb9faf5e3299c |
| test/transaction/dictionary_append_exception_test.cpp | 100644 | 795c90cd13ab262102afdd8ce537ad6b9e0a263e |

Contrôle clang-format18 réel des deux fichiersCPP modifiés : imageUbuntu24.04/18.1.3-1ubuntu1, digestd58d28d7a16180e13240382444b550ef1a0d186731665d6abccdad641343af45 ; invocation de scripts/run-clang-format.py SANS --dry-run/SANS --in-place, sortie vide/rc0, DockerRO/netnone/mémoire512M/CPU2, timeout60. Aucun contenu source réécrit. Ce contrôle ne vaut pas passe globale.

[Ticket fils](../tickets/2026-11-16-un-ajout-au-dictionnaire-refuse-change-la-derniere-chaine.md) : garantie pour refus BufferManager des réservations seulement ; stdunordered_setinsert/alias/concurrence horspreuve. RevueRO hotfixs favorable à ce périmètre ; pas de vert indépendant nouveau. TicketC1crash reste ouvert. sanityChecktautologique non corrigé, son ticket séparé reste sur branche diagnostique ; ne pas l’embarquer dans le code.

## Préconditions avant admission

Source888 figée propre, bundle local et SHA/lanceur exact à publier avant admission fraîcheC7. BUILD_ID moteur888 lu du header/ELF et réellement émis par un binaire joué ; aucun nom de lib écrit à la main dans un log. Poste lourd réel32G/swap0/j8/scratchdur32G/netnone, données de tests neuves, sourceRO et sorties extensions vector/geo privées durant le bâti puisRO durant les suites. Aucun SDKmagasin/GPU/service/basevivante/7970/cachepartagé. Même profilRelease sans sanitizers/runtimechecks/LTO, testsON/vector;geo, grammarOFF. Gpin/ArchiAlgareth/Recherche sont prioritaires : pas de slot implicite pris.

## Proposition globale complète, à arbitrer

Enveloppe neuve **10800s** (proposition, non réservée), unité10790+stop10/contrôleur10770. Config120 ; build1800 ; transaction2400 ; known_red3600 ; concurrence chaînes600 ; e2e1800. Chaque sous-délai MINsolde moins récolte/proofs, donc les maxima ne sont pas additionnés pour augmenter10800. Premier rouge inattendu/timeout/saut non admis = arrêt et diagnostic sans retry. Artefacts/headers/archives avant tests et vérifications, savehorsscratch et SHA/source/flags/argv/env/temps/codes/XML/quiescence/PIDrécoltés/posterendu. Les répétitions C1×10 sous200Mio ne sont pas incluses : ancienTSan ne remplace pas cette mesure.

Bâtir seulement transaction_test, concurrence_test, e2e_test, dictionary_append_exception_test et les extensions vector/geo correspondantes. Jouer le cas autonome offsets une fois dans cette passe uniquement pour l’identité moteur888 et le nouveau reçu, pas une campagne ciblée5cas distincte. Les deux suitesfinalize/split033 ne sont pas rejouées ad hoc faute de changement blob.

```
<build>/test/transaction/dictionary_append_exception_test --gtest_filter=DictionaryAppendException.OffsetAllocationFailureKeepsExistingStringsUnchanged --gtest_color=no --gtest_output=xml:<out>/offsets.xml
<build>/test/transaction/transaction_test --gtest_color=no --gtest_output=xml:<out>/transaction.xml
CONCURRENCE_LONG=1 ctest --test-dir <build>/test/transaction/concurrence -R '^concurrence_test.known_red$' --output-on-failure
<build>/test/transaction/concurrence/concurrence_test --gtest_filter='StringRewriteAcrossAddColumn.*' --gtest_color=no --gtest_output=xml:<out>/strings.xml
<build>/test/runner/e2e_test --gtest_filter='*' --gtest_color=no --gtest_output=xml:<out>/e2e.xml
```

Chaque suite doit avoir un nombre nonnul effectivement joué ; comparateurknown_red doit conserver le JSON/rawlog natif et expliquer rouges/étiquettes/verts inattendus. Ne pas transformer un « known red turned green » en succès silencieux : conserver puis mesurer le cas seul sous admission fraîche. Les suites utilisées pour le reçu de garde nomment réellement le BUILD_ID du binaire, pas la lib posée59ac.

### Exceptions connues et limites qui exigent arbitrage C7

- transaction_test : lire les sauts nommés ; seuls les outils VectorIndexCrashReopenProbe.OpenInAFreshProcess et WalReader.PrintAFile déjà attestés sont attendus. Tout saut supplémentaire/disabled ajouté = arrêt. F1PkLookup intermittent et tout rouge nouveau restent rouges.
- known_red courant :18rougesdéclarés avec étiquettes exactes,17probabilistes écartés par son comparateur ;29cas longs dans long.txt. Avec CONCURRENCE_LONG=1, longs non probabilistes effectivement exécutés, mais TwentyWriters.DdlEveryMinute reste écarté carprobabiliste : le dire NON JOUÉ et ne pas attribuer de vertC1 à ce comparateur.
- **Prérequis manquant relevéRO** : ~/.cache/rag3weaver-build/joignabilite-export/Scope_Chunk.csv absent sur luciepc. Les deux Paths/RealVectorsReachability.RowsNotFoundByTheirOwnVector/{Masse,Fond} sauteraient « no export ». Aucun export/basedeLucie créé ou ouvert. La variante complète ci-dessus est donc NON ADMISSIBLE comme touteconcurrenceverte sans fixture explicite ou exemption C7 nommée.
- Variante bornée à soumettre si C7 accepte l’exception des longs : known_red SANS CONCURRENCE_LONG, qui annonce explicitement INCOMPLETE:29longcasesnotrun. Conserver la liste29 et les17probabilistes, les nommer NON JOUÉS ; ce n’est pas une passe complète verte sans décision de portée explicite. Ne pas modifier les listes ni retirer deux tests pour passer. Le temps et les commandes exactes de cette variante doivent être publiés avant admission.
- e2e_test : seul rouge déjà admis DropNodeTableReclaim, sous ticket2026-11-14-apres-un-drop-la-table-recreee-ne-reemploie-pas-les-pages-rendues.md. Si un autre rouge/saut apparaît, arrêt. Ne pas retirer le cas du filtre ; conserver l’échec individuel et juger seulement l’exception expressément accordée.
- La garde actuelle cherche une seule identité rag3db-build-40hex dans J/*.log ; certains tests intentionnels ExtensionBuildIdTest impriment une fausse identitéffffffff pour vérifier le refus. Le journal brut doit rester fidèle et non réécrit. La structure des journaux/reçu doit distinguer ce message de fixture d’une bibliothèque réellement jouée, être relue avec C7 avant passe/push ; aucun pseudo-journal ni affaiblissement de garde proposé ici.

## État de cette préparation

Pas de nouvelle passe/ctest/buildglobal lancés ; pas de master ni lib posée autorisés. La source888 et les trois blobs sont prêts pour revue, les preuves033 restent033. Les prérequis export absent, portée des longs et reçu fidèle doivent être tranchés par coordination avant lancement. TSan historique lu séparément, pasFINau dernier relevé, pas d’arrêt/relance.
