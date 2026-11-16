# Passe C++ bornée : portée arbitrée et flux fidèle avant admission

C7 arbitre la portée du ticket fils offsets : globale **bornée** du moteur, sans CONCURRENCE_LONG ; pas de prétention concurrence complète ni crashC1 corrigé. Source et moteur à qualifier888c81825e4cd6556261233f0e238af3b95375c9, base68590270e21d00285b29800bd9cdecf0cb921461, branche proprecx-coeur-dictionary-append-delivery-2. Aucun nouveau runtime lancé pour publier cette page.

## Portée exacte, listes inchangées

[Liste calculée depuis les fichiers suivis](preuves/global-receipt/portee.json) :29longs,17probabilistes, intersectionexacte{TwentyWriters.DdlEveryMinute}, union**45casdistinctsNONJOUÉS**. Ces noms restent dans les listes ; aucune modification du comparateur ou des filtres source. Les deux RealVectorsReachability/Masse,Fond dont l’export manque sont dans leslongs ; aucune création/lecture d’export de Lucie. CONCURRENCE_LONG explicitement absent dans l’environnement vidé, connu par la recette.

Le comparateurknown_red sansLONG doit afficher INCOMPLETE et29longsnonjoués. Cela n’est pas renommé « concurrence complète ». Les17probabilistes ne deviennent pas des verts par leur exclusion. Une conclusion VERTdanslaportéearbitrée peut être écrite seulement si toutes les obligations de cette portée réussissent et exceptionsconformes, avec cet inventaire45toujoursvisible au résumé. Le parentC1crash n’est pas couvert par ces45exclusions ni par le TSan ancien.

Transaction : deux outils sautés nominaux seulement VectorIndexCrashReopenProbe.OpenInAFreshProcess et WalReader.PrintAFile ; cas désactivé historique ReadOnlyOpenTest.DISABLED_MeasureCostOfRevalidationOnABigJournal nommé séparément. Nouveau saut/disabled/F1rouge/autrerouge = arrêt, pas de blanket.

E2E : seul transaction~ddl~ddl_empty.DropNodeTableReclaim peut présenter l’ancienne assertion d’égalité resultTuples[i]="False" contre testAnswer.expectedResult[i]="True", tuple0, pour query `call file_info() with num_pages match (s:stats) where s.id = 0 return num_pages <= s.numPages`. Même nom mais poolplein, exception, assertion différente, échec additionnel ou query absente : refus. Lecture du brut historique luciepc dict-fix2/e2e.log confirme cette assertion/query ; ce n’est pas un nouveau test. Cas réellement exécuté et rouge conservé, jamais supprimé du filtre.

## Flux proposé avant entrée, pas de déplacement après refus

Arborescence créée AVANT première commande, sous journaux privés dédiés et horsscratch :

```
J/
  base                   # 685… réel, pas de lib posée comme substitut
  resume.log             # commit testé… / portée / codes / chemins+SHA / verdict final
  attestation.log        # STDOUT natif offsets, octet pour octet identique à son brut
  raw/
    configure/stdout.log
    build/stdout.log
    offsets/stdout.log, result.xml, argv.json, exit.json
    transaction/stdout.log, result.xml, argv.json, exit.json
    known-red/controller.log, concurrence_test_result.json, concurrence_test_result.json.log
    strings/stdout.log, result.xml, argv.json, exit.json
    e2e/stdout.log, result.xml, argv.json, exit.json
    artifacts/            # header/ELF/archives/flags/SHA avant tout contrôle postbuild
    parsing/receipt.json  # y compris fausses identités de fixture et lignes brutes référencées
```

Le lanceur du cas offsets écrit son stdout natif simultanément à raw/offsets/stdout.log et attestation.log, sans préfixer/réécrire le texte ; leur égalitéSHA/bytes est contrôlée. Cette sortie doit contenir exactement une ligne BUILD_ID générée par le vrai binaire moteur888. Pas de BUILD_ID manuscrit au résumé, ni de copie d’un ancien log033/59. Le résumé racine référence les bruts par chemin+taille+SHA et décrit la portée, sans les normaliser ni supprimer leurs erreurs. Les raw restent à leur emplacement annoncé depuis le départ, même si refus de garde ; rien n’est déplacé après une erreur pour masquer une identité.

Le rawtransaction conserve le message exact de ExtensionBuildIdTest.AtRecoveryAnExtensionFromAnotherBuildLeavesTheIndexBehind, qui altère volontairement8caractères de l’identité d’une copieprivée d’extension (f→e, autre→f) pour vérifier son refus. Le parseur relève chaque identité étrangère et admet uniquement cette mutationexacte sur la ligne du refus « could not be loaded at open / was built from » dans ce cas actif, avec l’identité réelle présente sur la même ligne. Il publie numérodeligne/texte/identité dans raw/parsing/receipt.json ; le résumé pointe cette preuve. Toute identité étrangère ailleurs ou sans ce contexte = arrêt. Aucun mélange de bibliothèques réellement liées n’est accepté sous prétexte de fixture.

La garde de push lit J/*.log : elle reçoit le stdout natif exact du cas offsets et le résumé du superviseur, avec références à tous les bruts conservés en sous-dossiers. Cette structure est soumise et fixée avant admission, pas inventée après un refus. Aucun changement de push_garde ni pseudo-journal, aucune réduction des bruts. L’identité de chaque binaire/header/extension sera en plus vérifiée par artefacts et manifeste indépendants avant suite.

## Petit témoin CPU du parseur

[Parseur proposé](outils/global-pass/parse_receipt.py) et [tests](outils/global-pass/test_parse_receipt.py) conservés sur branche de diagnostic, hors livraison propre. Commande jouée sans build/service :

```
PYTHONDONTWRITEBYTECODE=1 python3 docs/coeur-cpp/16-11-2026/outils/global-pass/test_parse_receipt.py
```

**9/9verts**, preuve [CPU/SHA](preuves/global-receipt/parse-cpu.json). Cas : attestationexacte/altération refusée ; identité native manquante/mélangée refusée ; fausse identité fixture admise mais brut inchangé ; identité étrangère horsfixture refusée ; union avec recoupement comptée une fois ; exceptionDROP précise et erreurs nouvelles refusées ; saut nonnominal etF1refusés ; suitevide refusée. Il s’agit de fixturesCPU, pas d’une passe moteur ni d’une preuve de code888 exécuté. Le parseur de résultats connu-red et l’intégration du superviseur restent à publier/revoir avec le lanceur exact avant admission ; aucun trou de validation n’est annoncé comme terminé.

## Temps et sortie proposés

Cap10800maxdistinctproposé accepté comme plafond par C7, mais aucun run avant lanceur exact/SHA et admission. Sous-mode sanslongs borné : config120/build1800/transaction2400/known_red1800/strings600/e2e1800, chaque délaiMINsoldecontrôleur−arrêt/proofs ; unité10790+stop10/contrôleur10770, j8, mémoire32G/swap0/scratchdur32G/netnone. Aucun modèle/GPU/basevivante/libmagasin/cachepartagé/service/7970. Capacités et priorité Gpin/Archi/Recherche/TSan contrôlées fraîchement avant entrée. Pas de relance automatique, premier rouge hors exceptions = arrêt ; erreur d’exceptiondifférente = rouge et préservation.

Le runner doit garder exit+timeout+PIDrécolté et XML/JSON de chaque suite ; snapshots d’artefacts horsscratch avantassert, quiescence MainPID0/cgroupvide/descendants attribués absents/FDsortie libérés, poste rendu, empreintes deuxpostes avant conclusion. Aucune fermetureC1parent sur cette passe ; ticketfils clos seulement après intégration autorisée et qualifications pertinentes. Aucun nouveau « à toi » ni lib posée implicite.
