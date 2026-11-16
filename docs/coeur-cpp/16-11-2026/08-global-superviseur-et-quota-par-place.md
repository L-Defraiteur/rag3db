# Superviseur global borné et quota par place — publié pour admission exacte

Suite du [flux arbitrée](07-global-borne-flux-fidele-et-parser.md), ticket fils [garantie BufferManager](../tickets/2026-11-16-un-ajout-au-dictionnaire-refuse-change-la-derniere-chaine.md). Scripts/preuves restent sur branchecx-coeur-dictionary-append-atomic ; la branche de livraison propre cx-coeur-dictionary-append-delivery-2 ne reçoit aucun fichier nonMD de ce superviseur.

**État : préparation publiée, aucun lancement natif global.** SourceETengine888c81825e4cd6556261233f0e238af3b95375c9, base68590270e21d00285b29800bd9cdecf0cb921461. Bundle319224octets SHA3fe2354c6acc7eb66ad3def9bbe932b45dd2433e02f0ea825a72e0ed1f817db3, référence localebranchelivraison, checkout exact888 avant config. Les trois fichiers natifs sont épinglésSHA dans [recipe.json](outils/global-pass/recipe.json), identiques à033 qualifié5ciblés ; aucun résultat de ce dernier étiqueté888.

## Entrée et quota effectifs

[launch.sh](outils/global-pass/launch.sh) appelle le vrai poste lourd avec POSTE_MEM_MAX32G/POSTE_SWAP_MAX0, sans overrideCPU/place/cœurslibres. [entry.py](outils/global-pass/entry.py), démarré seulement après acquisition, lit POSTE_FILS et le cpu.max effectif de sa portée ; quotaabsent/max/invalide ou mémoire non32G/swapnon0 = refus. Il applique **le même quotaCPU** à la portée native fille, sans800%fixe. Jobs=min(8,POSTE_FILS,floorquotaeffectif), au moins1, enregistré dans admission/limits et argvbuild.

Le poste courant calcule utiles/places en division entière puis CPUQuota=100×POSTE_FILS ; la règle n’est donc pas codée à550% ou500% : on reprend sa valeur effectivement admise, y compris si le poste change. La portée fille systemd dispose de son MemoryMax32G/swap0/CPUQuotarepris, KillModecontrol-group et borne runtime ; tous ses descendants forkés restent dans cette portée, pas de nouvelle portée de test nonbornée.

Le délai initial10800 commence à l’entrée après verrou. entry déduit le temps déjà écoulé pour RuntimeMax≤10790 et conserve10s d’arrêt ; deadline absolue communecontroller=entrée+10770. Le contrôleur [controller.py](outils/global-pass/controller.py) plafonne toutes les sous-phases au temps restant moinsrécolte ; pas de timerréinitialisé par suite. L’attente du poste ne consomme pas la fenêtre de qualification et ne dispense pas du contrôle de capacité/priorités/mesureexclusive juste avant.

Commande proposée exacte après staging neuf vérifié, **à ne pas exécuter sans admission exacte de C7** :

```
bash /home/lucied/.cache/rag3weaver-build/cx-dictionary-global-01/launch.sh
```

Les scripts/recipe/bundle doivent être copiés dans J=~/.cache/rag3weaver-build/cx-dictionary-global-01 inédit, point scratchvide ~/.cache/rag3weaver-build/cx-coeur-dictionary-global préparé ; aucun de ces gestes distants n’a été effectué pour cette page. Les quatre SHA des scripts sont dans recipe.json ; entry les vérifie avant lancement natif. Aucun besoin de nouveau mot Lucie pour le diagnostic autorisé, mais ce plafond préparé n’est pas une admission exacte automatique.

## Intégration du flux et du parseur

La structure raw/<suite> existe avant première commande. Staging clone/fetch/checkout gardent chacun leurs argv/logs/codes distincts. Configure/build sourceRO avec seules sorties extensionprivées autoriséesRW ; tests source/build/extensionsRO et tmp/home/preuvesprivésRW. known_red via ctest requiert buildRW pour discovery/résultats, source/extensions restentRO ; aucun fichier binaire modifié attendu, SHA revérifiés ensuite.

Targets construits : transaction_test, concurrence_test, e2e_test, dictionary_append_exception_test, extensionsvector/geo du même moteur. Pool/testMAX_NUM_THREADS2 pour environnement général, quotasCPU par portée et OMP/OPENBLAS bornésjobs ; scénarios internes gardent leurs propres nombres de fils. HOME/TMPDIR/RUNNER_TEMP privés, pas de service/GPU/PG/basevivante/7970/libmagasin/ccache/réseau.

Artefacts header/4ELF/archive/vector/geo/cache/compile_commands et manifestes sont exportés horsscratch **avant toute assertion postbuild**. Source/engine888 et marque header/ELF cohérentes ; le cas offsets doit ensuite émettre cette marque réellement. Son stdout est tee simultané sansaltération vers raw/offsets/stdout.log et racineattestation.log ; égalitéoctets/SHA vérifiée au contrôleur et après quiescence. Les autres logs restentraw dès leur ouverture ; aucun déplacement après échec pour masquer des identités. Les hashes/références définitifs sont écrits au résumé seulement après fermeture de l’unité et étatfinal, évitant une empreinte d’un log driver encore ouvert.

L’intégration joue dans l’ordre : offsets1, transactionentière, known_redSANS_LONG en ctestverbeux, StringRewriteAcrossAddColumn3, e2e_testfiltre*. Chaque code/cap/argv/env/PIDrécolté conservé. known_red doit retourner0, garder JSON/log natif, afficher benchmatches et INCOMPLETE29 ; JSON doit contenir des cas réellementCOMPLETED et aucun de l’union45exclue. Les autres parseurs refusent suitevide/sautsnonadmis/disablednouveau/F1nouveau, exigent les comptes1et3pourciblés, et n’acceptent DROP que dans le cas/tuple/assertion/query historiques. Les avertissements de fixture d’identitérestentdansraw avec ligne/texte/identité dérivée8octets ; tout autremélange = arrêt.

Le résultat déclare explicitement45casdistinctsNONJOUÉS (29long+17probabilistes−DdlMinutecommun). C1crash demeureouvert, concurrencecomplète nonprétendue. Les deuxhelpersoutils et ledisabledhistorique sont distingués des verts. Tout nouveau rouge/timeout hors exceptions arrête la passe, sans reprise automatique, puis récolteTERM/KILL3+3s si nécessaire et sauvegarde. e2e peut rendre1 seulement si le parseur prouve exactement l’exceptionDROP autorisée ; aucun blanc-seing par nom.

Après unité : systemctlMainPID0/cgroupvide et aucun contrôleur/testattribué restant, attestation et artefacts rehashés, inventairebrutsfinal, poste rendu avant rapport. Le résumé ne finitVERTque si suitesacceptéesdanslaportéearbitrée **et** quiescence/artefactsacceptés ; sinonROUGE. Pas de fermeture du ticket fils avant intégration et pas de pushmaster sans « à toi » neuf.

## Vérifications CPU faites, limites

10testsCPUduparseur/quota verts, ast.parse des scripts et bash-n du lanceur verts ; [preuve](preuves/global-receipt/parse-cpu-10.json). Ils ne simulent pas une passeC++ réelle et ne prouvent pas le futur runtime.

Contrôle **récursif des quatre racinesCI** par imageclang18épinglée, -j2/CPU2/mémoire1G/RO/netnone/timeout180, script sansdry-run ni in-place : code0, sortie0octet. [Reçuformat](preuves/global-receipt/format18.json) contient commande exacte/image/sourceSHA. Source inchangée, aucune autocorrection. Pas d’autreclanglocal23 utilisé.

Les journaux des unités précédentes rouges/vertes sont préservés sur branches de diagnostic ; aucun cleanup ni nouvelle lib posée. TSan historique demeure une preuve séparée, pas une attestation de888 ni du correctif.

## Justification du seul disabled historique, avant GO

Nom exact : **ReadOnlyOpenTest.DISABLED_MeasureCostOfRevalidationOnABigJournal**, read_only_open_test.cpp355. Mesure de coût de revalidation du grosWAL, pas un oracle de correction ; commentairehistorique50s/paliers16Mo/troispasseschecksum, activationexpliciteseulement --gtest_also_run_disabled_tests --gtest_filter=*MeasureCost*. Aucune telle activation ni nouvelle mesure dans cette recette.

[Baseline relue et empreinte](preuves/global-receipt/disabled-baseline.json) : XML transaction source74ad,360entrées/disabled1, caseexactstatusnotrun/resultsuppressed. Le fichier du test est identique entre74ad et888, blobOID94f83c6f84881b16bb1275db2319d7fc475e79ea ; aucun disabled ajouté par le lot. Il reste nommé NONJOUÉ, distinct des deuxhelperssautés, sans greenindividuel. Aucun autre nomnotrun/saut admis. Scripts/SHAf319inchangés pendant la revue hotfixs.

Consigne fraîcheC7 : pas de lancement global pendant fenêtre froide7970/CLIArbre ; aucune quatrième place prise avant acquisitionArbre et capacitéadmissible. Le cap10800 reste proposition préparée, pas de reçu réactivé ni nouveau service. Aucun staging distant global effectué à ce relevé.
